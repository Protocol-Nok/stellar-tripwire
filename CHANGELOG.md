# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Indexer replay harness (`tripwire-replay`, `tripwire replay generate|verify|run`):
  the RFP's other requested piece, distinct from the SDK-level `run`. An
  indexer adapter reads a corpus of whole, synthetic-but-structurally-valid
  `LedgerCloseMeta` values (built from the official `stellar-xdr` types —
  `GeneralizedTransactionSet`, `LedgerHeader`, per-operation
  `TransactionMeta`, `LedgerEntryChange`) and reports the facts it extracted
  per ledger; `tripwire replay run` compares that against what the manifest
  expects (`pass`/`dropped`/`missing`). See `docs/replay-protocol.md`.
  The first entry wraps the *same* real captured testnet transaction as
  `captured-testnet-signed-payload-signer` (verified by test to carry the
  identical signer, not just similar bytes) in a minimal real ledger, so the
  replay harness is tested against real network data from its first entry,
  not only synthetic ledgers.
  `replay/reference-js/` ships two reference indexers proving the harness
  actually catches a dropped fact, not just that it can print a report: one
  correct, and one (`broken-indexer.mjs`) reproducing wallet-backend#716's
  exact bug — storing/reporting only a signed-payload signer's 32-byte
  `ed25519` portion and silently discarding its payload.
  `scripts/prove-detection-replay.sh` asserts both outcomes in CI
  (`detection-replay`).
- Second replay entry, `cap73-sac-trust-ledger`, independent of the first
  (wholly synthetic, not derived from the captured transaction): reproduces
  wallet-backend#731 — a trustline created by a Soroban `InvokeHostFunction`
  call into the Stellar Asset Contract's `trust()` (CAP-0073), which has no
  `ChangeTrust` operation anywhere in the transaction. A test asserts the
  operation really is `InvokeHostFunction` while the expected observation is
  the trustline creation, so the entry actually exercises the envelope-vs-
  meta mismatch the sighting describes, not just a shape that happens to
  decode; cross-checked independently with the official `stellar xdr decode`
  CLI. `good-indexer.mjs` now derives this fact from the entry changes alone;
  `broken-indexer.mjs` now also reproduces #731 by dispatching on the
  envelope's operation type (only ever looking for a trustline change under
  `ChangeTrust`), alongside its existing CAP-40 truncation bug.
  `scripts/prove-detection-replay.sh` asserts both of its drops.
- Python adapter for `stellar-sdk` (`adapters/python`), a genuinely
  independent pure-Python XDR implementation. Finds no bug: every check with
  a real equivalent in this SDK passes, including the asset-ordering bug
  class Go and Java have each gotten wrong in some version — verified
  empirically, not assumed from reading its source. One check
  (`muxed_id_input`) is honestly `unsupported` for a Python-specific reason:
  the relevant parameter is type-hinted `int`, but Python doesn't enforce
  that at runtime, so a string is accepted and only fails later with an
  unrelated `TypeError`, not a real validation path.
- Investigated a Rust SDK adapter properly this time: searched crates.io,
  checked four candidates' dependency graphs and one's source against the
  official crate's (diffed, near-identical). None are independent and
  current; two depend on `stellar-xdr` directly, one is a republish, two are
  abandoned since 2019/2021 (predate Soroban and CAP-40 entirely). Recorded
  as a real finding in `ROADMAP.md`, not left as an unexplained gap.
- `linter/`: second rule, `unvalidated-field-as-key`, flagging a memo, muxed
  account, or Soroban authorization value used directly as a lookup key with
  no canonicalization first. Built on the RFP's own named allow-list of
  unvalidated fields. The third rule (`participants-from-envelope`) stays
  deferred after a real GitHub code search for a TypeScript instance of the
  bug came up empty; see `linter/README.md`'s Status section.
- `docs/indexer-hardening-guide.md`: the RFP's requested field-by-field
  guide (validated vs. unvalidated fields, depth/size caps, participants
  from meta not the envelope, muxed/signed-payload handling, a checklist).
  Every specific claim sourced to the RFP's own text, a sibling SDK's stated
  understanding of the network rule, Stellar's developer docs, or a public
  sighting; secondary claims not verified against `stellar-core`'s own
  source are marked inline. Marked as a draft pending the core-or-platform-
  engineer review the RFP itself asks for.

## [0.1.5]

