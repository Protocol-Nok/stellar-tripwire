# Adapter protocol

An **adapter** is any program that runs the corpus through one implementation (an SDK, an
indexer) using that implementation's own decoding and its public APIs, then reports what
happened. Adapters can be written in any language; the runner only reads stdout.

```
tripwire run <corpus-dir> [--json] -- <command> [args...]
```

The adapter command goes after `--`, so it can contain its own flags (`java -jar
x.jar`, `node --experimental-foo x.mjs`) without being mistaken for `tripwire`'s own —
see `crates/tripwire-cli/tests/cli.rs` for why this isn't just stylistic. The runner
appends the absolute corpus directory as the adapter's last argument.

## Output

One JSON object per line on stdout. Anything on stderr is passed through for humans.

1. First line, exactly once:

   ```json
   {"adapter": "js", "version": "@stellar/stellar-sdk 17.4.0"}
   ```

2. Then one line per `(entry, check)` pair the adapter handled:

   ```json
   {"entry": "muxed-ids-boundary", "check": "muxed_id_input", "status": "fail", "detail": "accepted \"0x10\""}
   ```

   `status` is one of:

   | status | meaning |
   |---|---|
   | `pass` | the implementation agrees with the manifest |
   | `fail` | the implementation disagrees. `detail` says how, with the offending value |
   | `unsupported` | the adapter does not implement this check kind |

A check the adapter never mentions is reported as `unsupported`, not as a pass.

## Exit code

`tripwire run` exits `1` if any check failed, `0` otherwise. Unsupported checks do not fail
the run, but they are listed, so a gap in an adapter is visible.

## Rules for adapters

- **Use public, high-level APIs.** The point is to find what downstream code would hit.
- **Do not special-case entry IDs.** Dispatch on the check `kind`, never on the entry.
- **A thrown error is a result.** Where a check says "must refuse", throwing is a pass; where
  it says "must accept", throwing is a `fail` with the message in `detail`.
- **Keep it offline.** No network, no keys, no funds.
