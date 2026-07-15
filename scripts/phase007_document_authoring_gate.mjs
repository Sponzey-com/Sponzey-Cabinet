import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const DocumentAuthoringGateErrorCode = Object.freeze({
  WorkspaceHomeMissing: "PHASE007_DOCUMENT_AUTHORING_WORKSPACE_HOME_MISSING",
  AuthoringModeMismatch: "PHASE007_DOCUMENT_AUTHORING_MODE_MISMATCH",
  QueryPathMixed: "PHASE007_DOCUMENT_AUTHORING_QUERY_PATH_MIXED",
  RequiredPreviewBlockMissing: "PHASE007_DOCUMENT_AUTHORING_PREVIEW_BLOCK_MISSING",
  IoFailed: "PHASE007_DOCUMENT_AUTHORING_IO_FAILED",
});

const requiredPreviewBlockKinds = ["heading", "table"];

export function evaluateDocumentAuthoringGate({ workspaceHomeText, authoringWorkspace }) {
  if (!workspaceHomeText.includes("phase007_workspace_home_gate=passed")) {
    return failed(
      DocumentAuthoringGateErrorCode.WorkspaceHomeMissing,
      ".tasks/phase007-workspace-home-gate-result.md",
    );
  }
  if (authoringWorkspace?.mode !== "document-authoring-workspace" || authoringWorkspace.viewMode !== "split") {
    return failed(DocumentAuthoringGateErrorCode.AuthoringModeMismatch, "split");
  }
  if (
    authoringWorkspace?.querySeparation?.currentReadQueryName !== "get-current-document" ||
    authoringWorkspace?.querySeparation?.historyReadQueryName !== "get-document-history"
  ) {
    return failed(DocumentAuthoringGateErrorCode.QueryPathMixed, "current_history_split");
  }
  const blockKinds = (authoringWorkspace?.preview?.blocks ?? []).map((block) => block.kind);
  for (const kind of requiredPreviewBlockKinds) {
    if (!blockKinds.includes(kind)) {
      return failed(DocumentAuthoringGateErrorCode.RequiredPreviewBlockMissing, kind);
    }
  }
  return {
    passed: true,
    marker: "phase007_document_authoring_gate=passed",
    viewMode: authoringWorkspace.viewMode,
    previewBlockCount: blockKinds.length,
  };
}

export function renderDocumentAuthoringGateResult(result) {
  if (result.passed) {
    return [
      "phase007_document_authoring_gate=passed",
      `view_mode=${result.viewMode}`,
      `preview_block_count=${result.previewBlockCount}`,
    ].join("\n");
  }
  return [
    "phase007_document_authoring_gate=failed",
    `error_code=${result.errorCode}`,
    `finding_id=${result.findingId}`,
  ].join("\n");
}

export function renderDocumentAuthoringGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 007 Document Authoring Gate Result",
    "",
    renderDocumentAuthoringGateResult(result),
    "",
    "- phase: `Phase 007.2`",
    "- gate: `Document Authoring`",
    `- status: \`${status}\``,
    "- commands:",
    "  - `npm run run:phase007-document-authoring-gate-tests`",
    "  - `npm run run:phase007-document-authoring-gate`",
    "- Product Log candidate: `document.preview.failed` with stable error code only",
    "- Field Debug metadata candidates: `block_count`, `table_count`, `unresolved_link_count`, `asset_reference_count`",
    "- sensitive data exclusion: this artifact records mode, counts, status, and stable error codes only.",
    "- follow-up limitation: persistence, autosave, and current/history storage wiring remain Phase 007.3.",
    "",
  ].join("\n");
}

async function runDocumentAuthoringGateCli() {
  try {
    const workspaceHomeText = await readFile(".tasks/phase007-workspace-home-gate-result.md", "utf8");
    const { createDesktopDocumentAuthoringWorkspace } = await import(
      pathToFileURL(join(process.cwd(), "apps/desktop/src/index.ts")).href
    );
    const authoringWorkspace = createDesktopDocumentAuthoringWorkspace(currentDocumentFixture(), historyPageFixture());
    const result = evaluateDocumentAuthoringGate({ workspaceHomeText, authoringWorkspace });
    await writeFile(
      ".tasks/phase007-document-authoring-gate-result.md",
      renderDocumentAuthoringGateArtifact(result),
    );
    const rendered = renderDocumentAuthoringGateResult(result);
    if (result.passed) {
      console.log(rendered);
      return;
    }
    console.error(rendered);
    process.exit(1);
  } catch (error) {
    const result = failed(
      DocumentAuthoringGateErrorCode.IoFailed,
      error instanceof Error ? error.message : "unknown",
    );
    await writeFile(
      ".tasks/phase007-document-authoring-gate-result.md",
      renderDocumentAuthoringGateArtifact(result),
    );
    console.error(renderDocumentAuthoringGateResult(result));
    process.exit(1);
  }
}

function currentDocumentFixture() {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    title: "Source Document",
    path: "docs/source.md",
    body: [
      "# Source Document",
      "",
      "| 항목 | 내용 | 상태 |",
      "| :--- | :---: | ---: |",
      "| 1번 그리드 | 좌측 정렬 | 우측 정렬 |",
    ].join("\n"),
    versionId: "version-current",
  };
}

function historyPageFixture() {
  return {
    workspaceId: "workspace-1",
    documentId: "doc-1",
    entries: [
      {
        versionId: "version-1",
        summary: "Created document",
        author: "local-user",
        createdAt: "2026-07-09T00:00:00Z",
      },
    ],
  };
}

function failed(errorCode, findingId) {
  return {
    passed: false,
    marker: "phase007_document_authoring_gate=failed",
    errorCode,
    findingId,
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runDocumentAuthoringGateCli();
}
