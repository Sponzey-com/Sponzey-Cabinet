import assert from "node:assert/strict";
import test from "node:test";

import {
  LocalPersistenceGateErrorCode,
  evaluateLocalPersistenceGate,
  renderLocalPersistenceGateResult,
} from "./phase007_local_persistence_gate.mjs";

test("local persistence gate rejects missing document authoring prerequisite", () => {
  const result = evaluateLocalPersistenceGate({
    documentAuthoringText: "phase007_document_authoring_gate=failed",
    saveResult: completeSaveResult(),
    currentHistorySplit: completeCurrentHistorySplit(),
    autosaveEvidence: completeAutosaveEvidence(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, LocalPersistenceGateErrorCode.DocumentAuthoringMissing);
});

test("local persistence gate rejects save result without version append", () => {
  const result = evaluateLocalPersistenceGate({
    documentAuthoringText: "phase007_document_authoring_gate=passed",
    saveResult: { ...completeSaveResult(), versionAppended: false },
    currentHistorySplit: completeCurrentHistorySplit(),
    autosaveEvidence: completeAutosaveEvidence(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, LocalPersistenceGateErrorCode.VersionAppendMissing);
});

test("local persistence gate passes complete evidence and renders safe marker", () => {
  const result = evaluateLocalPersistenceGate({
    documentAuthoringText: "phase007_document_authoring_gate=passed",
    saveResult: completeSaveResult(),
    currentHistorySplit: completeCurrentHistorySplit(),
    autosaveEvidence: completeAutosaveEvidence(),
  });
  const rendered = renderLocalPersistenceGateResult(result);

  assert.equal(result.passed, true);
  assert.match(rendered, /phase007_local_persistence_gate=passed/);
  assert.doesNotMatch(rendered, /raw document body/);
});

function completeSaveResult() {
  return {
    status: "saved-local",
    currentVersionId: "version-2",
    versionAppended: true,
  };
}

function completeCurrentHistorySplit() {
  return {
    currentReadQueryName: "get-current-document",
    historyReadQueryName: "get-document-history",
  };
}

function completeAutosaveEvidence() {
  return {
    failureKeepsDirtyContentRef: true,
    retrySupported: true,
    readOnlyPauseSupported: true,
  };
}
