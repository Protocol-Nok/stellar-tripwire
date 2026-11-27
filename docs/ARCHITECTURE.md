# Architecture

```
crates/tripwire-corpus/    generator + manifest types + verifier  (the source of truth)
crates/tripwire-cli/       `tripwire` binary: generate | verify | run
adapters/js/               one adapter: corpus -> @stellar/stellar-sdk -> results
corpus/                    (generated, not committed here) the published snapshot
                            lives in stellar-tripwire-corpus
```

## Why a manifest, not just XDR files

Each corpus entry needs an *expected* decoded value to compare an adapter's answer
against. The manifest carries that expectation as data (`docs/checks.md`), computed
once from the official `stellar-xdr` types (`tripwire-corpus::entries`), so no two
places can disagree about what "correct" means.

## Why adapters are external processes, not Rust trait impls

A JS or Go bug cannot be found by calling Rust. An adapter is any program that
reads the manifest, decodes each entry with its own implementation's public APIs,
and prints one JSON line per check (`docs/adapter-protocol.md`). This keeps the
corpus consumable from any language, including ones this repo never builds.

## Data flow for `tripwire run`

```
tripwire run <corpus> --adapter <cmd>
        |
        v
  reads manifest.json  ---->  spawns <cmd> <corpus>  ---->  adapter decodes each
        |                                                    entry.file with its
        |                                                    own SDK and prints
        |                                                    {entry, check, status}
        v
  compares adapter output to each entry's `checks`, reports fail/unsupported,
  exits non-zero if anything failed
```
