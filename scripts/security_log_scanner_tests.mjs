import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";

import {
  ScanEvent,
  ScanState,
  SecurityScanErrorCode,
  findDeniedFixturesInText,
  renderScanResult,
  runSecurityLogScan,
  transitionScanState,
  validateLogPolicyManifest,
} from "./security_log_scanner.mjs";

test("log policy manifest rejects allowed and denied field conflicts", () => {
  const manifest = validManifest({
    logClasses: [
      {
        name: "Product Log",
        allowedFields: ["event_name", "token"],
        deniedFields: ["token"],
      },
      {
        name: "Field Debug Log",
        allowedFields: ["scope", "query_hash"],
        deniedFields: ["secret"],
      },
      {
        name: "Development Log",
        allowedFields: ["fixture_id"],
        deniedFields: ["production_default"],
      },
    ],
  });

  assert.throws(
    () => validateLogPolicyManifest(manifest),
    /SECURITY_SCAN_MALFORMED_MANIFEST/,
  );
});

test("scanner detects denied fixture without returning raw token", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-security-scan-"));
  await mkdir(join(root, "release"), { recursive: true });
  await writeFile(join(root, "release", "output.txt"), "prefix document body fixture suffix");
  const manifest = validManifest({
    deniedFixtures: [
      {
        id: "document_body_fixture",
        kind: "document_body",
        value: "document body fixture",
      },
    ],
    scanTargets: [{ id: "release_output", path: "release/output.txt", required: true }],
  });
  await writeFile(join(root, "manifest.json"), JSON.stringify(manifest));

  const result = await runSecurityLogScan({ manifestPath: "manifest.json", root });
  const rendered = renderScanResult(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, SecurityScanErrorCode.SensitiveFixtureFound);
  assert.equal(result.tokenId, "document_body_fixture");
  assert.match(rendered, /token_id=document_body_fixture/);
  assert.doesNotMatch(rendered, /document body fixture/);
});

test("scanner passes allowed masked hash count and status fields", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-security-scan-"));
  await mkdir(join(root, "release"), { recursive: true });
  await writeFile(
    join(root, "release", "output.txt"),
    "event_name=document.publish.completed masked_user_id=masked:or-1 query_hash=abc123 candidate_count=2 status=ok",
  );
  const manifest = validManifest({
    deniedFixtures: [
      {
        id: "token_fixture",
        kind: "token",
        value: "token-fixture-raw-value",
      },
    ],
    scanTargets: [{ id: "release_output", path: "release/output.txt", required: true }],
  });
  await writeFile(join(root, "manifest.json"), JSON.stringify(manifest));

  const result = await runSecurityLogScan({ manifestPath: "manifest.json", root });

  assert.equal(result.passed, true);
  assert.equal(result.state, ScanState.Passed);
  assert.equal(result.scannedTargetCount, 1);
});

test("active security manifest includes Phase 004 graph artifacts and denied fixtures", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  validateLogPolicyManifest(manifest);

  const targetIds = manifest.scanTargets.map((target) => target.id);
  const fixtureIds = manifest.deniedFixtures.map((fixture) => fixture.id);

  assert.ok(targetIds.includes("phase004_graph_coverage_audit"));
  assert.ok(targetIds.includes("phase004_graph_product_gate_result"));
  assert.ok(targetIds.includes("desktop_remote_graph_smoke_output"));
  assert.ok(fixtureIds.includes("graph_raw_link_text_fixture"));
  assert.ok(fixtureIds.includes("graph_attachment_filename_fixture"));
});

test("active security manifest includes Phase 004 realtime collaboration artifacts and denied fixtures", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  validateLogPolicyManifest(manifest);

  const targetIds = manifest.scanTargets.map((target) => target.id);
  const fixtureIds = manifest.deniedFixtures.map((fixture) => fixture.id);
  const productGateTarget = manifest.scanTargets.find(
    (target) => target.id === "phase004_realtime_collaboration_product_gate_result",
  );
  const logClassDeniedFields = new Map(
    manifest.logClasses.map((logClass) => [logClass.name, new Set(logClass.deniedFields)]),
  );

  assert.ok(targetIds.includes("phase004_realtime_collaboration_smoke_result"));
  assert.ok(targetIds.includes("phase004_collaboration_coverage_audit"));
  assert.ok(targetIds.includes("phase004_realtime_collaboration_product_gate_result"));
  assert.equal(productGateTarget.required, false);
  assert.ok(fixtureIds.includes("realtime_operation_text_fixture"));
  assert.ok(fixtureIds.includes("realtime_selection_text_fixture"));
  assert.ok(fixtureIds.includes("realtime_document_body_fixture"));
  assert.ok(fixtureIds.includes("realtime_session_token_fixture"));

  for (const className of ["Product Log", "Field Debug Log", "Development Log"]) {
    const deniedFields = logClassDeniedFields.get(className);
    assert.ok(deniedFields.has("operation_text"));
    assert.ok(deniedFields.has("selection_text"));
    assert.ok(deniedFields.has("clipboard_content"));
    assert.ok(deniedFields.has("codemirror_transaction_dump"));
  }
});

