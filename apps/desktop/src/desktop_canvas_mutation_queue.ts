import {
  requestDesktopCanvasMutation,
  runDesktopCanvasMutation,
  type DesktopCanvasSurfaceSnapshot,
} from "./desktop_canvas_controller.ts";
import type { DesktopCanvasClient, DesktopCanvasMutationDraft } from "./tauri_canvas_transport.ts";

export type DesktopCanvasMutationQueueState = "Idle" | "Saving" | "SaveQueued" | "Disposed";
export type DesktopCanvasMutationEnqueueResult = "accepted" | "full" | "disposed";

export interface DesktopCanvasMutationQueueDependencies {
  readonly client: DesktopCanvasClient;
  readonly operationIdSource: () => string;
  readonly readSnapshot: () => DesktopCanvasSurfaceSnapshot;
  readonly commitSnapshot: (snapshot: DesktopCanvasSurfaceSnapshot) => void;
  readonly capacity: number;
}

export interface DesktopCanvasMutationQueue {
  enqueue(draft: DesktopCanvasMutationDraft): DesktopCanvasMutationEnqueueResult;
  state(): DesktopCanvasMutationQueueState;
  pendingCount(): number;
  whenIdle(): Promise<void>;
  dispose(): void;
}

export function createDesktopCanvasMutationQueue(
  dependencies: DesktopCanvasMutationQueueDependencies,
): DesktopCanvasMutationQueue {
  if (!Number.isSafeInteger(dependencies.capacity) || dependencies.capacity < 1) {
    throw new Error("CANVAS_MUTATION_QUEUE_CAPACITY_INVALID");
  }
  const pending: DesktopCanvasMutationDraft[] = [];
  let queueState: DesktopCanvasMutationQueueState = "Idle";
  let drainPromise: Promise<void> | undefined;

  const drain = async (): Promise<void> => {
    try {
      while (queueState !== "Disposed" && pending.length > 0) {
        const draft = pending.shift();
        if (!draft) break;
        queueState = pending.length > 0 ? "SaveQueued" : "Saving";
        const current = dependencies.readSnapshot();
        const mutating = requestDesktopCanvasMutation(
          current,
          draft,
          dependencies.operationIdSource(),
        );
        if (mutating === current) {
          pending.length = 0;
          break;
        }
        dependencies.commitSnapshot(mutating);
        const result = await runDesktopCanvasMutation(dependencies.client, mutating);
        if (queueState === "Disposed") break;
        dependencies.commitSnapshot(result);
        if (result.state !== "Ready") {
          pending.length = 0;
          break;
        }
      }
    } finally {
      if (queueState !== "Disposed") queueState = "Idle";
    }
  };

  return Object.freeze({
    enqueue(draft: DesktopCanvasMutationDraft): DesktopCanvasMutationEnqueueResult {
      if (queueState === "Disposed") return "disposed";
      if (pending.length >= dependencies.capacity) return "full";
      pending.push(Object.freeze({ ...draft }) as DesktopCanvasMutationDraft);
      queueState = drainPromise || pending.length > 1 ? "SaveQueued" : "Saving";
      if (!drainPromise) {
        drainPromise = drain().finally(() => { drainPromise = undefined; });
      }
      return "accepted";
    },
    state(): DesktopCanvasMutationQueueState {
      return queueState;
    },
    pendingCount(): number {
      return pending.length;
    },
    whenIdle(): Promise<void> {
      return drainPromise ?? Promise.resolve();
    },
    dispose(): void {
      if (queueState === "Disposed") return;
      queueState = "Disposed";
      pending.length = 0;
    },
  });
}
