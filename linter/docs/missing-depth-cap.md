# missing-depth-cap

Flags a function that recurses on a risky nested-structure parameter — an
`ScVal`, a Soroban authorization tree — with no apparent depth or size guard.

## Why this matters

A Soroban authorization tree, or a nested `ScVal` map or vector, is structured
data an attacker controls: they build the transaction. An indexer that
recursively walks one with no limit can be made to recurse arbitrarily deep,
exhausting the stack. This is exactly the "missing depth and size caps on
recursive structures such as authorization trees and nested ScVals" mistake
named in the [SCF RFP](https://stellar.gitbook.io/scf-handbook/scf-awards/scf-build/rfp-track)
this linter answers.

## Examples

❌ Incorrect:

```ts
function flatten(entry: SorobanAuthorizationEntry): SorobanAuthorizationEntry[] {
  return [entry, ...entry.children.flatMap(flatten)];
}
```

✅ Correct:

```ts
function flatten(entry: SorobanAuthorizationEntry, depth: number): SorobanAuthorizationEntry[] {
  if (depth > MAX_DEPTH) return [];
  return [entry, ...entry.children.flatMap((c) => flatten(c, depth + 1))];
}
```

A size budget in place of a depth counter also satisfies this rule, as long as
it's a parameter or local variable named `depth`/`level`/`maxDepth`/
`currentDepth`/`remainingDepth` (case-insensitive) that's compared with
`>`, `>=`, `<` or `<=` somewhere in the function body.

## Options

```js
{
  rules: {
    'stellar-ingestion/missing-depth-cap': ['error', {
      riskyTypes: ['ScVal', 'SorobanAuthorizationEntry', 'MyCustomTreeType'],
    }],
  },
}
```

`riskyTypes` (default: `ScVal`, `SorobanAuthorizationEntry`,
`SorobanAuthorizedInvocation`, `AuthorizedInvocation`, `XdrSCVal`) — the
parameter type names that mark a function as walking a risky structure. Only
a plain `Identifier` parameter with an explicit `TSTypeReference` annotation
is recognized; see the package README's "Known limitations."

## When not to use this rule

If your codebase's recursive helpers are all bounded by something this rule
can't see (a library that enforces its own recursion limit internally, for
example), this rule's default heuristic will still ask for a local depth
parameter. Disable it per-file or adjust `riskyTypes` to exclude types that
go through such a library.
