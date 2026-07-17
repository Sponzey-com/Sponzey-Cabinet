export const DOCUMENT_HISTORY_VIRTUALIZATION_THRESHOLD = 100;
export const DOCUMENT_HISTORY_WINDOW_SIZE = 50;

export interface DocumentHistoryWindow {
  readonly start: number;
  readonly endExclusive: number;
  readonly total: number;
  readonly windowSize: number;
  readonly virtualized: boolean;
  readonly hasPrevious: boolean;
  readonly hasNext: boolean;
}

export type DocumentHistoryFocusRequest = "None" | "FocusFirstVisible";

export interface DocumentHistoryWindowTransition {
  readonly window: DocumentHistoryWindow;
  readonly focusRequest: DocumentHistoryFocusRequest;
}

export function createDocumentHistoryWindow(totalInput: number, startInput = 0): DocumentHistoryWindow {
  const total = Math.max(0, Math.trunc(totalInput));
  const virtualized = total > DOCUMENT_HISTORY_VIRTUALIZATION_THRESHOLD;
  if (!virtualized) {
    return {
      start: 0, endExclusive: total, total, windowSize: DOCUMENT_HISTORY_WINDOW_SIZE,
      virtualized: false, hasPrevious: false, hasNext: false,
    };
  }
  const lastStart = Math.floor((total - 1) / DOCUMENT_HISTORY_WINDOW_SIZE) * DOCUMENT_HISTORY_WINDOW_SIZE;
  const requestedStart = Math.trunc(startInput / DOCUMENT_HISTORY_WINDOW_SIZE) * DOCUMENT_HISTORY_WINDOW_SIZE;
  const start = Math.min(lastStart, Math.max(0, requestedStart));
  const endExclusive = Math.min(total, start + DOCUMENT_HISTORY_WINDOW_SIZE);
  return {
    start, endExclusive, total, windowSize: DOCUMENT_HISTORY_WINDOW_SIZE,
    virtualized: true, hasPrevious: start > 0, hasNext: endExclusive < total,
  };
}

export function nextDocumentHistoryWindow(state: DocumentHistoryWindow): DocumentHistoryWindowTransition {
  if (!state.hasNext) return { window: state, focusRequest: "None" };
  return {
    window: createDocumentHistoryWindow(state.total, state.start + state.windowSize),
    focusRequest: "FocusFirstVisible",
  };
}

export function previousDocumentHistoryWindow(state: DocumentHistoryWindow): DocumentHistoryWindowTransition {
  if (!state.hasPrevious) return { window: state, focusRequest: "None" };
  return {
    window: createDocumentHistoryWindow(state.total, state.start - state.windowSize),
    focusRequest: "FocusFirstVisible",
  };
}

export function reconcileDocumentHistoryWindow(
  state: DocumentHistoryWindow,
  total: number,
  identitiesChanged: boolean,
): DocumentHistoryWindow {
  return createDocumentHistoryWindow(total, identitiesChanged ? 0 : state.start);
}

export function historyIdentityChangeRequiresReset(previous: readonly string[], current: readonly string[]): boolean {
  if (previous.length === 0) return false;
  if (previous.length > current.length) return true;
  return previous.some((identity, index) => current[index] !== identity);
}
