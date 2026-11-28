//! Regression tests for `tripwire run`'s adapter-command parsing. History, because it
//! took two tries to get right:
//!
//! 1. `--adapter <cmd...>` was defined with `allow_hyphen_values(true)` so the adapter
//!    command could itself contain flags. It then greedily swallowed a trailing `--json`
//!    as part of the adapter command instead of leaving it for clap to parse -- the
//!    adapter received `--json` as its corpus-directory argument. Caught by
//!    stellar-tripwire-action's own CI running the exact invocation its README documents.
//! 2. The fix -- drop `allow_hyphen_values`, so `--adapter` stops at the next
//!    flag-shaped token -- broke the opposite case: `tripwire run dir --adapter java -jar
//!    x.jar` now rejected `-jar` as an unrecognized top-level flag. `java -jar` is an
//!    extremely ordinary way to invoke a JVM program, which is exactly how the Java
//!    adapter needs to run.
//!
//! The actual fix: the adapter command goes after a literal `--`, same as `cargo run --
//! --flag` or `kubectl exec --`. Everything after `--` is unambiguously the adapter's,
//! regardless of what it looks like, so neither failure mode above is possible.

use std::fs;

use assert_cmd::Command;

fn fake_adapter() -> &'static str {
    "console.log(JSON.stringify({adapter:'fake',version:'0'}))"
}

#[test]
fn json_flag_before_the_separator_is_still_parsed_as_a_flag() {
    let dir = tempfile::tempdir().unwrap();
    tripwire_corpus::write_corpus(dir.path()).unwrap();
    let script = dir.path().join("adapter.js");
    fs::write(&script, fake_adapter()).unwrap();

    Command::cargo_bin("tripwire")
        .unwrap()
        .args([
            "run",
            dir.path().to_str().unwrap(),
            "--json",
            "--",
            "node",
            script.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates_contains("\"adapter\": \"fake\""));
}

#[test]
fn adapter_command_may_contain_its_own_flag_shaped_arguments() {
    // The case this design actually exists for: an adapter invocation that itself
    // looks like `<prog> -<flag> <value>`, same shape as `java -jar x.jar`. A fake
    // "interpreter" script stands in so this test needs no JVM.
    let dir = tempfile::tempdir().unwrap();
    tripwire_corpus::write_corpus(dir.path()).unwrap();
    let fake_jvm = dir.path().join("fake-java.sh");
    fs::write(
        &fake_jvm,
        "#!/bin/sh\n# usage: fake-java.sh -jar <unused> <corpus-dir>\nshift 2\necho '{\"adapter\":\"fake\",\"version\":\"0\"}'\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_jvm).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_jvm, perms).unwrap();
    }

    Command::cargo_bin("tripwire")
        .unwrap()
        .args([
            "run",
            dir.path().to_str().unwrap(),
            "--json",
            "--",
            fake_jvm.to_str().unwrap(),
            "-jar",
            "unused.jar",
        ])
        .assert()
        .success()
        .stdout(predicates_contains("\"adapter\": \"fake\""));
}

fn predicates_contains(needle: &'static str) -> impl predicates::Predicate<str> {
    predicates::str::contains(needle)
}