test("active security manifest includes Phase 004 Canvas artifacts and denied fixtures", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  validateLogPolicyManifest(manifest);

  const targetIds = manifest.scanTargets.map((target) => target.id);
  const fixtureIds = manifest.deniedFixtures.map((fixture) => fixture.id);
  const logClassDeniedFields = new Map(
    manifest.logClasses.map((logClass) => [logClass.name, new Set(logClass.deniedFields)]),
  );

  assert.ok(targetIds.includes("phase004_canvas_domain_model"));
  assert.ok(targetIds.includes("phase004_canvas_usecase_contract"));
  assert.ok(targetIds.includes("phase004_canvas_local_adapter"));
  assert.ok(targetIds.includes("phase004_canvas_coverage_audit"));
  assert.ok(fixtureIds.includes("canvas_raw_ui_state_fixture"));
  assert.ok(fixtureIds.includes("canvas_text_card_fixture"));
  assert.ok(fixtureIds.includes("canvas_heading_title_fixture"));
  assert.ok(fixtureIds.includes("canvas_attachment_filename_fixture"));

  for (const className of ["Product Log", "Field Debug Log", "Development Log"]) {
    const deniedFields = logClassDeniedFields.get(className);
    assert.ok(deniedFields.has("canvas_raw_ui_state"));
    assert.ok(deniedFields.has("card_text"));
    assert.ok(deniedFields.has("heading_title"));
    assert.ok(deniedFields.has("canvas_attachment_filename"));
  }
});

test("active security manifest includes Phase 004 product smoke gate artifact", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  validateLogPolicyManifest(manifest);

  const targetIds = manifest.scanTargets.map((target) => target.id);

  assert.ok(targetIds.includes("phase004_product_smoke_gate_result"));
});

test("active security manifest includes Phase 004 final release gate artifact", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  validateLogPolicyManifest(manifest);

  const finalReleaseTarget = manifest.scanTargets.find(
    (target) => target.id === "phase004_final_release_gate_result",
  );

  assert.equal(finalReleaseTarget?.path, ".tasks/phase004/phase004-final-release-gate-result.md");
  assert.equal(finalReleaseTarget?.required, true);
});

test("active security manifest includes Phase 004 runbook artifacts", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  validateLogPolicyManifest(manifest);

  const targetIds = manifest.scanTargets.map((target) => target.id);

  assert.ok(targetIds.includes("runbook_graph_reindex_diagnostics"));
  assert.ok(targetIds.includes("runbook_collaboration_room_recovery"));
  assert.ok(targetIds.includes("runbook_canvas_mobile_notification_diagnostics"));
});

test("active security manifest includes Phase 004 mobile push artifacts and denied fixtures", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  validateLogPolicyManifest(manifest);

  const targetIds = manifest.scanTargets.map((target) => target.id);
  const fixtureIds = manifest.deniedFixtures.map((fixture) => fixture.id);
  const logClassDeniedFields = new Map(
    manifest.logClasses.map((logClass) => [logClass.name, new Set(logClass.deniedFields)]),
  );

  assert.ok(targetIds.includes("phase004_mobile_push_payload_contract"));
  assert.ok(targetIds.includes("phase004_mobile_capability_audit"));
  assert.ok(fixtureIds.includes("mobile_push_document_body_fixture"));
  assert.ok(fixtureIds.includes("mobile_push_comment_body_fixture"));
  assert.ok(fixtureIds.includes("mobile_push_session_token_fixture"));
  assert.ok(fixtureIds.includes("mobile_push_raw_canvas_state_fixture"));

  for (const className of ["Product Log", "Field Debug Log", "Development Log"]) {
    const deniedFields = logClassDeniedFields.get(className);
    assert.ok(deniedFields.has("push_document_body"));
    assert.ok(deniedFields.has("push_comment_body"));
    assert.ok(deniedFields.has("push_session_token"));
    assert.ok(deniedFields.has("push_raw_canvas_state"));
  }
});

