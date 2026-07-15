import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const DocumentAuthoringGateErrorCode = Object.freeze({
  CommandRuntimeMissing: "PHASE009_AUTHORING_COMMAND_RUNTIME_MISSING",
  EditorStateMissing: "PHASE009_AUTHORING_EDITOR_STATE_MISSING",
  EditorStateTestMissing: "PHASE009_AUTHORING_EDITOR_STATE_TEST_MISSING",
  VisibleStateMarkerMissing: "PHASE009_AUTHORING_VISIBLE_STATE_MARKER_MISSING",
  BrowserSmokeMissing: "PHASE009_AUTHORING_BROWSER_SMOKE_MISSING",
  DesktopAuthoringTestMissing: "PHASE009_AUTHORING_DESKTOP_AUTHORING_TEST_MISSING",
  SensitiveArtifactContent: "PHASE009_AUTHORING_SENSITIVE_ARTIFACT_CONTENT",
  IoFailed: "PHASE009_AUTHORING_IO_FAILED",
});

const sensitivePatterns = [
  /raw document body fixture/i,
  /provider_api_key_fixture/i,
  /token_fixture/i,
  /credential_fixture/i,
  /\/Users\/[^`\s]+/i,
  /[A-Za-z]:\\Users\\/i,
];

const requiredEditorStateTerms = [
  "DocumentEditorState",
  "DocumentEditorEvent",
  "transitionDocumentEditorState",
  "DOCUMENT_EDITOR_INVALID_TRANSITION",
  "DOCUMENT_SAVE_FAILED",
  "ReadyClean",
  "ReadyDirty",
  "Saving",
  "Saved",
  "SaveFailed",
];

const requiredUiTestTerms = [
  "document editor state machine marks dirty content and save success explicitly",
  "document editor state machine returns stable error code for invalid transitions and save failure",
];

const requiredWebMarkers = [
  "data-cabinet-editor-state",
  "data-cabinet-save-state",
  "data-cabinet-saved-version",
  "data-cabinet-current-history-split",
];

const requiredBrowserSmokeTerms = [
  "dirtyMarkerObserved",
  "savedMarkerObserved",
  "savedVersionMarkerObserved",
  "currentHistorySplitReady",
  "previewTableRendered",
];

const requiredDesktopAuthoringTerms = [
  "desktop document authoring smoke exposes split source and preview mode",
  "document-authoring-workspace",
  "get-current-document",
  "get-document-history",
];

export function validatePhase009DocumentAuthoringEvidence(evidence) {
  if (!evidence.commandRuntimeText.includes("phase009_command_runtime_gate=passed")) {
    return failed(DocumentAuthoringGateErrorCode.CommandRuntimeMissing, "command_runtime_marker");
  }

  for (const term of requiredEditorStateTerms) {
    if (!evidence.uiModelText.includes(term)) {
      return failed(DocumentAuthoringGateErrorCode.EditorStateMissing, term);
    }
  }

  for (const term of requiredUiTestTerms) {
    if (!evidence.uiTestText.includes(term)) {
      return failed(DocumentAuthoringGateErrorCode.EditorStateTestMissing, term);
    }
  }

  for (const marker of requiredWebMarkers) {
    if (!evidence.webAppText.includes(marker)) {
      return failed(DocumentAuthoringGateErrorCode.VisibleStateMarkerMissing, marker);
    }
  }

  for (const term of requiredBrowserSmokeTerms) {
    if (!evidence.browserSmokeText.includes(term)) {
      return failed(DocumentAuthoringGateErrorCode.BrowserSmokeMissing, term);
    }
  }

  for (const term of requiredDesktopAuthoringTerms) {
    if (!evidence.desktopAuthoringTestText.includes(term)) {
      return failed(DocumentAuthoringGateErrorCode.DesktopAuthoringTestMissing, term);
    }
  }

  return {
    ok: true,
    marker: "phase009_document_authoring_gate=passed",
    changedLayers: ["ui-model", "browser-smoke", "gate-tooling"],
    validationCommands: [
      "node --test packages/ui/tests/document_authoring_preview_model_tests.ts",
      "node --test apps/desktop/tests/desktop_document_authoring_smoke_tests.ts apps/desktop/tests/desktop_document_ux_smoke_tests.ts",
      "npm run run:desktop-dist-browser-smoke",
      "npm run run:phase009-document-authoring-gate-tests",
      "npm run run:phase009-document-authoring-gate",
    ],
  };
}

export function renderPhase009DocumentAuthoringGateArtifact(result) {
  const lines = [
    "# Phase 009 Document Authoring Gate Result",
    "",
    result.ok ? "phase009_document_authoring_gate=passed" : "phase009_document_authoring_gate=failed",
    `validation_state=${result.ok ? "Passed" : "Failed"}`,
    "",
    "- phase: `Phase 009.3`",
    "- gate: `Daily Document Authoring UX`",
    `- status: \`${result.ok ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase009-command-runtime-gate-result.md` with `phase009_command_runtime_gate=passed`",
    "- changed layers:",
    ...((result.changedLayers ?? []).map((layer) => `  - \`${layer}\``)),
    "- validation commands:",
    ...((result.validationCommands ?? []).map((command) => `  - \`${command}\``)),
    "- state machine: `DocumentEditorState` covers `Loading`, `ReadyClean`, `ReadyDirty`, `Saving`, `Saved`, and `SaveFailed`.",
    "- Product Log candidates: `document.current.loaded`, `document.save.completed`, `document.save.failed` with stable error code only.",
    "- Field Debug metadata candidates: masked document id, version count, editor state, and duration bucket only.",
    "- Development Log scope: browser smoke diagnostics and gate failures remain test/development only.",
    "- p95 budget impact: this gate verifies visible authoring state; current/history query budget remains tracked in `.tasks/release/performance-budget-phase009.md` follow-up entries.",
    "- sensitive data exclusion: this artifact records marker names, state names, counts, layer ids, and stable error codes only. It does not record raw document body, rendered HTML dump, asset content, local path, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
  ];

  if (!result.ok) {
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId}\``);
  }

  const artifact = `${lines.join("\n")}\n`;
  for (const pattern of sensitivePatterns) {
    if (pattern.test(artifact)) {
      return [
        "# Phase 009 Document Authoring Gate Result",
        "",
        "phase009_document_authoring_gate=failed",
        "validation_state=Failed",
        `- error_code: \`${DocumentAuthoringGateErrorCode.SensitiveArtifactContent}\``,
        "- finding_id: `artifact_sensitive_content`",
        "",
      ].join("\n");
    }
  }
  return artifact;
}

