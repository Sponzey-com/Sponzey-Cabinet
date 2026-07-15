import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase009ReleaseEvidenceEvent,
  Phase009ReleaseEvidenceErrorCode,
  Phase009ReleaseEvidenceState,
  createPhase009SecurityLogManifest,
  renderPhase009LocalDesktopRunbook,
  renderPhase009PerformanceBudget,
  renderPhase009ProductLogMatrix,
  transitionPhase009ReleaseEvidenceState,
  validatePhase009ReleaseEvidence,
  writePhase009ReleaseEvidence,
} from "./phase009_release_evidence.mjs";

test("Phase009 release evidence validator accepts all required evidence markers", () => {
  const evidence = completeEvidence();
  const result = validatePhase009ReleaseEvidence(evidence);

  assert.equal(result.ok, true);
  assert.equal(result.state, Phase009ReleaseEvidenceState.Passed);
  assert.equal(result.markers.performanceBudget, "phase009_performance_budget=passed");
  assert.equal(result.markers.productLogMatrix, "phase009_product_log_matrix=passed");
  assert.equal(result.markers.securityManifest, "phase009_security_log_manifest=passed");
  assert.equal(result.markers.runbook, "phase009_runbook=passed");
});

test("Phase009 release evidence validator rejects missing performance marker", () => {
  const result = validatePhase009ReleaseEvidence({
    ...completeEvidence(),
    performanceBudgetText: "phase009_performance_budget=failed",
  });

  assert.equal(result.ok, false);
  assert.equal(result.state, Phase009ReleaseEvidenceState.Failed);
  assert.equal(result.errorCode, Phase009ReleaseEvidenceErrorCode.PerformanceBudgetMissing);
});

test("Phase009 release evidence validator rejects malformed security manifest", () => {
  const result = validatePhase009ReleaseEvidence({
    ...completeEvidence(),
    securityManifest: { schemaVersion: 1, marker: "phase009_security_log_manifest=passed" },
  });

  assert.equal(result.ok, false);
  assert.equal(result.state, Phase009ReleaseEvidenceState.Failed);
  assert.equal(result.errorCode, Phase009ReleaseEvidenceErrorCode.SecurityManifestMalformed);
});

test("Phase009 release evidence state machine rejects invalid transition", () => {
  const result = transitionPhase009ReleaseEvidenceState(
    Phase009ReleaseEvidenceState.NotStarted,
    Phase009ReleaseEvidenceEvent.Complete,
  );

  assert.equal(result.state, Phase009ReleaseEvidenceState.Failed);
  assert.equal(result.errorCode, Phase009ReleaseEvidenceErrorCode.InvalidTransition);
});

test("Phase009 release evidence renderers exclude sensitive fixtures", () => {
  const rendered = [
    renderPhase009PerformanceBudget(),
    renderPhase009ProductLogMatrix(),
    JSON.stringify(createPhase009SecurityLogManifest(), null, 2),
    renderPhase009LocalDesktopRunbook(),
  ].join("\n");

  assert.equal(rendered.includes("provider_api_key_fixture"), false);
  assert.equal(rendered.includes("raw markdown body should not leak"), false);
  assert.equal(rendered.includes("/Users/example/private"), false);
  assert.equal(rendered.includes("raw-token-example"), false);
});

test("Phase009 release evidence writer creates all release files", async () => {
  const root = await mkdtemp(join(tmpdir(), "phase009-release-evidence-"));
  await mkdir(join(root, ".tasks"), { recursive: true });

  const result = await writePhase009ReleaseEvidence({ rootDir: root });

  assert.equal(result.ok, true);
  assert.equal(
    await readFile(join(root, ".tasks/release/performance-budget-phase009.md"), "utf8")
      .then((text) => text.includes("phase009_performance_budget=passed")),
    true,
  );
  assert.equal(
    await readFile(join(root, ".tasks/release/product-log-event-matrix-phase009.md"), "utf8")
      .then((text) => text.includes("phase009_product_log_matrix=passed")),
    true,
  );
  assert.equal(
    await readFile(join(root, ".tasks/release/security-log-policy-manifest-phase009.json"), "utf8")
      .then((text) => text.includes("phase009_security_log_manifest=passed")),
    true,
  );
  assert.equal(
    await readFile(join(root, ".tasks/release/local-desktop-runbook-phase009.md"), "utf8")
      .then((text) => text.includes("phase009_runbook=passed")),
    true,
  );
});

function completeEvidence() {
  return {
    performanceBudgetText: renderPhase009PerformanceBudget(),
    productLogMatrixText: renderPhase009ProductLogMatrix(),
    securityManifest: createPhase009SecurityLogManifest(),
    runbookText: renderPhase009LocalDesktopRunbook(),
  };
}
