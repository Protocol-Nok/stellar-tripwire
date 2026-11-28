import { RuleTester } from '@typescript-eslint/rule-tester';
import { test, after } from 'node:test';
import { rule } from '../src/rules/unvalidated-field-as-key.js';

RuleTester.describe = (_name, fn) => fn();
RuleTester.it = (name, fn) => test(name, fn);
RuleTester.itOnly = test.only;
RuleTester.afterAll = after;

const ruleTester = new RuleTester();

ruleTester.run('unvalidated-field-as-key', rule, {
  valid: [
    // Canonicalized first: a plain string derived explicitly, then used as the key.
    `
    function cache(balances: Map<string, bigint>, account: MuxedAccount): bigint | undefined {
      const canonical = account.accountId();
      return balances.get(canonical);
    }
    `,
    // Using it as a *value*, not a key, is fine -- nothing here assumes it's unique
    // or comparable at the byte level.
    `
    function record(log: string[], memo: Memo): void {
      log.push(memo.toString());
    }
    `,
    // Not a risky type at all.
    `
    function cache(balances: Map<string, bigint>, accountId: string): bigint | undefined {
      return balances.get(accountId);
    }
    `,
  ],
  invalid: [
    {
      // The object-identity Map key case: two MuxedAccount instances that represent the
      // same underlying account but aren't the same object (or don't define value
      // equality) never collide as keys, silently losing data stellar-core never
      // promised was unique to begin with.
      code: `
      function cache(balances: Map<MuxedAccount, bigint>, account: MuxedAccount, amount: bigint): void {
        balances.set(account, amount);
      }
      `,
      errors: [{ messageId: 'unvalidatedFieldAsKey', data: { name: 'account', riskyType: 'MuxedAccount' } }],
    },
    {
      // Computed object-key access.
      code: `
      function lookup(seen: Record<string, boolean>, account: MuxedAccount): boolean {
        return seen[account] ?? false;
      }
      `,
      errors: [{ messageId: 'unvalidatedFieldAsKey', data: { name: 'account', riskyType: 'MuxedAccount' } }],
    },
    {
      // The template-literal key-building idiom, common in real indexer caches.
      code: `
      function cacheKey(account: MuxedAccount): string {
        return \`balance:\${account}\`;
      }
      `,
      errors: [{ messageId: 'unvalidatedFieldAsKey', data: { name: 'account', riskyType: 'MuxedAccount' } }],
    },
    {
      // A Soroban authorization entry used the same way.
      code: `
      function seen(cache: Set<SorobanAuthorizationEntry>, entry: SorobanAuthorizationEntry): boolean {
        return cache.has(entry);
      }
      `,
      errors: [{ messageId: 'unvalidatedFieldAsKey', data: { name: 'entry', riskyType: 'SorobanAuthorizationEntry' } }],
    },
  ],
});
