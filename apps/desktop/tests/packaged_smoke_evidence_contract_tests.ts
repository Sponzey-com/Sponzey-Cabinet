import assert from "node:assert/strict"
import test from "node:test"

import {
  PackagedSmokeEvidenceError,
  createPackagedSmokeEvidence,
  parseInitialPackagedSmokeOutput,
  parseRestartPackagedSmokeOutput,
  parseUpgradedProfileSmokeOutput,
} from "../src/packaged_smoke_evidence_contract.ts"

const fingerprint = (digit: string): string => digit.repeat(64)

const initialOutput = (overrides: Record<string, string> = {}): string => {
  const fields: Record<string, string> = {
    phase015_packaged_ui_smoke_initial: "passed",
    sample_count: "200",
    p95_ms: "24.5",
    error_count: "0",
    action_count: "96",
    durable_readback_count: "35",
    document_version_workflow_verified: "true",
    document_attachment_workflow_verified: "true",
    attachment_import_completed: "true",
    attachment_current_readback_verified: "true",
    attachment_document_readback_verified: "true",
    attachment_restart_readback_verified: "false",
    keyboard_document_workflow_verified: "true",
    graph_link_fixture_saved: "true",
    graph_local_edge_verified: "true",
    graph_global_edge_verified: "true",
    graph_safe_labels_verified: "true",
    accessibility_route_focus_count: "6",
    accessibility_keyboard_journey_count: "6",
    accessibility_focus_restoration_count: "6",
    accessibility_visible_control_count: "84",
    accessibility_named_control_count: "84",
    accessibility_text_zoom_percent: "200",
    accessibility_keyboard_error_count: "0",
    accessibility_focus_error_count: "0",
    accessibility_internal_exposure_count: "0",
    ...overrides,
  }
  return [
    "untrusted document content=/Users/example/private.md",
    ...Object.entries(fields).map(([key, value]) => `${key}=${value}`),
  ].join("\n")
}

const restartOutput = (overrides: Record<string, string> = {}): string => {
  const fields: Record<string, string> = {
    phase015_packaged_ui_smoke_restart: "passed",
    attachment_restart_readback_verified: "true",
    canvas_text_restart_readback_verified: "true",
    error_count: "0",
    ...overrides,
  }
  return Object.entries(fields).map(([key, value]) => `${key}=${value}`).join("\n")
}

const expectCode = (operation: () => unknown, code: string): void => {
  assert.throws(operation, (error: unknown) => {
    assert.ok(error instanceof PackagedSmokeEvidenceError)
    assert.equal(error.code, code)
    return true
  })
}

test("parses a complete initial smoke result without retaining stdout", () => {
  const result = parseInitialPackagedSmokeOutput(initialOutput(), 300, fingerprint("a"))

  assert.deepEqual(result, {
    stage: "InitialPassed",
    profileFingerprint: fingerprint("a"),
    sampleCount: 200,
    p95Ms: 24.5,
    errorCount: 0,
    actionCount: 96,
    durableReadbackCount: 35,
    accessibilityRouteFocusCount: 6,
    accessibilityKeyboardJourneyCount: 6,
    accessibilityFocusRestorationCount: 6,
    accessibilityVisibleControlCount: 84,
    accessibilityNamedControlCount: 84,
    accessibilityTextZoomPercent: 200,
  })
  assert.equal(JSON.stringify(result).includes("private.md"), false)
  assert.equal(Object.isFrozen(result), true)
})

test("initial parser rejects failed, incomplete, duplicate, malformed, and over-budget output", () => {
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ phase015_packaged_ui_smoke_initial: "failed" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_INITIAL_FAILED",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput().replace("sample_count=200\n", ""), 300, fingerprint("a")),
    "PACKAGED_SMOKE_FIELD_MISSING",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(`${initialOutput()}\nsample_count=200`, 300, fingerprint("a")),
    "PACKAGED_SMOKE_FIELD_DUPLICATE",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ sample_count: "two hundred" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_FIELD_MALFORMED",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ p95_ms: "300.01" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_PERFORMANCE_BUDGET_EXCEEDED",
  )
})

