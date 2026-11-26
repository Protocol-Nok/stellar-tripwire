//! Writing a corpus to disk and checking one that is already there.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use stellar_xdr::{ChangeTrustAsset, Limits, ReadXdr, ScVal, TransactionEnvelope, WriteXdr};

use crate::{Manifest, SCHEMA_VERSION, generate_entries, sha256_hex};

/// One problem found while verifying a corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub entry: String,
    pub message: String,
}

fn xdr_crate_label() -> String {
    let v = stellar_xdr::VERSION;
    format!("stellar-xdr {} (xdr {})", v.pkg, v.xdr.trim())
}

/// Generate the corpus and write `manifest.json` plus `entries/*.xdr` under `dir`.
pub fn write_corpus(dir: &Path) -> Result<Manifest> {
    fs::create_dir_all(dir.join("entries"))?;
    let generated = generate_entries()?;
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

/// Decode `bytes` as the named XDR type and encode it again.
fn reencode(xdr_type: &str, bytes: &[u8]) -> Result<Vec<u8>> {
    let limits = Limits::none();
    Ok(match xdr_type {
        "ScVal" => ScVal::from_xdr(bytes, limits.clone())?.to_xdr(limits)?,
        "TransactionEnvelope" => {
            TransactionEnvelope::from_xdr(bytes, limits.clone())?.to_xdr(limits)?
        }
        "ChangeTrustAsset" => ChangeTrustAsset::from_xdr(bytes, limits.clone())?.to_xdr(limits)?,
        other => anyhow::bail!("unsupported xdr_type {other:?}"),
    })
}

/// Check a corpus directory. With `regen`, also require that the committed bytes and manifest
/// are exactly what the generator produces today.
pub fn verify_dir(dir: &Path, regen: bool) -> Result<Vec<Finding>> {
    let raw = fs::read_to_string(dir.join("manifest.json")).context("reading manifest.json")?;
    let manifest: Manifest = serde_json::from_str(&raw).context("parsing manifest.json")?;
    let mut out = Vec::new();
    let mut bad = |entry: &str, message: String| {
        out.push(Finding {
            entry: entry.to_string(),
            message,
        });
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
    let mut seen = std::collections::BTreeSet::new();
    for e in &manifest.entries {
        if !seen.insert(e.id.clone()) {
            bad(&e.id, "duplicate entry id".into());
        }
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
        match reencode(&e.xdr_type, &bytes) {
            Ok(again) if again == bytes => {}
            Ok(_) => bad(
                &e.id,
                "decode then encode does not reproduce the bytes".into(),
            ),
            Err(err) => bad(&e.id, format!("does not decode as {}: {err}", e.xdr_type)),
        }
        if e.checks.is_empty() {
            bad(&e.id, "entry declares no checks".into());
        }
        if e.sightings.is_empty() && e.cap.is_none() {
            bad(
                &e.id,
                "entry names neither a CAP nor a sighting; say why it exists".into(),
            );
        }
    }

    if regen {
        let fresh = generate_entries()?;
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
