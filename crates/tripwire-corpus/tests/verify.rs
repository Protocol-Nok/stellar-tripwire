use std::fs;

use tripwire_corpus::{generate_entries, verify_dir, write_corpus};

#[test]
fn generation_is_deterministic() {
    let a = generate_entries().unwrap();
    let b = generate_entries().unwrap();
    assert_eq!(a, b);
    assert!(!a.is_empty());
}

#[test]
fn a_fresh_corpus_verifies_including_regen() {
    let dir = tempfile::tempdir().unwrap();
    write_corpus(dir.path()).unwrap();
    let findings = verify_dir(dir.path(), true).unwrap();
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn every_entry_says_why_it_exists() {
    for (e, _) in generate_entries().unwrap() {
        assert!(
            !e.sightings.is_empty() || e.cap.is_some(),
            "{} has no reason",
            e.id
        );
        assert!(!e.checks.is_empty(), "{} has no checks", e.id);
    }
}

#[test]
fn a_flipped_byte_is_caught() {
    let dir = tempfile::tempdir().unwrap();
    let m = write_corpus(dir.path()).unwrap();
    let path = dir.path().join(&m.entries[0].file);
    let mut bytes = fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 0x01;
    fs::write(&path, bytes).unwrap();
    let findings = verify_dir(dir.path(), false).unwrap();
    assert!(
        findings.iter().any(|f| f.message.contains("sha256")),
        "{findings:?}"
    );
}

#[test]
fn an_entry_without_a_reason_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    write_corpus(dir.path()).unwrap();
    let mp = dir.path().join("manifest.json");
    let mut v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&mp).unwrap()).unwrap();
    v["entries"][0]["sightings"] = serde_json::json!([]);
    v["entries"][0]["cap"] = serde_json::Value::Null;
    fs::write(&mp, v.to_string()).unwrap();
    let findings = verify_dir(dir.path(), false).unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("neither a CAP nor a sighting")),
        "{findings:?}"
    );
}

#[test]
fn a_wrong_schema_version_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    write_corpus(dir.path()).unwrap();
    let mp = dir.path().join("manifest.json");
    let mut v: serde_json::Value = serde_json::from_str(&fs::read_to_string(&mp).unwrap()).unwrap();
    v["schema"] = serde_json::json!(99);
    fs::write(&mp, v.to_string()).unwrap();
    let findings = verify_dir(dir.path(), false).unwrap();
    assert!(
        findings.iter().any(|f| f.message.contains("schema 99")),
        "{findings:?}"
    );
}
