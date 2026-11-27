# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
