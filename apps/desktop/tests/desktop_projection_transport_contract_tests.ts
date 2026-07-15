import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("packaged desktop registers bounded projection runtime and command", () => {
  const source = readFileSync(
    new URL("../src-tauri/src/main.rs", import.meta.url),
    "utf8",
  );

  assert.match(source, /DesktopProjectionRuntime::new\(/);
  assert.match(source, /app\.manage\(projection\)/);
  assert.match(source, /fn run_desktop_projection_worker\(/);
  assert.match(source, /run_desktop_projection_worker,\s*\n/);
  assert.match(source, /fn get_desktop_projection_freshness\(/);
  assert.match(source, /fn request_desktop_projection_reindex\(/);
  assert.match(source, /get_desktop_projection_freshness,\s*\n/);
  assert.match(source, /request_desktop_projection_reindex,\s*\n/);
});

test("authoring transport triggers projection without document payload", () => {
  const source = readFileSync(
    new URL("../src/tauri_authoring_transport.ts", import.meta.url),
    "utf8",
  );

  assert.match(
    source,
    /invoke\("run_desktop_projection_worker"\)\.catch/,
  );
  assert.doesNotMatch(
    source,
    /invoke\("run_desktop_projection_worker",\s*\{[^}]*body/s,
  );
});
