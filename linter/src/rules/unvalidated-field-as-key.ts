import type { TSESTree } from '@typescript-eslint/utils';
import { createRule } from '../rule-creator.js';

// The RFP this linter answers names these three categories as fields stellar-core
// validates less than everything else, or passes through untouched: "memos, Soroban
// authorization trees, muxed account IDs." These are the real type names
// @stellar/stellar-sdk exports for them -- not a guessed shape.
const DEFAULT_RISKY_TYPES = [
  'Memo',
  'MuxedAccount',
  'SorobanAuthorizationEntry',
  'SorobanAuthorizedInvocation',
];

const KEY_METHODS = new Set(['set', 'get', 'has', 'delete']);

function typeAnnotationName(id: TSESTree.Identifier): string | null {
  // Same scope as missing-depth-cap's equivalent: a plain Identifier with an explicit
  // TSTypeReference annotation. An inferred type or a destructured parameter is invisible
  // to this rule -- see docs/unvalidated-field-as-key.md.
  const ann = id.typeAnnotation?.typeAnnotation;
  if (ann?.type === 'TSTypeReference' && ann.typeName.type === 'Identifier') {
    return ann.typeName.name;
  }
  return null;
}

export const rule = createRule({
  name: 'unvalidated-field-as-key',
  meta: {
    type: 'problem',
    docs: {
      description:
        'Flags a memo, muxed account or Soroban authorization value used directly as a lookup key, without going through any canonicalization step first.',
    },
    schema: [
      {
        type: 'object',
        properties: {
          riskyTypes: { type: 'array', items: { type: 'string' } },
        },
        additionalProperties: false,
      },
    ],
    messages: {
      unvalidatedFieldAsKey:
        "'{{name}}' ({{riskyType}}) is used directly as a key, with no canonicalization in between. stellar-core does not validate or canonicalize this field -- two inputs an attacker can make look different (or identical) at the byte level may compare the opposite way once used as a lookup key. Derive an explicit canonical string from it first.",
    },
  },
  defaultOptions: [{ riskyTypes: DEFAULT_RISKY_TYPES }],
  create(context, [options]) {
    const riskyTypes = new Set(options.riskyTypes ?? DEFAULT_RISKY_TYPES);
    // Name-based, not scope-based, same documented tradeoff as missing-depth-cap: once a
    // name is seen with a risky type anywhere in the file, every later identifier with
    // that name is treated as risky, even in an unrelated function that reuses the name
    // for something else. Narrow on purpose rather than pull in full scope resolution.
    const riskyNames = new Map<string, string>(); // name -> the risky type it was declared with

    function checkRisky(node: TSESTree.Node): void {
      if (node.type !== 'Identifier') return;
      const riskyType = riskyNames.get(node.name);
      if (!riskyType) return;
      context.report({
        node,
        messageId: 'unvalidatedFieldAsKey',
        data: { name: node.name, riskyType },
      });
    }

    return {
      // Picks up both a typed parameter (function f(entry: MuxedAccount)) and a typed
      // variable declarator (const entry: MuxedAccount = ...) -- both attach
      // typeAnnotation to an Identifier node in this AST.
      Identifier(node) {
        if (!node.typeAnnotation) return;
        const t = typeAnnotationName(node);
        if (t && riskyTypes.has(t)) riskyNames.set(node.name, t);
      },

      'MemberExpression[computed=true]'(node: TSESTree.MemberExpression) {
        checkRisky(node.property);
      },

      CallExpression(node) {
        const callee = node.callee;
        if (
          callee.type === 'MemberExpression' &&
          !callee.computed &&
          callee.property.type === 'Identifier' &&
          KEY_METHODS.has(callee.property.name) &&
          node.arguments.length > 0
        ) {
          checkRisky(node.arguments[0]);
        }
      },

      TemplateLiteral(node: TSESTree.TemplateLiteral) {
        for (const expr of node.expressions) checkRisky(expr);
      },
    };
  },
});
