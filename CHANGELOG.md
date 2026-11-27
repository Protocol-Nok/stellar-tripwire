# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
