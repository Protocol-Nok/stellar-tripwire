# stellar-tripwire

[![CI](https://github.com/Protocol-Nok/stellar-tripwire/actions/workflows/ci.yml/badge.svg)](https://github.com/Protocol-Nok/stellar-tripwire/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

**Replays rare-but-valid Stellar ledger shapes through SDKs and indexers, and reports where they disagree with the network and with each other.**

[Corpus](https://github.com/Protocol-Nok/stellar-tripwire-corpus) | [GitHub Action](https://github.com/Protocol-Nok/stellar-tripwire-action) | [Adapter protocol](docs/adapter-protocol.md) | [Check kinds](docs/checks.md)

Every application that reads the ledger decodes XDR, and rare shapes keep breaking those readers because nobody tested them. A fix lands in one SDK and never reaches the others. Tripwire turns each of those bugs into a regression test that every implementation can run.

> Status: early. Three adapters (JavaScript, Go, Java), five corpus entries (one captured from a real ledger, four synthetic), six check kinds, and the first of three planned linter rules. The indexer replay harness and a Python adapter are not built yet; see [Roadmap](ROADMAP.md).

## What it already catches

Four synthetic entries, each tied to a public sighting. Run against `@stellar/stellar-sdk` releases from before the fixes:

| Entry | Sighting | 17.2.0 (pre-fix) | 17.4.0 |
|---|---|---|---|
| `scval-map-key-collision` | [js-stellar-sdk#1789](https://github.com/stellar/js-stellar-sdk/issues/1789) | **FAIL**: 2 entries in the XDR, 1 after `scValToNative`, no error | pass |
| `muxed-ids-boundary` | [js-stellar-sdk#1785](https://github.com/stellar/js-stellar-sdk/issues/1785) | **FAIL**: accepts `"0x10"`, `" 16 "`, `""` as muxed IDs | pass |
| `lp-asset-issuer-order` | [js-stellar-sdk#1766](https://github.com/stellar/js-stellar-sdk/issues/1766) | **FAIL**: rejects a pool asset pair the network accepts | pass |
| `set-options-signed-payload-signer` | [wallet-backend#729](https://github.com/stellar/wallet-backend/issues/729) | pass | pass |

`scripts/prove-detection.sh` re-runs this in CI, so a check that stops detecting its bug fails the build.

A fifth entry, `captured-testnet-signed-payload-signer`, is not constructed — it's a
real transaction from Stellar testnet (ledger 4437357, the exact ledger
[wallet-backend#716](https://github.com/stellar/wallet-backend/issues/716) names as
already containing this shape), checked in verbatim. It currently passes against
both SDK releases above; see [ADR 0003](docs/adr/0003-captured-entry-provenance.md)
for how captured entries are sourced and verified.

The [Go adapter](adapters/go) independently confirms a different real bug of the
same class: [`github.com/stellar/go-stellar-sdk`](https://github.com/stellar/go-stellar-sdk)`@v0.7.1`'s
`xdr.NewPoolId` ordered assets by strkey text instead of raw bytes, wrongly
rejecting the same pool-asset-order shape JS got wrong. Fixed in `v0.7.2`
([PR #5974](https://github.com/stellar/go-stellar-sdk/pull/5974));
`scripts/prove-detection-go.sh` proves it the same way. It also reports two
checks as honestly `unsupported` where the Go SDK's API design has no
equivalent surface to test — see [adapters/go/README.md](adapters/go/README.md).

The [Java adapter](adapters/java) found a **third** independent instance of the
same bug class in [`network.lightsail:stellar-sdk`](https://github.com/lightsail-network/java-stellar-sdk)`@6.0.0`
— except this one is **currently open**, not historical: `Asset.compareTo`
orders issuers by strkey text instead of raw bytes, today, in the current
release. Reported with a verified minimal reproduction before this was
documented anywhere public:
[lightsail-network/java-stellar-sdk#849](https://github.com/lightsail-network/java-stellar-sdk/issues/849).
`scripts/prove-detection-java.sh` asserts the current known-bad state and is
meant to start failing once upstream ships a fix, not pass forever — see
[adapters/java/README.md](adapters/java/README.md).

## Linter

The RFP this project answers also asks for a linter catching three ingestion
mistakes. [`linter/`](linter) is an ESLint plugin covering the first one —
missing depth/size caps on recursive structures — tested against a shape
modeled on the real wallet-backend ingestion bugs this corpus is built from.
The other two (trusting unvalidated fields; deriving participants from the
envelope instead of the meta) are not started; see
[`linter/README.md`](linter/README.md#status) for why, honestly.

## Try it

```sh
cargo run -p tripwire -- generate corpus          # writes corpus/manifest.json and corpus/entries/*.xdr
cargo run -p tripwire -- verify corpus --regen    # decodes, round-trips, and re-derives every expectation
(cd adapters/js && npm install)
cargo run -p tripwire -- run corpus -- node adapters/js/adapter.mjs
```

Test a specific SDK release by pointing `TRIPWIRE_SDK_DIR` at a directory that has it in `node_modules`.

## How it works

1. The generator builds each entry from the official [`stellar-xdr`](https://crates.io/crates/stellar-xdr) types. Expected values are computed in code, never typed by hand.
2. The corpus is language-neutral: raw XDR files and a JSON manifest ([checks](docs/checks.md)).
3. An **adapter** is any program that runs the corpus through one implementation using its public APIs and prints one JSON line per check ([protocol](docs/adapter-protocol.md)).
4. `tripwire run` compares what the adapter reports with the manifest and exits non-zero on a disagreement.

## Scope: what already exists

The official `stellar xdr` CLI decodes, encodes and compares XDR. If you only need to turn a blob into JSON, use it.

Tripwire does something different: it runs the same inputs through several independent implementations and reports where they disagree. A single-implementation test cannot find a bug that every user of that implementation shares, which is how a fix in one SDK went missing from the others.

## Roadmap

See [ROADMAP.md](ROADMAP.md) for what's done and what's next. Contributions are
welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0.
