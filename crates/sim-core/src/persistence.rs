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

pub fn encode(world: &World, content: &Content) -> Result<Vec<u8>, String> {
    world.validate(content)?;
    serde_json::to_vec(&Save {
        schema: 2,
        checksum: world.hash(),
        world: world.clone(),
    })
    .map_err(|e| e.to_string())
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
pub fn save(path: &Path, world: &World, content: &Content) -> Result<(), String> {
    let bytes = encode(world, content)?;
    let temp = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    let result = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        if path.exists() {
            fs::copy(path, &backup)?;
        }
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if let Err(e) = result {
        return Err(format!(
            "Save failed: {e}. Previous save/backup preserved. If an interrupted .tmp exists, inspect it before removing it."
        ));
    }
    Ok(())
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
    for entry in original.journal.iter().filter(|e| !e.ai) {
        while world.day < entry.command.day {
            world.advance(content, 1)?;
        }
        world.apply(content, entry.command.clone(), false)?;
    }
    while world.day < original.day {
        world.advance(content, 1)?;
    }
    if world.hash() != original.hash() {
        return Err(format!(
            "Replay diverged: expected {:016x}, got {:016x}",
            original.hash(),
            world.hash()
        ));
    }
    Ok(world)
}
