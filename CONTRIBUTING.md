# Contributing

The most useful contribution is **a rare but valid ledger shape that broke, or could break,
a real implementation**, with a link to where it broke.

## Adding a corpus entry

1. Add a function in `crates/tripwire-corpus/src/entries.rs` that builds the XDR and names
   the CAP and the public issue or PR that shows the shape matters.
2. Declare the checks it must satisfy (`docs/checks.md`). Expected values are computed from
   the `stellar-xdr` types in code, never typed by hand.
3. `cargo run -p tripwire -- generate corpus && cargo run -p tripwire -- verify corpus --regen`
4. Run at least one adapter against it. If an SDK disagrees, say which one, in the PR.

An entry that names neither a CAP nor a sighting is rejected by `verify`: every entry has to
say why it exists.

## Adding an adapter

Follow `docs/adapter-protocol.md`. Use the implementation's public APIs, do not special-case
entries, and include the command to run it in the adapter's README. Adapters for Go, Python,
Java, PHP, Swift, Kotlin and Dart are all wanted.

## Reporting a divergence upstream

If the corpus finds a bug in an SDK or indexer, open the issue upstream first with a minimal
reproduction, then link it from the entry's `sightings`.

## Checks before a PR

```
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
```
