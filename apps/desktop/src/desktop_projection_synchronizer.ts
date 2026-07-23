import type { DesktopProjectionTransport } from "./tauri_projection_transport.ts";

export const ProjectionSynchronizationState = Object.freeze({
  Idle: "Idle",
  Reindexing: "Reindexing",
  Running: "Running",
  Completed: "Completed",
  Failed: "Failed",
} as const);

export type ProjectionSynchronizationStateValue = typeof ProjectionSynchronizationState[keyof typeof ProjectionSynchronizationState];
type ProjectionSynchronizationEvent = "start" | "reindexed" | "completed" | "failed";

export type ProjectionSynchronizationResult =
  | Readonly<{ readonly state: "Completed"; readonly readyCount: number }>
  | Readonly<{ readonly state: "Failed"; readonly errorCode: "PROJECTION_IDENTITY_INVALID" | "PROJECTION_SYNC_INCOMPLETE" | "PROJECTION_SYNC_FAILED" }>;

export function transitionProjectionSynchronization(
  state: ProjectionSynchronizationStateValue,
  event: ProjectionSynchronizationEvent,
): ProjectionSynchronizationStateValue {
  if (event === "failed") return ProjectionSynchronizationState.Failed;
  const transitions = new Map<string, ProjectionSynchronizationStateValue>([
    [`${ProjectionSynchronizationState.Idle}:start`, ProjectionSynchronizationState.Reindexing],
    [`${ProjectionSynchronizationState.Reindexing}:reindexed`, ProjectionSynchronizationState.Running],
    [`${ProjectionSynchronizationState.Running}:completed`, ProjectionSynchronizationState.Completed],
  ]);
  return transitions.get(`${state}:${event}`) ?? ProjectionSynchronizationState.Failed;
}

export async function synchronizeCurrentDocumentProjections(
  projection: Pick<DesktopProjectionTransport, "requestReindex" | "runWorker">,
  workspaceId: string,
  documentId: string,
  onStateChanged: (state: ProjectionSynchronizationStateValue) => void = () => {},
): Promise<ProjectionSynchronizationResult> {
  const workspace = workspaceId.trim();
  const document = documentId.trim();
  if (!workspace || !document) {
    return Object.freeze({ state: "Failed", errorCode: "PROJECTION_IDENTITY_INVALID" });
  }

  let state = transitionProjectionSynchronization(ProjectionSynchronizationState.Idle, "start");
  onStateChanged(state);
  try {
    await projection.requestReindex(workspace, document);
    state = transitionProjectionSynchronization(state, "reindexed");
    onStateChanged(state);
    const worker = await projection.runWorker();
    if (worker.failedCount > 0 || worker.retryScheduledCount > 0) {
      onStateChanged(transitionProjectionSynchronization(state, "failed"));
      return Object.freeze({ state: "Failed", errorCode: "PROJECTION_SYNC_INCOMPLETE" });
    }
    state = transitionProjectionSynchronization(state, "completed");
    onStateChanged(state);
    return Object.freeze({ state: "Completed", readyCount: worker.readyCount });
  } catch {
    onStateChanged(transitionProjectionSynchronization(state, "failed"));
    return Object.freeze({ state: "Failed", errorCode: "PROJECTION_SYNC_FAILED" });
  }
}
