import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const DiscoveryAssetsGateErrorCode = Object.freeze({
  AuthoringGateMissing: "PHASE009_DISCOVERY_AUTHORING_GATE_MISSING",
  UiDiscoveryTestMissing: "PHASE009_DISCOVERY_UI_TEST_MISSING",
  DesktopDiscoveryTestMissing: "PHASE009_DISCOVERY_DESKTOP_TEST_MISSING",
  WebMarkerMissing: "PHASE009_DISCOVERY_WEB_MARKER_MISSING",
  BrowserSmokeEvidenceMissing: "PHASE009_DISCOVERY_BROWSER_SMOKE_MISSING",
  SensitiveArtifactContent: "PHASE009_DISCOVERY_SENSITIVE_ARTIFACT_CONTENT",
  IoFailed: "PHASE009_DISCOVERY_IO_FAILED",
});

const requiredUiDiscoveryTerms = [
  "local discovery panel separates backlinks unresolved links and asset metadata",
  "asset binary content should not leak",
];

const requiredDesktopDiscoveryTerms = [
  "desktop local discovery smoke hides raw query and asset content",
  "desktop graph smoke uses neighborhood contract",
];

const requiredWebMarkers = [
  "data-cabinet-discovery-panel",
  "data-cabinet-link-group",
  "data-cabinet-graph-panel",
  "data-cabinet-asset-panel",
  "data-cabinet-asset-metadata",
];

const requiredBrowserSmokeTerms = [
  "searchResultFound",
  "backlinkRendered",
  "unresolvedLinkRendered",
  "graphEdgeRendered",
  "graphNodeRendered",
  "assetMetadataDetailRendered",
  "assetPathHidden",
];

const sensitivePatterns = [
  /secret raw query text/i,
  /asset binary content should not leak/i,
  /\/Users\/[^`\s]+/i,
  /[A-Za-z]:\\Users\\/i,
  /provider_api_key_fixture/i,
  /token_fixture/i,
  /credential_fixture/i,
];

export function validatePhase009DiscoveryAssetsEvidence(evidence) {
  if (!evidence.authoringGateText.includes("phase009_document_authoring_gate=passed")) {
    return failed(DiscoveryAssetsGateErrorCode.AuthoringGateMissing, "authoring_gate_marker");
  }

  for (const term of requiredUiDiscoveryTerms) {
    if (!evidence.uiDiscoveryTestText.includes(term)) {
      return failed(DiscoveryAssetsGateErrorCode.UiDiscoveryTestMissing, term);
    }
  }

  for (const term of requiredDesktopDiscoveryTerms) {
    if (!evidence.desktopDiscoveryTestText.includes(term)) {
      return failed(DiscoveryAssetsGateErrorCode.DesktopDiscoveryTestMissing, term);
    }
  }

  for (const marker of requiredWebMarkers) {
    if (!evidence.webAppText.includes(marker)) {
      return failed(DiscoveryAssetsGateErrorCode.WebMarkerMissing, marker);
    }
  }

  for (const term of requiredBrowserSmokeTerms) {
    if (!evidence.browserSmokeText.includes(term)) {
      return failed(DiscoveryAssetsGateErrorCode.BrowserSmokeEvidenceMissing, term);
    }
  }

  return {
    ok: true,
    marker: "phase009_discovery_assets_gate=passed",
    changedLayers: ["browser-smoke", "gate-tooling"],
    validationCommands: [
      "node --test packages/ui/tests/local_discovery_panel_model_tests.ts",
      "node --test apps/desktop/tests/desktop_discovery_smoke_tests.ts",
      "npm run run:desktop-dist-browser-smoke",
      "npm run run:phase009-discovery-assets-gate-tests",
      "npm run run:phase009-discovery-assets-gate",
    ],
  };
}

export function renderPhase009DiscoveryAssetsGateArtifact(result) {
  const lines = [
    "# Phase 009 Discovery Assets Gate Result",
    "",
    result.ok ? "phase009_discovery_assets_gate=passed" : "phase009_discovery_assets_gate=failed",
    `validation_state=${result.ok ? "Passed" : "Failed"}`,
    "",
    "- phase: `Phase 009.4`",
    "- gate: `Discovery, Graph, and Asset UX`",
    `- status: \`${result.ok ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase009-document-authoring-gate-result.md` with `phase009_document_authoring_gate=passed`",
    "- changed layers:",
    ...((result.changedLayers ?? []).map((layer) => `  - \`${layer}\``)),
    "- validation commands:",
    ...((result.validationCommands ?? []).map((command) => `  - \`${command}\``)),
    "- visible UX evidence: search result, backlink group, unresolved link group, graph node/edge, and asset metadata panel are validated by browser smoke.",
    "- Product Log candidates: `search.completed`, `search.failed`, `link.backlinks.loaded`, `graph.projection.loaded`, `asset.metadata.loaded`, `asset.missing` with stable error code only.",
    "- Field Debug metadata candidates: query hash, result count, projection freshness, masked document id, and asset id only.",
    "- Development Log scope: browser smoke diagnostics and discovery gate failures remain test/development only.",
    "- p95 budget impact: this gate validates visible discovery UX; query budget evidence remains a follow-up in `.tasks/release/performance-budget-phase009.md`.",
    "- state-machine follow-up: search, projection freshness, and asset lifecycle states must be implemented in the performance/adapter hardening task.",
    "- sensitive data exclusion: this artifact records marker names, panel ids, counts, layer ids, and stable error codes only. It does not record raw query text, raw document body, graph dump, asset bytes, local path, provider key, token, credential, secret, or personal absolute path.",
  ];

  if (!result.ok) {
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId}\``);
  }

  const artifact = `${lines.join("\n")}\n`;
  for (const pattern of sensitivePatterns) {
    if (pattern.test(artifact)) {
      return [
        "# Phase 009 Discovery Assets Gate Result",
        "",
        "phase009_discovery_assets_gate=failed",
        "validation_state=Failed",
        `- error_code: \`${DiscoveryAssetsGateErrorCode.SensitiveArtifactContent}\``,
        "- finding_id: `artifact_sensitive_content`",
        "",
      ].join("\n");
    }
  }
  return artifact;
}

