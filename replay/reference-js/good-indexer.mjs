#!/usr/bin/env node
// A correct reference indexer for the stellar-tripwire replay harness.
//
//   node good-indexer.mjs <corpus-dir>
//
// Protocol: docs/replay-protocol.md. Derives facts from the ledger meta, never the
// envelope, per the indexer-hardening guide's own advice -- and reports the full strkey
// for every signer, whatever its length, instead of assuming a fixed size.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(path.join(path.dirname(fileURLToPath(import.meta.url)), 'x.js'));
const { xdr, StrKey, SignerKey } = require('@stellar/stellar-sdk');
const corpus = process.argv[2];
if (!corpus) {
  console.error('usage: good-indexer.mjs <corpus-dir>');
  process.exit(2);
}

const emit = (o) => process.stdout.write(JSON.stringify(o) + '\n');
emit({ indexer: 'reference-js-good', version: '0.1.0' });

const manifest = JSON.parse(fs.readFileSync(path.join(corpus, 'manifest.json'), 'utf8'));
for (const entry of manifest.entries) {
  const bytes = fs.readFileSync(path.join(corpus, entry.file));
  const observations = [];
  try {
    const ledger = xdr.LedgerCloseMeta.fromXDR(bytes).v1;
    for (const trm of ledger.txProcessing) {
      if (trm.txApplyProcessing.type !== 'v3') continue;
      for (const opMeta of trm.txApplyProcessing.v3.operations) {
        for (const change of opMeta.changes) {
          // Derived from the ledger meta's entry changes, not from the envelope's
          // operation type -- see docs/indexer-hardening-guide.md, section 3.
          const entryData = change.updated ?? change.created;
          if (!entryData || entryData.data.type !== 'account') continue;
          const account = entryData.data.account;
          const accountId = StrKey.encodeEd25519PublicKey(Buffer.from(account.accountId.ed25519.value));
          for (const signer of account.signers) {
            // The full strkey, whatever its length -- a signed-payload signer can be up
            // to 100 bytes, not the 32 a plain G-address needs. See section 4.
            observations.push({
              kind: 'signer_added',
              account_id: accountId,
              signer_key: SignerKey.encodeSignerKey(signer.key),
            });
          }
        }
      }
    }
  } catch {
    // A ledger this indexer can't process at all: report nothing for it. The harness
    // treats an entry with no output line as "missing" for every observation it expected.
  }
  emit({ ledger: entry.id, observations });
}
