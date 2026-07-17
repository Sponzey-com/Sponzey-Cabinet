export const DOCUMENT_DIFF_HUNK_WINDOW_SIZE = 50;

export interface DocumentDiffHunkWindow {
  readonly start: number;
  readonly endExclusive: number;
  readonly total: number;
  readonly size: number;
  readonly hasPrevious: boolean;
  readonly hasNext: boolean;
}

export function createDocumentDiffHunkWindow(
  totalInput: number,
  startInput = 0,
): DocumentDiffHunkWindow {
  const total = Math.max(0, Math.trunc(totalInput));
  const lastStart = total === 0
    ? 0
    : Math.floor((total - 1) / DOCUMENT_DIFF_HUNK_WINDOW_SIZE) * DOCUMENT_DIFF_HUNK_WINDOW_SIZE;
  const start = Math.min(lastStart, Math.max(0, Math.trunc(startInput / DOCUMENT_DIFF_HUNK_WINDOW_SIZE) * DOCUMENT_DIFF_HUNK_WINDOW_SIZE));
  const endExclusive = Math.min(total, start + DOCUMENT_DIFF_HUNK_WINDOW_SIZE);
  return {
    start,
    endExclusive,
    total,
    size: DOCUMENT_DIFF_HUNK_WINDOW_SIZE,
    hasPrevious: start > 0,
    hasNext: endExclusive < total,
  };
}

export function nextDocumentDiffHunkWindow(
  state: DocumentDiffHunkWindow,
): DocumentDiffHunkWindow {
  return state.hasNext
    ? createDocumentDiffHunkWindow(state.total, state.start + state.size)
    : state;
}

export function previousDocumentDiffHunkWindow(
  state: DocumentDiffHunkWindow,
): DocumentDiffHunkWindow {
  return state.hasPrevious
    ? createDocumentDiffHunkWindow(state.total, state.start - state.size)
    : state;
}
