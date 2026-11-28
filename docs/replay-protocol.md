# Replay protocol

A **replay harness** checks whether an *indexer* extracts the right facts from a whole
ledger -- not whether an SDK decodes one value, which `docs/adapter-protocol.md` covers.
An **indexer adapter** is any program that reads the replay corpus and reports what it
observed.

```
tripwire replay run <corpus-dir> [--json] -- <indexer-cmd>
```

Same `--` convention as `tripwire run`, for the same reason: the indexer command can
contain its own flags. The corpus directory is appended as its last argument.

## What the indexer adapter receives

The corpus directory, containing `manifest.json` and `entries/*.xdr` -- one raw
`LedgerCloseMeta` file per entry. The adapter reads `manifest.json` itself, decodes each
`entries/<id>.xdr` with its own ingestion logic (the same code path a real deployment
would use), and reports what it extracted.

## Output

One JSON object per line on stdout.

1. First line, exactly once:

   ```json
   {"indexer": "reference-js", "version": "0.1.0"}
   ```

2. Then one line per ledger entry it processed:

   ```json
   {"ledger": "captured-signed-payload-signer-ledger", "observations": [{"kind": "signer_added", "account_id": "G...", "signer_key": "P..."}]}
   ```

   `observations` lists every fact the indexer's own logic extracted from that ledger, in
   whatever order. An observation the manifest expects but the adapter's list doesn't
   contain is reported `dropped`, not failed silently.

## Comparison

For each entry, the harness checks whether every observation the manifest expects for it
appears (by exact structural equality) in the reported list:

| status | meaning |
|---|---|
| `pass` | the expected observation is in the indexer's reported list |
| `dropped` | the indexer reported something for this ledger, but not this observation -- the shape was mishandled or silently dropped |
| `missing` | the indexer reported nothing at all for this ledger (crashed, skipped it, or never emitted a line for it) |

## Exit code

Non-zero if any observation is `dropped` or `missing`.

## Rules for indexer adapters

- **Use your real ingestion path.** The point is to find what your actual pipeline does
  with rare shapes, not a hand-rolled parser written just to pass this.
- **Derive from the ledger meta, not the envelope**, per
  [the hardening guide](indexer-hardening-guide.md#3-participants-from-meta-not-from-the-envelope) --
  an adapter that reads the envelope to decide what to look for defeats the point of a
  replay harness built to catch exactly that mistake.
- **A crash is a result.** If your indexer can't process an entry, print nothing for it
  and exit normally if you can; the harness reports a ledger with no output line as
  `missing` for every observation it expected.
