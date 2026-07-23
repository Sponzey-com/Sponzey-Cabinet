import type { DesktopGraphCameraPreference } from "./desktop_graph_preference.ts";

export interface DesktopGraphCameraSaveScheduler {
  queue(camera: DesktopGraphCameraPreference, save: (camera: DesktopGraphCameraPreference) => void): void;
  dispose(): void;
  state(): "Idle" | "Scheduled" | "Disposed";
}

export function createDesktopGraphCameraSaveScheduler(options: {
  readonly delayMs: number;
  readonly schedule: (run: () => void, delayMs: number) => unknown;
  readonly cancel: (handle: unknown) => void;
}): DesktopGraphCameraSaveScheduler {
  let phase: "Idle" | "Scheduled" | "Disposed" = "Idle";
  let handle: unknown;
  return Object.freeze({
    queue(camera, save) {
      if (phase === "Disposed") return;
      if (phase === "Scheduled") options.cancel(handle);
      phase = "Scheduled";
      handle = options.schedule(() => {
        if (phase !== "Scheduled") return;
        phase = "Idle";
        handle = undefined;
        save(camera);
      }, options.delayMs);
    },
    dispose() {
      if (phase === "Disposed") return;
      if (phase === "Scheduled") options.cancel(handle);
      handle = undefined;
      phase = "Disposed";
    },
    state: () => phase,
  });
}
