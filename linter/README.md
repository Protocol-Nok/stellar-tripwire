# @protocol-nok/eslint-plugin-stellar-ingestion

ESLint rules that catch ingestion mistakes which have broken real Stellar
indexers. Part of [stellar-tripwire](https://github.com/Protocol-Nok/stellar-tripwire).

The [RFP this answers](https://stellar.gitbook.io/scf-handbook/scf-awards/scf-build/rfp-track)
asks for a linter covering three mistakes: trusting unvalidated fields, missing
depth/size caps on recursive structures, and deriving participants from the
envelope instead of the meta. **This package covers the first two.** The third
is real and unimplemented — see [Status](#status) for why, in detail, not
quietly dropped.

## Install

**Not yet published to npm** (tracked in the main repo's `ROADMAP.md`). Until
then, clone this repo and install from the local path:

```sh
git clone https://github.com/Protocol-Nok/stellar-tripwire
cd stellar-tripwire/linter && npm install && npm run build
cd /path/to/your/project
npm install --save-dev /path/to/stellar-tripwire/linter
```

npm's `github:owner/repo#path:subdir` shorthand looks like it should install
straight from a monorepo subdirectory without a local clone, but it doesn't
work for this package as of this writing: npm ignores the `path:` segment and
looks for a `package.json` at the repository root, which this repo (a Rust
workspace) doesn't have.

```js
// eslint.config.js
import stellarIngestion from '@protocol-nok/eslint-plugin-stellar-ingestion';

export default [
  stellarIngestion.configs.recommended,
  // ...your other config
];
```

Or enable the rule individually:

```js
import stellarIngestion from '@protocol-nok/eslint-plugin-stellar-ingestion';

export default [
  {
    plugins: { 'stellar-ingestion': stellarIngestion },
    rules: {
      'stellar-ingestion/missing-depth-cap': 'error',
      'stellar-ingestion/unvalidated-field-as-key': 'error',
    },
  },
];
```

## Rules

| Rule | What it catches |
|---|---|
| [`missing-depth-cap`](docs/missing-depth-cap.md) | A function that recurses on an `ScVal`, an authorization tree, or similar, with no apparent depth or size guard |
| [`unvalidated-field-as-key`](docs/unvalidated-field-as-key.md) | A memo, muxed account, or Soroban authorization value used directly as a lookup key, with no canonicalization step first |

## Status

Built and tested; not yet the full three-rule linter the RFP describes:

- ✅ **Missing depth/size caps** — `missing-depth-cap`, above.
- ✅ **Trusting unvalidated fields** — `unvalidated-field-as-key`, above.
  Scoped specifically to the key-usage case, built on the RFP's own named
  allow-list (memo, Soroban auth trees, muxed account IDs) rather than
  general-purpose taint analysis, which stayed out of scope for the same
  reason it did for the other rule.
- ❌ **Deriving participants from the envelope instead of the meta** — not
  started, after a real search, not a guess skipped for convenience: GitHub
  code search across `@stellar/stellar-sdk`-using TypeScript repositories
  (including known Stellar indexers from this project's own ecosystem
  research) found no TypeScript codebase with a recognizable
  "participants"/"effects"-from-envelope pattern to scope a rule against.
  The only concrete instance of this exact bug this project has in hand is
  [wallet-backend#731](https://github.com/stellar/wallet-backend/issues/731),
  written in Go: effects are gated on the operation's envelope-sourced
  `type`, so an operation type nobody anticipated (`InvokeHostFunction`
  producing a trustline change via CAP-0073) silently produces no effect,
  even though the ledger meta shows the change happened. Translating that
  into a precise, low-false-positive TypeScript AST pattern — without a real
  TypeScript example that has the bug — would mean guessing a shape and
  publishing a rule nobody's code is known to match. Revisit if a concrete
  TypeScript instance turns up.

See the main repo's [ROADMAP.md](https://github.com/Protocol-Nok/stellar-tripwire/blob/main/ROADMAP.md).

## Known limitations

Documented honestly rather than left for a user to discover. Shared by both
rules:

- **Name-based, not scope-based.** `missing-depth-cap` detects recursion by
  matching the function's own name as an identifier anywhere in its body.
  `unvalidated-field-as-key` tracks a risky-typed name across the whole file
  once declared. Neither does real scope resolution, so a nested function or
  variable that shadows a tracked name, in either rule, is a false positive.

- **Syntactic typing only, in both rules.** A risky parameter or variable is
  recognized by its explicit type annotation (`entry: SorobanAuthorizationEntry`)
  textually, via `@typescript-eslint/parser` without a type-checked project. A
  destructured parameter, an inferred type, or a type imported under an alias
  is invisible to both rules.
- **The risky-type list is configurable but has a default, in both rules.**
  See each rule's own doc page for its `riskyTypes` option if your codebase
  uses different names for these shapes.

Each rule's doc page (linked above) covers what's specific to it —
`unvalidated-field-as-key`'s one-hop-only aliasing gap, for example.

## Development

```sh
npm install
npm run build   # tsc -p tsconfig.json
npm test        # compiles test/ and runs node --test against it
npm run lint-self
```
