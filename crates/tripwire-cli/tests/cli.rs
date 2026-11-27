//! Regression test for a real bug: `--adapter <cmd...>` used to be defined with
//! `allow_hyphen_values(true)`, so it greedily swallowed a trailing `--json` as
//! part of the adapter command instead of leaving it for clap to parse. The
//! adapter then received `--json` as its corpus-directory argument. Caught by
//! stellar-tripwire-action's own CI, of all places.

use std::fs;

use assert_cmd::Command;

fn fake_adapter() -> &'static str {
    "console.log(JSON.stringify({adapter:'fake',version:'0'}))"
}

#[test]
fn json_flag_after_adapter_is_still_parsed_as_a_flag() {
    let dir = tempfile::tempdir().unwrap();
    tripwire_corpus::write_corpus(dir.path()).unwrap();
    let script = dir.path().join("adapter.js");
    fs::write(&script, fake_adapter()).unwrap();

    Command::cargo_bin("tripwire")
        .unwrap()
        .args([
            "run",
            dir.path().to_str().unwrap(),
            "--adapter",
            "node",
            script.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicates_contains("\"adapter\": \"fake\""));
}

fn predicates_contains(needle: &'static str) -> impl predicates::Predicate<str> {
    predicates::str::contains(needle)
}
