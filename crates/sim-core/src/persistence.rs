//! Versioned snapshots, checksums, atomic replacement and deterministic replay.
use crate::developer::CommandOrigin;
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
pub const SAVE_SCHEMA: u32 = 3;
/// New rules require a distinct envelope so older clients reject unfamiliar state.
/// Fixed-price, market, financing and constitutional campaigns use formats 3–37.
pub fn snapshot_schema(world: &World) -> u32 {
    snapshot_schema_for_rules(world.rules)
}
/// Envelope version for a validated campaign's rules.
pub fn snapshot_schema_for_rules(rules: u32) -> u32 {
    match rules {
        36 => 37,
        35 => 36,
        34 => 35,
        33 => 34,
        32 => 33,
        31 => 32,
        30 => 31,
        29 => 30,
        28 => 29,
        27 => 28,
        26 => 27,
        25 => 26,
        24 => 25,
        23 => 24,
        22 => 23,
        21 => 22,
        20 => 21,
        19 => 20,
        18 => 19,
        17 => 18,
        16 => 17,
        15 => 16,
        14 => 15,
        13 => 14,
        12 => 13,
        11 => 12,
        10 => 11,
        9 => 10,
        8 => 9,
        7 => 8,
        6 => 7,
        5 => 6,
        4 => 5,
        3 => 4,
        _ => SAVE_SCHEMA,
    }
}
/// Country/scenario/content-specific names remain inside the saves directory,
/// including for packs whose IDs contain punctuation or non-ASCII characters.
pub fn autosave_path(world: &World) -> std::path::PathBuf {
    let mut label: String = world
        .player
        .0
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(12)
        .collect();
    if label.is_empty() {
        label = "country".into();
    }
    std::path::PathBuf::from("saves").join(format!(
        "autosave-{label}-{:016x}-{}-{:016x}-v{}.json",
        crate::stable_hash(world.player.0.as_bytes()),
        world.scenario.id(),
        world.content_hash,
        snapshot_schema(world)
    ))
}
pub fn encode(world: &World, content: &Content) -> Result<Vec<u8>, String> {
    world.validate(content)?;
    let bytes = serde_json::to_vec(&Save {
        schema: snapshot_schema(world),
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
    decode_save(bytes, content).map(|save| save.world)
}
fn decode_save(bytes: &[u8], content: &Content) -> Result<Save, String> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Save exceeds the 32 MiB limit".into());
    }
    let header: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("Damaged save: {e}"))?;
    if !matches!(header.get("schema").and_then(|v| v.as_u64()), Some(2..=37)) {
        return Err("Unsupported save version; supported snapshot formats are 2 through 37. The original file is preserved".into());
    }
    let save: Save = serde_json::from_slice(bytes).map_err(|e| format!("Damaged save: {e}"))?;
    if save.world.rules == 36 && save.schema != 37 {
        return Err(
            "Delivered consumption requires snapshot format 37; original file preserved".into(),
        );
    }
    if save.world.rules == 35 && save.schema != 36 {
        return Err(
            "Business ownership requires snapshot format 36; original file preserved".into(),
        );
    }
    if save.world.rules == 34 && save.schema != 35 {
        return Err("Industrial trade requires snapshot format 35; original file preserved".into());
    }
    if save.world.rules == 33 && save.schema != 34 {
        return Err(
            "Public industry retirement requires snapshot format 34; original file preserved"
                .into(),
        );
    }
    if save.world.rules == 32 && save.schema != 33 {
        return Err(
            "Industrial capacity requires snapshot format 33; original file preserved".into(),
        );
    }
    if save.world.rules == 31 && save.schema != 32 {
        return Err(
            "Auxiliary production requires snapshot format 32; original file preserved".into(),
        );
    }
    if save.world.rules == 30 && save.schema != 31 {
        return Err(
            "Joint social memberships require snapshot format 31; original file preserved".into(),
        );
    }
    if save.world.rules == 29 && save.schema != 30 {
        return Err(
            "Population balances require snapshot format 30; original file preserved".into(),
        );
    }
    if save.world.rules == 28 && save.schema < 29 {
        return Err(
            "Governing institution context requires snapshot format 29; original file preserved"
                .into(),
        );
    }
    if save.world.rules == 27 && save.schema < 28 {
        return Err(
            "Live executive identities require snapshot format 28; original file preserved".into(),
        );
    }
    if save.world.rules == 26 && save.schema < 27 {
        return Err(
            "Statutory authority requires snapshot format 27; original file preserved".into(),
        );
    }
    if save.world.rules == 25 && save.schema < 26 {
        return Err(
            "Archived review statistics require snapshot format 26; original file preserved".into(),
        );
    }
    if save.world.rules == 24 && save.schema < 25 {
        return Err(
            "Stable political selection requires snapshot format 25; original file preserved"
                .into(),
        );
    }
    if save.world.rules == 23 && save.schema < 24 {
        return Err(
            "Linked programme events require snapshot format 24; original file preserved".into(),
        );
    }
    if save.world.rules == 22 && save.schema < 23 {
        return Err(
            "Stable event references require snapshot format 23; original file preserved".into(),
        );
    }
    if save.world.rules == 21 && save.schema < 22 {
        return Err("National reserves require snapshot format 22; original file preserved".into());
    }
    if save.world.rules == 20 && save.schema < 21 {
        return Err(
            "Party policy preferences require snapshot format 21; original file preserved".into(),
        );
    }
    if save.world.rules == 19 && save.schema < 20 {
        return Err(
            "Bookmark sovereignty references require snapshot format 20; original file preserved"
                .into(),
        );
    }
    if save.world.rules == 18 && save.schema < 19 {
        return Err(
            "Dated technology eligibility requires snapshot format 19; original file preserved"
                .into(),
        );
    }
    if save.world.rules == 17 && save.schema < 18 {
        return Err(
            "Carried daily interest requires snapshot format 18; original file preserved".into(),
        );
    }
    if save.world.rules == 16 && save.schema < 17 {
        return Err(
            "Temporary cash retention requires snapshot format 17; original file preserved".into(),
        );
    }
    if save.world.rules == 15 && save.schema < 16 {
        return Err(
            "Dated loan principal requires snapshot format 16; original file preserved".into(),
        );
    }
    if save.world.rules == 14 && save.schema < 15 {
        return Err(
            "National fiscal AI requires snapshot format 15; original file preserved".into(),
        );
    }
    if save.world.rules == 13 && save.schema < 14 {
        return Err(
            "Private-income bands require snapshot format 14; original file preserved".into(),
        );
    }
    if save.world.rules == 12 && save.schema < 13 {
        return Err("Fiscal plans require snapshot format 13; original file preserved".into());
    }
    if save.world.rules == 11 && save.schema < 12 {
        return Err(
            "Delivery undertakings require snapshot format 12; original file preserved".into(),
        );
    }
    if save.world.rules == 10 && save.schema < 11 {
        return Err(
            "Territorial reforms require snapshot format 11; original file preserved".into(),
        );
    }
    if save.world.rules == 9 && save.schema < 10 {
        return Err(
            "Subnational mandates require snapshot format 10; original file preserved".into(),
        );
    }
    if save.world.rules == 8 && save.schema < 9 {
        return Err(
            "Electoral authority workflow requires snapshot format 9; original file preserved"
                .into(),
        );
    }
    if save.world.rules == 7 && save.schema < 8 {
        return Err(
            "Ministerial appointments require snapshot format 8; original file preserved".into(),
        );
    }
    if save.world.rules == 6 && save.schema < 7 {
        return Err(
            "Commitment lifecycle requires snapshot format 7; original file preserved".into(),
        );
    }
    if save.world.rules == 5 && save.schema < 6 {
        return Err(
            "Constitutional-transition rules require snapshot format 6; original file preserved"
                .into(),
        );
    }
    if save.world.rules == 4 && save.schema < 5 {
        return Err(
            "Financing-authority rules require snapshot format 5; original file preserved".into(),
        );
    }
    if save.world.rules == 3 && save.schema < 4 {
        return Err(
            "Relative-price rules require snapshot format 4; original file preserved".into(),
        );
    }
    if save.schema == 2 && save.world.is_developer_modified() {
        return Err(
            "Developer command journals require snapshot format 3; original file preserved".into(),
        );
    }
    if save.checksum != save.world.hash() {
        return Err("Save checksum mismatch; try the .bak recovery file".into());
    }
    save.world.validate(content)?;
    Ok(save)
}
/// Preserve a file with create_new so recovery never overwrites an earlier archive.
fn archive_file(path: &Path, label: &str) -> std::io::Result<(fs::File, std::path::PathBuf)> {
    for index in 1..=10000 {
        let mut name = path.as_os_str().to_os_string();
        name.push(format!(".{label}-{index}"));
        let destination = std::path::PathBuf::from(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
        {
            Ok(file) => return Ok((file, destination)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::other("Too many preserved recovery files"))
}
fn preserve(path: &Path, label: &str) -> std::io::Result<std::path::PathBuf> {
    let (mut file, destination) = archive_file(path, label)?;
    let mut source = fs::File::open(path)?;
    std::io::copy(&mut source, &mut file)?;
    file.sync_all()?;
    Ok(destination)
}
fn preserve_bytes(path: &Path, label: &str, bytes: &[u8]) -> std::io::Result<std::path::PathBuf> {
    let (mut file, destination) = archive_file(path, label)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(destination)
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
            let old = read_bytes(path)?;
            if let Ok(previous) = decode_save(&old, content) {
                if previous.schema != snapshot_schema(world) {
                    preserve_bytes(path, &format!("format-v{}", previous.schema), &old)
                        .map_err(|e| e.to_string())?;
                }
                // Finish the backup independently before committing the new primary.
                let backup_stage = path.with_extension("json.backup.tmp");
                write_atomic(&backup, &backup_stage, &old).map_err(|e| e.to_string())?;
            } else {
                preserve_bytes(path, "rejected", &old).map_err(|e| e.to_string())?;
            }
        }
        write_atomic(path, &stage, &bytes).map_err(|e| e.to_string())
    })();
    result.map_err(|error| {
        format!("Save failed: {error}. Existing snapshots are retained for recovery.")
    })
}
/// Save the active campaign and retain a unique archived snapshot before a user
/// explicitly replaces it with a new campaign. Later backup rotation cannot
/// overwrite this archive.
pub fn save_before_new_campaign(
    path: &Path,
    world: &World,
    content: &Content,
) -> Result<std::path::PathBuf, String> {
    let bytes = encode(world, content)?;
    save(path, world, content)?;
    preserve_bytes(path, "before-new-campaign", &bytes).map_err(|error| format!("Current campaign saved, but campaign archive failed: {error}. New campaign was not started."))
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
    pub day: Option<crate::CampaignDay>,
    pub schema: Option<u32>,
    pub content_id: Option<String>,
    pub rules: Option<u32>,
    pub error: Option<String>,
}
/// Inspect without loading into the active game or mutating any files.
pub fn recovery_candidates(path: &Path, content: &Content) -> Vec<RecoveryCandidate> {
    inspect_candidates(path, |candidate| {
        load_save(candidate, content).map(|save| (content.id.clone(), save))
    })
}
/// The default clients may resolve only these exact bundled packs. An explicit
/// --content selection continues to use recovery_candidates/load instead.
pub fn recovery_candidates_bundled(path: &Path) -> Vec<RecoveryCandidate> {
    inspect_candidates(path, |candidate| {
        load_bundled_save(candidate).map(|(content, save)| (content.id, save))
    })
}
fn inspect_candidates(
    path: &Path,
    resolve: impl Fn(&Path) -> Result<(String, Save), String>,
) -> Vec<RecoveryCandidate> {
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
        let (day, schema, content_id, rules, error) = match resolve(&path) {
            Ok((id, save)) => (
                Some(save.world.day),
                Some(save.schema),
                Some(id),
                Some(save.world.rules),
                None,
            ),
            Err(e) => (None, None, None, None, Some(e)),
        };
        RecoveryCandidate {
            path,
            kind,
            day,
            schema,
            content_id,
            rules,
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
    let original_bytes = read_bytes(candidate)?;
    let original = decode_save(&original_bytes, content)?;
    let original_schema = original.schema;
    let world = original.world;
    let bytes = encode(&world, content)?;
    let stage = path.with_extension("json.recovering");
    let pending = path.with_extension("json.tmp");
    let backup_pending = path.with_extension("json.backup.tmp");
    let result = (|| -> std::io::Result<()> {
        if original_schema != snapshot_schema(&world) {
            preserve_bytes(
                candidate,
                &format!("format-v{original_schema}"),
                &original_bytes,
            )?;
        }
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
    load_save(path, content).map(|save| save.world)
}
pub fn load_bundled(path: &Path) -> Result<(Content, World), String> {
    load_bundled_save(path).map(|(content, save)| (content, save.world))
}
fn load_bundled_save(path: &Path) -> Result<(Content, Save), String> {
    let bytes = read_bytes(path)?;
    let header: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("Damaged save: {error}"))?;
    if !matches!(
        header.get("schema").and_then(|value| value.as_u64()),
        Some(2..=37)
    ) {
        return Err("Unsupported save version; original file preserved".into());
    }
    let hash = header
        .get("world")
        .and_then(|world| world.get("content_hash"))
        .and_then(|value| value.as_u64())
        .ok_or("Damaged save: missing content identity")?;
    let content = Content::bundled_for_hash(hash)?;
    let save = decode_save(&bytes, &content)?;
    Ok((content, save))
}
pub fn restore_bundled(path: &Path, candidate: &Path) -> Result<(Content, World), String> {
    if !recovery_candidates_bundled(path)
        .iter()
        .any(|entry| entry.path == candidate && entry.error.is_none())
    {
        return Err("Choose a compatible, validated bundled recovery snapshot".into());
    }
    let (content, _) = load_bundled_save(candidate)?;
    let world = restore(path, candidate, &content)?;
    Ok((content, world))
}
fn load_save(path: &Path, content: &Content) -> Result<Save, String> {
    decode_save(&read_bytes(path)?, content)
}
fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_SAVE_BYTES {
        return Err("Snapshot exceeds the 32 MiB limit".into());
    }
    Ok(bytes)
}
/// Replay human commands; deterministic AI runs through its usual daily phase.
pub fn replay(original: &World, content: &Content) -> Result<World, String> {
    replay_observed(original, content, true, |_, _| Ok(()))
}

pub(crate) fn replay_observed(
    original: &World,
    content: &Content,
    check_final: bool,
    mut observe: impl FnMut(&World, crate::replay_trace::ReplayBoundary) -> Result<(), String>,
) -> Result<World, String> {
    original.validate(content)?;
    let mut world = World::new_scenario(
        content,
        original.player.clone(),
        original.seed,
        original.scenario,
    )?;
    observe(&world, crate::replay_trace::ReplayBoundary::Opening)?;
    let mut verified = 0;
    for entry in original
        .journal
        .iter()
        .filter(|e| !e.ai && e.budget_bill.is_none())
    {
        while world.day.units() < entry.command.day.units() {
            world.advance(content, crate::DaySpan::new(1))?;
            observe(&world, crate::replay_trace::ReplayBoundary::Day)?;
            verify_journal(original, &world, &mut verified)?;
        }
        world
            .apply(
                content,
                entry.command.clone(),
                if entry.developer {
                    CommandOrigin::Developer
                } else {
                    CommandOrigin::Player
                },
            )
            .map_err(|reason| {
                format!(
                    "Replay command #{} on {} was rejected: {reason}",
                    entry.command.id,
                    crate::date(entry.command.day)
                )
            })?;
        observe(
            &world,
            crate::replay_trace::ReplayBoundary::Command(entry.command.id),
        )?;
        verify_journal(original, &world, &mut verified)?;
    }
    while world.day.units() < original.day.units() {
        world.advance(content, crate::DaySpan::new(1))?;
        observe(&world, crate::replay_trace::ReplayBoundary::Day)?;
        verify_journal(original, &world, &mut verified)?;
    }
    if verified != original.journal.len() {
        return Err(format!(
            "Replay first command divergence at entry {}: expected {:?}, got <missing>",
            verified + 1,
            original.journal.get(verified)
        ));
    }
    if check_final && world.hash() != original.hash() {
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
