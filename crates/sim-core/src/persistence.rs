//! Versioned snapshots, checksums, atomic replacement and deterministic replay.
use crate::{Content, World};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

#[derive(Serialize, Deserialize)]
struct Save {
    schema: u32,
    checksum: u64,
    world: World,
}

const MAX_SAVE_BYTES: usize = 32 * 1024 * 1024;
pub fn encode(world: &World, content: &Content) -> Result<Vec<u8>, String> {
    world.validate(content)?;
    let bytes = serde_json::to_vec(&Save {
        schema: 2,
        checksum: world.hash(),
        world: world.clone(),
    })
    .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_SAVE_BYTES {
        return Err("Campaign exceeds the 32 MiB snapshot limit".into());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8], content: &Content) -> Result<World, String> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Save exceeds the 32 MiB limit".into());
    }
    let header: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("Damaged save: {e}"))?;
    if header.get("schema").and_then(|v| v.as_u64()) != Some(2) {
        return Err("Unsupported save version: rules v2 require a new campaign; the original file is preserved".into());
    }
    let save: Save = serde_json::from_slice(bytes).map_err(|e| format!("Damaged save: {e}"))?;
    if save.schema != 2 {
        return Err("Unsupported save version; original file is preserved".into());
    }
    if save.checksum != save.world.hash() {
        return Err("Save checksum mismatch; try the .bak recovery file".into());
    }
    save.world.validate(content)?;
    Ok(save.world)
}
/// Preserve a file with create_new so recovery never overwrites an earlier archive.
fn preserve(path: &Path, label: &str) -> std::io::Result<std::path::PathBuf> {
    for index in 1..=10000 {
        let mut name = path.as_os_str().to_os_string();
        name.push(format!(".{label}-{index}"));
        let destination = std::path::PathBuf::from(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
        {
            Ok(mut file) => {
                let mut source = fs::File::open(path)?;
                std::io::copy(&mut source, &mut file)?;
                file.sync_all()?;
                return Ok(destination);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::other("Too many preserved recovery files"))
}
fn write_atomic(path: &Path, stage: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(stage)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(stage, path)?;
    #[cfg(unix)]
    {
        fs::File::open(
            path.parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new(".")),
        )?
        .sync_all()?;
    }
    Ok(())
}
pub fn save(path: &Path, world: &World, content: &Content) -> Result<(), String> {
    let bytes = encode(world, content)?;
    let stage = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    if stage.exists() {
        return Err(
            "An interrupted .json.tmp exists. Inspect recovery snapshots before saving again."
                .into(),
        );
    }
    let result = (|| -> Result<(), String> {
        if path.exists() {
            let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
            if size > MAX_SAVE_BYTES as u64 {
                return Err(
                    "Existing file is too large to be a campaign; choose another save path".into(),
                );
            }
            if load(path, content).is_ok() {
                // Finish the backup independently before committing the new primary.
                let backup_stage = path.with_extension("json.backup.tmp");
                let old = fs::read(path).map_err(|e| e.to_string())?;
                write_atomic(&backup, &backup_stage, &old).map_err(|e| e.to_string())?;
            } else {
                preserve(path, "rejected").map_err(|e| e.to_string())?;
            }
        }
        write_atomic(path, &stage, &bytes).map_err(|e| e.to_string())
    })();
    result.map_err(|error| {
        format!("Save failed: {error}. Existing snapshots are retained for recovery.")
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryKind {
    Primary,
    PendingSave,
    Backup,
    PendingRecovery,
    PendingBackup,
}
#[derive(Clone, Debug)]
pub struct RecoveryCandidate {
    pub path: std::path::PathBuf,
    pub kind: RecoveryKind,
    pub day: Option<u32>,
    pub error: Option<String>,
}
/// Inspect without loading into the active game or mutating any files.
pub fn recovery_candidates(path: &Path, content: &Content) -> Vec<RecoveryCandidate> {
    [
        (path.to_path_buf(), RecoveryKind::Primary),
        (path.with_extension("json.tmp"), RecoveryKind::PendingSave),
        (path.with_extension("json.bak"), RecoveryKind::Backup),
        (
            path.with_extension("json.recovering"),
            RecoveryKind::PendingRecovery,
        ),
        (
            path.with_extension("json.backup.tmp"),
            RecoveryKind::PendingBackup,
        ),
    ]
    .into_iter()
    .filter(|(p, _)| p.exists())
    .map(|(path, kind)| {
        let (day, error) = match load(&path, content) {
            Ok(w) => (Some(w.day), None),
            Err(e) => (None, Some(e)),
        };
        RecoveryCandidate {
            path,
            kind,
            day,
            error,
        }
    })
    .collect()
}
/// Explicitly restore a reviewed candidate. Originals and interrupted files are
/// archived first; a damaged primary never replaces the last good backup.
pub fn restore(path: &Path, candidate: &Path, content: &Content) -> Result<World, String> {
    let known = recovery_candidates(path, content);
    if !known
        .iter()
        .any(|c| c.path == candidate && c.error.is_none())
    {
        return Err("Choose a compatible, validated recovery snapshot".into());
    }
    let world = load(candidate, content)?;
    let bytes = encode(&world, content)?;
    let stage = path.with_extension("json.recovering");
    let pending = path.with_extension("json.tmp");
    let backup_pending = path.with_extension("json.backup.tmp");
    let result = (|| -> std::io::Result<()> {
        if path.exists() {
            preserve(path, "before-recovery")?;
        }
        if pending.exists() {
            preserve(&pending, "interrupted")?;
        }
        if backup_pending.exists() {
            preserve(&backup_pending, "interrupted")?;
        }
        if stage.exists() {
            preserve(&stage, "interrupted")?;
            fs::remove_file(&stage)?;
        }
        write_atomic(path, &stage, &bytes)?;
        if pending.exists() {
            fs::remove_file(&pending)?;
        }
        if backup_pending.exists() {
            fs::remove_file(&backup_pending)?;
        }
        Ok(())
    })();
    result.map_err(|e| format!("Recovery failed: {e}; preserved files remain available"))?;
    Ok(world)
}
pub fn load(path: &Path, content: &Content) -> Result<World, String> {
    let file = fs::File::open(path).map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    decode(&bytes, content)
}
/// Replay human commands; deterministic AI runs through its usual daily phase.
pub fn replay(original: &World, content: &Content) -> Result<World, String> {
    original.validate(content)?;
    let mut world = World::new_scenario(
        content,
        original.player.clone(),
        original.seed,
        original.scenario,
    )?;
    let mut verified = 0;
    for entry in original.journal.iter().filter(|e| !e.ai) {
        while world.day < entry.command.day {
            world.advance(content, 1)?;
            verify_journal(original, &world, &mut verified)?;
        }
        world
            .apply(content, entry.command.clone(), false)
            .map_err(|reason| {
                format!(
                    "Replay command #{} on {} was rejected: {reason}",
                    entry.command.id,
                    crate::date(entry.command.day)
                )
            })?;
        verify_journal(original, &world, &mut verified)?;
    }
    while world.day < original.day {
        world.advance(content, 1)?;
        verify_journal(original, &world, &mut verified)?;
    }
    if verified != original.journal.len() {
        return Err(format!(
            "Replay first command divergence at entry {}: expected {:?}, got <missing>",
            verified + 1,
            original.journal.get(verified)
        ));
    }
    if world.hash() != original.hash() {
        let detail = first_state_difference(original, &world)
            .map(|difference| {
                format!(
                    "; first differing final-state field {}: expected {}, got {}",
                    difference.path, difference.expected, difference.actual
                )
            })
            .unwrap_or_default();
        return Err(format!(
            "Replay diverged at final snapshot {}: expected {:016x}, got {:016x}{detail}",
            crate::date(original.day),
            original.hash(),
            world.hash()
        ));
    }
    Ok(world)
}

fn verify_journal(original: &World, replayed: &World, verified: &mut usize) -> Result<(), String> {
    while *verified < replayed.journal.len() {
        let actual = &replayed.journal[*verified];
        let expected = original.journal.get(*verified);
        if expected != Some(actual) {
            return Err(format!(
                "Replay first command divergence at entry {} on {}: expected {:?}, got {:?}",
                *verified + 1,
                crate::date(actual.command.day),
                expected,
                actual
            ));
        }
        *verified += 1;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateDifference {
    /// Escaped JSON pointer, including stable country IDs for country fields.
    pub path: String,
    pub expected: String,
    pub actual: String,
}

/// Compare trusted snapshots for developer inspection. The first field in
/// canonical key/index order is not necessarily the earliest divergent tick.
pub fn first_state_difference(expected: &World, actual: &World) -> Option<StateDifference> {
    fn preview(value: Option<&serde_json::Value>) -> String {
        let text = value.map_or_else(|| "<missing>".into(), ToString::to_string);
        if text.chars().count() > 120 {
            format!("{}…", text.chars().take(120).collect::<String>())
        } else {
            text
        }
    }
    fn compare(
        path: String,
        expected: Option<&serde_json::Value>,
        actual: Option<&serde_json::Value>,
    ) -> Option<StateDifference> {
        if expected == actual {
            return None;
        }
        match (expected, actual) {
            (Some(serde_json::Value::Object(a)), Some(serde_json::Value::Object(b))) => {
                let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
                for key in keys {
                    let escaped = key.replace('~', "~0").replace('/', "~1");
                    if let Some(difference) =
                        compare(format!("{path}/{escaped}"), a.get(key), b.get(key))
                    {
                        return Some(difference);
                    }
                }
            }
            (Some(serde_json::Value::Array(a)), Some(serde_json::Value::Array(b))) => {
                for index in 0..a.len().max(b.len()) {
                    if let Some(difference) =
                        compare(format!("{path}/{index}"), a.get(index), b.get(index))
                    {
                        return Some(difference);
                    }
                }
            }
            _ => {}
        }
        Some(StateDifference {
            path,
            expected: preview(expected),
            actual: preview(actual),
        })
    }
    compare(
        String::new(),
        Some(&serde_json::to_value(expected).expect("world serialization")),
        Some(&serde_json::to_value(actual).expect("world serialization")),
    )
}
