//! Manifest types and the deterministic generator for the stellar-tripwire corpus.
//!
//! A corpus is a directory of raw XDR files plus a `manifest.json`. The manifest is the
//! language-neutral contract: every SDK adapter reads it, decodes each entry's bytes with
//! its own implementation, and reports whether it agrees with the expectations recorded here.
//! Expectations are computed by this crate from the official `stellar-xdr` types, never typed
//! by hand, and `verify` recomputes them.

mod entries;
mod verify;

use serde::{Deserialize, Serialize};

pub use entries::generate_entries;
pub use verify::{Finding, verify_dir, write_corpus};

/// Manifest schema version. Bump on any breaking change to the JSON shape.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub schema: u32,
    /// Version of the `stellar-xdr` crate that produced and verified the bytes.
    pub xdr_crate: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    /// Stable identifier, also the file stem under `entries/`.
    pub id: String,
    pub title: String,
    /// The CAP that introduced or changed the shape, when one applies.
    pub cap: Option<String>,
    /// Name of the XDR type the bytes decode as, e.g. `TransactionEnvelope`.
    pub xdr_type: String,
    /// Path of the raw XDR file, relative to the corpus root.
    pub file: String,
    /// Lower-case hex SHA-256 of the file.
    pub sha256: String,
    pub source: Source,
    /// Public issues or pull requests where this shape broke a real implementation.
    pub sightings: Vec<String>,
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    /// `synthetic` entries are built by the generator. `captured` entries come from a
    /// network ledger and must name the ledger in `detail`.
    pub kind: SourceKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Synthetic,
    Captured,
}

/// One assertion an implementation must satisfy for an entry. The set of kinds is small and
/// closed on purpose: an adapter either implements a kind or reports it as unsupported.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Check {
    /// Decoding the bytes and encoding the result must reproduce the bytes exactly.
    Roundtrip,
    /// Converting the decoded `ScVal::Map` into the SDK's native value must either keep all
    /// `entries` entries or fail loudly. Returning fewer entries without an error is a defect.
    NativePreservesMapEntries { entries: u32 },
    /// Every muxed account in the transaction must round-trip through the SDK's muxed type
    /// with exactly these decimal IDs, in order of appearance (source, then operations).
    MuxedIdsRoundtrip { ids: Vec<String> },
    /// Strictness of the SDK's muxed-ID string input. `accept` values must be taken as the
    /// same number; every `reject` value must be refused. Leading zeros are deliberately
    /// absent: implementations legitimately differ and the network does not decide it.
    MuxedIdInput {
        accept: Vec<String>,
        reject: Vec<String>,
    },
    /// Building a constant-product pool asset from the two assets, in the order the network
    /// requires, must be accepted and produce this pool ID (lower-case hex).
    LiquidityPoolAssetOrder { fee: i32, pool_id: String },
    /// The signer key must encode to this `P...` strkey and decode back to the same bytes.
    SignedPayloadStrkey { strkey: String },
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}
