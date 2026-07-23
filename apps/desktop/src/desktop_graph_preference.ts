export interface DesktopGraphCameraPreference {
  readonly centerX: number;
  readonly centerY: number;
  readonly zoomPercent: number;
}

export interface DesktopGraphPreference {
  readonly schemaVersion: 2;
  readonly depth: 1 | 2;
  readonly direction: "incoming" | "outgoing" | "both";
  readonly includeUnresolved: boolean;
  readonly includeAssets: boolean;
  readonly includeExternal: boolean;
  readonly camera: DesktopGraphCameraPreference;
}

export interface DesktopGraphPreferenceParseResult {
  readonly valid: boolean;
  readonly preference: DesktopGraphPreference;
}

const MAX_CAMERA_COORDINATE = 1_000_000;
const MIN_ZOOM_PERCENT = 25;
const MAX_ZOOM_PERCENT = 400;

export function createDefaultDesktopGraphPreference(): DesktopGraphPreference {
  return Object.freeze({
    schemaVersion: 2,
    depth: 1,
    direction: "both",
    includeUnresolved: true,
    includeAssets: true,
    includeExternal: false,
    camera: Object.freeze({ centerX: 0, centerY: 0, zoomPercent: 100 }),
  });
}

export function parseDesktopGraphPreference(value: unknown): DesktopGraphPreferenceParseResult {
  const fallback = createDefaultDesktopGraphPreference();
  if (!isRecord(value) || ![1, 2].includes(Number(value.schemaVersion)) || !isRecord(value.camera)) {
    return Object.freeze({ valid: false, preference: fallback });
  }
  const camera = value.camera;
  if (
    (value.depth !== 1 && value.depth !== 2)
    || !["incoming", "outgoing", "both"].includes(String(value.direction))
    || typeof value.includeUnresolved !== "boolean"
    || typeof value.includeAssets !== "boolean"
    || (value.schemaVersion === 2 && typeof value.includeExternal !== "boolean")
    || !boundedFinite(camera.centerX, -MAX_CAMERA_COORDINATE, MAX_CAMERA_COORDINATE)
    || !boundedFinite(camera.centerY, -MAX_CAMERA_COORDINATE, MAX_CAMERA_COORDINATE)
    || !boundedFinite(camera.zoomPercent, MIN_ZOOM_PERCENT, MAX_ZOOM_PERCENT)
  ) return Object.freeze({ valid: false, preference: fallback });

  return Object.freeze({
    valid: true,
    preference: Object.freeze({
      schemaVersion: 2,
      depth: value.depth,
      direction: value.direction as DesktopGraphPreference["direction"],
      includeUnresolved: value.includeUnresolved,
      includeAssets: value.includeAssets,
      includeExternal: value.schemaVersion === 2 ? value.includeExternal as boolean : false,
      camera: Object.freeze({
        centerX: camera.centerX as number,
        centerY: camera.centerY as number,
        zoomPercent: camera.zoomPercent as number,
      }),
    }),
  });
}

export function graphQueryPatchFromPreference(preference: DesktopGraphPreference) {
  return Object.freeze({
    depth: preference.depth,
    direction: preference.direction,
    includeUnresolved: preference.includeUnresolved,
    includeAssets: preference.includeAssets,
  });
}

export function preferenceFromGraphQuery(
  query: Pick<DesktopGraphPreference, "depth" | "direction" | "includeUnresolved" | "includeAssets">,
  camera: DesktopGraphCameraPreference,
  includeExternal = false,
): DesktopGraphPreference {
  return parseDesktopGraphPreference({ schemaVersion: 2, ...query, includeExternal, camera }).preference;
}

export function rendererCameraFromPreference(camera: DesktopGraphCameraPreference) {
  return Object.freeze({ x: camera.centerX, y: camera.centerY, ratio: 100 / camera.zoomPercent });
}

export function cameraPreferenceFromRenderer(
  camera: Readonly<{ readonly x: number; readonly y: number; readonly ratio: number }>,
): DesktopGraphCameraPreference | undefined {
  if (![camera.x, camera.y, camera.ratio].every(Number.isFinite) || camera.ratio <= 0) return undefined;
  const candidate = Object.freeze({ centerX: camera.x, centerY: camera.y, zoomPercent: 100 / camera.ratio });
  const parsed = parseDesktopGraphPreference({
    ...createDefaultDesktopGraphPreference(),
    camera: candidate,
  });
  return parsed.valid ? parsed.preference.camera : undefined;
}

function boundedFinite(value: unknown, minimum: number, maximum: number): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= minimum && value <= maximum;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
