import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  RunbookValidationErrorCode,
  RunbookValidationEvent,
  RunbookValidationState,
  renderRunbookValidationResult,
  runRunbookValidation,
  transitionRunbookValidationState,
  validateRunbookManifest,
  validateRunbookText,
} from "./runbook_validator.mjs";

test("runbook validator reports missing required section with stable error code", async () => {
  const root = await createFixtureRoot({
    runbookBody: [
      "# Self-Host Installation Runbook",
      "",
      "## Purpose",
      "",
      "- install once",
      "",
      "## Command",
      "",
      "```text",
      "scripts/run_self_host_server.sh --profile default",
      "```",
    ].join("\n"),
  });

  const result = await runRunbookValidation({ root, manifestPath: "manifest.json" });
  const rendered = renderRunbookValidationResult(result);

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, RunbookValidationErrorCode.RequiredSectionMissing);
  assert.match(rendered, /error_code=RUNBOOK_REQUIRED_SECTION_MISSING/);
  assert.doesNotMatch(rendered, /scripts\/run_self_host_server/);
});

test("runbook validator rejects forbidden manual env edit and sensitive examples", () => {
  const findings = validateRunbookText({
    runbook: validRunbookEntry(),
    text: [
      "# Self-Host Installation Runbook",
      "",
      "## Purpose",
      "Install once.",
      "## Command",
      "edit .env and paste raw-token-example",
      "## Expected Output",
      "server_health=ok",
      "## Failure Categories",
      "SELF_HOST_CONFIG_INVALID",
      "## Recovery Actions",
      "Stop and retry without changing current workspace data.",
      "## Configuration Rules",
      "Runtime config is read once at bootstrap and passed explicitly.",
      "## Logging Rules",
      "Product Log, Field Debug Log, Development Log are separated.",
      "## State Machine",
      "NotStarted -> Running -> Completed or Failed",
    ].join("\n"),
  });

  assert.equal(findings[0].errorCode, RunbookValidationErrorCode.ForbiddenTextFound);
  assert.equal(findings[0].findingId, "manual_env_edit");
});

test("runbook validator passes complete runbook fixture", async () => {
  const root = await createFixtureRoot({
    runbookBody: validRunbookText(),
  });

  const result = await runRunbookValidation({ root, manifestPath: "manifest.json" });

  assert.equal(result.passed, true);
  assert.equal(result.state, RunbookValidationState.Passed);
  assert.equal(result.checkedRunbookCount, 1);
});

test("runbook manifest validates required shape", () => {
  assert.throws(
    () => validateRunbookManifest({ schemaVersion: 1, runbooks: [] }),
    /RUNBOOK_MALFORMED_MANIFEST/,
  );
});

test("active runbook manifest includes Phase 004 operational runbooks", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/runbook-validation-manifest.json", "utf8"),
  );
  const ids = manifest.runbooks.map((runbook) => runbook.id);

  assert.ok(ids.includes("graph_reindex_diagnostics"));
  assert.ok(ids.includes("collaboration_room_recovery"));
  assert.ok(ids.includes("canvas_mobile_notification_diagnostics"));
});

test("active runbook manifest includes Phase 005 AI and integration operational runbooks", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/runbook-validation-manifest.json", "utf8"),
  );
  validateRunbookManifest(manifest);

  const ids = manifest.runbooks.map((runbook) => runbook.id);

  assert.ok(ids.includes("ai_retrieval_degradation"));
  assert.ok(ids.includes("ai_provider_outage"));
  assert.ok(ids.includes("webhook_dead_letter_recovery"));
  assert.ok(ids.includes("connector_authorization_failure"));
});

test("runbook validator reports missing Phase 004 runbook with stable error code", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-runbook-active-"));
  await mkdir(join(root, ".tasks", "release", "runbooks"), { recursive: true });
  const manifest = {
    schemaVersion: 1,
    policyId: "active.runbooks",
    requiredSections: [
      "## Purpose",
      "## Command",
      "## Expected Output",
      "## Failure Categories",
      "## Recovery Actions",
      "## Configuration Rules",
      "## Logging Rules",
      "## State Machine",
    ],
    requiredPhrases: ["failure category", "recovery action", "Product Log", "Field Debug Log", "Development Log"],
    forbiddenText: [
      { id: "manual_env_file_edit", value: "edit .env" },
      { id: "raw_token_example", value: "raw-token-example" },
    ],
    runbooks: [
      {
        id: "graph_reindex_diagnostics",
        path: ".tasks/release/runbooks/graph-reindex-diagnostics.md",
        requiredPhrases: ["Graph Reindex", "ReindexRequested", "Reindexing", "Clean", "Degraded", "p95 300ms"],
      },
      {
        id: "collaboration_room_recovery",
        path: ".tasks/release/runbooks/collaboration-room-recovery.md",
        requiredPhrases: ["Collaboration Room Recovery", "Disconnected", "Connecting", "Connected", "ReplayingLocalChanges", "ConflictDetected"],
      },
      {
        id: "canvas_mobile_notification_diagnostics",
        path: ".tasks/release/runbooks/canvas-mobile-notification-diagnostics.md",
        requiredPhrases: ["Canvas Mobile Notification Diagnostics", "Draft", "Saved", "Embedded", "Queued", "Sent", "Failed", "Retry"],
      },
    ],
  };
  await writeFile(join(root, "manifest.json"), JSON.stringify(manifest, null, 2));

  const result = await runRunbookValidation({ root, manifestPath: "manifest.json" });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, RunbookValidationErrorCode.MissingRunbook);
  assert.equal(result.runbookId, "graph_reindex_diagnostics");
});

