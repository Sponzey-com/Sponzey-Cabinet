export const MAX_CANVAS_TEXT_EDIT_CHARS = 20_000;

export type CanvasTextEditDialogState =
  | Readonly<{ readonly kind: "Closed" }>
  | Readonly<{
      readonly kind: "Editing";
      readonly nodeId: string;
      readonly originalText: string;
      readonly draft: string;
    }>;

export interface CanvasTextEditIntent {
  readonly nodeId: string;
  readonly text: string;
}

export function createClosedCanvasTextEditDialog(): CanvasTextEditDialogState {
  return Object.freeze({ kind: "Closed" });
}

export function openCanvasTextEditDialog(
  state: CanvasTextEditDialogState,
  nodeId: string,
  text: string,
  editable: boolean,
): CanvasTextEditDialogState {
  const identity = nodeId.trim();
  if (!editable || !identity || !text.trim()) return state;
  return Object.freeze({ kind: "Editing", nodeId: identity, originalText: text, draft: text });
}

export function changeCanvasTextEditDraft(
  state: CanvasTextEditDialogState,
  draft: string,
): CanvasTextEditDialogState {
  if (state.kind !== "Editing") return state;
  return Object.freeze({ ...state, draft });
}

export function closeCanvasTextEditDialog(_: CanvasTextEditDialogState): CanvasTextEditDialogState {
  return createClosedCanvasTextEditDialog();
}

export function createCanvasTextEditIntent(
  state: CanvasTextEditDialogState | undefined,
): CanvasTextEditIntent | undefined {
  if (!state || state.kind !== "Editing") return undefined;
  const text = state.draft.trim();
  if (!text || text === state.originalText.trim() || text.length > MAX_CANVAS_TEXT_EDIT_CHARS) {
    return undefined;
  }
  return Object.freeze({ nodeId: state.nodeId, text });
}
