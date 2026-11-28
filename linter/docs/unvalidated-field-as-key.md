# unvalidated-field-as-key

Flags a memo, muxed account, or Soroban authorization value used directly as
a lookup key (an object's computed property, a `Map`/`Set` method's key
argument, or interpolated into a template literal), with no canonicalization
step in between.

## Why this matters

The [SCF RFP](https://stellar.gitbook.io/scf-handbook/scf-awards/scf-build/rfp-track)
this linter answers names the general risk directly: `stellar-core` validates
some transaction fields (signatures, sequence numbers, fees, time bounds) and
passes others through untouched — "memos, Soroban authorization trees, muxed
account IDs." An indexer that treats one of those fields as already
canonical, and uses it as a lookup key without deriving an explicit canonical
form first, can silently merge entries that should stay distinct, or split
one logical entry across two keys that never collide when they should.

## Examples

❌ Incorrect — the raw typed value goes straight into the key:

```ts
function cacheKey(account: MuxedAccount): string {
  return `balance:${account}`;
}
```

✅ Correct — an explicit canonical form is derived first:

```ts
function cacheKey(account: MuxedAccount): string {
  const canonical = account.accountId();
  return `balance:${canonical}`;
}
```

Using the value for something that isn't a lookup key — logging it, passing
it to a function, pushing it onto an array — is fine and not flagged; the
risk this rule checks for is specifically about keys, where an unintended
identity relationship (two things treated as equal, or two things treated as
different, when the network doesn't guarantee either) causes data loss or a
cache that never hits.

## Options

```js
{
  rules: {
    'stellar-ingestion/unvalidated-field-as-key': ['error', {
      riskyTypes: ['Memo', 'MuxedAccount', 'SorobanAuthorizationEntry', 'SorobanAuthorizedInvocation'],
    }],
  },
}
```

`riskyTypes` (default shown above — the real type names `@stellar/stellar-sdk`
exports for these three RFP-named categories) — which parameter or variable
type names mark a value as risky. Only a plain `Identifier` with an explicit
`TSTypeReference` annotation is recognized; see "Known limitations" in the
package README.

## Known limitations

- **Name-based, not scope-based**, the same tradeoff as `missing-depth-cap`:
  once a name is seen with a risky type anywhere in the file, every later
  identifier with that name is treated as risky — including in an unrelated
  function that happens to reuse the name for something else.
- **One hop only.** `const copy = account; cache.set(copy, ...)` is not
  flagged, because `copy`'s own declaration has no type annotation. The rule
  catches the direct case, not every alias of it.
- **"Canonicalized" is not verified.** The rule only checks whether the
  *original* risky-typed identifier is used raw. It has no way to tell a real
  canonicalization call (`account.accountId()`) from a no-op one
  (`String(account)` on an object with a poor default `toString`) — either
  one satisfies the rule, as long as the result is assigned to a new name
  before being used as a key.
