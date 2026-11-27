# @protocol-nok/eslint-plugin-stellar-ingestion

ESLint rules that catch ingestion mistakes which have broken real Stellar
indexers. Part of [stellar-tripwire](https://github.com/Protocol-Nok/stellar-tripwire).

The [RFP this answers](https://stellar.gitbook.io/scf-handbook/scf-awards/scf-build/rfp-track)
asks for a linter covering three mistakes: trusting unvalidated fields, missing
depth/size caps on recursive structures, and deriving participants from the
envelope instead of the meta. **This package covers the second one so far.**
The other two are real and unimplemented — see [Status](#status) — not quietly
dropped.

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
    rules: { 'stellar-ingestion/missing-depth-cap': 'error' },
  },
];
```

## Rules

| Rule | What it catches |
|---|---|
| [`missing-depth-cap`](docs/missing-depth-cap.md) | A function that recurses on an `ScVal`, an authorization tree, or similar, with no apparent depth or size guard |

## Status

Built and tested; not yet the full three-rule linter the RFP describes:

- ✅ **Missing depth/size caps** — `missing-depth-cap`, above.
- ❌ **Trusting unvalidated fields** — not started. The hard part is that
  "unvalidated" is a semantic property (which fields `stellar-core` checks),
  not a syntactic one; a precise rule likely needs a short allow-list of
  known-unvalidated field names (memo, Soroban auth trees, muxed account IDs)
  rather than general-purpose taint analysis.
- ❌ **Deriving participants from the envelope instead of the meta** — not
  started. Needs recognizable call patterns from a real TS indexer codebase to
  scope against; speculating a shape here risked a rule nobody's code
  actually matches.

See the main repo's [ROADMAP.md](https://github.com/Protocol-Nok/stellar-tripwire/blob/main/ROADMAP.md).

## Known limitations of `missing-depth-cap`

Documented honestly rather than left for a user to discover:

- **Name-based, not scope-based.** Recursion is detected by matching the
  function's own name as an identifier anywhere in its body. A nested
  function or variable that shadows that name and happens to reference it
  would be a false positive. No scope resolution is done in v1.
- **Syntactic typing only.** A risky parameter is recognized by its explicit
  type annotation (`entry: SorobanAuthorizationEntry`) textually, via
  `@typescript-eslint/parser` without a type-checked project. A destructured
  parameter, an inferred type, or a type imported under an alias is invisible
  to this rule.
- **The risky-type list is configurable but has a default.** See
  `docs/missing-depth-cap.md` for the `riskyTypes` option if your codebase
  uses different names for these shapes.

## Development

```sh
npm install
npm run build   # tsc -p tsconfig.json
npm test        # compiles test/ and runs node --test against it
npm run lint-self
```
