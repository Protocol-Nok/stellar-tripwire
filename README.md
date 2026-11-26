# stellar-tripwire

**Replays rare-but-valid Stellar ledger shapes through SDKs and indexers, and reports where they disagree with the network and with each other.**

Every application that reads the ledger decodes XDR, and rare shapes keep breaking those readers because nobody tested them. A fix lands in one SDK and never reaches the others. Tripwire turns each of those bugs into a regression test that every implementation can run.

> Status: early. One adapter (JavaScript), four corpus entries, six check kinds. The indexer replay harness, the linter and the Go, Python and Java adapters are not built yet; see [Roadmap](#roadmap).

## What it already catches

Four entries, each tied to a public sighting. Run against `@stellar/stellar-sdk` releases from before the fixes:

| Entry | Sighting | 17.2.0 (pre-fix) | 17.4.0 |
|---|---|---|---|
| `scval-map-key-collision` | [js-stellar-sdk#1789](https://github.com/stellar/js-stellar-sdk/issues/1789) | **FAIL**: 2 entries in the XDR, 1 after `scValToNative`, no error | pass |
| `muxed-ids-boundary` | [js-stellar-sdk#1785](https://github.com/stellar/js-stellar-sdk/issues/1785) | **FAIL**: accepts `"0x10"`, `" 16 "`, `""` as muxed IDs | pass |
| `lp-asset-issuer-order` | [js-stellar-sdk#1766](https://github.com/stellar/js-stellar-sdk/issues/1766) | **FAIL**: rejects a pool asset pair the network accepts | pass |
| `set-options-signed-payload-signer` | [wallet-backend#729](https://github.com/stellar/wallet-backend/issues/729) | pass | pass |

`scripts/prove-detection.sh` re-runs this in CI, so a check that stops detecting its bug fails the build.

## Try it

```sh
cargo run -p tripwire -- generate corpus          # writes corpus/manifest.json and corpus/entries/*.xdr
cargo run -p tripwire -- verify corpus --regen    # decodes, round-trips, and re-derives every expectation
(cd adapters/js && npm install)
cargo run -p tripwire -- run corpus --adapter node adapters/js/adapter.mjs
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

- [ ] Adapters: Go, Python, Rust SDK, Java
- [ ] Captured entries from testnet and mainnet history, with ledger provenance
- [ ] Replay harness that feeds the corpus to an indexer through Galexie-style ledger files
- [ ] Linter for three ingestion mistakes: trusting unvalidated fields, missing depth caps, deriving participants from the envelope
- [ ] Indexer-hardening guide
- [ ] Per-CAP coverage manifest and a published divergence matrix

Contributions are welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Apache-2.0.
