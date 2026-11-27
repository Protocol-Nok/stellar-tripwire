// stellar-tripwire adapter for github.com/stellar/go-stellar-sdk (the actively
// maintained Go SDK; github.com/stellar/go is deprecated as of this writing).
//
//	go run . <corpus-dir>
//
// Protocol: ../../docs/adapter-protocol.md. One JSON object per line on stdout.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"math/big"
	"os"
	"path/filepath"
	"runtime/debug"

	"github.com/stellar/go-stellar-sdk/xdr"
)

type manifest struct {
	Entries []entry `json:"entries"`
}

type entry struct {
	ID      string  `json:"id"`
	XdrType string  `json:"xdr_type"`
	File    string  `json:"file"`
	Checks  []check `json:"checks"`
}

// check carries every field any check kind might use; unused fields are zero.
type check struct {
	Kind    string   `json:"kind"`
	Entries uint32   `json:"entries"`
	Ids     []string `json:"ids"`
	Accept  []string `json:"accept"`
	Reject  []string `json:"reject"`
	Fee     int32    `json:"fee"`
	PoolID  string   `json:"pool_id"`
	Strkey  string   `json:"strkey"`
}

type result struct {
	Entry  string `json:"entry"`
	Check  string `json:"check"`
	Status string `json:"status"`
	Detail string `json:"detail"`
}

func emit(v any) {
	b, _ := json.Marshal(v)
	fmt.Println(string(b))
}

func pass(detail string) result        { return result{Status: "pass", Detail: detail} }
func fail(detail string) result        { return result{Status: "fail", Detail: detail} }
func unsupported(detail string) result { return result{Status: "unsupported", Detail: detail} }

func sdkVersion() string {
	if bi, ok := debug.ReadBuildInfo(); ok {
		for _, dep := range bi.Deps {
			if dep.Path == "github.com/stellar/go-stellar-sdk" {
				return "github.com/stellar/go-stellar-sdk " + dep.Version
			}
		}
	}
	return "github.com/stellar/go-stellar-sdk (version unknown)"
}

// decodeTx fully decodes a v1 transaction envelope; everything we check lives there.
func decodeTx(raw []byte) (*xdr.Transaction, error) {
	var env xdr.TransactionEnvelope
	if err := env.UnmarshalBinary(raw); err != nil {
		return nil, err
	}
	if env.V1 == nil {
		return nil, fmt.Errorf("not a v1 transaction envelope")
	}
	return &env.V1.Tx, nil
}

// muxedAccounts returns the source account, then each payment destination, in order.
func muxedAccounts(tx *xdr.Transaction) []xdr.MuxedAccount {
	out := []xdr.MuxedAccount{tx.SourceAccount}
	for _, op := range tx.Operations {
		if op.Body.PaymentOp != nil {
			out = append(out, op.Body.PaymentOp.Destination)
		}
	}
	muxedOnly := out[:0]
	for _, m := range out {
		if m.Type == xdr.CryptoKeyTypeKeyTypeMuxedEd25519 {
			muxedOnly = append(muxedOnly, m)
		}
	}
	return muxedOnly
}

func checkRoundtrip(e entry, raw []byte) result {
	var again []byte
	var err error
	switch e.XdrType {
	case "ScVal":
		var v xdr.ScVal
		if err = v.UnmarshalBinary(raw); err == nil {
			again, err = v.MarshalBinary()
		}
	case "TransactionEnvelope":
		var v xdr.TransactionEnvelope
		if err = v.UnmarshalBinary(raw); err == nil {
			again, err = v.MarshalBinary()
		}
	case "ChangeTrustAsset":
		var v xdr.ChangeTrustAsset
		if err = v.UnmarshalBinary(raw); err == nil {
			again, err = v.MarshalBinary()
		}
	default:
		return unsupported("unknown xdr_type " + e.XdrType)
	}
	if err != nil {
		return fail("does not decode as " + e.XdrType + ": " + err.Error())
	}
	if !bytes.Equal(again, raw) {
		return fail("decode then encode changed the bytes")
	}
	return pass("")
}

