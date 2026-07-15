import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("desktop app launcher starts the Tauri devUrl server before cargo run", async () => {
  const launcher = await readFile("scripts/run_desktop_app.sh", "utf8");

  assert.match(launcher, /dev_port="5173"/);
  assert.match(launcher, /SPONZEY_CABINET_WEB_PUBLIC_DIR=apps\/desktop\/dist/);
  assert.match(launcher, /SPONZEY_CABINET_REQUIRE_EXACT_PORT=1/);
  assert.match(launcher, /node scripts\/run_web_app\.mjs "\$dev_port" &/);
  assert.ok(
    launcher.indexOf("node scripts/run_web_app.mjs") <
      launcher.indexOf("cargo run -p cabinet-desktop-shell"),
  );
});

test("web app server supports exact-port mode for Tauri devUrl", async () => {
  const server = await readFile("scripts/run_web_app.mjs", "utf8");

  assert.match(server, /SPONZEY_CABINET_REQUIRE_EXACT_PORT/);
  assert.match(server, /exact port is required/);
  assert.match(server, /Port \$\{candidatePort\} is unavailable; trying \$\{nextPort\}/);
});
