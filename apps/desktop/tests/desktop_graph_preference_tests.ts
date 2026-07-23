import assert from "node:assert/strict";
import test from "node:test";

import {
  cameraPreferenceFromRenderer,
  createDefaultDesktopGraphPreference,
  parseDesktopGraphPreference,
  rendererCameraFromPreference,
} from "../src/desktop_graph_preference.ts";

test("graph preference defaults match the local product policy and are immutable", () => {
  const value = createDefaultDesktopGraphPreference();
  assert.deepEqual(value, {
    schemaVersion: 2,
    depth: 1,
    direction: "both",
    includeUnresolved: true,
    includeAssets: true,
    includeExternal: false,
    camera: { centerX: 0, centerY: 0, zoomPercent: 100 },
  });
  assert.equal(Object.isFrozen(value), true);
  assert.equal(Object.isFrozen(value.camera), true);
});

test("graph camera mapping is reversible and rejects unsafe renderer values", () => {
  const preference = { centerX: 0.25, centerY: 0.75, zoomPercent: 160 };
  const renderer = rendererCameraFromPreference(preference);
  assert.deepEqual(renderer, { x: 0.25, y: 0.75, ratio: 0.625 });
  assert.deepEqual(cameraPreferenceFromRenderer(renderer), preference);
  assert.equal(cameraPreferenceFromRenderer({ x: Number.NaN, y: 0.5, ratio: 1 }), undefined);
  assert.equal(cameraPreferenceFromRenderer({ x: 0.5, y: 0.5, ratio: 0.1 }), undefined);
});

test("graph preference parser accepts one bounded schema and rejects the whole invalid value", () => {
  const valid = parseDesktopGraphPreference({
    schemaVersion: 2, depth: 2, direction: "incoming", includeUnresolved: false,
    includeAssets: true, includeExternal: true, camera: { centerX: 120, centerY: -80, zoomPercent: 175 },
  });
  assert.equal(valid.valid, true);
  assert.equal(valid.preference.depth, 2);

  for (const invalid of [
    { ...valid.preference, schemaVersion: 3 },
    { ...valid.preference, direction: "sideways" },
    { ...valid.preference, camera: { centerX: Number.NaN, centerY: 0, zoomPercent: 100 } },
    { ...valid.preference, camera: { centerX: 0, centerY: 0, zoomPercent: 401 } },
  ]) {
    const parsed = parseDesktopGraphPreference(invalid);
    assert.equal(parsed.valid, false);
    assert.deepEqual(parsed.preference, createDefaultDesktopGraphPreference());
  }
});

test("graph preference migrates schema v1 with external links hidden", () => {
  const parsed = parseDesktopGraphPreference({
    schemaVersion: 1, depth: 1, direction: "both", includeUnresolved: true,
    includeAssets: true, camera: { centerX: 0, centerY: 0, zoomPercent: 100 },
  });
  assert.equal(parsed.valid, true);
  assert.equal(parsed.preference.schemaVersion, 2);
  assert.equal(parsed.preference.includeExternal, false);
});
