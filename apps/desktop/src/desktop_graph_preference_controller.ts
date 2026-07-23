import {
  createDefaultDesktopGraphPreference,
  parseDesktopGraphPreference,
  type DesktopGraphPreference,
} from "./desktop_graph_preference.ts";

export type DesktopGraphPreferenceState = "Idle" | "Loading" | "Ready" | "Defaulted" | "Saving" | "SaveFailed";

export interface DesktopGraphPreferenceSnapshot {
  readonly state: DesktopGraphPreferenceState;
  readonly workspaceId: string;
  readonly generation: number;
  readonly preference: DesktopGraphPreference;
}

export function createDesktopGraphPreferenceSnapshot(workspaceId: string): DesktopGraphPreferenceSnapshot {
  return Object.freeze({ state: "Idle", workspaceId, generation: 0, preference: createDefaultDesktopGraphPreference() });
}

export function requestDesktopGraphPreferenceLoad(snapshot: DesktopGraphPreferenceSnapshot): DesktopGraphPreferenceSnapshot {
  return Object.freeze({ ...snapshot, state: "Loading", generation: snapshot.generation + 1 });
}

export function applyDesktopGraphPreferenceLoad(
  snapshot: DesktopGraphPreferenceSnapshot,
  generation: number,
  workspaceId: string,
  candidate: unknown,
): DesktopGraphPreferenceSnapshot {
  if (snapshot.state !== "Loading" || snapshot.generation !== generation || snapshot.workspaceId !== workspaceId) return snapshot;
  const parsed = parseDesktopGraphPreference(candidate);
  return Object.freeze({ ...snapshot, state: parsed.valid ? "Ready" : "Defaulted", preference: parsed.preference });
}

export function requestDesktopGraphPreferenceSave(
  snapshot: DesktopGraphPreferenceSnapshot,
  candidate: unknown,
): DesktopGraphPreferenceSnapshot {
  const parsed = parseDesktopGraphPreference(candidate);
  if (!parsed.valid) return snapshot;
  return Object.freeze({ ...snapshot, state: "Saving", generation: snapshot.generation + 1, preference: parsed.preference });
}

export function applyDesktopGraphPreferenceSave(
  snapshot: DesktopGraphPreferenceSnapshot,
  generation: number,
  workspaceId: string,
): DesktopGraphPreferenceSnapshot {
  if (snapshot.state !== "Saving" || snapshot.generation !== generation || snapshot.workspaceId !== workspaceId) return snapshot;
  return Object.freeze({ ...snapshot, state: "Ready" });
}

export function applyDesktopGraphPreferenceSaveFailure(
  snapshot: DesktopGraphPreferenceSnapshot,
  generation: number,
  workspaceId: string,
): DesktopGraphPreferenceSnapshot {
  if (snapshot.state !== "Saving" || snapshot.generation !== generation || snapshot.workspaceId !== workspaceId) return snapshot;
  return Object.freeze({ ...snapshot, state: "SaveFailed" });
}

