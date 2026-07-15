import assert from "node:assert/strict";
import test from "node:test";

import { createPlatformCapabilityMatrix } from "../src/index.ts";

test("platform capability matrix documents AI query, citation, and connector admin support", () => {
  const matrix = createPlatformCapabilityMatrix();

  for (const platform of [matrix.web, matrix.desktop, matrix.windows, matrix.macos, matrix.linux]) {
    assert.equal(platform.aiQuerySupport, "interactive");
    assert.equal(platform.aiCitationSupport, "interactive");
  }
  assert.equal(matrix.web.connectorAdminSupport, "interactive");
  assert.equal(matrix.desktop.connectorAdminSupport, "view_only");
  assert.equal(matrix.ios.aiQuerySupport, "interactive");
  assert.equal(matrix.android.aiQuerySupport, "interactive");
  assert.equal(matrix.ios.aiCitationSupport, "view_only");
  assert.equal(matrix.android.aiCitationSupport, "view_only");
  assert.equal(matrix.ios.connectorAdminSupport, "unsupported");
  assert.equal(matrix.android.connectorAdminSupport, "unsupported");
  assert.equal("permissionRules" in matrix.web, false);
});
