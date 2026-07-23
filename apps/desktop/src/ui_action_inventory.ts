import type { UiActionContract } from "./ui_action_contract.ts";

export interface UiActionContractSource {
  readonly source: string;
  readonly contracts: readonly UiActionContract[];
}

export interface UiActionCatalogIssue {
  readonly actionId: string;
  readonly code: "ACTION_CONTRACT_CONFLICT";
  readonly sources: readonly string[];
}

export interface UnifiedUiActionCatalog {
  readonly contracts: readonly UiActionContract[];
  readonly issues: readonly UiActionCatalogIssue[];
}

export interface LiteralUiAction {
  readonly actionId: string;
  readonly line: number;
  readonly source: string;
}

export interface LiteralUiActionIssue extends LiteralUiAction {
  readonly code: "LITERAL_ACTION_EMPTY" | "LITERAL_ACTION_UNCLASSIFIED";
}

export interface ConnectedUiActionCoverageIssue {
  readonly actionId: string;
  readonly code: "CONNECTED_ACTION_NOT_IN_SOURCE";
}

export interface TemplateUiActionExpression {
  readonly expression: string;
  readonly line: number;
  readonly source: string;
}

export interface DynamicUiActionFamily {
  readonly source: string;
  readonly expression: string;
  readonly actionIds: readonly string[];
}

export type DynamicUiActionIssue = Readonly<
  | (TemplateUiActionExpression & { readonly code: "DYNAMIC_ACTION_FAMILY_MISSING" })
  | ({ readonly actionId: string; readonly code: "DYNAMIC_ACTION_UNCLASSIFIED"; readonly expression: string; readonly source: string })
  | ({ readonly code: "DYNAMIC_ACTION_FAMILY_EMPTY" | "DYNAMIC_ACTION_FAMILY_STALE"; readonly expression: string; readonly source: string })
>;

export type ConditionalUiActionFamily = DynamicUiActionFamily;
export type ConditionalUiActionExpression = TemplateUiActionExpression;
export type ConditionalUiActionIssue = Readonly<
  | (ConditionalUiActionExpression & { readonly code: "CONDITIONAL_ACTION_FAMILY_MISSING" })
  | ({ readonly actionId: string; readonly code: "CONDITIONAL_ACTION_UNCLASSIFIED"; readonly expression: string; readonly source: string })
  | ({ readonly code: "CONDITIONAL_ACTION_FAMILY_EMPTY" | "CONDITIONAL_ACTION_FAMILY_STALE"; readonly expression: string; readonly source: string })
>;

export function createUnifiedUiActionCatalog(sources: readonly UiActionContractSource[]): UnifiedUiActionCatalog {
  const contractsById = new Map<string, { contract: UiActionContract; sources: string[] }>();
  const issues: UiActionCatalogIssue[] = [];

  for (const source of sources) {
    for (const contract of source.contracts) {
      const existing = contractsById.get(contract.actionId);
      if (!existing) {
        contractsById.set(contract.actionId, { contract, sources: [source.source] });
        continue;
      }
      if (!compatible(existing.contract, contract)) {
        issues.push(Object.freeze({
          actionId: contract.actionId,
          code: "ACTION_CONTRACT_CONFLICT",
          sources: Object.freeze([...existing.sources, source.source]),
        }));
        continue;
      }
      if (!existing.sources.includes(source.source)) existing.sources.push(source.source);
    }
  }

  return Object.freeze({
    contracts: Object.freeze([...contractsById.values()].map((entry) => entry.contract)),
    issues: Object.freeze(issues),
  });
}

