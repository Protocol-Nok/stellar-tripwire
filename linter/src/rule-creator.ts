import { ESLintUtils } from '@typescript-eslint/utils';

export const createRule = ESLintUtils.RuleCreator(
  (name) =>
    `https://github.com/Protocol-Nok/stellar-tripwire/blob/main/linter/docs/${name}.md`,
);
