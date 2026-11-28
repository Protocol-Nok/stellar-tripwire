use std::fs;

use tripwire_replay::{generate_replay_entries, verify_dir, write_replay_corpus};

#[test]
fn generation_is_deterministic() {
    let a = generate_replay_entries().unwrap();
    let b = generate_replay_entries().unwrap();
    assert_eq!(a, b);
    assert!(!a.is_empty());
}

#[test]
fn a_fresh_corpus_verifies_including_regen() {
    let dir = tempfile::tempdir().unwrap();
    write_replay_corpus(dir.path()).unwrap();
    let findings = verify_dir(dir.path(), true).unwrap();
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn every_entry_says_why_it_exists_and_what_to_expect() {
    for (e, _) in generate_replay_entries().unwrap() {
        assert!(
            !e.sightings.is_empty() || e.cap.is_some(),
            "{} has no reason",
            e.id
        );
        assert!(!e.observations.is_empty(), "{} has no observations", e.id);
    }
}

#[test]
fn a_flipped_byte_is_caught() {
    let dir = tempfile::tempdir().unwrap();
    let m = write_replay_corpus(dir.path()).unwrap();
    let path = dir.path().join(&m.entries[0].file);
    let mut bytes = fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 0x01;
    fs::write(&path, bytes).unwrap();
    let findings = verify_dir(dir.path(), false).unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("sha256") || f.message.contains("decode")),
        "{findings:?}"
    );
}

#[test]
fn the_real_captured_transaction_is_actually_embedded() {
    // Not just "some bytes decode" -- the entry really does wrap the exact transaction
    // tripwire-corpus's captured-testnet-signed-payload-signer entry carries.
    use stellar_xdr::{LedgerCloseMeta, Limits, OperationBody, ReadXdr, TransactionEnvelope};

    let (entry, bytes) = generate_replay_entries()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let ledger = LedgerCloseMeta::from_xdr(&bytes, Limits::none()).unwrap();
    let LedgerCloseMeta::V1(v1) = ledger else {
        panic!("expected V1")
    };
    let stellar_xdr::GeneralizedTransactionSet::V1(set) = v1.tx_set;
    let stellar_xdr::TransactionPhase::V0(components) = &set.phases[0] else {
        panic!("expected a V0 phase")
    };
    let stellar_xdr::TxSetComponent::TxsetCompTxsMaybeDiscountedFee(c) = &components[0];
    let TransactionEnvelope::Tx(tx) = &c.txs[0] else {
        panic!("expected a v1 envelope")
    };
    let OperationBody::SetOptions(op) = &tx.tx.operations[0].body else {
        panic!("expected a SetOptions operation")
    };
    op.signer.as_ref().expect("a signer");
    let encoded = tripwire_corpus::generate_entries()
        .unwrap()
        .into_iter()
        .find(|(e, _)| e.id == "captured-testnet-signed-payload-signer")
        .unwrap()
        .0;
    let expected_strkey = encoded
        .checks
        .iter()
        .find_map(|c| match c {
            tripwire_corpus::Check::SignedPayloadStrkey { strkey } => Some(strkey.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(entry.observations.len(), 1);
    match &entry.observations[0] {
        tripwire_replay::Observation::SignerAdded { signer_key, .. } => {
            assert_eq!(*signer_key, expected_strkey);
        }
        other => panic!("expected a SignerAdded observation, got {other:?}"),
    }
}

#[test]
fn the_cap73_entry_reports_a_trustline_created_by_invoke_host_function() {
    // Not just "some ledger decodes" -- the entry's one operation really is
    // InvokeHostFunction (no ChangeTrust anywhere), yet the expected observation is a
    // trustline created, proving the harness tests exactly the envelope-vs-meta mismatch
    // wallet-backend#731 describes.
    use stellar_xdr::{LedgerCloseMeta, Limits, OperationBody, ReadXdr, TransactionEnvelope};

    let (entry, bytes) = generate_replay_entries()
        .unwrap()
        .into_iter()
        .find(|(e, _)| e.id == "cap73-sac-trust-ledger")
        .unwrap();
    let ledger = LedgerCloseMeta::from_xdr(&bytes, Limits::none()).unwrap();
    let LedgerCloseMeta::V1(v1) = ledger else {
        panic!("expected V1")
    };
    let stellar_xdr::GeneralizedTransactionSet::V1(set) = v1.tx_set;
    let stellar_xdr::TransactionPhase::V0(components) = &set.phases[0] else {
        panic!("expected a V0 phase")
    };
    let stellar_xdr::TxSetComponent::TxsetCompTxsMaybeDiscountedFee(c) = &components[0];
    let TransactionEnvelope::Tx(tx) = &c.txs[0] else {
        panic!("expected a v1 envelope")
    };
    assert!(
        matches!(
            tx.tx.operations[0].body,
            OperationBody::InvokeHostFunction(_)
        ),
        "expected an InvokeHostFunction operation"
    );

    assert_eq!(entry.observations.len(), 1);
    match &entry.observations[0] {
        tripwire_replay::Observation::TrustlineCreated { asset, .. } => {
            assert!(asset.starts_with("USD:G"), "unexpected asset: {asset}");
        }
        other => panic!("expected a TrustlineCreated observation, got {other:?}"),
    }
}
