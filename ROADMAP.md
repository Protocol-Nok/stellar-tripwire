# Roadmap

Tracked here so status is visible without reading every issue. Checked items have
a merged PR; see the linked issue for detail.

## Now

- [x] Corpus generator and verifier (`tripwire-corpus`)
- [x] CLI: `generate`, `verify`, `run`
- [x] JS adapter for `@stellar/stellar-sdk`
- [x] CI proves the corpus catches its three source bugs on a pre-fix SDK release
- [ ] First tagged release with prebuilt binaries (needed by `stellar-tripwire-action`)

## Next

- [ ] Rust SDK (`soroban-sdk` / `stellar-xdr`) adapter
- [ ] Go adapter
- [ ] Captured entries (real testnet/mainnet ledger history, with provenance)
- [ ] Python and Java adapters

## Later

- [ ] Indexer replay harness (Galexie-style ledger file input)
- [ ] Linter: trusting unvalidated fields, missing depth/size caps, participants-from-envelope
- [ ] Indexer-hardening guide, publishable on developers.stellar.org
- [ ] Per-CAP coverage manifest
