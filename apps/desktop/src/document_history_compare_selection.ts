export interface DocumentHistoryCompareSelectionItem {
  readonly versionId: string;
  readonly versionLabel: string;
}

export type DocumentHistoryCompareSelectionState =
  | { readonly status: "Idle"; readonly selections: readonly [] }
  | { readonly status: "OneSelected"; readonly selections: readonly [DocumentHistoryCompareSelectionItem] }
  | { readonly status: "TwoSelected"; readonly selections: readonly [DocumentHistoryCompareSelectionItem, DocumentHistoryCompareSelectionItem] };

export type DocumentHistoryCompareSelectionEvent =
  | ({ readonly type: "Toggle" } & DocumentHistoryCompareSelectionItem)
  | { readonly type: "Clear" };

export function createDocumentHistoryCompareSelection(): DocumentHistoryCompareSelectionState {
  return { status: "Idle", selections: [] };
}

export function transitionDocumentHistoryCompareSelection(
  state: DocumentHistoryCompareSelectionState,
  event: DocumentHistoryCompareSelectionEvent,
): DocumentHistoryCompareSelectionState {
  if (event.type === "Clear") return createDocumentHistoryCompareSelection();
  const versionId = event.versionId.trim();
  const versionLabel = event.versionLabel.trim();
  if (!versionId || !versionLabel) throw new Error("HISTORY_COMPARE_SELECTION_INVALID");

  const remaining = state.selections.filter((selection) => selection.versionId !== versionId);
  if (remaining.length !== state.selections.length) return fromSelections(remaining);
  return fromSelections([...remaining, { versionId, versionLabel }].slice(-2));
}

function fromSelections(
  selections: readonly DocumentHistoryCompareSelectionItem[],
): DocumentHistoryCompareSelectionState {
  if (selections.length === 0) return createDocumentHistoryCompareSelection();
  if (selections.length === 1) return { status: "OneSelected", selections: [selections[0]!] };
  return { status: "TwoSelected", selections: [selections[0]!, selections[1]!] };
}
