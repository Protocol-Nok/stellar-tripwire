#!/usr/bin/env node
// A deliberately broken reference indexer: it reproduces wallet-backend#716 exactly.
// For a signed-payload signer (CAP-0040), it stores/reports only the 32-byte ed25519
// portion in the shape a plain Ed25519 signer needs, silently discarding the payload --
// the same fixed-size-storage truncation #716 described. Every other signer type is
// reported correctly. The replay harness should catch this as a DROP, not a crash: the
// indexer processes the ledger fine, it just reports the wrong fact.
//
//   node broken-indexer.mjs <corpus-dir>
//
// See docs/replay-protocol.md and docs/indexer-hardening-guide.md section 4.

import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const require = createRequire(path.join(path.dirname(fileURLToPath(import.meta.url)), 'x.js'));
const { xdr, StrKey, SignerKey } = require('@stellar/stellar-sdk');
const corpus = process.argv[2];
if (!corpus) {
  console.error('usage: broken-indexer.mjs <corpus-dir>');
  process.exit(2);
}

const emit = (o) => process.stdout.write(JSON.stringify(o) + '\n');
emit({ indexer: 'reference-js-broken-cap40-truncation', version: '0.1.0' });

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
          const entryData = change.updated ?? change.created;
          if (!entryData || entryData.data.type !== 'account') continue;
          const account = entryData.data.account;
          const accountId = StrKey.encodeEd25519PublicKey(Buffer.from(account.accountId.ed25519.value));
          for (const signer of account.signers) {
            // THE BUG: a signed-payload signer has a 32-byte ed25519 pubkey plus a
            // separate payload. Report only the pubkey, re-encoded as a plain Ed25519
            // signer -- the payload is silently dropped, exactly as #716 described.
            const signerKey =
              signer.key.type === 'signerKeyTypeEd25519SignedPayload'
                ? StrKey.encodeEd25519PublicKey(
                    Buffer.from(signer.key.ed25519SignedPayload.ed25519.value),
                  )
                : SignerKey.encodeSignerKey(signer.key);
            observations.push({
              kind: 'signer_added',
              account_id: accountId,
              signer_key: signerKey,
            });
          }
        }
      }
    }
  } catch {
    // report nothing for a ledger this (broken) indexer can't process at all
  }
  emit({ ledger: entry.id, observations });
}
