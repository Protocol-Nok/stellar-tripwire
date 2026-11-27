//! The generated entries. Each one is tied to a public sighting where a real implementation
//! broke, so a reviewer can follow the link and see why the shape is in the corpus.
//!
//! Generation is fully deterministic: no clocks, no randomness, so `verify --regen` can prove
//! the committed bytes are exactly what this code produces.

use anyhow::Result;
use stellar_xdr::{
    AccountId, AlphaNum4, Asset, AssetCode4, BytesM, ChangeTrustAsset, Limits,
    LiquidityPoolConstantProductParameters, LiquidityPoolParameters, Memo, MuxedAccount,
    MuxedAccountMed25519, Operation, OperationBody, PaymentOp, Preconditions, PublicKey, ScMap,
    ScMapEntry, ScSymbol, ScVal, SequenceNumber, SetOptionsOp, Signer, SignerKey,
    SignerKeyEd25519SignedPayload, StringM, Transaction, TransactionEnvelope, TransactionExt,
    TransactionV1Envelope, Uint256, VecM, WriteXdr,
};

use crate::{Check, Entry, Source, SourceKind, sha256_hex};

const JS_SDK: &str = "https://github.com/stellar/js-stellar-sdk/issues";
const WALLET_BACKEND: &str = "https://github.com/stellar/wallet-backend/issues";

/// A 32-byte key whose first byte is `first` and the rest a fixed fill. Not a valid curve
/// point on purpose: XDR does not check, and nothing here signs anything.
fn key(first: u8, fill: u8) -> Uint256 {
    let mut b = [fill; 32];
    b[0] = first;
    Uint256(b)
}

fn account_id(k: Uint256) -> AccountId {
    AccountId(PublicKey::PublicKeyTypeEd25519(k))
}

fn envelope(source: MuxedAccount, ops: Vec<Operation>) -> Result<TransactionEnvelope> {
    Ok(TransactionEnvelope::Tx(TransactionV1Envelope {
        tx: Transaction {
            source_account: source,
            fee: 100,
            seq_num: SequenceNumber(1),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: VecM::try_from(ops)?,
            ext: TransactionExt::V0,
        },
        signatures: VecM::default(),
    }))
}

fn to_bytes<T: WriteXdr>(v: &T) -> Result<Vec<u8>> {
    Ok(v.to_xdr(Limits::none())?)
}

