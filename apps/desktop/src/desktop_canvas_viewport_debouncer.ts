import type { DesktopCanvasMutationDraft } from "./tauri_canvas_transport.ts";

type ViewportDraft = Extract<DesktopCanvasMutationDraft, { readonly kind: "update_viewport" }>;
export type DesktopCanvasViewportDebounceState = "Idle" | "Scheduled" | "Disposed";

export interface DesktopCanvasViewportScheduler {
  schedule(delayMs: number, callback: () => void): unknown;
  cancel(handle: unknown): void;
}

export interface DesktopCanvasViewportDebouncer {
  queue(draft: ViewportDraft, dispatch: (draft: ViewportDraft) => void): void;
  dispose(): void;
  state(): DesktopCanvasViewportDebounceState;
}

export function createDesktopCanvasViewportDebouncer(
  scheduler: DesktopCanvasViewportScheduler,
  delayMs: number,
): DesktopCanvasViewportDebouncer {
  if (!Number.isInteger(delayMs) || delayMs <= 0) throw new Error("CANVAS_VIEWPORT_INVALID_DELAY");
  let state: DesktopCanvasViewportDebounceState = "Idle";
  let handle: unknown;
  let generation = 0;

  return Object.freeze({
    queue(draft: ViewportDraft, dispatch: (draft: ViewportDraft) => void): void {
      if (state === "Disposed") return;
      if (state === "Scheduled") scheduler.cancel(handle);
      const currentGeneration = ++generation;
      state = "Scheduled";
      handle = scheduler.schedule(delayMs, () => {
        if (state !== "Scheduled" || generation !== currentGeneration) return;
        state = "Idle";
        handle = undefined;
        dispatch(draft);
      });
    },
    dispose(): void {
      if (state === "Disposed") return;
      if (state === "Scheduled") scheduler.cancel(handle);
      generation += 1;
      handle = undefined;
      state = "Disposed";
    },
    state(): DesktopCanvasViewportDebounceState {
      return state;
    },
  });
}
