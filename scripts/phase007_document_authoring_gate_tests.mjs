import assert from "node:assert/strict";
import test from "node:test";

import {
  DocumentAuthoringGateErrorCode,
  evaluateDocumentAuthoringGate,
  renderDocumentAuthoringGateResult,
} from "./phase007_document_authoring_gate.mjs";

test("document authoring gate rejects missing workspace home prerequisite", () => {
  const result = evaluateDocumentAuthoringGate({
    workspaceHomeText: "phase007_workspace_home_gate=failed",
    authoringWorkspace: completeAuthoringWorkspace(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DocumentAuthoringGateErrorCode.WorkspaceHomeMissing);
});

test("document authoring gate rejects missing split mode", () => {
  const result = evaluateDocumentAuthoringGate({
    workspaceHomeText: "phase007_workspace_home_gate=passed",
    authoringWorkspace: { ...completeAuthoringWorkspace(), viewMode: "source" },
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, DocumentAuthoringGateErrorCode.AuthoringModeMismatch);
});

test("document authoring gate passes complete workspace and renders safe marker", () => {
  const result = evaluateDocumentAuthoringGate({
    workspaceHomeText: "phase007_workspace_home_gate=passed",
    authoringWorkspace: completeAuthoringWorkspace(),
  });
  const rendered = renderDocumentAuthoringGateResult(result);

  assert.equal(result.passed, true);
  assert.match(rendered, /phase007_document_authoring_gate=passed/);
  assert.doesNotMatch(rendered, /raw_document_body_should_not_log/);
});

function completeAuthoringWorkspace() {
  return {
    mode: "document-authoring-workspace",
    viewMode: "split",
    querySeparation: {
      currentReadQueryName: "get-current-document",
      historyReadQueryName: "get-document-history",
    },
    preview: {
      state: "Rendered",
      blocks: [
        { kind: "heading", anchor: "source-document" },
        { kind: "table" },
        { kind: "checklist" },
        { kind: "callout" },
      ],
    },
  };
}