struct Built {
    entry: Entry,
    bytes: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
fn built(
    id: &str,
    title: &str,
    cap: Option<&str>,
    xdr_type: &str,
    bytes: Vec<u8>,
    detail: &str,
    sightings: &[&str],
    checks: Vec<Check>,
) -> Built {
    built_with_source(
        id,
        title,
        cap,
        xdr_type,
        bytes,
        SourceKind::Synthetic,
        detail,
        sightings,
        checks,
    )
}

/// As `built`, but for an entry taken verbatim from a real ledger rather than
/// constructed from the official types. `detail` should say exactly where it came
/// from: network, ledger, transaction hash, and how it was captured.
#[allow(clippy::too_many_arguments)]
fn built_with_source(
    id: &str,
    title: &str,
    cap: Option<&str>,
    xdr_type: &str,
    bytes: Vec<u8>,
    kind: SourceKind,
    detail: &str,
    sightings: &[&str],
    checks: Vec<Check>,
) -> Built {
    Built {
        entry: Entry {
            id: id.to_string(),
            title: title.to_string(),
            cap: cap.map(str::to_string),
            xdr_type: xdr_type.to_string(),
            file: format!("entries/{id}.xdr"),
            sha256: sha256_hex(&bytes),
            source: Source {
                kind,
                detail: detail.to_string(),
            },
            sightings: sightings.iter().map(|s| s.to_string()).collect(),
            checks,
        },
        bytes,
    }
}

/// Map keys of different types that print the same: `u32 1` and `symbol "1"`.
fn scval_map_key_collision() -> Result<Built> {
    let map = ScVal::Map(Some(ScMap(VecM::try_from(vec![
        ScMapEntry {
            key: ScVal::U32(1),
            val: ScVal::U32(10),
        },
        ScMapEntry {
            key: ScVal::Symbol(ScSymbol(StringM::try_from("1")?)),
            val: ScVal::U32(20),
        },
    ])?)));
    Ok(built(
        "scval-map-key-collision",
        "ScVal map whose keys of different types convert to the same native key",
        None,
        "ScVal",
        to_bytes(&map)?,
        "u32 1 and symbol \"1\" are distinct ScVal keys; a string-keyed native object cannot hold both",
        &[&format!("{JS_SDK}/1789")],
        vec![
            Check::Roundtrip,
            Check::NativePreservesMapEntries { entries: 2 },
        ],
    ))
}

/// Muxed source and destination, one with the largest possible ID.
fn muxed_ids() -> Result<Built> {
    let src = MuxedAccount::MuxedEd25519(MuxedAccountMed25519 {
        id: 16,
        ed25519: key(0x3F, 0x11),
    });
    let dst = MuxedAccount::MuxedEd25519(MuxedAccountMed25519 {
        id: u64::MAX,
        ed25519: key(0x40, 0x22),
    });
    let op = Operation {
        source_account: None,
        body: OperationBody::Payment(PaymentOp {
            destination: dst,
            asset: Asset::Native,
            amount: 1,
        }),
    };
    Ok(built(
        "muxed-ids-boundary",
        "Muxed source (id 16) and muxed destination (id u64::MAX)",
        Some("CAP-0027"),
        "TransactionEnvelope",
        to_bytes(&envelope(src, vec![op])?)?,
        "ids 16 and 18446744073709551615 exercise the full unsigned 64-bit range",
        &[&format!("{JS_SDK}/1785"), &format!("{WALLET_BACKEND}/716")],
        vec![
            Check::Roundtrip,
            Check::MuxedIdsRoundtrip {
                ids: vec!["16".into(), "18446744073709551615".into()],
            },
            Check::MuxedIdInput {
                accept: vec!["0".into(), "16".into(), "18446744073709551615".into()],
                reject: vec![
                    "0x10".into(),
                    " 16 ".into(),
                    "".into(),
                    "-1".into(),
                    "1e3".into(),
                    "18446744073709551616".into(),
                ],
            },
        ],
    ))
}

/// A pool share trustline whose issuers sort one way as strkey text and the other as bytes.
fn pool_issuer_order() -> Result<Built> {
    // Same code, different issuers. Base32 puts '2'..'7' before 'A'..'Z' in text, but they are
    // the *larger* 5-bit values, so text order and byte order disagree for these two keys.
    let issuer_low = account_id(key(0x00, 0x11)); // strkey "GAA..."; smaller as bytes
    let issuer_high = account_id(key(0x3E, 0x11)); // strkey "GA7..."; larger as bytes
    let usd = |issuer| {
        Asset::CreditAlphanum4(AlphaNum4 {
            asset_code: AssetCode4(*b"USD\0"),
            issuer,
        })
    };
    // The network requires asset_a < asset_b, comparing issuer bytes.
    let params = LiquidityPoolParameters::LiquidityPoolConstantProduct(
        LiquidityPoolConstantProductParameters {
            asset_a: usd(issuer_low),
            asset_b: usd(issuer_high),
            fee: 30,
        },
    );
    let pool_id = sha256_hex(&to_bytes(&params)?);
    Ok(built(
        "lp-asset-issuer-order",
        "Pool share asset whose issuers order differently as strkey text and as XDR bytes",
        Some("CAP-0038"),
        "ChangeTrustAsset",
        to_bytes(&ChangeTrustAsset::PoolShare(params))?,
        "valid for the network: asset_a issuer bytes < asset_b issuer bytes",
        &[&format!("{JS_SDK}/1766")],
        vec![
            Check::Roundtrip,
            Check::LiquidityPoolAssetOrder { fee: 30, pool_id },
        ],
    ))
}

/// A CAP-40 signed-payload signer, with a payload that needs XDR padding.
fn signed_payload_signer() -> Result<Built> {
    let ed = key(0x07, 0x33);
    let payload = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x01]; // 5 bytes -> padded to 8 in XDR
    let strkey = stellar_strkey::ed25519::SignedPayload {
        ed25519: ed.0,
        payload: heapless::Vec::from_slice(&payload)
            .map_err(|_| anyhow::anyhow!("payload longer than 64 bytes"))?,
    }
    .to_string()
    .to_string();
    let signer = Signer {
        key: SignerKey::Ed25519SignedPayload(SignerKeyEd25519SignedPayload {
            ed25519: ed,
            payload: BytesM::try_from(payload)?,
        }),
        weight: 1,
    };
    let op = Operation {
        source_account: None,
        body: OperationBody::SetOptions(SetOptionsOp {
            inflation_dest: None,
            clear_flags: None,
            set_flags: None,
            master_weight: None,
            low_threshold: None,
            med_threshold: None,
            high_threshold: None,
            home_domain: None,
            signer: Some(signer),
        }),
    };
    Ok(built(
        "set-options-signed-payload-signer",
        "SetOptions adding a CAP-40 signed-payload signer with a 5-byte payload",
        Some("CAP-0040"),
        "TransactionEnvelope",
        to_bytes(&envelope(MuxedAccount::Ed25519(key(0x01, 0x44)), vec![op])?)?,
        "the P... strkey is 40-100 bytes, longer than any other key an indexer stores",
        &[
            &format!("{WALLET_BACKEND}/729"),
            &format!("{WALLET_BACKEND}/716"),
        ],
        vec![Check::Roundtrip, Check::SignedPayloadStrkey { strkey }],
    ))
}

