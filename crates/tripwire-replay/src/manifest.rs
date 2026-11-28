use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub schema: u32,
    pub xdr_crate: String,
    pub entries: Vec<ReplayEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayEntry {
    pub id: String,
    pub title: String,
    pub cap: Option<String>,
    /// Path of the raw `LedgerCloseMeta` XDR file, relative to the corpus root.
    pub file: String,
    pub sha256: String,
    pub source: Source,
    pub sightings: Vec<String>,
    /// Facts an indexer that extracts this ledger correctly must report. See
    /// `docs/replay-protocol.md` in the main repo for exactly how.
    pub observations: Vec<Observation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Source {
    pub kind: SourceKind,
    /// What's real and what's synthetic about this ledger, stated explicitly -- never
    /// just "captured" or "synthetic" alone, since a replay entry can wrap a real
    /// transaction inside an otherwise-synthetic ledger.
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// The transaction is real (captured from a real ledger); the surrounding ledger
    /// (header, hashes, close time) is synthetic.
    RealTxSyntheticLedger,
    /// Everything, including the transaction, is constructed.
    Synthetic,
}

/// A fact a correct indexer must report having seen. Closed set, like `tripwire-corpus`'s
/// `Check`: an indexer either implements a kind or reports it unsupported, and a kind it
/// never mentions counts as dropped, not passed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Observation {
    /// A signer was added to `account_id`, with signer key `signer_key` (a strkey,
    /// possibly a long `P...` CAP-40 key -- the point of this observation kind).
    SignerAdded {
        account_id: String,
        signer_key: String,
    },
    /// A trustline for `asset` (canonical `CODE:ISSUER`, or `"native"`) was created for
    /// `account_id`. The point: this can happen via a Soroban `InvokeHostFunction` call
    /// into the Stellar Asset Contract's `trust()` (CAP-0073), not only via a classic
    /// `ChangeTrust` operation -- an indexer that only looks for the latter misses it.
    TrustlineCreated { account_id: String, asset: String },
}