func checkMuxedIdsRoundtrip(raw []byte, c check) result {
	tx, err := decodeTx(raw)
	if err != nil {
		return fail(err.Error())
	}
	found := muxedAccounts(tx)
	if len(found) != len(c.Ids) {
		return fail(fmt.Sprintf("found %d muxed accounts, expected %d", len(found), len(c.Ids)))
	}
	var bad []string
	for i, m := range found {
		id, err := m.GetId()
		if err != nil {
			bad = append(bad, fmt.Sprintf("#%d: GetId failed: %v", i, err))
			continue
		}
		got := new(big.Int).SetUint64(id).String()
		if got != c.Ids[i] {
			bad = append(bad, fmt.Sprintf("#%d: expected %s, SDK says %s", i, c.Ids[i], got))
		}
	}
	if len(bad) > 0 {
		return fail(fmt.Sprintf("%v", bad))
	}
	return pass("")
}

func checkLiquidityPoolAssetOrder(raw []byte, c check) result {
	var cta xdr.ChangeTrustAsset
	if err := cta.UnmarshalBinary(raw); err != nil {
		return fail(err.Error())
	}
	if cta.LiquidityPool == nil || cta.LiquidityPool.ConstantProduct == nil {
		return fail("entry is not a constant-product pool asset")
	}
	p := cta.LiquidityPool.ConstantProduct
	id, err := xdr.NewPoolId(p.AssetA, p.AssetB, xdr.Int32(c.Fee))
	if err != nil {
		return fail("refused an asset pair the network accepts: " + err.Error())
	}
	got := xdr.Hash(id).HexString()
	if got != c.PoolID {
		return fail(fmt.Sprintf("pool id %s, expected %s", got, c.PoolID))
	}
	return pass("")
}

func checkSignedPayloadStrkey(raw []byte, c check) result {
	tx, err := decodeTx(raw)
	if err != nil {
		return fail(err.Error())
	}
	if len(tx.Operations) == 0 || tx.Operations[0].Body.SetOptionsOp == nil ||
		tx.Operations[0].Body.SetOptionsOp.Signer == nil {
		return fail("entry has no SetOptions signer")
	}
	key := tx.Operations[0].Body.SetOptionsOp.Signer.Key
	got := key.Address()
	if got != c.Strkey {
		return fail(fmt.Sprintf("encoded %s, expected %s", got, c.Strkey))
	}
	var back xdr.SignerKey
	if err := back.SetAddress(got); err != nil {
		return fail("strkey did not decode: " + err.Error())
	}
	origBytes, err1 := key.MarshalBinary()
	backBytes, err2 := back.MarshalBinary()
	if err1 != nil || err2 != nil || !bytes.Equal(origBytes, backBytes) {
		return fail("strkey did not decode back to the same signer key")
	}
	return pass("")
}

func runCheck(e entry, raw []byte, c check) result {
	switch c.Kind {
	case "roundtrip":
		return checkRoundtrip(e, raw)
	case "native_preserves_map_entries":
		return unsupported("this SDK exposes no generic ScVal-to-native-value conversion to test")
	case "muxed_ids_roundtrip":
		return checkMuxedIdsRoundtrip(raw, c)
	case "muxed_id_input":
		return unsupported("MuxedAccountFromAccountId takes a native uint64, not a string; " +
			"there is no string-parsing boundary in this SDK's API for this check to exercise")
	case "liquidity_pool_asset_order":
		return checkLiquidityPoolAssetOrder(raw, c)
	case "signed_payload_strkey":
		return checkSignedPayloadStrkey(raw, c)
	default:
		return unsupported("check kind not implemented by this adapter")
	}
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: adapter <corpus-dir>")
		os.Exit(2)
	}
	dir := os.Args[1]

	emit(map[string]string{"adapter": "go", "version": sdkVersion()})

	mb, err := os.ReadFile(filepath.Join(dir, "manifest.json"))
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	var m manifest
	if err := json.Unmarshal(mb, &m); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}

	for _, e := range m.Entries {
		raw, err := os.ReadFile(filepath.Join(dir, e.File))
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		for _, c := range e.Checks {
			r := func() (r result) {
				defer func() {
					if rec := recover(); rec != nil {
						r = fail(fmt.Sprintf("panicked: %v", rec))
					}
				}()
				return runCheck(e, raw, c)
			}()
			r.Entry = e.ID
			r.Check = c.Kind
			emit(r)
		}
	}
}
