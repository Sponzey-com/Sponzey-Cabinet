import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const LocalPersistenceGateErrorCode = Object.freeze({
  DocumentAuthoringMissing: "PHASE007_LOCAL_PERSISTENCE_DOCUMENT_AUTHORING_MISSING",
  VersionAppendMissing: "PHASE007_LOCAL_PERSISTENCE_VERSION_APPEND_MISSING",
  CurrentHistoryMixed: "PHASE007_LOCAL_PERSISTENCE_CURRENT_HISTORY_MIXED",
  AutosaveEvidenceMissing: "PHASE007_LOCAL_PERSISTENCE_AUTOSAVE_EVIDENCE_MISSING",
  IoFailed: "PHASE007_LOCAL_PERSISTENCE_IO_FAILED",
});

export function evaluateLocalPersistenceGate({
  documentAuthoringText,
  saveResult,
  currentHistorySplit,
  autosaveEvidence,
}) {
  if (!documentAuthoringText.includes("phase007_document_authoring_gate=passed")) {
    return failed(
      LocalPersistenceGateErrorCode.DocumentAuthoringMissing,
      ".tasks/phase007-document-authoring-gate-result.md",
    );
  }
  if (saveResult?.status !== "saved-local" || saveResult.versionAppended !== true || !saveResult.currentVersionId) {
    return failed(LocalPersistenceGateErrorCode.VersionAppendMissing, "version_appended");
  }
  if (
    currentHistorySplit?.currentReadQueryName !== "get-current-document" ||
    currentHistorySplit?.historyReadQueryName !== "get-document-history"
  ) {
    return failed(LocalPersistenceGateErrorCode.CurrentHistoryMixed, "current_history_split");
  }
  if (
    autosaveEvidence?.failureKeepsDirtyContentRef !== true ||
    autosaveEvidence?.retrySupported !== true ||
    autosaveEvidence?.readOnlyPauseSupported !== true
  ) {
    return failed(LocalPersistenceGateErrorCode.AutosaveEvidenceMissing, "autosave_state_machine");
  }
  return {
    passed: true,
    marker: "phase007_local_persistence_gate=passed",
    currentReadQueryName: "get-current-document",
    historyReadQueryName: "get-document-history",
    versionAppended: true,
    autosaveStateMachine: "explicit",
  };
}

export function renderLocalPersistenceGateResult(result) {
  if (result.passed) {
    return [
      "phase007_local_persistence_gate=passed",
      `current_read_query=${result.currentReadQueryName}`,
      `history_read_query=${result.historyReadQueryName}`,
      `version_appended=${result.versionAppended}`,
      `autosave_state_machine=${result.autosaveStateMachine}`,
    ].join("\n");
  }
  return [
    "phase007_local_persistence_gate=failed",
    `error_code=${result.errorCode}`,
    `finding_id=${result.findingId}`,
  ].join("\n");
}

export function renderLocalPersistenceGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 007 Local Persistence Gate Result",
    "",
    renderLocalPersistenceGateResult(result),
    "",
    "- phase: `Phase 007.3`",
    "- gate: `Local Persistence Facade`",
    `- status: \`${status}\``,
    "- commands:",
    "  - `npm run run:phase007-local-persistence-gate-tests`",
    "  - `npm run run:phase007-local-persistence-gate`",
    "- Product Log candidates: `document.saved`, `document.save.failed` with stable error code only",
    "- Field Debug metadata candidates: `autosave_state`, `version_id`, `retry_count`, `masked_document_id`",
    "- sensitive data exclusion: this artifact records query names, status, booleans, and stable error codes only.",
    "- follow-up limitation: discovery search/link/graph/asset panels remain Phase 007.4.",
    "",
  ].join("\n");
}

async function runLocalPersistenceGateCli() {
  try {
    const documentAuthoringText = await readFile(
      ".tasks/phase007-document-authoring-gate-result.md",
      "utf8",
    );
    const result = evaluateLocalPersistenceGate({
      documentAuthoringText,
      saveResult: {
        status: "saved-local",
        documentId: "doc-1",
        currentVersionId: "version-2",
        versionAppended: true,
      },
      currentHistorySplit: {
        currentReadQueryName: "get-current-document",
        historyReadQueryName: "get-document-history",
      },
      autosaveEvidence: {
        failureKeepsDirtyContentRef: true,
        retrySupported: true,
        readOnlyPauseSupported: true,
      },
    });
    await writeFile(
      ".tasks/phase007-local-persistence-gate-result.md",
      renderLocalPersistenceGateArtifact(result),
    );
    const rendered = renderLocalPersistenceGateResult(result);
    if (result.passed) {
      console.log(rendered);
      return;
    }
    console.error(rendered);
    process.exit(1);
  } catch (error) {
    const result = failed(
      LocalPersistenceGateErrorCode.IoFailed,
      error instanceof Error ? error.message : "unknown",
    );
    await writeFile(
      ".tasks/phase007-local-persistence-gate-result.md",
      renderLocalPersistenceGateArtifact(result),
    );
    console.error(renderLocalPersistenceGateResult(result));
    process.exit(1);
  }
}

function failed(errorCode, findingId) {
  return {
    passed: false,
    marker: "phase007_local_persistence_gate=failed",
    errorCode,
    findingId,
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runLocalPersistenceGateCli();
}
