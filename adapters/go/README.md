# Go adapter

stellar-tripwire adapter for [`github.com/stellar/go-stellar-sdk`](https://github.com/stellar/go-stellar-sdk),
the actively maintained Go SDK (`github.com/stellar/go` is deprecated).

```sh
go build -o tripwire-go-adapter .
tripwire run <corpus-dir> -- ./tripwire-go-adapter
```

## Coverage

| Check | Status | Notes |
|---|---|---|
| `roundtrip` | implemented | `UnmarshalBinary` / `MarshalBinary` on the generated types |
| `muxed_ids_roundtrip` | implemented | `MuxedAccount.GetId()` |
| `muxed_id_input` | **unsupported** | `MuxedAccountFromAccountId` takes a native `uint64`, not a string — there's no string-parsing boundary in this SDK's API for the check to exercise. The type system rules out the whole bug class this check looks for. |
| `liquidity_pool_asset_order` | implemented | `xdr.NewPoolId` |
| `signed_payload_strkey` | implemented | `SignerKey.Address()` / `.SetAddress()` |
| `native_preserves_map_entries` | **unsupported** | this SDK exposes no generic `ScVal`-to-native-value conversion (Go indexer code in this repo handles specific known event shapes, not a general conversion utility) |

Two checks come back `unsupported`, honestly: no code here claims to test something
the SDK has no API for.

## A real bug this catches

`github.com/stellar/go-stellar-sdk@v0.7.1`'s `xdr.NewPoolId` ordered assets by
strkey text via `Asset.LessThan`, not by raw issuer bytes — the same class of
bug as [js-stellar-sdk#1766](https://github.com/stellar/js-stellar-sdk/issues/1766).
Fixed in `v0.7.2` ([PR #5974](https://github.com/stellar/go-stellar-sdk/pull/5974)).
`scripts/prove-detection-go.sh` (at the repo root) proves this: it builds this
adapter against both releases and requires the known failure on `v0.7.1`, none
on current.
