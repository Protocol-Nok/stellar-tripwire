# Roadmap

Tracked here so status is visible without reading every issue. Checked items have
a merged PR; see the linked issue for detail.

## Now

- [x] Corpus generator and verifier (`tripwire-corpus`)
- [x] CLI: `generate`, `verify`, `run`
- [x] JS adapter for `@stellar/stellar-sdk`
- [x] CI proves the corpus catches its three source bugs on a pre-fix SDK release
- [x] First tagged release with prebuilt binaries (needed by `stellar-tripwire-action`):
      Linux x86_64, macOS arm64, Windows x86_64
- [ ] Intel macOS (`x86_64-apple-darwin`) binary — dropped from v0.1.0 after its
      runner queued 30+ minutes with every other target done in under 5; revisit
      if this turns out to matter to anyone
- [x] `tripwire run`'s adapter command moved to after `--` (was `--adapter`),
      fixing a real ambiguity discovered via the Java adapter (`java -jar`
      looks like a flag). See CHANGELOG `[Unreleased]`.

## Next

- [x] Captured entries (real testnet/mainnet ledger history, with provenance):
      one so far — a real CAP-40 signed-payload signer at testnet ledger 4437357,
      the exact ledger wallet-backend#716 names. See ADR 0003.
- [ ] More captured entries: muxed accounts and the pool-issuer-order shape are
      still synthetic only
- [x] Go adapter for `github.com/stellar/go-stellar-sdk` (the actively
      maintained module; `github.com/stellar/go` is deprecated). Catches a
      real historical bug (`v0.7.1`'s `xdr.NewPoolId`, same class as
      js-stellar-sdk#1766), proven the same way as the JS adapter
      (`scripts/prove-detection-go.sh`). Two checks are honestly
      `unsupported` — the SDK's API has no surface to test them; see
      `adapters/go/README.md`.
- [x] Java adapter for `network.lightsail:stellar-sdk` (the actively
      maintained module; GitHub redirects `stellar/java-stellar-sdk` here).
      Found a third, independent instance of the asset-ordering bug class —
      this one currently open, not historical. Filed
      [lightsail-network/java-stellar-sdk#849](https://github.com/lightsail-network/java-stellar-sdk/issues/849)
      with a verified reproduction. `scripts/prove-detection-java.sh` tracks
      it; see `adapters/java/README.md`.
- [x] Python adapter for `stellar-sdk` (py-stellar-base), a genuinely
      independent pure-Python XDR implementation. Finds **no bug** — every
      check with a real equivalent in this SDK passes, including the
      asset-ordering class Go and Java each got wrong in some version;
      `LiquidityPoolAsset`'s own source comments "Compare asset issuers by
      raw key bytes, as stellar-core does," and it does. `pip`/`venv` weren't
      preinstalled in this environment; bootstrapped from Ubuntu's own signed
      `python3-pip` package (checksum-verified, extracted without running
      `dpkg` or any script) rather than an unapproved remote script. See
      `adapters/python/README.md`.
- [ ] Rust SDK (`soroban-sdk` / `stellar-xdr`) adapter — searched crates.io
      properly this time (4 candidates: `stellar-baselib`, `stellar-base`,
      `substrate-stellar-xdr`, `rust-stellar-xdr`, `kuknos-xdr-tmp`). None
      qualify: two depend on the official `stellar-xdr` crate directly, one
      is a near-verbatim republish of its generated code (diffed to confirm),
      and two are abandoned (2019, 2021) and predate Soroban, CAP-40 and even
      liquidity pools. Unlike JS/Go/Java/Python, Rust's Stellar XDR ecosystem
      is close to a single-implementation monopoly — a real finding, not a
      gap in the search.

## Linter

- [x] `missing-depth-cap`: an ESLint rule flagging a function that recurses on
      an `ScVal` or authorization-tree-shaped parameter with no depth/size
      guard. Real tests, including a case the first implementation actually
      missed (recursion via `.flatMap(fn)`, not just `fn(x)` — caught by
      testing, then fixed). See `linter/README.md`.
- [ ] `trusting-unvalidated-fields`: needs a concrete allow-list design (which
      fields `stellar-core` validates vs. passes through) before a rule can be
      precise rather than guessing
- [ ] `participants-from-envelope`: needs recognizable call patterns from a
      real indexer codebase to scope against, not a guessed shape
- [ ] Publish `linter/` to npm (currently install-from-source only)

## Later

- [ ] Indexer replay harness (Galexie-style ledger file input)
- [ ] Indexer-hardening guide, publishable on developers.stellar.org
- [ ] Per-CAP coverage manifest
