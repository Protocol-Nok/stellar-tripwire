#!/usr/bin/env node
// A correct reference indexer for the stellar-tripwire replay harness.
//
//   node good-indexer.mjs <corpus-dir>
//
// Protocol: docs/replay-protocol.md. Derives facts from the ledger meta, never the
// envelope, per the indexer-hardening guide's own advice -- reports the full strkey for
// every signer, whatever its length, instead of assuming a fixed size, and finds a
// trustline creation from the entry changes alone, regardless of which operation type
// produced it (so a Soroban InvokeHostFunction call creating one via CAP-0073 is caught
// the same as a classic ChangeTrust operation).

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(path.join(path.dirname(fileURLToPath(import.meta.url)), 'x.js'));
const { xdr, StrKey, SignerKey } = require('@stellar/stellar-sdk');

// Canonical CODE:ISSUER form for a credit asset, "native" for XLM. Takes the ledger
// meta's own TrustLineAsset, never anything from the operation envelope.
function canonicalAsset(trustLineAsset) {
  if (trustLineAsset.type === 'assetTypeNative') return 'native';
  const an4 = trustLineAsset.alphaNum4;
  const an12 = trustLineAsset.alphaNum12;
  const alphaNum = an4 ?? an12;
  const code = Buffer.from(alphaNum.assetCode.value).toString('ascii').replace(/\0+$/, '');
  const issuer = StrKey.encodeEd25519PublicKey(Buffer.from(alphaNum.issuer.ed25519.value));
  return `${code}:${issuer}`;
}

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
          // operation type -- see docs/indexer-hardening-guide.md, section 3. This is
          // what catches a trustline created by a Soroban InvokeHostFunction call
          // (CAP-0073 SAC trust()) just as well as one from a ChangeTrust operation --
          // the code below never looks at which operation type ran.
          const entryData = change.updated ?? change.created;
          if (!entryData) continue;

          if (entryData.data.type === 'account') {
            const account = entryData.data.account;
            const accountId = StrKey.encodeEd25519PublicKey(Buffer.from(account.accountId.ed25519.value));
            for (const signer of account.signers) {
              // The full strkey, whatever its length -- a signed-payload signer can be
              // up to 100 bytes, not the 32 a plain G-address needs. See section 4.
              observations.push({
                kind: 'signer_added',
                account_id: accountId,
                signer_key: SignerKey.encodeSignerKey(signer.key),
              });
            }
          } else if (entryData.data.type === 'trustline' && change.type === 'ledgerEntryCreated') {
            const trustLine = entryData.data.trustLine;
            observations.push({
              kind: 'trustline_created',
              account_id: StrKey.encodeEd25519PublicKey(Buffer.from(trustLine.accountId.ed25519.value)),
              asset: canonicalAsset(trustLine.asset),
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
