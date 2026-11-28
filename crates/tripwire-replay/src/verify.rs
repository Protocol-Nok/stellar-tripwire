use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use stellar_xdr::{LedgerCloseMeta, Limits, ReadXdr, WriteXdr};

use crate::{Manifest, SCHEMA_VERSION, generate_replay_entries, sha256_hex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub entry: String,
    pub message: String,
}

fn xdr_crate_label() -> String {
    let v = stellar_xdr::VERSION;
    format!("stellar-xdr {} (xdr {})", v.pkg, v.xdr.trim())
}

/// Generate the replay corpus and write `manifest.json` plus `entries/*.xdr` under `dir`.
pub fn write_replay_corpus(dir: &Path) -> Result<Manifest> {
    fs::create_dir_all(dir.join("entries"))?;
    let generated = generate_replay_entries()?;
    let mut entries = Vec::new();
    for (entry, bytes) in generated {
        fs::write(dir.join(&entry.file), &bytes)
            .with_context(|| format!("writing {}", entry.file))?;
        entries.push(entry);
    }
    let manifest = Manifest {
        schema: SCHEMA_VERSION,
        xdr_crate: xdr_crate_label(),
        entries,
    };
    let mut json = serde_json::to_string_pretty(&manifest)?;
    json.push('\n');
    fs::write(dir.join("manifest.json"), json)?;
    Ok(manifest)
}

/// Check a replay corpus directory: every entry decodes as `LedgerCloseMeta`, round-trips
/// byte for byte, and its hash matches the manifest. With `regen`, also require the
/// committed files to equal what the generator produces today.
pub fn verify_dir(dir: &Path, regen: bool) -> Result<Vec<Finding>> {
    let raw = fs::read_to_string(dir.join("manifest.json")).context("reading manifest.json")?;
    let manifest: Manifest = serde_json::from_str(&raw).context("parsing manifest.json")?;
    let mut out = Vec::new();
    let mut bad = |entry: &str, message: String| {
        out.push(Finding {
            entry: entry.to_string(),
            message,
        })
    };

    if manifest.schema != SCHEMA_VERSION {
        bad(
            "(manifest)",
            format!(
                "schema {} but this tool reads {}",
                manifest.schema, SCHEMA_VERSION
            ),
        );
    }

    for e in &manifest.entries {
        let bytes = match fs::read(dir.join(&e.file)) {
            Ok(b) => b,
            Err(err) => {
                bad(&e.id, format!("cannot read {}: {err}", e.file));
                continue;
            }
        };
        if sha256_hex(&bytes) != e.sha256 {
            bad(&e.id, "sha256 in manifest does not match the file".into());
        }
        match LedgerCloseMeta::from_xdr(&bytes, Limits::none()) {
            Ok(decoded) => match decoded.to_xdr(Limits::none()) {
                Ok(again) if again == bytes => {}
                Ok(_) => bad(
                    &e.id,
                    "decode then encode does not reproduce the bytes".into(),
                ),
                Err(err) => bad(&e.id, format!("failed to re-encode: {err}")),
            },
            Err(err) => bad(&e.id, format!("does not decode as LedgerCloseMeta: {err}")),
        }
        if e.observations.is_empty() {
            bad(&e.id, "entry declares no observations".into());
        }
        if e.sightings.is_empty() && e.cap.is_none() {
            bad(
                &e.id,
                "entry names neither a CAP nor a sighting; say why it exists".into(),
            );
        }
    }

    if regen {
        let fresh = generate_replay_entries()?;
        let fresh_ids: Vec<_> = fresh.iter().map(|(e, _)| e.id.clone()).collect();
        let have_ids: Vec<_> = manifest.entries.iter().map(|e| e.id.clone()).collect();
        if fresh_ids != have_ids {
            bad(
                "(manifest)",
                format!(
                    "entries differ from the generator: have {have_ids:?}, generator makes {fresh_ids:?}"
                ),
            );
        }
        for (e, bytes) in &fresh {
            match manifest.entries.iter().find(|m| m.id == e.id) {
                Some(m) if m == e => {
                    if fs::read(dir.join(&e.file)).ok().as_deref() != Some(bytes.as_slice()) {
                        bad(
                            &e.id,
                            "file differs from what the generator produces".into(),
                        );
                    }
                }
                Some(_) => bad(
                    &e.id,
                    "manifest record differs from what the generator produces".into(),
                ),
                None => {}
            }
        }
    }
    Ok(out)
}
