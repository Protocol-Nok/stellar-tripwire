# Hardening a Stellar indexer: a field-by-field guide

**Status: draft.** This follows the outline the [SCF RFP](https://stellar.gitbook.io/scf-handbook/scf-awards/scf-build/rfp-track)
this project answers asks for, and every specific claim below is sourced —
either to the [SCF RFP's own text](#sources), to a sibling SDK's own stated
understanding of the network rule, to Stellar's developer docs, or to a public
sighting where a real indexer broke. Where a claim is secondary rather than
read from `stellar-core` itself, that's said plainly. The RFP that asked for
this guide also says publication should be **gated on review by a core or
platform engineer** before it goes anywhere official — treat this as a
well-sourced draft for that review, not the final word.

## 1. Validated vs. unvalidated fields

Two rules run through this whole guide, both stated in the RFP itself:

1. `stellar-core` validates some fields of a transaction — signatures,
   sequence numbers, fees, time bounds — and passes others through
   untouched: memos, Soroban authorization trees, muxed account IDs.
2. An indexer should work out who a transaction affected from the **ledger
   meta**, which records what core actually changed, not from the
   **transaction envelope**, which records what the submitter claimed.

| Field | Who validates it | What that means for an indexer |
|---|---|---|
| Signature weight | `stellar-core`, before applying the transaction. The combined weight of signatures for the source account must meet that account's low threshold ([Stellar docs, transaction lifecycle](https://developers.stellar.org/docs/learn/fundamentals/transactions/transaction-lifecycle)). | An indexer can trust that an applied transaction was authorized by the source account. It does **not** mean every operation's own sub-authorizations (Soroban auth entries, below) were checked the same way. |
| Sequence number | `stellar-core`. By default the transaction's sequence number must be exactly one greater than the account's stored sequence number; a minimum-sequence-number precondition relaxes this to a range ([Stellar docs](https://developers.stellar.org/docs/learn/fundamentals/transactions/transaction-lifecycle)). | Safe to trust as applied; an indexer doesn't need its own sequence-gap detection to protect itself, only to detect gaps in its *own* ingestion. |
| Fee | `stellar-core`, as part of admission and fee-bidding. | Safe to trust the fee actually charged; the *resource fee breakdown* for Soroban transactions is a separate, more detailed structure if an indexer wants to report cost — see the SorobanTransactionData in the transaction envelope's `ext` field. |
| Time bounds / ledger bounds | `stellar-core`, as a precondition check before applying. | Safe to trust; an expired-bounds transaction never gets this far as "applied" to begin with. |
| **Memo** | **Nobody**, beyond its own encoding (text length, hash size). `stellar-core` carries it unexamined; the 28-byte text-memo limit and similar constraints live in client SDKs, not core ([secondary: core's `checkValid` validates signatures, sequence numbers and fees, not memo content](https://deepwiki.com/stellar/stellar-core/1.3-component-relationships) — unverified directly against `TransactionFrame.cpp`). | An indexer that treats a memo's bytes as already-safe (printable text, a fixed length, valid UTF-8) can be fed anything that encodes to a valid `Memo` XDR union, because nothing upstream checked it. |
| **Muxed account ID** | **Nobody**, in the sense that matters here. Multiplexing is a purely off-chain addressing convention: the network carries the 64-bit sub-ID (CAP-0027) but never checks that it means anything, that it's unique, or that the "owner" of that sub-ID agrees it's theirs. | An indexer must not assume a muxed ID round-trips cleanly through its own storage. This corpus's `muxed-ids-boundary` entry exercises both a small ID (`16`) and the maximum `u64` value (`18446744073709551615`) for exactly this reason; see [`docs/checks.md`](checks.md). |
| **Soroban authorization tree** | *Partially.* `soroban-env-host` checks each authorization entry's cryptographic signature and its ledger-based expiration, and records a nonce to prevent replay ([Stellar docs, signing Soroban invocations](https://developers.stellar.org/docs/build/guides/transactions/signing-soroban-invocations); secondary, not independently verified against `soroban-env-host` source). What is **not** capped for an indexer's benefit is the authorization tree's own *shape* — how deep the nested sub-invocations go, how many entries it holds. `stellar-core`'s own resource accounting bounds this for execution cost, but an indexer walking the same structure with its own code inherits none of that protection automatically. | Signature validity is not the same question as "is this safe to recurse over with my own code." See §2. |
| **CAP-0040 signed-payload signer** | The signature-weight check above covers it like any other signer, but the key's *shape* (40–100 bytes, a payload length embedded in the key) is exactly the kind of variable-length field that has broken more than one indexer in practice ([wallet-backend#716](https://github.com/stellar/wallet-backend/issues/716): a fixed-32-byte storage column silently truncated it). See §4. |
| **CAP-0073 SAC-created trustlines** | The *ledger entry change* is real and core-applied; what breaks is indexer code that only looks for a `TRUSTLINE` effect from a `ChangeTrust` *operation type*, and misses the trustline a Soroban `InvokeHostFunction` call created via the Stellar Asset Contract's `trust()` ([wallet-backend#731](https://github.com/stellar/wallet-backend/issues/731)). This is the concrete, real instance of the envelope-vs-meta rule in §3. |

This table should agree with `stellar-tripwire-corpus`'s manifest: every entry
there names either a CAP or a public sighting for exactly this reason — see
[ADR 0001](adr/0001-manifest-first-corpus.md).

## 2. Depth and size caps

A Soroban authorization tree, or a nested `ScVal` map or vector, is structured
data the transaction's *submitter* builds — not something `stellar-core`
limits on an indexer's behalf once it's inside the ledger meta your code
reads. `stellar-tripwire`'s own corpus and linter were built around exactly
this: see [`linter/docs/missing-depth-cap.md`](../linter/docs/missing-depth-cap.md)
for the rule, and run `stellar-tripwire-action` with your own adapter to
check your SDK's decode path, not just your own application code, against
this.

**Recommendation:** any function that recurses into an `ScVal`, an
`SorobanAuthorizationEntry`, or a similar nested structure should carry an
explicit depth parameter (or a size budget) and refuse to go past a fixed
maximum — not rely on the structure "usually" being shallow, since nothing
guarantees that for data an attacker controls.

## 3. Participants from meta, not from the envelope

The transaction **envelope** records what the submitter *claimed* the
transaction would do. The ledger **meta** records what `stellar-core`
*actually changed*. They can disagree, and code that branches on the
envelope's operation `type` to decide what to look for can miss changes a
different operation type produced.

The concrete, real instance: [wallet-backend#731](https://github.com/stellar/wallet-backend/issues/731).
Effects were generated only for a `ChangeTrust` operation's own code path. A
`trust()` call inside a Soroban `InvokeHostFunction` operation — introduced by
CAP-0073, so a trustline can now be created by a contract call, not only by
the classic `ChangeTrust` operation — produced a real trustline on the
ledger, visible in the ledger entry changes, with **no corresponding code
path** because nothing dispatched on operation type `InvokeHostFunction`
looking for a trustline change.

**Recommendation:** derive "what changed" from the ledger entry changes
(`LedgerEntryChanges` / the per-operation `OperationMeta` in `TransactionMeta`)
directly, for every ledger-entry type you care about, rather than gating that
search on which operation type the envelope says ran. If you must branch on
operation type for efficiency, add an explicit test: for each ledger-entry
type your indexer tracks, confirm you've checked *every* operation type that
CAPs allow to produce it — not just the "obvious" one.

This project's own `linter/` doesn't yet have a rule for this one; see
[`linter/README.md`](../linter/README.md#status) for why, stated plainly
rather than guessed at.

## 4. Muxed and signed-payload addresses

Two shapes have independently broken a real indexer (`wallet-backend`, in
Go) by being treated as if they were the same size as an ordinary `G...`
address:

- **Muxed accounts** (`M...`, CAP-0027) decode to 40 raw bytes, not the 32 a
  plain `G...` address needs.
- **CAP-0040 signed-payload signers** (`P...`) decode to 40–100 bytes,
  depending on the embedded payload's length (1–64 bytes, padded to a
  multiple of four).

Both broke the same way in [wallet-backend#716](https://github.com/stellar/wallet-backend/issues/716):
a storage column sized for a fixed 32-byte key silently truncated the longer
value, and the truncated bytes were not decodable afterward — not merely
wrong, but **unrecoverable**, because the information needed to reconstruct
the original value was gone.

**Recommendation:**
- Never size a column, buffer, or fixed-length type for "a Stellar address"
  without checking which address *kinds* it must hold. If it can hold a
  signer key, it must hold up to 100 bytes, not 32.
- Treat a muxed address's multiplexed ID as an opaque 64-bit number, not
  something to validate, interpret, or assume is small, round-trippable
  through a numeric column type, or stable across requests.
- This corpus's `set-options-signed-payload-signer` and
  `captured-testnet-signed-payload-signer` entries (the latter a real
  testnet transaction, not constructed — see
  [ADR 0003](adr/0003-captured-entry-provenance.md)) exercise exactly this
  shape; run them through your own indexer's decode path with
  `stellar-tripwire-action` before trusting that it handles them.

## 5. Checklist

- [ ] Memo bytes are treated as untrusted input, not assumed to be valid
      UTF-8 text of a bounded length.
- [ ] Every column, buffer, or fixed-length type that can hold a Stellar
      address is sized for the *longest* address kind it can actually
      receive (100 bytes for a signed-payload signer, 40 for muxed, not 32).
- [ ] Muxed account IDs are stored and compared as opaque values, not
      interpreted, range-checked beyond the XDR type's own bounds, or
      assumed unique.
- [ ] Every function that recurses into an `ScVal` or a Soroban authorization
      tree has an explicit depth or size limit, not an assumption that real
      data is shallow.
- [ ] "What changed" is derived from ledger entry changes for every entry
      type you track, checked against every operation type any CAP allows to
      produce that change — not gated on the operation type the envelope
      names.
- [ ] Your indexer's actual decode path — the SDK it uses, not just your own
      code — has been run against a real conformance corpus (such as
      [`stellar-tripwire-corpus`](https://github.com/Protocol-Nok/stellar-tripwire-corpus))
      rather than only your own test fixtures.

## Sources

Direct: the [SCF RFP](https://stellar.gitbook.io/scf-handbook/scf-awards/scf-build/rfp-track)
this project answers; [wallet-backend#716](https://github.com/stellar/wallet-backend/issues/716)
and [#731](https://github.com/stellar/wallet-backend/issues/731); Stellar's
[transaction lifecycle](https://developers.stellar.org/docs/learn/fundamentals/transactions/transaction-lifecycle)
and [Soroban authorization](https://developers.stellar.org/docs/build/guides/transactions/signing-soroban-invocations)
docs.

Secondary, not independently verified against `stellar-core`'s own C++
source — flagged inline above, and exactly the kind of claim this guide's
own review gate exists to catch: the summary of `checkValid`'s scope (via a
third-party wiki) and the claim that memo content specifically has no
core-side validation.