export async function runPhase009DocumentAuthoringGate({ rootDir = process.cwd() } = {}) {
  const evidence = await readEvidence(rootDir);
  const result = validatePhase009DocumentAuthoringEvidence(evidence);
  const artifact = renderPhase009DocumentAuthoringGateArtifact(result);
  await mkdir(join(rootDir, ".tasks"), { recursive: true });
  await writeFile(join(rootDir, ".tasks/phase009-document-authoring-gate-result.md"), artifact);
  if (!result.ok) {
    throw new Error(`${result.errorCode}:${result.findingId}`);
  }
  return result;
}

async function readEvidence(rootDir) {
  try {
    const [
      commandRuntimeText,
      uiModelText,
      uiTestText,
      webAppText,
      browserSmokeText,
      desktopAuthoringTestText,
    ] = await Promise.all([
      readFile(join(rootDir, ".tasks/phase009-command-runtime-gate-result.md"), "utf8"),
      readFile(join(rootDir, "packages/ui/src/index.ts"), "utf8"),
      readFile(join(rootDir, "packages/ui/tests/document_authoring_preview_model_tests.ts"), "utf8"),
      readFile(join(rootDir, "apps/web/public/app.js"), "utf8"),
      readFile(join(rootDir, "scripts/run_browser_smoke.mjs"), "utf8"),
      readFile(join(rootDir, "apps/desktop/tests/desktop_document_authoring_smoke_tests.ts"), "utf8"),
    ]);
    return {
      commandRuntimeText,
      uiModelText,
      uiTestText,
      webAppText,
      browserSmokeText,
      desktopAuthoringTestText,
    };
  } catch (error) {
    throw new Error(`${DocumentAuthoringGateErrorCode.IoFailed}:${error.code ?? "read_failed"}`);
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
  runPhase009DocumentAuthoringGate()
    .then((result) => {
      console.log(result.marker);
    })
    .catch((error) => {
      console.error(error.message);
      process.exit(1);
    });
}
