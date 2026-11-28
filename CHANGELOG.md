# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
