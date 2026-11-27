import type { TSESTree } from '@typescript-eslint/utils';
import { createRule } from '../rule-creator.js';
import { walkUntil } from '../walk.js';

type FunctionNode =
  | TSESTree.FunctionDeclaration
  | TSESTree.FunctionExpression
  | TSESTree.ArrowFunctionExpression;

const DEFAULT_RISKY_TYPES = [
  'ScVal',
  'SorobanAuthorizationEntry',
  'SorobanAuthorizedInvocation',
  'AuthorizedInvocation',
  'XdrSCVal',
];

const DEPTH_NAME = /^(depth|level|maxdepth|currentdepth|remainingdepth)$/i;

function typeAnnotationName(param: TSESTree.Parameter): string | null {
  // Handles only a plain Identifier parameter with an explicit TSTypeReference
  // annotation, e.g. `(entry: SorobanAuthorizationEntry) => ...`. A destructured
  // parameter, an inferred type, or a type imported under an alias is out of
  // scope for this heuristic -- see linter/docs/missing-depth-cap.md.
  if (param.type !== 'Identifier' || !param.typeAnnotation) return null;
  const annotation = param.typeAnnotation.typeAnnotation;
  if (annotation.type === 'TSTypeReference' && annotation.typeName.type === 'Identifier') {
    return annotation.typeName.name;
  }
  return null;
}

function riskyParamType(fn: FunctionNode, riskyTypes: ReadonlySet<string>): string | null {
  for (const param of fn.params) {
    const name = typeAnnotationName(param);
    if (name && riskyTypes.has(name)) return name;
  }
  return null;
}

function isDepthIdentifier(node: TSESTree.Node): boolean {
  return node.type === 'Identifier' && DEPTH_NAME.test(node.name);
}

function hasDepthGuard(fn: FunctionNode): boolean {
  // A parameter literally named depth/level/... counts: the function at least
  // has somewhere to carry a count, which is the point of this check.
  if (fn.params.some((p) => p.type === 'Identifier' && DEPTH_NAME.test(p.name))) {
    return true;
  }
  // Or a comparison against something depth/level-shaped anywhere in the body --
  // e.g. `if (depth > MAX_DEPTH) return;` guarding a recursive call.
  return walkUntil(
    fn.body,
    (node) =>
      node.type === 'BinaryExpression' &&
      ['>', '>=', '<', '<='].includes(node.operator) &&
      (isDepthIdentifier(node.left) || isDepthIdentifier(node.right)),
  );
}

function selfName(fn: FunctionNode): string | null {
  if (fn.type === 'FunctionDeclaration' && fn.id) return fn.id.name;
  const parent = fn.parent;
  if (parent?.type === 'VariableDeclarator' && parent.id.type === 'Identifier') {
    return parent.id.name;
  }
  return null;
}

function isSelfRecursive(fn: FunctionNode, name: string): boolean {
  // Any reference to the function's own name counts, not just a direct call:
  // `entries.flatMap(walk)` recurses just as much as `walk(entries[0])` does,
  // and passing a recursive walker by reference to map/flatMap/reduce is the
  // common shape for exactly the tree-walking code this rule targets.
  //
  // Name-based, not scope-based: a nested function or variable that redeclares
  // `name` for something unrelated would be a false positive here. Narrow on
  // purpose rather than pull in full scope resolution for a v1 rule.
  return walkUntil(fn.body, (node) => node.type === 'Identifier' && node.name === name);
}

export const rule = createRule({
  name: 'missing-depth-cap',
  meta: {
    type: 'problem',
    docs: {
      description:
        'Flags a function that recurses on a risky nested-structure parameter (an ScVal, an authorization tree) with no apparent depth or size guard.',
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
      missingDepthCap:
        "'{{name}}' recurses on a {{riskyType}} parameter with no apparent depth or size guard. A nested structure an attacker controls -- an authorization tree, a nested ScVal -- can recurse past a safe limit. Add a depth parameter (or a size budget) and bail out past a maximum.",
    },
  },
  defaultOptions: [{ riskyTypes: DEFAULT_RISKY_TYPES }],
  create(context, [options]) {
    const riskyTypes = new Set(options.riskyTypes ?? DEFAULT_RISKY_TYPES);

    function check(fn: FunctionNode): void {
      const name = selfName(fn);
      if (!name || !isSelfRecursive(fn, name)) return;
      const riskyType = riskyParamType(fn, riskyTypes);
      if (!riskyType || hasDepthGuard(fn)) return;
      context.report({
        node: fn.type === 'FunctionDeclaration' && fn.id ? fn.id : fn,
        messageId: 'missingDepthCap',
        data: { name, riskyType },
      });
    }

    return {
      FunctionDeclaration: check,
      FunctionExpression: check,
      ArrowFunctionExpression: check,
    };
  },
});
