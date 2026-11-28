//! The generated replay entries.

use anyhow::{Context, Result, bail};
use stellar_xdr::{
    AccountEntry, AccountEntryExt, AccountId, ExtensionPoint, GeneralizedTransactionSet, Hash,
    LedgerCloseMeta, LedgerCloseMetaExt, LedgerCloseMetaV1, LedgerEntry, LedgerEntryChange,
    LedgerEntryChanges, LedgerEntryData, LedgerEntryExt, LedgerHeader, LedgerHeaderExt,
    LedgerHeaderHistoryEntry, LedgerHeaderHistoryEntryExt, Limits, MuxedAccount, OperationBody,
    OperationMeta, OperationResult, OperationResultTr, PublicKey, ReadXdr, SequenceNumber,
    SetOptionsResult, Signer, StellarValue, StellarValueExt, String32, Thresholds, TimePoint,
    TransactionEnvelope, TransactionMeta, TransactionMetaV3, TransactionPhase, TransactionResult,
    TransactionResultExt, TransactionResultMeta, TransactionResultPair, TransactionResultResult,
    TransactionSetV1, TxSetComponent, TxSetComponentTxsMaybeDiscountedFee, Uint256, VecM, WriteXdr,
};

use crate::manifest::{Observation, ReplayEntry, Source, SourceKind};
use crate::sha256_hex;

const WALLET_BACKEND: &str = "https://github.com/stellar/wallet-backend/issues";

fn zero_hash() -> Hash {
    Hash([0u8; 32])
}

/// A minimal, structurally valid ledger header. Linkage fields (previous-ledger hash,
/// bucket-list hash, tx-set result hash) are zeroed: this isn't a historical-chain-valid
/// ledger, only a structurally valid one an indexer's decoder will accept.
fn minimal_ledger_header(ledger_seq: u32, tx_set_hash: Hash) -> Result<LedgerHeader> {
    Ok(LedgerHeader {
        ledger_version: 26,
        previous_ledger_hash: zero_hash(),
        scp_value: StellarValue {
            tx_set_hash,
            close_time: TimePoint(0),
            upgrades: VecM::default(),
            ext: StellarValueExt::Basic,
        },
        tx_set_result_hash: zero_hash(),
        bucket_list_hash: zero_hash(),
        ledger_seq,
        total_coins: 0,
        fee_pool: 0,
        inflation_seq: 0,
        id_pool: 0,
        base_fee: 100,
        base_reserve: 5_000_000,
        max_tx_set_size: 1000,
        skip_list: [zero_hash(), zero_hash(), zero_hash(), zero_hash()],
        ext: LedgerHeaderExt::V0,
    })
}

/// Wraps `tx` as the sole transaction in a minimal, structurally valid synthetic ledger,
/// with `entry_changes` as the single operation's recorded ledger-entry changes (what a
/// correctly-implemented `stellar-core` would have produced).
fn minimal_ledger(
    ledger_seq: u32,
    tx: &TransactionEnvelope,
    entry_changes: Vec<LedgerEntryChange>,
) -> Result<LedgerCloseMeta> {
    let tx_hash = zero_hash(); // not a real transaction hash; nothing here checks it

    let tx_set = GeneralizedTransactionSet::V1(TransactionSetV1 {
        previous_ledger_hash: zero_hash(),
        phases: VecM::try_from(vec![TransactionPhase::V0(VecM::try_from(vec![
            TxSetComponent::TxsetCompTxsMaybeDiscountedFee(TxSetComponentTxsMaybeDiscountedFee {
                base_fee: None,
                txs: VecM::try_from(vec![tx.clone()])?,
            }),
        ])?)])?,
    });

    let op_meta = OperationMeta {
        changes: LedgerEntryChanges(VecM::try_from(entry_changes)?),
    };
    let tx_meta = TransactionMeta::V3(TransactionMetaV3 {
        ext: ExtensionPoint::V0,
        tx_changes_before: LedgerEntryChanges(VecM::default()),
        operations: VecM::try_from(vec![op_meta])?,
        tx_changes_after: LedgerEntryChanges(VecM::default()),
        soroban_meta: None,
    });

    let result_pair = TransactionResultPair {
        transaction_hash: tx_hash,
        result: TransactionResult {
            fee_charged: 100,
            result: TransactionResultResult::TxSuccess(VecM::try_from(vec![
                // A single no-op-shaped success result; the harness doesn't check this.
                OperationResult::OpInner(OperationResultTr::SetOptions(SetOptionsResult::Success)),
            ])?),
            ext: TransactionResultExt::V0,
        },
    };

    let tx_processing = vec![TransactionResultMeta {
        result: result_pair,
        fee_processing: LedgerEntryChanges(VecM::default()),
        tx_apply_processing: tx_meta,
    }];

    let header = minimal_ledger_header(ledger_seq, zero_hash())?;
    let header_bytes = header.to_xdr(Limits::none())?;
    let header_hash = Hash(sha256_32(&header_bytes));

    Ok(LedgerCloseMeta::V1(LedgerCloseMetaV1 {
        ext: LedgerCloseMetaExt::V0,
        ledger_header: LedgerHeaderHistoryEntry {
            hash: header_hash,
            header,
            ext: LedgerHeaderHistoryEntryExt::V0,
        },
        tx_set,
        tx_processing: VecM::try_from(tx_processing)?,
        upgrades_processing: VecM::default(),
        scp_info: VecM::default(),
        total_byte_size_of_live_soroban_state: 0,
        evicted_keys: VecM::default(),
        unused: VecM::default(),
    }))
}

