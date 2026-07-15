import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { buildDesktopAssets } from "./phase011_desktop_asset_builder.mjs";

test("desktop asset builder bundles React desktop entry without web seed demo", async () => {
  const result = await buildDesktopAssets(process.cwd());
  const [html, css, bundle] = await Promise.all([
    readFile(result.indexHtml, "utf8"),
    readFile(result.stylesCss, "utf8"),
    readFile(result.appBundle, "utf8"),
  ]);

  assert.match(html, /data-cabinet-bootstrap-state="loading"/);
  assert.match(css, /\.desktop-shell/);
  assert.match(bundle, /get_desktop_workspace_home/);
  assert.match(bundle, /execute_desktop_document_authoring/);
  assert.match(bundle, /data-codemirror-host/);
  assert.match(bundle, /Mod-s/);
  assert.match(bundle, /data-cabinet-react-root/);
  for (const forbidden of [
    "seedWorkspace",
    "sponzey-cabinet.local-workspace.v1",
    "Reset Demo",
    "serverBaseUrl",
    "tenant_admin",
  ]) {
    assert.equal(bundle.includes(forbidden), false, forbidden);
  }
});
