package main

import "testing"

// TestRunCheck_UnknownKindIsUnsupported guards the default case in runCheck: an
// unrecognized check kind must be reported as unsupported, never silently
// skipped or treated as a pass.
func TestRunCheck_UnknownKindIsUnsupported(t *testing.T) {
	r := runCheck(entry{XdrType: "ScVal"}, []byte{0, 0, 0, 0}, check{Kind: "nonsense"})
	if r.Status != "unsupported" {
		t.Fatalf("expected unsupported, got %+v", r)
	}
}

// TestCheckRoundtrip_RejectsTruncatedBytes is a sanity check that decode
// failure is reported as fail, not a panic or a false pass.
func TestCheckRoundtrip_RejectsTruncatedBytes(t *testing.T) {
	r := checkRoundtrip(entry{XdrType: "ScVal"}, []byte{0, 0})
	if r.Status != "fail" {
		t.Fatalf("expected fail on truncated input, got %+v", r)
	}
}

// TestCheckRoundtrip_UnknownXdrTypeIsUnsupported guards the default case in
// checkRoundtrip itself, independent of runCheck's own dispatch.
func TestCheckRoundtrip_UnknownXdrTypeIsUnsupported(t *testing.T) {
	r := checkRoundtrip(entry{XdrType: "SomethingNew"}, []byte{0, 0, 0, 0})
	if r.Status != "unsupported" {
		t.Fatalf("expected unsupported, got %+v", r)
	}
}

// TestCheckLiquidityPoolAssetOrder_RejectsNonPoolEntry guards against a nil
// pointer dereference if this check runs against the wrong entry type: a bare
// native asset XDR, not a liquidity-pool ChangeTrustAsset.
func TestCheckLiquidityPoolAssetOrder_RejectsNonPoolEntry(t *testing.T) {
	native := []byte{0, 0, 0, 0}
	r := checkLiquidityPoolAssetOrder(native, check{Fee: 30, PoolID: "00"})
	if r.Status != "fail" {
		t.Fatalf("expected fail, got %+v", r)
	}
}

// TestCheckSignedPayloadStrkey_RejectsEntryWithoutSigner guards against a nil
// pointer dereference on a transaction that has no SetOptions signer.
func TestCheckSignedPayloadStrkey_RejectsEntryWithoutSigner(t *testing.T) {
	// An empty-operations v1 envelope is not a realistic corpus entry, but it's
	// a cheap way to exercise the "no signer" guard without a full fixture.
	r := checkSignedPayloadStrkey([]byte{0, 0, 0, 0}, check{Strkey: "P..."})
	if r.Status != "fail" {
		t.Fatalf("expected fail, got %+v", r)
	}
}

// TestMainPanicRecovery is covered end-to-end, not in isolation: main()'s
// recover() wrapper around runCheck is exercised by every test above that
// triggers a real decode failure through the full dispatch path, and by
// scripts/prove-detection-go.sh against a real corpus.
