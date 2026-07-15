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
  ].join("\n"), 0);
  assert.equal(result.passed, true);
  assert.equal(result.p95Ms, 88);
});

test("rejects missing marker, incomplete samples, slow p95, and process failure", () => {
  const coverage = "action_count=15\ndurable_readback_count=4";
  assert.equal(analyzePackagedUiSmokeOutput(`sample_count=200\np95_ms=20\nerror_count=0\n${coverage}`, 0).errorCode, PackagedUiSmokeRunnerErrorCode.MarkerMissing);
  assert.equal(analyzePackagedUiSmokeOutput(`phase012_packaged_ui_smoke=passed\nsample_count=199\np95_ms=20\nerror_count=0\n${coverage}`, 0).errorCode, PackagedUiSmokeRunnerErrorCode.SampleCountInvalid);
  assert.equal(analyzePackagedUiSmokeOutput(`phase012_packaged_ui_smoke=passed\nsample_count=200\np95_ms=301\nerror_count=0\n${coverage}`, 0).errorCode, PackagedUiSmokeRunnerErrorCode.PerformanceBudgetExceeded);
  assert.equal(analyzePackagedUiSmokeOutput(`phase012_packaged_ui_smoke=passed\nsample_count=200\np95_ms=20\nerror_count=0\n${coverage}`, 1).errorCode, PackagedUiSmokeRunnerErrorCode.ProcessFailed);
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
