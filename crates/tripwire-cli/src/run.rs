//! Running an adapter and reporting where it disagrees with the manifest.
//!
//! Adapter protocol (see docs/adapter-protocol.md): the adapter is started with the corpus
//! directory as its last argument and prints one JSON object per line on stdout:
//!   {"adapter":"js","version":"stellar-sdk 17.4.0"}                      (first line)
//!   {"entry":"<id>","check":"<kind>","status":"pass|fail|unsupported","detail":"..."}

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use tripwire_corpus::Manifest;

#[derive(Debug, Deserialize)]
struct Hello {
    adapter: String,
    version: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Result_ {
    entry: String,
    check: String,
    status: String,
    #[serde(default)]
    detail: String,
}

fn check_kind(c: &tripwire_corpus::Check) -> String {
    serde_json::to_value(c)
        .ok()
        .and_then(|v| v["kind"].as_str().map(str::to_string))
        .unwrap_or_default()
}

pub fn run(dir: &Path, adapter: &[String], json: bool) -> Result<ExitCode> {
    let manifest: Manifest = serde_json::from_str(
        &std::fs::read_to_string(dir.join("manifest.json")).context("reading manifest.json")?,
    )?;
    let abs = std::fs::canonicalize(dir)?;
    let out = Command::new(&adapter[0])
        .args(&adapter[1..])
        .arg(&abs)
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("starting adapter {:?}", adapter[0]))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let hello: Hello = serde_json::from_str(lines.next().context("adapter printed nothing")?)
        .context("first adapter line must be {\"adapter\":..,\"version\":..}")?;

    let mut got: BTreeMap<(String, String), Result_> = BTreeMap::new();
    for l in lines {
        let r: Result_ =
            serde_json::from_str(l).with_context(|| format!("bad adapter line: {l}"))?;
        if !matches!(r.status.as_str(), "pass" | "fail" | "unsupported") {
            bail!(
                "adapter reported unknown status {:?} for {}/{}",
                r.status,
                r.entry,
                r.check
            );
        }
        got.insert((r.entry.clone(), r.check.clone()), r);
    }

    let (mut pass, mut fail, mut unsupported) = (0u32, 0u32, 0u32);
    let mut rows = Vec::new();
    for e in &manifest.entries {
        for c in &e.checks {
            let kind = check_kind(c);
            let (status, detail) = match got.get(&(e.id.clone(), kind.clone())) {
                Some(r) => (r.status.clone(), r.detail.clone()),
                None => (
                    "unsupported".to_string(),
                    "adapter did not report this check".to_string(),
                ),
            };
            match status.as_str() {
                "pass" => pass += 1,
                "fail" => fail += 1,
                _ => unsupported += 1,
            }
            rows.push((e.id.clone(), kind, status, detail));
        }
    }

    if json {
        let v: Vec<_> = rows
            .iter()
            .map(|(e, k, s, d)| serde_json::json!({"entry":e,"check":k,"status":s,"detail":d}))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "adapter": hello.adapter, "version": hello.version,
                "pass": pass, "fail": fail, "unsupported": unsupported, "results": v
            }))?
        );
    } else {
        println!("{} ({})", hello.adapter, hello.version);
        for (e, k, s, d) in &rows {
            let mark = match s.as_str() {
                "pass" => "PASS",
                "fail" => "FAIL",
                _ => "skip",
            };
            if d.is_empty() || s == "pass" {
                println!("  {mark}  {e} / {k}");
            } else {
                println!("  {mark}  {e} / {k}\n        {d}");
            }
        }
        println!("{pass} passed, {fail} failed, {unsupported} unsupported");
    }
    Ok(if fail > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}
