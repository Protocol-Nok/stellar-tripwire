# Java adapter

stellar-tripwire adapter for [`network.lightsail:stellar-sdk`](https://github.com/lightsail-network/java-stellar-sdk),
the actively maintained Java SDK (GitHub redirects `stellar/java-stellar-sdk`
here, same as `github.com/stellar/go` → `go-stellar-sdk`).

```sh
mvn package
tripwire run <corpus-dir> -- java -jar target/tripwire-adapter-java-0.1.0.jar
```

Note the `--`: `tripwire run`'s adapter command goes after it, so `-jar` isn't
mistaken for a `tripwire` flag. See `docs/adapter-protocol.md` in the main repo.

## Coverage

| Check | Status | Notes |
|---|---|---|
| `roundtrip` | implemented | `fromXdrByteArray` / `toXdrByteArray` on the generated types |
| `native_preserves_map_entries` | implemented, passes | `Scv.fromMap` keeps keys as typed `SCVal` objects rather than collapsing them to a native `String`/`Number` key, so this SDK's design has no entry-collision surface for this check to find |
| `muxed_ids_roundtrip` | implemented | `MuxedAccount.fromXdr(...).getMuxedId()` |
| `muxed_id_input` | **unsupported** | `MuxedAccount`'s constructor takes a `java.math.BigInteger` id, not a string — no string-parsing boundary in this SDK's API for the check to exercise, the same reason the Go adapter reports this one unsupported too |
| `liquidity_pool_asset_order` | implemented, **currently fails** | see below |
| `signed_payload_strkey` | implemented | `SignerKey.fromXdr(...).getEncodedSignerKey()` / `.fromEncodedSignerKey(...)` |

## A real, currently open bug this found

`network.lightsail:stellar-sdk@6.0.0`'s `Asset.compareTo` orders two assets'
issuers by their G-address **string**, not by the issuer's raw 32-byte public
key. The network compares raw bytes, so `LiquidityPool`'s constructor (which
uses `compareTo` to enforce `assetA < assetB`) wrongly rejects a valid,
correctly-ordered pair. Reported upstream:
[lightsail-network/java-stellar-sdk#849](https://github.com/lightsail-network/java-stellar-sdk/issues/849).

This is the same bug class as two already-fixed sibling-SDK bugs:
[js-stellar-sdk#1766](https://github.com/stellar/js-stellar-sdk/issues/1766)
and `go-stellar-sdk`'s pre-`0.7.2` `Asset.LessThan`
([PR #5974](https://github.com/stellar/go-stellar-sdk/pull/5974)) — except
this one is live in the current release, not a historical pre-fix version.

`scripts/prove-detection-java.sh` (at the repo root) asserts this *current*
state rather than comparing two releases, since there's no fixed release yet
to compare against. It's meant to start failing once upstream ships a fix —
that's the signal to update this README and the script, not a bug to
silence.
