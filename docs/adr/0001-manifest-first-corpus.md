# ADR 0001: a manifest-first corpus, not bare XDR files

## Status
Accepted

## Context
A conformance corpus could just be a folder of `.xdr` files, leaving each adapter
to decide independently what a "correct" decode looks like.

## Decision
Every entry carries, in `manifest.json`, the checks it must satisfy and the
expected values for each — computed once in Rust from the official `stellar-xdr`
types, never typed by hand in the manifest. Adapters compare against this, not
against each other's guesses.

## Consequences
- Adding an entry means writing code that builds it, not hand-crafting JSON.
- `verify --regen` can prove the committed bytes and manifest are exactly what
  the generator produces today, catching manual edits or merge damage.
- A check kind not implemented by an adapter is reported `unsupported`, distinct
  from a pass, so coverage gaps stay visible instead of silently passing.
