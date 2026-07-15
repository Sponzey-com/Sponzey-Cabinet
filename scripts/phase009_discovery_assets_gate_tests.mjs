import assert from "node:assert/strict";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  DiscoveryAssetsGateErrorCode,
  renderPhase009DiscoveryAssetsGateArtifact,
  validatePhase009DiscoveryAssetsEvidence,
} from "./phase009_discovery_assets_gate.mjs";

test("Phase009 discovery assets gate accepts visible search backlink graph and asset evidence", () => {
  const result = validatePhase009DiscoveryAssetsEvidence(completeEvidence());

  assert.equal(result.ok, true);
  assert.equal(result.marker, "phase009_discovery_assets_gate=passed");
  assert.equal(result.changedLayers.includes("browser-smoke"), true);
  assert.equal(result.changedLayers.includes("gate-tooling"), true);
});

test("Phase009 discovery assets gate rejects missing document authoring prerequisite", () => {
  const result = validatePhase009DiscoveryAssetsEvidence({
    ...completeEvidence(),
    authoringGateText: "phase009_document_authoring_gate=failed",
  });

  assert.equal(result.ok, false);
  assert.equal(result.errorCode, DiscoveryAssetsGateErrorCode.AuthoringGateMissing);
});

test("Phase009 discovery assets gate rejects missing visible graph or backlink evidence", () => {
  const result = validatePhase009DiscoveryAssetsEvidence({
    ...completeEvidence(),
    browserSmokeText: completeEvidence().browserSmokeText.replace("graphEdgeRendered", "graphEdgeMissing"),
  });

  assert.equal(result.ok, false);
  assert.equal(result.errorCode, DiscoveryAssetsGateErrorCode.BrowserSmokeEvidenceMissing);
});

test("Phase009 discovery assets artifact excludes raw query paths and asset content", () => {
  const result = validatePhase009DiscoveryAssetsEvidence(completeEvidence());
  const artifact = renderPhase009DiscoveryAssetsGateArtifact(result);

  assert.equal(artifact.includes("phase009_discovery_assets_gate=passed"), true);
  assert.equal(artifact.includes("secret raw query text"), false);
  assert.equal(artifact.includes("/Users/example/private-asset.pdf"), false);
  assert.equal(artifact.includes("asset binary content should not leak"), false);
});

test("Phase009 discovery assets gate CLI writes marker artifact", async () => {
  const root = await mkdtemp(join(tmpdir(), "phase009-discovery-assets-gate-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, "packages/ui/tests"), { recursive: true });
  await mkdir(join(root, "apps/desktop/tests"), { recursive: true });
  await mkdir(join(root, "apps/web/public"), { recursive: true });
  await mkdir(join(root, "scripts"), { recursive: true });

  await writeFile(
    join(root, ".tasks/phase009-document-authoring-gate-result.md"),
    "phase009_document_authoring_gate=passed",
  );
  await writeFile(
    join(root, "packages/ui/tests/local_discovery_panel_model_tests.ts"),
    completeEvidence().uiDiscoveryTestText,
  );
  await writeFile(
    join(root, "apps/desktop/tests/desktop_discovery_smoke_tests.ts"),
    completeEvidence().desktopDiscoveryTestText,
  );
  await writeFile(join(root, "apps/web/public/app.js"), completeEvidence().webAppText);
  await writeFile(join(root, "scripts/run_browser_smoke.mjs"), completeEvidence().browserSmokeText);

  const { runPhase009DiscoveryAssetsGate } = await import("./phase009_discovery_assets_gate.mjs");
  const result = await runPhase009DiscoveryAssetsGate({ rootDir: root });

  assert.equal(result.ok, true);
});

function completeEvidence() {
  return {
    authoringGateText: "phase009_document_authoring_gate=passed",
    uiDiscoveryTestText:
      "local discovery panel separates backlinks unresolved links and asset metadata asset binary content should not leak",
    desktopDiscoveryTestText:
      "desktop local discovery smoke hides raw query and asset content desktop graph smoke uses neighborhood contract",
    webAppText:
      "data-cabinet-discovery-panel data-cabinet-link-group data-cabinet-graph-panel data-cabinet-asset-panel data-cabinet-asset-metadata",
    browserSmokeText:
      "searchResultFound backlinkRendered unresolvedLinkRendered graphEdgeRendered graphNodeRendered assetMetadataDetailRendered assetPathHidden",
  };
}
