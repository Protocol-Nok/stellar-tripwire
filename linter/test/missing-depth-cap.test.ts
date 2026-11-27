import { RuleTester } from '@typescript-eslint/rule-tester';
import { test, after } from 'node:test';
import { rule } from '../src/rules/missing-depth-cap.js';

RuleTester.describe = (_name, fn) => fn();
RuleTester.it = (name, fn) => test(name, fn);
RuleTester.itOnly = test.only;
RuleTester.afterAll = after;

const ruleTester = new RuleTester();

ruleTester.run('missing-depth-cap', rule, {
  valid: [
    // Has an explicit depth parameter that's used to cap recursion.
    `
    function walk(entry: SorobanAuthorizationEntry, depth: number): void {
      if (depth > MAX_DEPTH) return;
      for (const child of entry.children) walk(child, depth + 1);
    }
    `,
    // Has a depth-shaped local variable checked before recursing.
    `
    function visit(node: ScVal, level: number): number {
      if (level >= 32) throw new Error('too deep');
      return 1 + visit(node.inner, level + 1);
    }
    `,
    // Recursive, but not on a risky type -- plain numeric recursion is fine.
    `
    function factorial(n: number): number {
      return n <= 1 ? 1 : n * factorial(n - 1);
    }
    `,
    // Takes a risky type but isn't recursive at all.
    `
    function summarize(entry: ScVal): string {
      return JSON.stringify(entry);
    }
    `,
    // Arrow function with a depth guard.
    `
    const walkAuth = (inv: AuthorizedInvocation, depth: number): void => {
      if (depth > 16) return;
      for (const sub of inv.subInvocations) walkAuth(sub, depth + 1);
    };
    `,
  ],
  invalid: [
    {
      // The shape of the real bug class this rule exists for: an authorization
      // tree walked with no bound on nesting depth.
      code: `
      function flatten(entry: SorobanAuthorizationEntry): SorobanAuthorizationEntry[] {
        return [entry, ...entry.children.flatMap(flatten)];
      }
      `,
      errors: [{ messageId: 'missingDepthCap', data: { name: 'flatten', riskyType: 'SorobanAuthorizationEntry' } }],
    },
    {
      code: `
      function toNative(val: ScVal): unknown {
        if (val.kind === 'map') {
          const out: Record<string, unknown> = {};
          for (const [k, v] of val.entries) out[k] = toNative(v);
          return out;
        }
        return val.value;
      }
      `,
      errors: [{ messageId: 'missingDepthCap', data: { name: 'toNative', riskyType: 'ScVal' } }],
    },
    {
      // Arrow function assigned to a const, no depth guard.
      code: `
      const flatten = (inv: AuthorizedInvocation): AuthorizedInvocation[] => {
        return [inv, ...inv.subInvocations.flatMap(flatten)];
      };
      `,
      errors: [{ messageId: 'missingDepthCap', data: { name: 'flatten', riskyType: 'AuthorizedInvocation' } }],
    },
  ],
});