export function extractLiteralUiActions(source: string, contents: string): readonly LiteralUiAction[] {
  const actions: LiteralUiAction[] = [];
  const pattern = /["']data-action["']\s*:\s*["']([a-z0-9-]*)["']/g;
  for (const match of contents.matchAll(pattern)) {
    const index = match.index ?? 0;
    actions.push(Object.freeze({
      actionId: match[1],
      line: countLines(contents, index),
      source,
    }));
  }
  return Object.freeze(actions);
}

export function auditLiteralUiActions(
  contracts: readonly UiActionContract[],
  inventory: readonly LiteralUiAction[],
): readonly LiteralUiActionIssue[] {
  const classified = new Set(contracts.map((contract) => contract.actionId));
  return Object.freeze(inventory.flatMap((action) => {
    if (action.actionId.length === 0) return [Object.freeze({ ...action, code: "LITERAL_ACTION_EMPTY" as const })];
    if (!classified.has(action.actionId)) return [Object.freeze({ ...action, code: "LITERAL_ACTION_UNCLASSIFIED" as const })];
    return [];
  }));
}

export function auditConnectedUiActionCoverage(
  contracts: readonly UiActionContract[],
  literalInventory: readonly LiteralUiAction[],
  dynamicFamilies: readonly DynamicUiActionFamily[],
  conditionalFamilies: readonly ConditionalUiActionFamily[],
): readonly ConnectedUiActionCoverageIssue[] {
  const observed = new Set([
    ...literalInventory.map((action) => action.actionId),
    ...dynamicFamilies.flatMap((family) => family.actionIds),
    ...conditionalFamilies.flatMap((family) => family.actionIds),
  ]);
  return Object.freeze(contracts.flatMap((contract) =>
    contract.availability === "connected" && !observed.has(contract.actionId)
      ? [Object.freeze({
          actionId: contract.actionId,
          code: "CONNECTED_ACTION_NOT_IN_SOURCE" as const,
        })]
      : []
  ));
}

export function extractTemplateUiActionExpressions(
  source: string,
  contents: string,
): readonly TemplateUiActionExpression[] {
  const expressions: TemplateUiActionExpression[] = [];
  const pattern = /["']data-action["']\s*:\s*`([^`]+)`/g;
  for (const match of contents.matchAll(pattern)) {
    expressions.push(Object.freeze({
      expression: match[1],
      line: countLines(contents, match.index ?? 0),
      source,
    }));
  }
  return Object.freeze(expressions);
}

export function auditDynamicUiActionFamilies(
  contracts: readonly UiActionContract[],
  occurrences: readonly TemplateUiActionExpression[],
  families: readonly DynamicUiActionFamily[],
): readonly DynamicUiActionIssue[] {
  const issues: DynamicUiActionIssue[] = [];
  const classified = new Set(contracts.map((contract) => contract.actionId));
  const occurrencesByKey = new Map(occurrences.map((occurrence) => [familyKey(occurrence), occurrence]));
  const familiesByKey = new Map(families.map((family) => [familyKey(family), family]));

  for (const occurrence of occurrences) {
    if (!familiesByKey.has(familyKey(occurrence))) {
      issues.push(Object.freeze({ ...occurrence, code: "DYNAMIC_ACTION_FAMILY_MISSING" }));
    }
  }
  for (const family of families) {
    if (!occurrencesByKey.has(familyKey(family))) {
      issues.push(Object.freeze({ code: "DYNAMIC_ACTION_FAMILY_STALE", expression: family.expression, source: family.source }));
      continue;
    }
    if (family.actionIds.length === 0) {
      issues.push(Object.freeze({ code: "DYNAMIC_ACTION_FAMILY_EMPTY", expression: family.expression, source: family.source }));
      continue;
    }
    for (const actionId of family.actionIds) {
      if (!classified.has(actionId)) {
        issues.push(Object.freeze({
          actionId,
          code: "DYNAMIC_ACTION_UNCLASSIFIED",
          expression: family.expression,
          source: family.source,
        }));
      }
    }
  }
  return Object.freeze(issues);
}

export function extractConditionalUiActionExpressions(
  source: string,
  contents: string,
): readonly ConditionalUiActionExpression[] {
  const expressions: ConditionalUiActionExpression[] = [];
  const lines = contents.split("\n");
  const pattern = /["']data-action["']\s*:\s*([^,}\n]+)/g;
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    for (const match of lines[lineIndex].matchAll(pattern)) {
      const expression = normalizeExpression(match[1]);
      if (expression.startsWith('"') || expression.startsWith("'") || expression.startsWith("`")) continue;
      expressions.push(Object.freeze({ expression, line: lineIndex + 1, source }));
    }
  }
  return Object.freeze(expressions);
}

export function auditConditionalUiActionFamilies(
  contracts: readonly UiActionContract[],
  occurrences: readonly ConditionalUiActionExpression[],
  families: readonly ConditionalUiActionFamily[],
): readonly ConditionalUiActionIssue[] {
  const classified = new Set(contracts.map((contract) => contract.actionId));
  const occurrenceKeys = new Set(occurrences.map(familyKey));
  const familyByKey = new Map(families.map((family) => [familyKey(family), family]));
  const issues: ConditionalUiActionIssue[] = [];
  for (const occurrence of occurrences) {
    if (!familyByKey.has(familyKey(occurrence))) {
      issues.push(Object.freeze({ ...occurrence, code: "CONDITIONAL_ACTION_FAMILY_MISSING" }));
    }
  }
  for (const family of families) {
    if (!occurrenceKeys.has(familyKey(family))) {
      issues.push(Object.freeze({ code: "CONDITIONAL_ACTION_FAMILY_STALE", expression: family.expression, source: family.source }));
      continue;
    }
    if (family.actionIds.length === 0) {
      issues.push(Object.freeze({ code: "CONDITIONAL_ACTION_FAMILY_EMPTY", expression: family.expression, source: family.source }));
      continue;
    }
    for (const actionId of family.actionIds) {
      if (!classified.has(actionId)) {
        issues.push(Object.freeze({
          actionId,
          code: "CONDITIONAL_ACTION_UNCLASSIFIED",
          expression: family.expression,
          source: family.source,
        }));
      }
    }
  }
  return Object.freeze(issues);
}

function compatible(left: UiActionContract, right: UiActionContract): boolean {
  return left.availability === right.availability
    && left.boundary === right.boundary
    && left.target === right.target
    && left.durability === right.durability;
}

function countLines(contents: string, index: number): number {
  let line = 1;
  for (let cursor = 0; cursor < index; cursor += 1) {
    if (contents.charCodeAt(cursor) === 10) line += 1;
  }
  return line;
}

function familyKey(value: { readonly source: string; readonly expression: string }): string {
  return `${value.source}\u0000${value.expression}`;
}

function normalizeExpression(expression: string): string {
  return expression.trim().replace(/\s+/g, " ");
}
