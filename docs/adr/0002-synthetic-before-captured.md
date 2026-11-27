# ADR 0002: start with synthetic entries, add captured ones deliberately

## Status
Accepted

## Context
A corpus entry can be built by hand to exercise one shape (synthetic), or taken
verbatim from real network history (captured). Captured entries are stronger
evidence that the shape actually occurs, but need a source, a ledger reference,
and a decision about network (testnet vs. mainnet) and provenance format.

## Decision
Ship synthetic entries first, each tied to a public sighting (an issue or PR
where the shape broke something real) so the entry's relevance isn't in doubt
even without a captured example. Add captured entries as a distinct, explicit
`source.kind` once the provenance format is designed (ROADMAP "Next").

## Consequences
- Every entry already in the corpus is reproducible by anyone from source, with
  no dependency on archived ledger data or a specific RPC endpoint.
- `Source::Captured` exists in the schema now so captured entries slot in later
  without a breaking manifest change.