test("runbook validator reports missing Phase 005 runbook with stable error code", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-runbook-phase005-"));
  await mkdir(join(root, ".tasks", "release", "runbooks"), { recursive: true });
  const manifest = {
    schemaVersion: 1,
    policyId: "phase005.runbooks",
    requiredSections: [
      "## Purpose",
      "## Command",
      "## Expected Output",
      "## Failure Categories",
      "## Recovery Actions",
      "## Configuration Rules",
      "## Logging Rules",
      "## State Machine",
    ],
    requiredPhrases: ["failure category", "recovery action", "Product Log", "Field Debug Log", "Development Log"],
    forbiddenText: [
      { id: "manual_env_file_edit", value: "edit .env" },
      { id: "raw_token_example", value: "raw-token-example" },
    ],
    runbooks: [
      {
        id: "ai_retrieval_degradation",
        path: ".tasks/release/runbooks/ai-retrieval-degradation.md",
        requiredPhrases: [
          "AI Retrieval Degradation",
          "Healthy",
          "Degraded",
          "ReindexQueued",
          "Reindexing",
          "Recovered",
          "p95 300ms",
        ],
      },
      {
        id: "ai_provider_outage",
        path: ".tasks/release/runbooks/ai-provider-outage.md",
        requiredPhrases: [
          "AI Provider Outage",
          "ProviderUnavailable",
          "RetryScheduled",
          "Refused",
          "Recovered",
        ],
      },
      {
        id: "webhook_dead_letter_recovery",
        path: ".tasks/release/runbooks/webhook-dead-letter-recovery.md",
        requiredPhrases: [
          "Webhook Dead Letter Recovery",
          "DeadLettered",
          "Replaying",
          "Delivered",
          "Abandoned",
        ],
      },
      {
        id: "connector_authorization_failure",
        path: ".tasks/release/runbooks/connector-authorization-failure.md",
        requiredPhrases: [
          "Connector Authorization Failure",
          "AuthorizationFailed",
          "ReauthorizeRequired",
          "Disabled",
          "Recovered",
        ],
      },
    ],
  };
  await writeFile(join(root, "manifest.json"), JSON.stringify(manifest, null, 2));

  const result = await runRunbookValidation({ root, manifestPath: "manifest.json" });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, RunbookValidationErrorCode.MissingRunbook);
  assert.equal(result.runbookId, "ai_retrieval_degradation");
});

test("runbook validation state machine exposes terminal transitions", () => {
  const reading = transitionRunbookValidationState(
    RunbookValidationState.NotStarted,
    RunbookValidationEvent.Start,
  );
  const validating = transitionRunbookValidationState(
    reading.state,
    RunbookValidationEvent.ManifestLoaded,
  );
  const passed = transitionRunbookValidationState(
    validating.state,
    RunbookValidationEvent.Complete,
  );
  const failed = transitionRunbookValidationState(
    validating.state,
    RunbookValidationEvent.Fail,
    {
      errorCode: RunbookValidationErrorCode.ForbiddenTextFound,
      runbookId: "install",
      findingId: "raw_token_example",
    },
  );

  assert.equal(reading.state, RunbookValidationState.ReadingManifest);
  assert.equal(validating.state, RunbookValidationState.Validating);
  assert.equal(passed.state, RunbookValidationState.Passed);
  assert.equal(failed.state, RunbookValidationState.Failed);
  assert.equal(failed.runbookId, "install");
});

async function createFixtureRoot({ runbookBody }) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-runbook-validation-"));
  await mkdir(join(root, "runbooks"), { recursive: true });
  await writeFile(join(root, "runbooks", "install.md"), runbookBody);
  await writeFile(
    join(root, "manifest.json"),
    JSON.stringify(
      {
        schemaVersion: 1,
        policyId: "fixture.runbooks",
        requiredSections: [
          "## Purpose",
          "## Command",
          "## Expected Output",
          "## Failure Categories",
          "## Recovery Actions",
          "## Configuration Rules",
          "## Logging Rules",
          "## State Machine",
        ],
        requiredPhrases: [
          "install once",
          "Runtime config is read once at bootstrap",
          "Product Log",
          "Field Debug Log",
          "Development Log",
          "failure category",
          "recovery action",
        ],
        forbiddenText: [
          { id: "manual_env_edit", value: "edit .env" },
          { id: "raw_token_example", value: "raw-token-example" },
        ],
        runbooks: [validRunbookEntry()],
      },
      null,
      2,
    ),
  );
  return root;
}

function validRunbookEntry() {
  return {
    id: "self_host_installation",
    path: "runbooks/install.md",
    requiredPhrases: ["server_health=ok"],
  };
}

function validRunbookText() {
  return [
    "# Self-Host Installation Runbook",
    "",
    "## Purpose",
    "",
    "- The operator can install once and start the default profile.",
    "",
    "## Command",
    "",
    "```text",
    "scripts/run_self_host_server.sh --profile default",
    "```",
    "",
    "## Expected Output",
    "",
    "```text",
    "server_health=ok",
    "```",
    "",
    "## Failure Categories",
    "",
    "- SELF_HOST_CONFIG_INVALID: failure category for invalid bootstrap config.",
    "",
    "## Recovery Actions",
    "",
    "- Preserve current workspace data and retry with the generated default profile; this is the recovery action.",
    "",
    "## Configuration Rules",
    "",
    "- Runtime config is read once at bootstrap, validated, and passed explicitly through constructor or context objects.",
    "",
    "## Logging Rules",
    "",
    "- Product Log records stable event names only.",
    "- Field Debug Log requires scope and TTL.",
    "- Development Log is local/test only.",
    "",
    "## State Machine",
    "",
    "- NotStarted -> Running -> Completed.",
    "- Running -> Failed when bootstrap validation fails.",
  ].join("\n");
}
