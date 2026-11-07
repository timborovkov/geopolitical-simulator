use sim_core::{persistence, *};
use std::path::PathBuf;
fn directory(test: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("geosim-recovery-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    path
}
fn setup() -> (Content, World) {
    let c = Content::bundled().unwrap();
    let w = World::new(&c, CountryId("GBR".into()), 5).unwrap();
    (c, w)
}
#[test]
fn damaged_primary_never_overwrites_a_good_backup() {
    let dir = directory("damaged");
    let path = dir.join("campaign.json");
    let (c, mut w) = setup();
    persistence::save(&path, &w, &c).unwrap();
    let original = w.hash();
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    persistence::save(&path, &w, &c).unwrap();
    std::fs::write(&path, b"damaged primary").unwrap();
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    persistence::save(&path, &w, &c).unwrap();
    assert_eq!(
        persistence::load(&path.with_extension("json.bak"), &c)
            .unwrap()
            .hash(),
        original
    );
    assert_eq!(persistence::load(&path, &c).unwrap().hash(), w.hash());
    assert!(dir.join("campaign.json.rejected-1").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn interrupted_valid_write_can_be_reviewed_and_restored_without_losing_originals() {
    let dir = directory("interrupted");
    let path = dir.join("campaign.json");
    let (c, mut w) = setup();
    persistence::save(&path, &w, &c).unwrap();
    let original = w.hash();
    w.advance(&c, sim_core::DaySpan::new(3)).unwrap();
    let pending = path.with_extension("json.tmp");
    std::fs::write(&pending, persistence::encode(&w, &c).unwrap()).unwrap();
    let candidates = persistence::recovery_candidates(&path, &c);
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates
            .iter()
            .find(|r| r.kind == persistence::RecoveryKind::PendingSave)
            .unwrap()
            .day,
        Some(3)
    );
    assert!(persistence::save(&path, &w, &c).is_err());
    let restored = persistence::restore(&path, &pending, &c).unwrap();
    assert_eq!(restored.hash(), w.hash());
    assert!(!pending.exists());
    assert_eq!(
        persistence::load(&dir.join("campaign.json.before-recovery-1"), &c)
            .unwrap()
            .hash(),
        original
    );
    assert!(dir.join("campaign.json.tmp.interrupted-1").exists());
    persistence::save(&path, &restored, &c).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn invalid_restore_does_not_mutate_any_snapshot() {
    let dir = directory("invalid");
    let path = dir.join("campaign.json");
    let (c, w) = setup();
    persistence::save(&path, &w, &c).unwrap();
    let before = std::fs::read(&path).unwrap();
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, b"corrupt").unwrap();
    assert!(persistence::restore(&path, &temp, &c).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(&temp).unwrap(), b"corrupt");
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn interrupted_backup_is_visible_and_can_be_archived_by_explicit_recovery() {
    let dir = directory("backup-stage");
    let path = dir.join("campaign.json");
    let (c, mut w) = setup();
    persistence::save(&path, &w, &c).unwrap();
    std::fs::write(
        path.with_extension("json.backup.tmp"),
        b"interrupted backup",
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    assert!(persistence::save(&path, &w, &c).is_err());
    assert!(
        persistence::recovery_candidates(&path, &c)
            .iter()
            .any(|r| r.kind == persistence::RecoveryKind::PendingBackup && r.error.is_some())
    );
    let restored = persistence::restore(&path, &path, &c).unwrap();
    assert_eq!(restored.day.units(), 0);
    assert!(!path.with_extension("json.backup.tmp").exists());
    persistence::save(&path, &w, &c).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}
