import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";

import { createDesktopBuildIdentity } from "../src/desktop_build_identity_contract.ts";
import {
  PackagedReleaseEvidenceHarnessError,
  createPackagedReleaseEvidenceReceipt,
} from "../src/packaged_release_evidence_harness.ts";

const sha256 = (value: string): string => createHash("sha256").update(value).digest("hex");
const hash = (character: string): string => character.repeat(64);

test("packaged harness creates exact current requirement receipt without stdout", () => {
  const receipt = createPackagedReleaseEvidenceReceipt({
    requirementIds: ["DOC-001", "BACKUP-001"],
    buildIdentity: identity(),
    initialProfileFingerprint: hash("d"),
    restartProfileFingerprint: hash("d"),
    p95BudgetMs: 300,
    initialRun: { exitCode: 0, stdout: initialOutput() },
    restartRun: { exitCode: 0, stdout: restartOutput() },
    testOrJourneyName: "phase016.package.initial-restart",
    harnessVersion: "1",
    fixtureClass: "clean-profile-2-documents",
    startedAt: "2026-07-20T00:00:00.000Z",
    hash: sha256,
  });

  assert.equal(receipt.status, "Passed");
  assert.equal(receipt.aggregate.passedCount, 2);
  assert.deepEqual(receipt.records.map((record) => record.requirementId), ["BACKUP-001", "DOC-001"]);
  assert.equal(receipt.sourceFingerprint, identity().sourceFingerprint);
  assert.equal(receipt.appFingerprint, identity().appFingerprint);
  assert.equal("stdout" in receipt, false);
  assert.equal("profilePath" in receipt, false);
});

test("packaged harness rejects non-zero initial and restart process exits", () => {
  assert.throws(() => createPackagedReleaseEvidenceReceipt({
    ...validInput(),
    initialRun: { exitCode: 1, stdout: initialOutput() },
  }), harnessError("PACKAGED_RELEASE_INITIAL_PROCESS_FAILED"));
  assert.throws(() => createPackagedReleaseEvidenceReceipt({
    ...validInput(),
    restartRun: { exitCode: 9, stdout: restartOutput() },
  }), harnessError("PACKAGED_RELEASE_RESTART_PROCESS_FAILED"));
});

test("packaged harness rejects mismatched profile and incomplete output", () => {
  assert.throws(() => createPackagedReleaseEvidenceReceipt({
    ...validInput(),
    restartProfileFingerprint: hash("e"),
  }), harnessError("PACKAGED_RELEASE_PROFILE_MISMATCH"));
  assert.throws(() => createPackagedReleaseEvidenceReceipt({
    ...validInput(),
    initialRun: { exitCode: 0, stdout: initialOutput().replace("graph_global_edge_verified=true\n", "") },
  }));
});

function validInput() {
  return {
    requirementIds: ["DOC-001", "BACKUP-001"],
    buildIdentity: identity(),
    initialProfileFingerprint: hash("d"),
    restartProfileFingerprint: hash("d"),
    p95BudgetMs: 300,
    initialRun: { exitCode: 0, stdout: initialOutput() },
    restartRun: { exitCode: 0, stdout: restartOutput() },
    testOrJourneyName: "phase016.package.initial-restart",
    harnessVersion: "1",
    fixtureClass: "clean-profile-2-documents",
    startedAt: "2026-07-20T00:00:00.000Z",
    hash: sha256,
  } as const;
}

function identity() {
  return createDesktopBuildIdentity({
    sourceFingerprint: hash("a"), sourceFileCount: 89,
    appFingerprint: hash("b"), artifactCount: 3, totalArtifactBytes: 3_259_348,
    hash: sha256,
  });
}

function initialOutput(): string {
  return [
    "phase015_packaged_ui_smoke_initial=passed",
    "sample_count=200",
    "p95_ms=34",
    "error_count=0",
    "action_count=135",
    "durable_readback_count=61",
    "document_version_workflow_verified=true",
    "document_attachment_workflow_verified=true",
    "attachment_import_completed=true",
    "attachment_current_readback_verified=true",
    "attachment_document_readback_verified=true",
    "keyboard_document_workflow_verified=true",
    "graph_link_fixture_saved=true",
    "graph_local_edge_verified=true",
    "graph_global_edge_verified=true",
    "graph_safe_labels_verified=true",
    "accessibility_route_focus_count=6",
    "accessibility_keyboard_journey_count=6",
    "accessibility_focus_restoration_count=6",
    "accessibility_visible_control_count=84",
    "accessibility_named_control_count=84",
    "accessibility_text_zoom_percent=200",
    "accessibility_keyboard_error_count=0",
    "accessibility_focus_error_count=0",
    "accessibility_internal_exposure_count=0",
    "",
  ].join("\n");
}

function restartOutput(): string {
  return [
    "phase015_packaged_ui_smoke_restart=passed",
    "attachment_restart_readback_verified=true",
    "canvas_text_restart_readback_verified=true",
    "error_count=0",
    "",
  ].join("\n");
}

function harnessError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof PackagedReleaseEvidenceHarnessError && error.code === code;
}