test("initial parser enforces native coverage and workflow evidence", () => {
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ sample_count: "199" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_SAMPLE_COUNT_INVALID",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ action_count: "89" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_ACTION_COVERAGE_INCOMPLETE",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ durable_readback_count: "32" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_ACTION_COVERAGE_INCOMPLETE",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ graph_global_edge_verified: "false" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_WORKFLOW_EVIDENCE_MISSING",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ error_count: "1" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_UI_ERROR_REPORTED",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput().replace("accessibility_route_focus_count=6\n", ""), 300, fingerprint("a")),
    "PACKAGED_SMOKE_FIELD_MISSING",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ accessibility_named_control_count: "83" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_ACCESSIBILITY_INCOMPLETE",
  )
  expectCode(
    () => parseInitialPackagedSmokeOutput(initialOutput({ accessibility_focus_error_count: "1" }), 300, fingerprint("a")),
    "PACKAGED_SMOKE_ACCESSIBILITY_INCOMPLETE",
  )
})

test("parses restart evidence and rejects false readback or errors", () => {
  const result = parseRestartPackagedSmokeOutput(restartOutput(), fingerprint("a"))
  assert.deepEqual(result, {
    stage: "RestartPassed",
    profileFingerprint: fingerprint("a"),
    errorCount: 0,
    attachmentRestartReadbackVerified: true,
    canvasTextRestartReadbackVerified: true,
  })
  assert.equal(Object.isFrozen(result), true)

  expectCode(
    () => parseRestartPackagedSmokeOutput(restartOutput({ attachment_restart_readback_verified: "false" }), fingerprint("a")),
    "PACKAGED_SMOKE_RESTART_READBACK_MISSING",
  )
  expectCode(
    () => parseRestartPackagedSmokeOutput(restartOutput({ canvas_text_restart_readback_verified: "false" }), fingerprint("a")),
    "PACKAGED_SMOKE_RESTART_READBACK_MISSING",
  )
  expectCode(
    () => parseRestartPackagedSmokeOutput(restartOutput().replace("canvas_text_restart_readback_verified=true\n", ""), fingerprint("a")),
    "PACKAGED_SMOKE_FIELD_MISSING",
  )
  expectCode(
    () => parseRestartPackagedSmokeOutput(restartOutput({ error_count: "2" }), fingerprint("a")),
    "PACKAGED_SMOKE_UI_ERROR_REPORTED",
  )
})

test("creates immutable evidence only for matching profile and valid fingerprints", () => {
  const initial = parseInitialPackagedSmokeOutput(initialOutput(), 300, fingerprint("a"))
  const restart = parseRestartPackagedSmokeOutput(restartOutput(), fingerprint("a"))
  const evidence = createPackagedSmokeEvidence({
    sourceFingerprint: fingerprint("b"),
    appFingerprint: fingerprint("c"),
    initial,
    restart,
  })

  assert.deepEqual(evidence, {
    status: "Passed",
    sourceFingerprint: fingerprint("b"),
    appFingerprint: fingerprint("c"),
    profileFingerprint: fingerprint("a"),
    sampleCount: 200,
    p95Ms: 24.5,
    actionCount: 96,
    durableReadbackCount: 35,
    accessibilityRouteFocusCount: 6,
    accessibilityKeyboardJourneyCount: 6,
    accessibilityFocusRestorationCount: 6,
    accessibilityVisibleControlCount: 84,
    accessibilityNamedControlCount: 84,
    accessibilityTextZoomPercent: 200,
    attachmentRestartReadbackVerified: true,
    canvasTextRestartReadbackVerified: true,
  })
  assert.equal(Object.isFrozen(evidence), true)

  const differentRestart = parseRestartPackagedSmokeOutput(restartOutput(), fingerprint("d"))
  expectCode(
    () => createPackagedSmokeEvidence({ sourceFingerprint: fingerprint("b"), appFingerprint: fingerprint("c"), initial, restart: differentRestart }),
    "PACKAGED_SMOKE_PROFILE_MISMATCH",
  )
  expectCode(
    () => createPackagedSmokeEvidence({ sourceFingerprint: "not-a-hash", appFingerprint: fingerprint("c"), initial, restart }),
    "PACKAGED_SMOKE_FINGERPRINT_INVALID",
  )
})

test("upgraded profile marker is required and retained without stdout", () => {
  assert.deepEqual(parseUpgradedProfileSmokeOutput(
    "upgrade_existing_document_readback_verified=true\n",
  ), { upgradeExistingDocumentReadbackVerified: true })
  expectCode(
    () => parseUpgradedProfileSmokeOutput(""),
    "PACKAGED_SMOKE_FIELD_MISSING",
  )
  expectCode(
    () => parseUpgradedProfileSmokeOutput("upgrade_existing_document_readback_verified=false\n"),
    "PACKAGED_SMOKE_UPGRADE_READBACK_MISSING",
  )
})
