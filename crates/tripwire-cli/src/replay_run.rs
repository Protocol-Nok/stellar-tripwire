//! Runs an indexer adapter against the replay corpus and compares what it reports it
//! observed against what the manifest expects. See `docs/replay-protocol.md`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use tripwire_replay::Manifest;

#[derive(Debug, Deserialize)]
struct Hello {
    indexer: String,
    version: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LedgerReport {
    ledger: String,
    #[serde(default)]
    observations: Vec<serde_json::Value>,
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
        .with_context(|| format!("starting indexer adapter {:?}", adapter[0]))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let hello: Hello = serde_json::from_str(lines.next().context("adapter printed nothing")?)
        .context("first adapter line must be {\"indexer\":..,\"version\":..}")?;

    let mut reported: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for l in lines {
        let r: LedgerReport =
            serde_json::from_str(l).with_context(|| format!("bad adapter line: {l}"))?;
        reported.insert(r.ledger, r.observations);
    }

    let (mut pass, mut dropped, mut missing) = (0u32, 0u32, 0u32);
    let mut rows = Vec::new();
    for e in &manifest.entries {
        let seen = reported.get(&e.id);
        for obs in &e.observations {
            let obs_json = serde_json::to_value(obs)?;
            let (status, detail) = match seen {
                None => {
                    missing += 1;
                    (
                        "missing",
                        "indexer reported nothing for this ledger".to_string(),
                    )
                }
                Some(list) if list.contains(&obs_json) => {
                    pass += 1;
                    ("pass", String::new())
                }
                Some(_) => {
                    dropped += 1;
                    (
                        "dropped",
                        "indexer processed this ledger but didn't report this observation"
                            .to_string(),
                    )
                }
            };
            rows.push((e.id.clone(), obs_json, status, detail));
        }
    }

    if json {
        let v: Vec<_> = rows
            .iter()
            .map(|(e, o, s, d)| serde_json::json!({"ledger": e, "observation": o, "status": s, "detail": d}))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "indexer": hello.indexer, "version": hello.version,
                "pass": pass, "dropped": dropped, "missing": missing, "results": v
            }))?
        );
    } else {
        println!("{} ({})", hello.indexer, hello.version);
        for (e, o, s, d) in &rows {
            let mark = match *s {
                "pass" => "PASS",
                "dropped" => "DROP",
                _ => "MISS",
            };
            if d.is_empty() {
                println!("  {mark}  {e} / {o}");
            } else {
                println!("  {mark}  {e} / {o}\n        {d}");
            }
        }
        println!("{pass} passed, {dropped} dropped, {missing} missing");
    }

    if dropped > 0 || missing > 0 {
        return Ok(ExitCode::from(1));
    }
    if pass == 0 {
        bail!("no observations were checked at all -- the replay corpus may be empty");
    }
    Ok(ExitCode::SUCCESS)
}