fn sha256_32(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// The real captured CAP-40 signed-payload-signer transaction
/// (`captured-testnet-signed-payload-signer` in `tripwire-corpus`), wrapped in a minimal
/// synthetic ledger, with the ledger-entry change a correct `stellar-core` would have
/// produced: the signer added to the source account.
fn captured_signed_payload_signer_ledger() -> Result<(ReplayEntry, Vec<u8>)> {
    let generated = tripwire_corpus::generate_entries().context("generating base corpus")?;
    let (base_entry, raw_tx) = generated
        .into_iter()
        .find(|(e, _)| e.id == "captured-testnet-signed-payload-signer")
        .context("captured-testnet-signed-payload-signer missing from tripwire-corpus")?;

    let strkey = base_entry
        .checks
        .iter()
        .find_map(|c| match c {
            tripwire_corpus::Check::SignedPayloadStrkey { strkey } => Some(strkey.clone()),
            _ => None,
        })
        .context("base entry has no SignedPayloadStrkey check")?;

    let tx = TransactionEnvelope::from_xdr(&raw_tx, Limits::none())
        .context("decoding the base entry's transaction")?;
    let source_account_id = match &tx {
        TransactionEnvelope::Tx(v1) => match &v1.tx.source_account {
            MuxedAccount::Ed25519(k) => AccountId(PublicKey::PublicKeyTypeEd25519(k.clone())),
            MuxedAccount::MuxedEd25519(m) => {
                AccountId(PublicKey::PublicKeyTypeEd25519(m.ed25519.clone()))
            }
        },
        _ => bail!("expected a v1 transaction envelope"),
    };
    let signer_key = match &tx {
        TransactionEnvelope::Tx(v1) => match &v1.tx.operations[0].body {
            OperationBody::SetOptions(op) => op
                .signer
                .clone()
                .map(|s| s.key)
                .context("base entry's transaction has no SetOptions signer")?,
            _ => bail!("expected a SetOptions operation"),
        },
        _ => bail!("expected a v1 transaction envelope"),
    };

    // What a correct stellar-core would have changed: the account entry gains this signer.
    // The "before" account state is a minimal, otherwise-empty account -- this harness
    // only checks that the *signer* fact is reported, not the rest of the account.
    let before_account = AccountEntry {
        account_id: source_account_id.clone(),
        balance: 100_000_000,
        seq_num: SequenceNumber(1),
        num_sub_entries: 0,
        inflation_dest: None,
        flags: 0,
        home_domain: String32::default(),
        thresholds: Thresholds([1, 0, 0, 0]),
        signers: VecM::default(),
        ext: AccountEntryExt::V0,
    };
    let mut after_account = before_account.clone();
    after_account.signers = VecM::try_from(vec![Signer {
        key: signer_key.clone(),
        weight: 1,
    }])?;

    let changes = vec![
        LedgerEntryChange::State(LedgerEntry {
            last_modified_ledger_seq: 1,
            data: LedgerEntryData::Account(before_account),
            ext: LedgerEntryExt::V0,
        }),
        LedgerEntryChange::Updated(LedgerEntry {
            last_modified_ledger_seq: 2,
            data: LedgerEntryData::Account(after_account),
            ext: LedgerEntryExt::V0,
        }),
    ];

    let ledger = minimal_ledger(2, &tx, changes)?;
    let bytes = ledger.to_xdr(Limits::none())?;

    let entry = ReplayEntry {
        id: "captured-signed-payload-signer-ledger".to_string(),
        title: "A real CAP-40 signed-payload signer, added, inside a minimal ledger".to_string(),
        cap: Some("CAP-0040".to_string()),
        file: "entries/captured-signed-payload-signer-ledger.xdr".to_string(),
        sha256: sha256_hex(&bytes),
        source: Source {
            kind: SourceKind::RealTxSyntheticLedger,
            detail:
                "The transaction is real, captured from Stellar testnet (see tripwire-corpus's \
                 captured-testnet-signed-payload-signer entry for provenance). The surrounding \
                 ledger -- header, hashes, close time, the account's prior state -- is synthetic: \
                 structurally valid, not chain-valid. ledger_seq is a placeholder (2), not the \
                 real testnet ledger number."
                    .to_string(),
        },
        sightings: vec![
            format!("{WALLET_BACKEND}/716"),
            format!("{WALLET_BACKEND}/729"),
        ],
        observations: vec![Observation::SignerAdded {
            account_id: encode_account_id(&source_account_id)?,
            signer_key: strkey,
        }],
    };

    Ok((entry, bytes))
}

fn encode_account_id(id: &AccountId) -> Result<String> {
    let PublicKey::PublicKeyTypeEd25519(Uint256(bytes)) = &id.0;
    Ok(stellar_strkey::ed25519::PublicKey(*bytes)
        .to_string()
        .to_string())
}

pub fn generate_replay_entries() -> Result<Vec<(ReplayEntry, Vec<u8>)>> {
    Ok(vec![captured_signed_payload_signer_ledger()?])
}
