# ADR 0003: how a captured entry stores its provenance

## Status
Accepted

## Context
ADR 0002 deferred captured entries until the provenance format was designed.
The first one (`captured-testnet-signed-payload-signer`) needed that design:
where the raw bytes live, and how a reader checks where they came from.

## Decision
A captured entry's bytes are a binary `.xdr` file checked into
`crates/tripwire-corpus/captured/`, pulled into the generator with
`include_bytes!`. This keeps generation offline and deterministic: no network
call happens at generate time or in CI, and the committed file is exactly what
ships, byte for byte, which `verify --regen` already checks for every entry
regardless of `source.kind`.

Provenance is a free-text `source.detail` string, not a structured field:
network, ledger sequence, transaction hash, the exact endpoint and response
field it was read from, the capture date, and that it was base64-decoded
verbatim with nothing else changed. This is less queryable than a structured
record, but it's enough for a reviewer to independently refetch and verify,
and a corpus of a handful of captured entries doesn't yet justify a schema
more complex than a string. Revisit if that stops being true.

## Consequences
- Adding a captured entry is: fetch it once (e.g. via Horizon), save the raw
  bytes under `captured/`, write an entry function using `include_bytes!` and
  the real decoded values as expected results (never hand-typed), and record
  where it came from in `detail`.
- A captured entry's `cap` and `sightings` mean the same thing a synthetic
  entry's do: they're what makes it belong in the corpus, not a claim about
  how it was produced.
- The raw file and the manifest's `sha256` must agree, same as any entry;
  `verify` doesn't distinguish captured from synthetic when checking that.