export async function runPhase009DiscoveryAssetsGate({ rootDir = process.cwd() } = {}) {
  const evidence = await readEvidence(rootDir);
  const result = validatePhase009DiscoveryAssetsEvidence(evidence);
  const artifact = renderPhase009DiscoveryAssetsGateArtifact(result);
  await mkdir(join(rootDir, ".tasks"), { recursive: true });
  await writeFile(join(rootDir, ".tasks/phase009-discovery-assets-gate-result.md"), artifact);
  if (!result.ok) {
    throw new Error(`${result.errorCode}:${result.findingId}`);
  }
  return result;
}

async function readEvidence(rootDir) {
  try {
    const [
      authoringGateText,
      uiDiscoveryTestText,
      desktopDiscoveryTestText,
      webAppText,
      browserSmokeText,
    ] = await Promise.all([
      readFile(join(rootDir, ".tasks/phase009-document-authoring-gate-result.md"), "utf8"),
      readFile(join(rootDir, "packages/ui/tests/local_discovery_panel_model_tests.ts"), "utf8"),
      readFile(join(rootDir, "apps/desktop/tests/desktop_discovery_smoke_tests.ts"), "utf8"),
      readFile(join(rootDir, "apps/web/public/app.js"), "utf8"),
      readFile(join(rootDir, "scripts/run_browser_smoke.mjs"), "utf8"),
    ]);
    return {
      authoringGateText,
      uiDiscoveryTestText,
      desktopDiscoveryTestText,
      webAppText,
      browserSmokeText,
    };
  } catch (error) {
    throw new Error(`${DiscoveryAssetsGateErrorCode.IoFailed}:${error.code ?? "read_failed"}`);
  }
}

function failed(errorCode, findingId) {
  return {
    ok: false,
    errorCode,
    findingId,
    changedLayers: [],
    validationCommands: [],
  };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  runPhase009DiscoveryAssetsGate()
    .then((result) => {
      console.log(result.marker);
    })
    .catch((error) => {
      console.error(error.message);
      process.exit(1);
    });
}
