# Per-CAP coverage

**Status: first pass.** This project's own [indexer-hardening guide](indexer-hardening-guide.md)
tells indexer authors to check "what changed" against *every operation type any CAP
allows to produce that change" — not just the obvious one. This document holds tripwire
to the same standard: a real audit of which Stellar CAPs change a ledger-entry,
operation, transaction-envelope, or transaction-meta shape (the problem space tripwire
exists for), and whether this project actually has a corpus or replay entry for it.

## Method

Source: the [stellar/stellar-protocol](https://github.com/stellar/stellar-protocol)
repo's `core/README.md` index and every individual CAP file, fetched directly, not
recalled from memory — several claims below ended up different from what the CAP's
*title* alone would suggest (see CAP-0087, under "Out of scope," and CAP-0046-06, under
"Flagged mechanically," below), which is exactly why this was checked file by file
instead of guessed.

**Scope**: CAPs with Status `Final` or `Implemented` only — i.e. actually live on
mainnet. `Accepted`-but-`Protocol version: TBD` CAPs (CAP-0017, CAP-0081) aren't
implemented yet, so they're listed once, separately, as not-yet-real. `Draft` and
`Rejected` CAPs were never real on any ledger and aren't listed at all.

**"Does this change a shape tripwire cares about"** was decided mechanically first —
does the CAP's markdown contain a non-trivial `### XDR` / `### XDR Changes` section, or
an inline `union`/`struct`/`enum` definition anywhere in the body — then every CAP that
mechanical check flagged, or that seemed plausibly relevant from its title, had the
actual matched section read before it went in a table below. CAPs in the last table
were **not** individually read past their title; that's stated there, not hidden.

## Foundational: not a shape under test, but what the replay ledgers are built from

| CAP | Title | Protocol | What it is |
|---|---|---|---|
| [CAP-0042](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0042.md) | Multi-Part Transaction Sets | 20 | Introduces `GeneralizedTransactionSet`, `TxSetComponent`, and `TxsetCompTxsMaybeDiscountedFee` — confirmed directly in the CAP's own XDR diff. `tripwire-replay`'s `minimal_ledger()` constructs exactly these types for every entry it generates. This CAP isn't a shape tripwire tests; it's the scaffolding tripwire's own synthetic ledgers are built out of. |

## Covered

Every entry below has a real corpus or replay entry, a test, and (where applicable) a
public sighting — see each entry's own documentation for detail; this table just maps
CAP to entry.

| CAP | Title | Protocol | Shape | tripwire coverage | Sighting(s) |
|---|---|---|---|---|---|
| [CAP-0027](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0027.md) | First-class multiplexed accounts | 13 | `MuxedAccount` union (verified: the CAP embeds the union inline under "Specification," with no separate "XDR Changes" heading — an early-CAP template variation the mechanical check alone would have missed) | `tripwire-corpus`: `muxed-ids-boundary` | [js-stellar-sdk#1785](https://github.com/stellar/js-stellar-sdk/issues/1785) |
| [CAP-0038](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0038.md) | Automated Market Makers | 18 | `LiquidityPoolEntry`, `PoolID` (verified: `typedef Hash PoolID; // SHA256(LiquidityPoolParameters)` in the CAP's own diff) | `tripwire-corpus`: `lp-asset-issuer-order` | [js-stellar-sdk#1766](https://github.com/stellar/js-stellar-sdk/issues/1766); independently also wrong in `go-stellar-sdk` pre-`0.7.2` and `java-stellar-sdk@6.0.0` |
| [CAP-0040](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0040.md) | Ed25519 Signed Payload Signer | 19 | New `SignerKeyType` case, `SignerKey.ed25519SignedPayload` (40–100 bytes) | `tripwire-corpus`: `set-options-signed-payload-signer`, `captured-testnet-signed-payload-signer`; `tripwire-replay`: `captured-signed-payload-signer-ledger` | [wallet-backend#716](https://github.com/stellar/wallet-backend/issues/716), [#729](https://github.com/stellar/wallet-backend/issues/729) |
| [CAP-0046-01](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-01.md) | WebAssembly Smart Contract Runtime Environment | 20 | `SCVal`/`SCValType` (verified: "`SCVal` is a new XDR type," stated directly in this CAP — not in a later, more plausible-sounding sub-CAP; its name alone doesn't say so) | `tripwire-corpus`: `scval-map-key-collision` | [js-stellar-sdk#1789](https://github.com/stellar/js-stellar-sdk/issues/1789) |
| [CAP-0073](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0073.md) | Allow SAC to create G-account balances | 26 | No new XDR type — a new code path (a Soroban `InvokeHostFunction` call into the Stellar Asset Contract's `trust()`) producing a change to the *existing* `TrustLineEntry` type, with no `ChangeTrust` operation anywhere in the transaction | `tripwire-replay`: `cap73-sac-trust-ledger` | [wallet-backend#731](https://github.com/stellar/wallet-backend/issues/731) |

## Verified gaps: real, read-in-depth, not yet covered

Every row below was confirmed by reading the CAP's actual Specification/XDR section —
not assumed from the title. Several are directly relevant to this project's *own* code,
noted inline. None of these are claimed as bugs; they're untested shapes, which is a
different, narrower claim.

| CAP | Title | Protocol | What's untested | Why it matters |
|---|---|---|---|---|
| [CAP-0015](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0015.md) | Fee Bump Transactions | 13 | `TransactionEnvelope::TxFeeBump` | Every entry this project builds (corpus and replay) uses `TransactionEnvelope::Tx`; none wraps a fee-bumped transaction. An SDK or indexer that only unwraps the inner `Tx` would double-charge or mis-attribute a fee-bumped transaction's effects. |
| [CAP-0019](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0019.md) | Future-upgradable TransactionEnvelope | 13 | The `TransactionEnvelope` union itself, beyond the one case tripwire constructs | Foundational to every transaction this project builds; tripwire has only ever exercised `Tx`, never `TxV0` (the legacy pre-CAP-0019 case some tooling still has to accept) or `TxFeeBump`. |
| [CAP-0021](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0021.md) | Generalized transaction preconditions | 19 | `Preconditions` beyond `None` (verified: `union Preconditions switch (PreconditionType type)`, with `PreconditionsV2` carrying time bounds, ledger bounds, min sequence age/gap, and up to two extra signers) | Every transaction tripwire builds uses `Preconditions::None`. The richer `PreconditionsV2` case (extra signers, min-seq-age) has never been exercised. |
| [CAP-0023](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0023.md) | Two-Part Payments with ClaimableBalanceEntry | 14 | `ClaimableBalanceEntry`, `CreateClaimableBalanceOp`, `ClaimClaimableBalanceOp` | A real ledger-entry type and two operations with zero coverage. Mature (2019) and likely well-trodden by now, but untested is untested. |
| [CAP-0024](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0024.md) | Make PathPayment Symmetrical | 12 | `PathPaymentStrictSendOp` | tripwire has no path-payment coverage of either strict-receive or strict-send shape. |
| [CAP-0033](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0033.md) | Sponsored Reserves | 14/15 | Sponsoring-ID ledger-entry extensions, `BeginSponsoringFutureReservesOp`/`EndSponsoringFutureReservesOp`/`RevokeSponsorshipOp` | Touches the `ext` field of every sponsorable ledger-entry type (account, trustline, offer, data, claimable balance, signer) — a wide, zero-coverage surface. |
| [CAP-0035](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0035.md) | Asset Clawback | 17 | `ClawbackOp`, the `TRUSTLINE_CLAWBACK_ENABLED`/`AUTH_CLAWBACK_ENABLED` trustline flags | No coverage of the clawback operation or the trustline/account flags that gate it. |
| [CAP-0046-02](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-02.md) | Smart Contract Lifecycle | 20 | `ContractCodeEntry`, `ContractDataEntry`, contract instantiation via `InvokeHostFunctionOp` | tripwire's `cap73-sac-trust-ledger` entry calls a contract but never decodes or asserts anything about the contract's own ledger entries. No check or observation exercises `ContractDataEntry`/`ContractCodeEntry` at all — a real, structural gap in Soroban coverage. |
| [CAP-0046-07](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-07.md) + [-09](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-09.md) + [-10](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-10.md) | Fee model / Network Configuration Ledger Entries / Budget Metering | 20 | `ConfigSettingEntry` and its variants (`ConfigSettingContractExecutionLanesV0`, `ConfigSettingContractComputeV0`, cost-parameter settings) | Grouped as one row because all three extend the *same* ledger-entry type over time (verified: -07 and -10's settings are stated to live inside the `ConfigSettingEntry` -09 introduces). Zero coverage of this entry type at all. |
| [CAP-0046-11](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-11.md) (and [CAP-0071](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0071.md)/[-01](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0071-01.md)/[-02](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0071-02.md), same family) | Soroban Authorization Framework (+ auth delegation, address-bound credentials) | 20 / 27 | `SorobanAuthorizationEntry` beyond an empty list | `tripwire-replay`'s `cap73-sac-trust-ledger` entry already carries a `SorobanAuthorizationEntry` field — it's just always empty (`auth: VecM::default()`). The type is already in this project's dependency graph; no entry has ever populated or decoded one. |
| [CAP-0046-12](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0046-12.md) | Soroban State Archival Interface | 20 | `TTLEntry`, the `EVICTED` state, `ExtendFootprintTTLOp`/`RestoreFootprintOp` | `LedgerCloseMetaV1.evicted_keys` is a field tripwire's own `minimal_ledger()` already sets — always to an empty `VecM`. Same situation as CAP-0046-11: the plumbing exists, nothing exercises it. |
| [CAP-0058](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0058.md) | Constructors for Soroban Contracts | 22 | `CreateContractArgsV2`, `ContractIdPreimage` with constructor args | `HostFunction::CreateContractV2` (the variant this CAP adds) is a case of the same union `cap73-sac-trust-ledger` already uses `InvokeContract` from — zero coverage of the sibling case. |
| [CAP-0063](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0063.md) | Parallelism-friendly Transaction Scheduling | 23 | A new `TransactionPhase` case for parallel execution (verified: adds `ParallelTxExecutionStage`/`DependentTxCluster` and a new case to the `TransactionPhase` union) | Every ledger tripwire builds uses `TransactionPhase::V0` only. An indexer tested only against tripwire's ledgers has never seen the parallel-execution phase shape that can appear on real ledgers today. |
| [CAP-0067](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0067.md) | Unified Asset Events | 23 | Classic-asset transfer/mint events represented as `ContractEvent`s | The same envelope-vs-meta risk class as CAP-0073: an indexer that derives effects from operation type instead of the event stream can miss a classic-asset event now emitted through the unified, contract-event-shaped path. A strong candidate for the next replay entry. |
| [CAP-0083](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0083.md) | Validators vote to drop a ledger's transaction set | 28 | A new `StellarValueExt` case (verified: "This CAP introduces a new `StellarValue` extension type") | Every ledger header tripwire builds hardcodes `StellarValueExt::Basic`. The newer extension case has never been constructed or decoded. |
| [CAP-0084](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0084.md) | Muxed Contract Addresses | 30 | `ScAddress::MuxedContract` (verified: `struct MuxedContract` added as a new `ScAddress` case, plus a `to_muxed_id` field on the classic `transfer`/`mint` event shapes) | Recent (protocol 30) and directly in the same family as CAP-0027's muxed accounts, which tripwire already tests — but the contract-address muxing case has zero coverage. |
| [CAP-0088](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0088.md) | Millisecond-Resolution Close Times | 30 | `closeTimeMs: TimePointMs` on `StellarValue` (verified directly in the CAP's XDR diff) | Every ledger header tripwire builds sets `close_time: TimePoint(0)` and has no `closeTimeMs` at all. Recent (protocol 30) and directly affects whether tripwire's synthetic ledgers still look like a *current* ledger header, not just a structurally-valid one. |
| [CAP-0085](https://github.com/stellar/stellar-protocol/blob/master/core/cap-0085.md) | Externally managed contract executables | 28 | A new `ContractExecutable` case — **title-based only, not excerpt-verified** | Flagged from the title and the CAP index alone; the actual XDR section wasn't read for this pass. Listed here rather than silently dropped so the gap stays visible even at lower confidence. |

## Out of scope (a real reason, not a gap)

| CAP(s) | Reason |
|---|---|
| CAP-0020 | Changes the *bucket list*'s internal entry format (`INITENTRY`/`METAENTRY`) — stellar-core's own state-snapshot storage, not `LedgerCloseMeta`, `TransactionMeta`, or anything an SDK or an RPC/Horizon-fed indexer ever decodes. |
| CAP-0002, 0004, 0005, 0025, 0026, 0028, 0029, 0030, 0034, 0051, 0053, 0065, 0069, 0076, 0078, 0079 | Verified: validation-rule, fee-policy, consensus-nomination, or internal-implementation changes with no new or changed ledger-entry/operation/envelope XDR. (Several of these, e.g. CAP-0029's `AllowTrust` relaxation or CAP-0028's pre-auth-signer clearing, change *semantics* of an existing field, not its shape — out of scope for a decode/meta-extraction conformance tool.) |
| CAP-0054, 0055, 0056, 0059, 0062, 0066, 0070, 0074, 0075, 0077, 0080, 0082, 0086, 0087 | Soroban host-function-only additions: new contract-callable primitives (BLS12-381, Poseidon, ML-DSA verification, map-unpacking helpers, etc.) and the matching `ContractCostType` cost-accounting enum entries. Verified directly for CAP-0087 (its "XDR changes" section is a pure `ContractCostType` enum addition) and CAP-0086 (host functions only, confirmed by reading its Specification in full); the rest were confirmed from the same README/title pattern, not individually read in as much depth as those two. |
| CAP-0017, CAP-0081 | `Status: Accepted`, `Protocol version: TBD` — accepted in principle but not yet implemented. Not a real ledger shape until a protocol version actually ships it. |

## Flagged mechanically, not individually read

These CAPs' markdown contains an XDR-changing signal (a dedicated section or an inline
type definition) but weren't read past that for this pass. Not claimed as gaps or as
out of scope — just not yet assessed, stated as such rather than silently omitted.

CAP-0001, CAP-0003, CAP-0006, CAP-0018, CAP-0046 (the Soroban overview CAP itself —
its own XDR is covered by its numbered sub-CAPs above, which is why it has no row of
its own), CAP-0046-03, CAP-0046-05, CAP-0046-06, CAP-0046-08.

One exception worth naming even unread: CAP-0046-06 ("Smart Contract Standardized
Asset," the Stellar Asset Contract interface CAP-0073 extends) was spot-checked enough
to confirm it carries no XDR of its own — "see the XDR diffs in the Soroban overview
CAP" — and that its data types (`AllowanceDataKey`, `BalanceValue`, etc.) are the SAC's
own contract-internal storage layout inside `ContractDataEntry`, not new top-level
ledger-entry XDR. Left in this list rather than the gaps table because, unlike
CAP-0073, no public sighting or own-code dependency motivated reading it any further.

## Keeping this honest

The "Covered" table's entry IDs are checked by `scripts/check-cap-coverage.sh`
against what `tripwire generate` and `tripwire replay generate` actually produce, so a
renamed or removed entry fails CI instead of leaving this document quietly wrong.
Nothing currently re-derives the "Verified gaps" or "Out of scope" tables — they're a
point-in-time research finding, not a generated artifact, so they can go stale exactly
the way any audit can. Re-running the method above against a fresh CAP index is the way
to check.
