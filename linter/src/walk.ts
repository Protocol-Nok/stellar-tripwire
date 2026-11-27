import type { TSESTree } from '@typescript-eslint/utils';

/**
 * Visits every descendant of `node` (not `node` itself), stopping as soon as
 * `visitor` returns `true`. Plain structural walk over whatever the parser
 * gave us; it does not consult scope information, so it cannot tell a
 * reference from an unrelated shadowed binding of the same name. Rules using
 * this document that as a known limitation rather than silently assume
 * correctness they have not earned.
 */
export function walkUntil(
  node: TSESTree.Node | null | undefined,
  visitor: (node: TSESTree.Node) => boolean,
): boolean {
  if (!node) return false;
  const stack: TSESTree.Node[] = [node];
  while (stack.length > 0) {
    const current = stack.pop()!;
    if (visitor(current)) return true;
    for (const key of Object.keys(current)) {
      if (key === 'parent') continue;
      const value = (current as unknown as Record<string, unknown>)[key];
      if (Array.isArray(value)) {
        for (const item of value) {
          if (item && typeof item === 'object' && 'type' in item) {
            stack.push(item as TSESTree.Node);
          }
        }
      } else if (value && typeof value === 'object' && 'type' in value) {
        stack.push(value as TSESTree.Node);
      }
    }
  }
  return false;
}
