import assert from "node:assert/strict";
import test from "node:test";

import {
  PackagedUiSmokeRunnerErrorCode,
  analyzePackagedUiSmokeOutput,
  executeWithDeadline,
} from "./desktop_packaged_ui_smoke.mjs";

test("accepts only the sanitized full packaged workflow evidence", () => {
  const result = analyzePackagedUiSmokeOutput([
    "phase012_packaged_ui_smoke=passed",
    "sample_count=200",
    "p95_ms=88",
    "error_count=0",
    "action_count=15",
    "durable_readback_count=4",
    "document_version_workflow_verified=true",
    "document_attachment_workflow_verified=true",
    "keyboard_document_workflow_verified=true",
  ].join("\n"), 0);
  assert.equal(result.passed, true);
  assert.equal(result.p95Ms, 88);
});

test("rejects missing marker, incomplete samples, slow p95, and process failure", () => {
  const coverage = "action_count=15\ndurable_readback_count=4\ndocument_version_workflow_verified=true\ndocument_attachment_workflow_verified=true\nkeyboard_document_workflow_verified=true";
  assert.equal(analyzePackagedUiSmokeOutput(`sample_count=200\np95_ms=20\nerror_count=0\n${coverage}`, 0).errorCode, PackagedUiSmokeRunnerErrorCode.MarkerMissing);
  assert.equal(analyzePackagedUiSmokeOutput(`phase012_packaged_ui_smoke=passed\nsample_count=199\np95_ms=20\nerror_count=0\n${coverage}`, 0).errorCode, PackagedUiSmokeRunnerErrorCode.SampleCountInvalid);
  assert.equal(analyzePackagedUiSmokeOutput(`phase012_packaged_ui_smoke=passed\nsample_count=200\np95_ms=301\nerror_count=0\n${coverage}`, 0).errorCode, PackagedUiSmokeRunnerErrorCode.PerformanceBudgetExceeded);
  assert.equal(analyzePackagedUiSmokeOutput(`phase012_packaged_ui_smoke=passed\nsample_count=200\np95_ms=20\nerror_count=0\n${coverage}`, 1).errorCode, PackagedUiSmokeRunnerErrorCode.ProcessFailed);
});

test("rejects missing or false document version workflow evidence", () => {
  const base = "phase012_packaged_ui_smoke=passed\nsample_count=200\np95_ms=20\nerror_count=0\naction_count=15\ndurable_readback_count=4";
  assert.equal(
    analyzePackagedUiSmokeOutput(base, 0).errorCode,
    PackagedUiSmokeRunnerErrorCode.DocumentVersionWorkflowMissing,
  );
  assert.equal(
    analyzePackagedUiSmokeOutput(`${base}\ndocument_version_workflow_verified=false`, 0).errorCode,
    PackagedUiSmokeRunnerErrorCode.DocumentVersionWorkflowMissing,
  );
  assert.equal(
    analyzePackagedUiSmokeOutput(`${base}\ndocument_version_workflow_verified=true\ndocument_attachment_workflow_verified=false\nkeyboard_document_workflow_verified=true`, 0).errorCode,
    PackagedUiSmokeRunnerErrorCode.DocumentAttachmentWorkflowMissing,
  );
});

test("rejects missing or false keyboard document workflow evidence", () => {
  const base = "phase012_packaged_ui_smoke=passed\nsample_count=200\np95_ms=20\nerror_count=0\naction_count=15\ndurable_readback_count=4\ndocument_version_workflow_verified=true\ndocument_attachment_workflow_verified=true";
  assert.equal(
    analyzePackagedUiSmokeOutput(base, 0).errorCode,
    PackagedUiSmokeRunnerErrorCode.KeyboardDocumentWorkflowMissing,
  );
  assert.equal(
    analyzePackagedUiSmokeOutput(`${base}\nkeyboard_document_workflow_verified=false`, 0).errorCode,
    PackagedUiSmokeRunnerErrorCode.KeyboardDocumentWorkflowMissing,
  );
});

test("preserves only a stable native failure code", () => {
  assert.equal(
    analyzePackagedUiSmokeOutput(
      "phase012_packaged_ui_smoke=failed\nerror_code=PHASE012_PACKAGED_UI_ASSET_IMPORT_FAILED",
      0,
    ).errorCode,
    "PHASE012_PACKAGED_UI_ASSET_IMPORT_FAILED",
  );
  assert.equal(
    analyzePackagedUiSmokeOutput(
      "phase012_packaged_ui_smoke=failed\nerror_code=private path and body",
      0,
    ).errorCode,
    PackagedUiSmokeRunnerErrorCode.MarkerMissing,
  );
});

test("deadline terminates a non-completing packaged process", async () => {
  let terminated = false;
  const result = await executeWithDeadline(
    () => new Promise(() => undefined),
    5,
    () => { terminated = true; },
  );
  assert.equal(result.passed, false);
  assert.equal(result.errorCode, PackagedUiSmokeRunnerErrorCode.Timeout);
  assert.equal(terminated, true);
});
