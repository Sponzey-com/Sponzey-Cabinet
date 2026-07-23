export interface DesktopSearchResultWindow {
  readonly generation: number;
  readonly offset: number;
  readonly pageSize: number;
  readonly total: number;
}

export type DesktopSearchResultWindowEvent =
  | Readonly<{ type: "Next" }>
  | Readonly<{ type: "Previous" }>
  | Readonly<{ type: "Reconcile"; generation: number; total: number }>;

export interface DesktopSearchResultWindowSelection<T> {
  readonly items: readonly T[];
  readonly start: number;
  readonly end: number;
  readonly canPrevious: boolean;
  readonly canNext: boolean;
}

const DEFAULT_PAGE_SIZE = 20;

export function createDesktopSearchResultWindow(
  generation: number,
  total: number,
  pageSize = DEFAULT_PAGE_SIZE,
): DesktopSearchResultWindow {
  requireValues(generation, total, pageSize);
  return Object.freeze({ generation, offset: 0, pageSize, total });
}

export function transitionDesktopSearchResultWindow(
  state: DesktopSearchResultWindow,
  event: DesktopSearchResultWindowEvent,
): DesktopSearchResultWindow {
  requireState(state);
  if (event.type === "Reconcile") {
    requireValues(event.generation, event.total, state.pageSize);
    if (event.generation !== state.generation) {
      return createDesktopSearchResultWindow(event.generation, event.total, state.pageSize);
    }
    const offset = lastPageOffset(event.total, state.pageSize, state.offset);
    if (event.total === state.total && offset === state.offset) return state;
    return Object.freeze({ ...state, total: event.total, offset });
  }
  if (event.type === "Next") {
    const offset = state.offset + state.pageSize;
    if (offset >= state.total) return state;
    return Object.freeze({ ...state, offset });
  }
  if (state.offset === 0) return state;
  return Object.freeze({ ...state, offset: Math.max(0, state.offset - state.pageSize) });
}

export function selectDesktopSearchResultWindow<T>(
  state: DesktopSearchResultWindow,
  items: readonly T[],
): DesktopSearchResultWindowSelection<T> {
  requireState(state);
  if (items.length !== state.total) throw new Error("INVALID_SEARCH_RESULT_WINDOW");
  const end = Math.min(state.total, state.offset + state.pageSize);
  return Object.freeze({
    items: Object.freeze(items.slice(state.offset, end)),
    start: state.total === 0 ? 0 : state.offset + 1,
    end,
    canPrevious: state.offset > 0,
    canNext: end < state.total,
  });
}

function lastPageOffset(total: number, pageSize: number, currentOffset: number): number {
  if (total === 0) return 0;
  const last = Math.floor((total - 1) / pageSize) * pageSize;
  return Math.min(currentOffset, last);
}

function requireState(state: DesktopSearchResultWindow): void {
  requireValues(state.generation, state.total, state.pageSize);
  if (!Number.isSafeInteger(state.offset) || state.offset < 0 || state.offset > state.total
    || state.offset % state.pageSize !== 0) {
    throw new Error("INVALID_SEARCH_RESULT_WINDOW");
  }
}

function requireValues(generation: number, total: number, pageSize: number): void {
  if (!Number.isSafeInteger(generation) || generation < 0
    || !Number.isSafeInteger(total) || total < 0 || total > 100
    || !Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > DEFAULT_PAGE_SIZE) {
    throw new Error("INVALID_SEARCH_RESULT_WINDOW");
  }
}
