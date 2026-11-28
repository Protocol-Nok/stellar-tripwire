import { rule as missingDepthCap } from './rules/missing-depth-cap.js';
import { rule as unvalidatedFieldAsKey } from './rules/unvalidated-field-as-key.js';

const plugin = {
  meta: {
    name: '@protocol-nok/eslint-plugin-stellar-ingestion',
    version: '0.1.0',
  },
  rules: {
    'missing-depth-cap': missingDepthCap,
    'unvalidated-field-as-key': unvalidatedFieldAsKey,
  },
  configs: {},
};

// A flat-config recommended preset, defined after `plugin` so it can reference
// itself -- the standard pattern for an ESLint 9+ flat-config plugin.
Object.assign(plugin.configs, {
  recommended: {
    plugins: { 'stellar-ingestion': plugin },
    rules: {
      'stellar-ingestion/missing-depth-cap': 'error',
      'stellar-ingestion/unvalidated-field-as-key': 'error',
    },
  },
});

export default plugin;