/// A real CAP-40 signed-payload signer, captured from Stellar testnet. This is the
/// exact ledger wallet-backend#716 names as already containing such a signer; the
/// synthetic `signed_payload_signer` entry above exercises a constructed example,
/// this one proves it is not a hypothetical shape.
fn captured_signed_payload_signer() -> Result<Built> {
    const RAW: &[u8] = include_bytes!("../captured/testnet-cap40-signed-payload-tx.xdr");
    Ok(built_with_source(
        "captured-testnet-signed-payload-signer",
        "A real testnet SetOptions adding a CAP-40 signed-payload signer",
        Some("CAP-0040"),
        "TransactionEnvelope",
        RAW.to_vec(),
        SourceKind::Captured,
        "Stellar testnet, ledger 4437357, tx d9f78749cd01297cd5f22c7f2c798c9eec93d56f399e1c4cc2f8977ca52b9fed, \
         captured 2026-11-27 via GET horizon-testnet.stellar.org/transactions/<hash>, field envelope_xdr, \
         base64-decoded and checked in verbatim",
        &[
            &format!("{WALLET_BACKEND}/716"),
            &format!("{WALLET_BACKEND}/729"),
        ],
        vec![
            Check::Roundtrip,
            Check::SignedPayloadStrkey {
                strkey: "PBCK2RYYLIYOSHBEL2FQLAVRSHRH3GP3ID476SGKBV7BEZHMQ6NSAAAAAAB55LN6AANMS"
                    .into(),
            },
        ],
    ))
}

/// All entries with their raw bytes, in a stable order.
pub fn generate_entries() -> Result<Vec<(Entry, Vec<u8>)>> {
    let all = vec![
        scval_map_key_collision()?,
        muxed_ids()?,
        pool_issuer_order()?,
        signed_payload_signer()?,
        captured_signed_payload_signer()?,
    ];
    Ok(all.into_iter().map(|b| (b.entry, b.bytes)).collect())
}
