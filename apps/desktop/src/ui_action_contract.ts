import React from "react";

export type UiActionAvailability = "connected" | "hidden_out_of_scope";
export type UiActionBoundary = "route" | "view_state" | "query" | "usecase" | "native_command";
export type UiActionDurability = "none" | "readback" | "reopen";

export interface UiActionContract {
  readonly actionId: string;
  readonly surface: string;
  readonly availability: UiActionAvailability;
  readonly visibleCondition: string;
  readonly enabledCondition: string;
  readonly disabledReasonKey?: string;
  readonly hiddenReasonKey?: string;
  readonly input: string;
  readonly boundary: UiActionBoundary;
  readonly target: string;
  readonly progressState: string;
  readonly successReadback: string;
  readonly failureMapping: string;
  readonly recoveryAction: string;
  readonly durability: UiActionDurability;
  readonly interactionTest: string;
  readonly packagedTest: string;
}

export interface RenderedUiAction {
  readonly actionId: string;
  readonly enabled: boolean;
  readonly callbackConnected: boolean;
}

export type UiActionContractIssueCode =
  | "ACTION_CONTRACT_DUPLICATE"
  | "ACTION_CONTRACT_NOT_RENDERED"
  | "RENDERED_ACTION_UNCLASSIFIED"
  | "ENABLED_ACTION_CALLBACK_MISSING"
  | "DISABLED_ACTION_REASON_MISSING"
  | "HIDDEN_ACTION_RENDERED"
  | "RENDERED_CONTROL_ACTION_ID_MISSING";

export interface UiActionContractIssue {
  readonly code: UiActionContractIssueCode;
  readonly actionId: string;
}

export function defineUiActionContract(contract: UiActionContract): UiActionContract {
  return Object.freeze({ ...contract });
}

export function validateUiActionContracts(
  contracts: readonly UiActionContract[],
  renderedActions: readonly RenderedUiAction[],
  unidentifiedControlCount = 0,
): readonly UiActionContractIssue[] {
  const issues: UiActionContractIssue[] = [];
  const contractCounts = countByActionId(contracts);
  const renderedById = groupRenderedActions(renderedActions);

  if (unidentifiedControlCount > 0) {
    issues.push(issue("RENDERED_CONTROL_ACTION_ID_MISSING", `count:${unidentifiedControlCount}`));
  }

  for (const [actionId, count] of contractCounts) {
    if (count > 1) issues.push(issue("ACTION_CONTRACT_DUPLICATE", actionId));
  }

  const uniqueContracts = new Map<string, UiActionContract>();
  for (const contract of contracts) {
    if (!uniqueContracts.has(contract.actionId)) uniqueContracts.set(contract.actionId, contract);
  }
  for (const contract of uniqueContracts.values()) {
    const observations = renderedById.get(contract.actionId) ?? [];
    if (contract.availability === "hidden_out_of_scope") {
      if (observations.length > 0) issues.push(issue("HIDDEN_ACTION_RENDERED", contract.actionId));
      continue;
    }
    if (contract.visibleCondition === "always" && observations.length === 0) {
      issues.push(issue("ACTION_CONTRACT_NOT_RENDERED", contract.actionId));
    }
  }

  for (const [actionId, observations] of renderedById) {
    const contract = uniqueContracts.get(actionId);
    if (!contract) {
      issues.push(issue("RENDERED_ACTION_UNCLASSIFIED", actionId));
      continue;
    }
    if (contract.availability !== "connected") continue;
    if (observations.some((observation) => observation.enabled && !observation.callbackConnected)) {
      issues.push(issue("ENABLED_ACTION_CALLBACK_MISSING", actionId));
    }
    if (observations.some((observation) => !observation.enabled) && !contract.disabledReasonKey) {
      issues.push(issue("DISABLED_ACTION_REASON_MISSING", actionId));
    }
  }

  return Object.freeze(issues);
}

export interface ReactUiActionCollection {
  readonly actions: readonly RenderedUiAction[];
  readonly unidentifiedControlCount: number;
}

export function collectReactUiActions(node: React.ReactNode): ReactUiActionCollection {
  const actions: RenderedUiAction[] = [];
  let unidentifiedControlCount = 0;
  visit(node);
  return Object.freeze({ actions: Object.freeze(actions), unidentifiedControlCount });

  function visit(current: React.ReactNode): void {
    if (Array.isArray(current)) {
      current.forEach(visit);
      return;
    }
    if (!React.isValidElement(current)) return;
    const props = current.props as Record<string, unknown>;
    if (typeof current.type === "string" && isInteractiveHost(current.type, props)) {
      const actionId = props["data-action"];
      if (typeof actionId !== "string" || actionId.length === 0) {
        unidentifiedControlCount += 1;
      } else {
        actions.push(Object.freeze({
          actionId,
          enabled: props.disabled !== true,
          callbackConnected: hasInteractionCallback(current.type, props),
        }));
      }
    }
    visit(props.children as React.ReactNode);
  }
}

function isInteractiveHost(type: string, props: Record<string, unknown>): boolean {
  if (["button", "input", "select", "textarea"].includes(type)) return true;
  return type === "a" && typeof props.href === "string";
}

function hasInteractionCallback(type: string, props: Record<string, unknown>): boolean {
  if (type === "input" || type === "select" || type === "textarea") {
    return typeof props.onChange === "function" || typeof props.onInput === "function";
  }
  if (type === "a" && typeof props.href === "string") return true;
  return typeof props.onClick === "function";
}

function countByActionId(contracts: readonly UiActionContract[]): Map<string, number> {
  const counts = new Map<string, number>();
  for (const contract of contracts) counts.set(contract.actionId, (counts.get(contract.actionId) ?? 0) + 1);
  return counts;
}

function groupRenderedActions(renderedActions: readonly RenderedUiAction[]): Map<string, RenderedUiAction[]> {
  const grouped = new Map<string, RenderedUiAction[]>();
  for (const action of renderedActions) {
    const current = grouped.get(action.actionId) ?? [];
    current.push(action);
    grouped.set(action.actionId, current);
  }
  return grouped;
}

function issue(code: UiActionContractIssueCode, actionId: string): UiActionContractIssue {
  return Object.freeze({ code, actionId });
}
