//! Builds minimal, structurally valid synthetic `LedgerCloseMeta` values wrapping a
//! transaction, for the indexer replay harness. Unlike `tripwire-corpus`, which checks
//! whether an *SDK* decodes a shape correctly, this checks whether an *indexer* extracts
//! the right facts from that shape once it's embedded in a whole ledger -- the thing a
//! real ingestion pipeline actually consumes.
//!
//! A ledger built here is NOT a historical-chain-valid ledger: previous-ledger hashes,
//! bucket-list hashes and similar linkage fields are zeroed, not computed, because a
//! replay harness only needs an indexer's *own* decode/extraction logic to be exercised,
//! not consensus-chain validation. Where an entry wraps a real captured transaction, only
//! the transaction itself is real; the surrounding ledger is synthetic. Every entry's
//! `Source` says which.

mod entries;
mod manifest;
mod verify;

pub use entries::generate_replay_entries;
pub use manifest::{Manifest, Observation, ReplayEntry, SCHEMA_VERSION, Source, SourceKind};
pub use verify::{Finding, verify_dir, write_replay_corpus};

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(bytes))
}