test("active security manifest includes Phase 005 AI and integration artifacts and denied fixtures", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  validateLogPolicyManifest(manifest);

  const targetIds = manifest.scanTargets.map((target) => target.id);
  const fixtureIds = manifest.deniedFixtures.map((fixture) => fixture.id);
  const logClassDeniedFields = new Map(
    manifest.logClasses.map((logClass) => [logClass.name, new Set(logClass.deniedFields)]),
  );

  assert.ok(targetIds.includes("phase005_retrieval_coverage_audit"));
  assert.ok(targetIds.includes("phase005_semantic_search_gate_result"));
  assert.ok(targetIds.includes("phase005_ai_answer_product_gate_result"));
  assert.ok(targetIds.includes("phase005_mcp_api_product_gate_result"));
  assert.ok(targetIds.includes("phase005_webhook_connector_product_gate_result"));
  assert.ok(targetIds.includes("phase005_product_smoke_gate_result"));
  assert.ok(targetIds.includes("runbook_ai_retrieval_degradation"));
  assert.ok(targetIds.includes("runbook_ai_provider_outage"));
  assert.ok(targetIds.includes("runbook_webhook_dead_letter_recovery"));
  assert.ok(targetIds.includes("runbook_connector_authorization_failure"));

  for (const fixtureId of [
    "ai_prompt_fixture",
    "ai_answer_fixture",
    "retrieval_source_text_fixture",
    "embedding_input_fixture",
    "provider_api_key_fixture",
    "connector_access_token_fixture",
    "connector_refresh_token_fixture",
    "connector_client_secret_fixture",
    "webhook_secret_fixture",
    "webhook_payload_body_fixture",
  ]) {
    assert.ok(fixtureIds.includes(fixtureId));
  }

  for (const className of ["Product Log", "Field Debug Log", "Development Log"]) {
    const deniedFields = logClassDeniedFields.get(className);
    for (const field of [
      "ai_prompt",
      "ai_answer",
      "retrieval_source_text",
      "embedding_input",
      "provider_api_key",
      "connector_access_token",
      "connector_refresh_token",
      "connector_client_secret",
      "webhook_secret",
      "webhook_payload_body",
    ]) {
      assert.ok(deniedFields.has(field));
    }
  }
});

test("scanner reports missing required target with stable error code", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-security-scan-"));
  const manifest = validManifest({
    scanTargets: [{ id: "missing", path: "release/missing.txt", required: true }],
  });
  await writeFile(join(root, "manifest.json"), JSON.stringify(manifest));

  const result = await runSecurityLogScan({ manifestPath: "manifest.json", root });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, SecurityScanErrorCode.MissingTarget);
  assert.equal(result.filePath, "release/missing.txt");
});

test("scanner state machine exposes explicit terminal transitions", () => {
  const reading = transitionScanState(ScanState.NotStarted, ScanEvent.Start);
  const scanning = transitionScanState(reading.state, ScanEvent.ManifestLoaded);
  const passed = transitionScanState(scanning.state, ScanEvent.Complete);
  const failed = transitionScanState(ScanState.Scanning, ScanEvent.MatchFound, {
    errorCode: SecurityScanErrorCode.SensitiveFixtureFound,
    filePath: "release/output.txt",
    tokenId: "token_fixture",
  });

  assert.equal(reading.state, ScanState.ReadingManifest);
  assert.equal(scanning.state, ScanState.Scanning);
  assert.equal(passed.state, ScanState.Passed);
  assert.equal(failed.state, ScanState.Failed);
  assert.equal(failed.tokenId, "token_fixture");
});

test("denied fixture detection returns token ids and kinds only", () => {
  const findings = findDeniedFixturesInText("raw-secret-value", [
    { id: "secret_fixture", kind: "secret", value: "raw-secret-value" },
  ]);

  assert.deepEqual(findings, [
    {
      tokenId: "secret_fixture",
      kind: "secret",
      errorCode: SecurityScanErrorCode.SensitiveFixtureFound,
    },
  ]);
});

function validManifest(overrides = {}) {
  return {
    schemaVersion: 1,
    policyId: "test.security-log-policy",
    logClasses: [
      {
        name: "Product Log",
        allowedFields: ["event_name", "masked_user_id", "status", "error_code"],
        deniedFields: ["document_body", "comment_body", "asset_content", "token", "secret"],
      },
      {
        name: "Field Debug Log",
        allowedFields: ["scope", "ttl_seconds", "query_hash", "candidate_count"],
        deniedFields: ["document_body", "comment_body", "asset_content", "token", "secret"],
      },
      {
        name: "Development Log",
        allowedFields: ["fixture_id", "fake_port_name", "call_count"],
        deniedFields: ["production_default", "customer_data", "token", "secret"],
      },
    ],
    deniedFixtures: [
      {
        id: "default_token_fixture",
        kind: "token",
        value: "default-token-fixture",
      },
    ],
    scanTargets: [{ id: "release_output", path: "release/output.txt", required: true }],
    ...overrides,
  };
}
