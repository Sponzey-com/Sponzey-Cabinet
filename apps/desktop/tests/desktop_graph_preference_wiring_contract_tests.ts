import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";

const entry = readFileSync(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");
const native = readFileSync(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");

test("desktop composition loads graph preference once and saves only explicit graph filters", () => {
  assert.match(entry, /createTauriGraphPreferenceTransport\(bootstrapInvoke\)/);
  assert.match(entry, /graphPreferenceClient\.load\(homeQuery\.workspaceId\)/);
  assert.match(entry, /preferenceFromGraphQuery/);
  assert.match(entry, /graphPreferenceClient\.save/);
  assert.doesNotMatch(entry, /localStorage|sessionStorage/);
});

test("native graph preference commands are managed and registered at the Tauri boundary", () => {
  assert.match(native, /fn get_desktop_graph_preference/);
  assert.match(native, /fn save_desktop_graph_preference/);
  assert.match(native, /app\.manage\(DesktopGraphPreferenceRuntime::new\(app_data_dir\.clone\(\)\)\)/);
  assert.match(native, /get_desktop_graph_preference,[\s\S]*save_desktop_graph_preference/);
});

