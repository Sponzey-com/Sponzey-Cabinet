import assert from "node:assert/strict";
import test from "node:test";

import {
  applyDesktopGraphPreferenceLoad,
  applyDesktopGraphPreferenceSaveFailure,
  createDesktopGraphPreferenceSnapshot,
  requestDesktopGraphPreferenceLoad,
  requestDesktopGraphPreferenceSave,
} from "../src/desktop_graph_preference_controller.ts";
import { createDefaultDesktopGraphPreference } from "../src/desktop_graph_preference.ts";

test("graph preference lifecycle defaults corrupt load and ignores stale workspace completion", () => {
  const initial = createDesktopGraphPreferenceSnapshot("workspace-1");
  const loading = requestDesktopGraphPreferenceLoad(initial);
  const stale = applyDesktopGraphPreferenceLoad(loading, loading.generation - 1, "workspace-1", { broken: true });
  assert.equal(stale, loading);
  const other = applyDesktopGraphPreferenceLoad(loading, loading.generation, "workspace-2", createDefaultDesktopGraphPreference());
  assert.equal(other, loading);
  const defaulted = applyDesktopGraphPreferenceLoad(loading, loading.generation, "workspace-1", { broken: true });
  assert.equal(defaulted.state, "Defaulted");
  assert.deepEqual(defaulted.preference, createDefaultDesktopGraphPreference());
});

test("graph preference save keeps validated session data when persistence fails", () => {
  const loading = requestDesktopGraphPreferenceLoad(createDesktopGraphPreferenceSnapshot("workspace-1"));
  const ready = applyDesktopGraphPreferenceLoad(loading, loading.generation, "workspace-1", createDefaultDesktopGraphPreference());
  const saving = requestDesktopGraphPreferenceSave(ready, { ...ready.preference, depth: 2 });
  assert.equal(saving.state, "Saving");
  assert.equal(saving.preference.depth, 2);
  const failed = applyDesktopGraphPreferenceSaveFailure(saving, saving.generation, "workspace-1");
  assert.equal(failed.state, "SaveFailed");
  assert.equal(failed.preference.depth, 2);
});