### Added
- Go adapter for `github.com/stellar/go-stellar-sdk` (`adapters/go`), the
  actively maintained Go SDK module (`github.com/stellar/go` is deprecated).
  Catches a real historical bug: `v0.7.1`'s `xdr.NewPoolId` ordered assets by
  strkey text instead of raw bytes, the same class of bug as
  js-stellar-sdk#1766, fixed in `v0.7.2` (PR #5974). Proven the same way as
  the JS adapter: `scripts/prove-detection-go.sh`, wired into CI as
  `detection-go`. Two checks report honestly as `unsupported` where the Go
  SDK's API has no equivalent surface — see `adapters/go/README.md`.
- `linter/`: `@protocol-nok/eslint-plugin-stellar-ingestion`, an ESLint
  plugin with one rule so far, `missing-depth-cap` (flags recursion on an
  `ScVal` or authorization-tree-shaped parameter with no depth/size guard).
  Not yet published to npm; install from source (see `linter/README.md`).
- Java adapter for `network.lightsail:stellar-sdk` (`adapters/java`), the
  actively maintained Java SDK (GitHub redirects `stellar/java-stellar-sdk`
  here). Found and reported a third, independent instance of the same asset-
  ordering bug class as js-stellar-sdk#1766 and go-stellar-sdk's pre-`0.7.2`
  `Asset.LessThan` — except this one is **currently open**, not historical:
  `Asset.compareTo` in the current release (`6.0.0`) orders issuers by strkey
  text instead of raw bytes. Filed as
  [lightsail-network/java-stellar-sdk#849](https://github.com/lightsail-network/java-stellar-sdk/issues/849)
  with a verified minimal reproduction before this was documented anywhere
  public. `scripts/prove-detection-java.sh` asserts the current known-bad
  state and is meant to start failing once upstream ships a fix — see
  `adapters/java/README.md`.

### Changed
- **Breaking CLI change**: `tripwire run`'s adapter command now goes after a
  literal `--` (`tripwire run <dir> [--json] -- <cmd>`), not after `--adapter`.
  Second fix to the same underlying design problem: dropping
  `allow_hyphen_values` in `0.1.2` fixed `--json` being swallowed, but broke
  the opposite case — `tripwire run dir --adapter java -jar x.jar` rejected
  `-jar` as an unrecognized flag, discovered while wiring up the Java adapter.
  The `--` separator removes the ambiguity entirely rather than trading one
  edge case for another; see `crates/tripwire-cli/tests/cli.rs` for both
  regression tests side by side.

## [0.1.4]

### Added
- First captured corpus entry: `captured-testnet-signed-payload-signer`, a real
  CAP-40 signed-payload signer from Stellar testnet (ledger 4437357, the exact
  ledger wallet-backend#716 names), checked in verbatim. See ADR 0003.

## [0.1.3]

### Fixed
- The Windows release asset's `.sha256` sidecar had a trailing `\r` in its
  filename field (PowerShell's `Out-File` writes CRLF), so `sha256sum -c`
  under Git Bash failed with "No such file or directory" looking for a file
  literally named `...zip\r`. Found by stellar-tripwire-action's own CI on
  its first real Windows run. Fixed by writing the sidecar with an explicit
  LF via `[System.IO.File]::WriteAllText`, matching the Unix step.

## [0.1.2]

### Fixed
- `tripwire run <dir> --adapter <cmd> --json` silently swallowed `--json` into
  the adapter command instead of parsing it as a flag, because `--adapter` was
  defined with `allow_hyphen_values(true)`. The adapter then received `--json`
  as its corpus-directory argument and crashed. Found by
  stellar-tripwire-action's own CI running the exact invocation its README
  documents. `--json` and `--adapter` now parse correctly in either order;
  regression-tested in `crates/tripwire-cli/tests/cli.rs`, which fails against
  the old code and passes against the fix (verified both ways before this was
  committed, not just after).

## [0.1.1] - first published release

### Added
- Corpus generator and verifier, with four entries tied to public SDK/indexer bugs.
- `tripwire` CLI (`generate`, `verify`, `run`).
- JS adapter for `@stellar/stellar-sdk`, using its high-level APIs.
- CI job that proves detection against a pre-fix SDK release.
- Prebuilt binaries: Linux x86_64, macOS arm64, Windows x86_64.

### Note
- `v0.1.0` was tagged but its release workflow never completed: the
  `x86_64-apple-darwin` runner queued 30+ minutes while every other target
  finished in under 5, so the release job (which needed all four) never ran.
  No release was ever published at that tag. Fixed by dropping that target
  (see `ROADMAP.md`) and publishing `v0.1.1` instead.
