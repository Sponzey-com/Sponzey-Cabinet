import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase010SettingsObservabilityState = Object.freeze({
  Pending: "Pending",
  ReadingPrerequisites: "ReadingPrerequisites",
  RunningSettingsTests: "RunningSettingsTests",
  RunningAiTests: "RunningAiTests",
  RunningFieldDebugTests: "RunningFieldDebugTests",
  ValidatingArtifacts: "ValidatingArtifacts",
  RunningSelfCheck: "RunningSelfCheck",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase010SettingsObservabilityEvent = Object.freeze({
  Start: "Start",
  PrerequisitesRead: "PrerequisitesRead",
  SettingsTestsPassed: "SettingsTestsPassed",
  AiTestsPassed: "AiTestsPassed",
  FieldDebugTestsPassed: "FieldDebugTestsPassed",
  ArtifactsValidated: "ArtifactsValidated",
  SelfCheckPassed: "SelfCheckPassed",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase010SettingsObservabilityErrorCode = Object.freeze({
  IndexHealthMarkerMissing: "PHASE010_INDEX_HEALTH_MARKER_MISSING",
  SettingsTestsFailed: "PHASE010_SETTINGS_TESTS_FAILED",
  AiTestsFailed: "PHASE010_AI_PROVIDER_TESTS_FAILED",
  FieldDebugTestsFailed: "PHASE010_FIELD_DEBUG_TESTS_FAILED",
  SelfCheckFailed: "PHASE010_SETTINGS_OBSERVABILITY_SELF_CHECK_FAILED",
  ArtifactInvalid: "PHASE010_SETTINGS_OBSERVABILITY_ARTIFACT_INVALID",
  UnsafeArtifactContent: "PHASE010_SETTINGS_OBSERVABILITY_UNSAFE_ARTIFACT_CONTENT",
  IoFailed: "PHASE010_SETTINGS_OBSERVABILITY_IO_FAILED",
  InvalidTransition: "PHASE010_SETTINGS_OBSERVABILITY_INVALID_TRANSITION",
});

const settingsStepIds = ["settingsUiModels", "desktopSettingsSmoke"];
const aiStepIds = ["aiProviderModels", "aiPromptBuilder", "aiSummaryUsecases", "aiUsecases"];
const fieldDebugStepIds = ["fieldDebugUsecases"];
const selfCheckStepIds = ["settingsObservabilitySelfCheck"];
const unsafeArtifactTerms = [
  "provider_api_key_fixture",
  "raw_document_body_fixture",
  "personal_absolute_path_fixture",
  "token_fixture",
  "credential_fixture",
  "secret_fixture",
  "raw_prompt_fixture",
  "raw_answer_fixture",
  "/Users/",
  "C:\\Users\\",
];

export function transitionPhase010SettingsObservabilityState(currentState, event, detail = {}) {
  if (
    currentState === Phase010SettingsObservabilityState.Pending &&
    event === Phase010SettingsObservabilityEvent.Start
  ) {
    return { state: Phase010SettingsObservabilityState.ReadingPrerequisites };
  }
  if (
    currentState === Phase010SettingsObservabilityState.ReadingPrerequisites &&
    event === Phase010SettingsObservabilityEvent.PrerequisitesRead
  ) {
    return { state: Phase010SettingsObservabilityState.RunningSettingsTests };
  }
  if (
    currentState === Phase010SettingsObservabilityState.RunningSettingsTests &&
    event === Phase010SettingsObservabilityEvent.SettingsTestsPassed
  ) {
    return { state: Phase010SettingsObservabilityState.RunningAiTests };
  }
  if (
    currentState === Phase010SettingsObservabilityState.RunningAiTests &&
    event === Phase010SettingsObservabilityEvent.AiTestsPassed
  ) {
    return { state: Phase010SettingsObservabilityState.RunningFieldDebugTests };
  }
  if (
    currentState === Phase010SettingsObservabilityState.RunningFieldDebugTests &&
    event === Phase010SettingsObservabilityEvent.FieldDebugTestsPassed
  ) {
    return { state: Phase010SettingsObservabilityState.ValidatingArtifacts };
  }
  if (
    currentState === Phase010SettingsObservabilityState.ValidatingArtifacts &&
    event === Phase010SettingsObservabilityEvent.ArtifactsValidated
  ) {
    return { state: Phase010SettingsObservabilityState.RunningSelfCheck };
  }
  if (
    currentState === Phase010SettingsObservabilityState.RunningSelfCheck &&
    event === Phase010SettingsObservabilityEvent.SelfCheckPassed
  ) {
    return { state: Phase010SettingsObservabilityState.WritingResult };
  }
  if (
    currentState === Phase010SettingsObservabilityState.WritingResult &&
    event === Phase010SettingsObservabilityEvent.ResultWritten
  ) {
    return { state: Phase010SettingsObservabilityState.Passed };
  }
  if (
    [
      Phase010SettingsObservabilityState.ReadingPrerequisites,
      Phase010SettingsObservabilityState.RunningSettingsTests,
      Phase010SettingsObservabilityState.RunningAiTests,
      Phase010SettingsObservabilityState.RunningFieldDebugTests,
      Phase010SettingsObservabilityState.ValidatingArtifacts,
      Phase010SettingsObservabilityState.RunningSelfCheck,
      Phase010SettingsObservabilityState.WritingResult,
    ].includes(currentState) &&
    event === Phase010SettingsObservabilityEvent.Fail
  ) {
    return {
      state: Phase010SettingsObservabilityState.Failed,
      errorCode: detail.errorCode ?? Phase010SettingsObservabilityErrorCode.IoFailed,
      findingId: detail.findingId,
      failedStepId: detail.failedStepId,
    };
  }
  return {
    state: Phase010SettingsObservabilityState.Failed,
    errorCode: Phase010SettingsObservabilityErrorCode.InvalidTransition,
  };
}

export function buildPhase010SettingsObservabilityCommandPlan() {
  return [
    step("settingsUiModels", [
      "node",
      "--test",
      "packages/ui/tests/personal_workspace_shell_model_tests.ts",
      "packages/ui/tests/personal_workspace_home_model_tests.ts",
      "packages/ui/tests/backup_restore_staging_model_tests.ts",
    ]),
    step("desktopSettingsSmoke", [
      "node",
      "--test",
      "apps/desktop/tests/desktop_personal_workspace_shell_tests.ts",
      "apps/desktop/tests/desktop_personal_workspace_home_tests.ts",
    ]),
    step("aiProviderModels", [
      "node",
      "--test",
      "packages/ui/tests/ai_citation_tool_scope_model_tests.ts",
      "packages/ui/tests/ai_query_ui_model_tests.ts",
      "apps/desktop/tests/desktop_ai_local_ux_smoke_tests.ts",
      "apps/desktop/tests/desktop_ai_product_smoke_tests.ts",
    ]),
    step("aiPromptBuilder", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "ai_prompt_builder_tests",
    ]),
    step("aiSummaryUsecases", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "ai_summary_usecase_tests",
    ]),
    step("aiUsecases", ["cargo", "test", "-p", "cabinet-usecases", "--test", "ai_usecase_tests"]),
    step("fieldDebugUsecases", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "field_debug_usecase_tests",
    ]),
    step("settingsObservabilitySelfCheck", [
      "node",
      "scripts/phase010_settings_observability_self_check.mjs",
    ]),
  ];
}

export function buildPhase010ProductLogMatrix() {
  return {
    marker: "phase010_product_log_matrix=passed",
    phase: "Phase 010.6",
    currentScope: "personal local desktop",
    events: [
      event("settings.opened", "Product Log", ["section", "masked_workspace_id", "duration_bucket"]),
      event("settings.storage.loaded", "Product Log", ["section", "status", "duration_bucket"]),
      event("settings.backup.updated", "Product Log", ["section", "status", "stable_error_code"]),
      event("ai.provider.status.loaded", "Product Log", ["status", "duration_bucket"]),
      event("field_debug.activation.created", "Product Log", [
        "scope_hash",
        "expiry_bucket",
        "reason_code",
        "masking_policy_id",
      ]),
      event("field_debug.activation.expired", "Product Log", [
        "scope_hash",
        "expiry_bucket",
        "stable_error_code",
      ]),
      event("field_debug.diagnostic.checked", "Field Debug Log", [
        "scope_hash",
        "component_id",
        "count",
        "stable_error_code",
      ]),
      event("settings.model.fixture.checked", "Development Log", [
        "fixture_id",
        "section_count",
        "command_id",
      ]),
    ],
    deniedFields: [
      "provider raw key",
      "authentication material",
      "raw local path",
      "raw prompt",
      "raw answer",
      "raw document body",
      "asset bytes",
    ],
  };
}

export function buildPhase010SecurityLogManifest() {
  return {
    marker: "phase010_security_log_manifest=passed",
    schemaVersion: 1,
    phase: "Phase 010.6",
    productScope: "personal_local_desktop",
    logClasses: [
      {
        name: "Product Log",
        allowedFields: [
          "event_name",
          "correlation_id",
          "masked_workspace_id",
          "section",
          "status",
          "duration_bucket",
          "stable_error_code",
        ],
        deniedFields: [
          "providerRawKey",
          "authMaterial",
          "rawLocalPath",
          "rawPrompt",
          "rawAnswer",
          "rawDocumentBody",
          "assetBytes",
        ],
      },
      {
        name: "Field Debug Log",
        allowedFields: [
          "event_name",
          "correlation_id",
          "scope_hash",
          "component_id",
          "count",
          "stable_error_code",
          "expires_at_bucket",
        ],
        deniedFields: [
          "providerRawKey",
          "authMaterial",
          "rawLocalPath",
          "rawPrompt",
          "rawAnswer",
          "rawDocumentBody",
          "assetBytes",
        ],
      },
      {
        name: "Development Log",
        allowedFields: ["event_name", "fixture_id", "section_count", "command_id"],
        deniedFields: [
          "providerRawKey",
          "authMaterial",
          "rawLocalPath",
          "rawPrompt",
          "rawAnswer",
          "rawDocumentBody",
          "assetBytes",
        ],
      },
    ],
    deniedFixtures: [
      { id: "auth_material_sample", kind: "auth_material", value: "AUTH_MATERIAL_SAMPLE" },
      { id: "document_body_sample", kind: "document_body", value: "RAW_DOC_BODY_SAMPLE" },
      { id: "personal_path_sample", kind: "path", value: "PERSONAL_PATH_SAMPLE" },
      { id: "ai_prompt_sample", kind: "ai_prompt", value: "AI_PROMPT_SAMPLE" },
      { id: "ai_answer_sample", kind: "ai_answer", value: "AI_ANSWER_SAMPLE" },
    ],
    scanTargets: [
      {
        id: "settings_observability_gate_result",
        path: ".tasks/phase010-settings-observability-gate-result.md",
        required: true,
      },
      {
        id: "product_log_matrix",
        path: ".tasks/release/product-log-event-matrix-phase010.md",
        required: true,
      },
      {
        id: "local_desktop_runbook",
        path: ".tasks/release/local-desktop-runbook-phase010.md",
        required: true,
      },
    ],
  };
}

export function renderPhase010ProductLogMatrix(matrix = buildPhase010ProductLogMatrix()) {
  return [
    "# Phase 010 Product Log Event Matrix",
    "",
    matrix.marker,
    "",
    `- phase: \`${matrix.phase}\``,
    `- current scope: \`${matrix.currentScope}\``,
    "- rule: Product Log records only user impact, key state changes, and stable failure codes.",
    "- Field Debug Log is disabled by default and requires scope, expiry, reason, and masking policy.",
    "- Development Log is limited to local tests and release gate diagnostics.",
    "- sensitive-data exclusion: records ids, hashes, counts, status, duration buckets, and stable error codes only. It excludes provider raw key, authentication material, raw local path, raw prompt, raw answer, raw document body, and asset bytes.",
    "",
    "| event | log class | allowed fields |",
    "| --- | --- | --- |",
    ...matrix.events.map(
      (item) => `| \`${item.name}\` | ${item.logClass} | ${item.allowedFields.join(", ")} |`,
    ),
    "",
    "## Denied Fields",
    "",
    ...matrix.deniedFields.map((field) => `- ${field}`),
    "",
  ].join("\n");
}

export function renderPhase010SecurityLogManifest(manifest = buildPhase010SecurityLogManifest()) {
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

export function renderPhase010LocalDesktopRunbook() {
  return [
    "# Phase 010 Local Desktop Runbook",
    "",
    "phase010_runbook=passed",
    "",
    "## Clean Install",
    "",
    "- Launch the packaged desktop app after a normal installation.",
    "- The app must create the local workspace, document store, internal version store, asset store, search index, graph projection, backup area, and settings state automatically.",
    "- No external database, external search service, Git CLI, Node runtime, manual environment edit, or manual file edit is required for default startup.",
    "",
    "## Packaged Launch",
    "",
    "- Use `npm run run:phase010-packaged-launch-gate` to validate the packaged launch path.",
    "- The packaged launch path must use built desktop assets and must not require a dev server.",
    "- A blank first screen is a release blocker.",
    "",
    "## Reinstall Preservation",
    "",
    "- Reinstall must keep the existing personal workspace data unless the user explicitly chooses removal.",
    "- The first-run initializer must be idempotent and must not overwrite existing documents, versions, attachments, backups, settings, or indexes.",
    "",
    "## Blank Screen Recovery",
    "",
    "- If the shell opens without content, run the packaged launch gate and the first-run workspace gate.",
    "- If workspace health is unhealthy, run startup repair and re-open the packaged app.",
    "- The runbook records command ids and stable result markers only.",
    "",
    "## Index Repair",
    "",
    "- If search, backlink, graph, or asset metadata status is stale, use the explicit index repair action.",
    "- Normal user-facing reads must use current projections and must not scan full version history.",
    "- The p95 target for current read, history read, search, backlink, graph projection, and asset metadata status is 300ms in normal indexed state.",
    "",
    "## Export Import",
    "",
    "- Export writes portable markdown and attachment metadata through the export usecase.",
    "- Import preview must not mutate the workspace.",
    "- Apply import only after validation reports no blocking issue.",
    "",
    "## Backup Restore",
    "",
    "- Backup and restore use the local backup store boundary.",
    "- Restore preview must report document, version, attachment metadata, and projection effects before applying changes.",
    "- Restore execution must preserve current/history query separation.",
    "",
    "## Field Debug",
    "",
    "- Field Debug is disabled by default.",
    "- Activation requires scope, expiry, reason, and masking policy.",
    "- Field Debug records hashes, counts, component ids, status, and stable error codes only.",
    "- Activation and expiry must be visible in Product Log.",
    "",
    "## Data Export",
    "",
    "- Data export includes documents, current versions, history metadata, attachment metadata, backlinks, graph projection metadata, and backup metadata.",
    "- Export artifacts must exclude authentication material, raw AI prompts, raw AI answers, and personal absolute paths.",
    "",
  ].join("\n");
}

export function evaluatePhase010SettingsObservabilityGate({
  indexHealthText,
  commandResults,
  artifacts = defaultArtifactTexts(),
}) {
  let state = transitionPhase010SettingsObservabilityState(
    Phase010SettingsObservabilityState.Pending,
    Phase010SettingsObservabilityEvent.Start,
  );

  if (!indexHealthText.includes("phase010_index_health_repair_gate=passed")) {
    state = transitionPhase010SettingsObservabilityState(
      state.state,
      Phase010SettingsObservabilityEvent.Fail,
      {
        errorCode: Phase010SettingsObservabilityErrorCode.IndexHealthMarkerMissing,
        findingId: ".tasks/phase010-index-health-repair-gate-result.md",
      },
    );
    return failedResult(state, commandResults, artifacts);
  }
  state = transitionPhase010SettingsObservabilityState(
    state.state,
    Phase010SettingsObservabilityEvent.PrerequisitesRead,
  );

  for (const stepId of settingsStepIds) {
    if (!commandResults[stepId]?.passed) {
      state = transitionPhase010SettingsObservabilityState(
        state.state,
        Phase010SettingsObservabilityEvent.Fail,
        {
          errorCode: Phase010SettingsObservabilityErrorCode.SettingsTestsFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults, artifacts);
    }
  }
  state = transitionPhase010SettingsObservabilityState(
    state.state,
    Phase010SettingsObservabilityEvent.SettingsTestsPassed,
  );

  for (const stepId of aiStepIds) {
    if (!commandResults[stepId]?.passed) {
      state = transitionPhase010SettingsObservabilityState(
        state.state,
        Phase010SettingsObservabilityEvent.Fail,
        {
          errorCode: Phase010SettingsObservabilityErrorCode.AiTestsFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults, artifacts);
    }
  }
  state = transitionPhase010SettingsObservabilityState(
    state.state,
    Phase010SettingsObservabilityEvent.AiTestsPassed,
  );

  for (const stepId of fieldDebugStepIds) {
    if (!commandResults[stepId]?.passed) {
      state = transitionPhase010SettingsObservabilityState(
        state.state,
        Phase010SettingsObservabilityEvent.Fail,
        {
          errorCode: Phase010SettingsObservabilityErrorCode.FieldDebugTestsFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults, artifacts);
    }
  }
  state = transitionPhase010SettingsObservabilityState(
    state.state,
    Phase010SettingsObservabilityEvent.FieldDebugTestsPassed,
  );

  const artifactFinding = validateArtifactTexts(artifacts);
  if (artifactFinding) {
    state = transitionPhase010SettingsObservabilityState(
      state.state,
      Phase010SettingsObservabilityEvent.Fail,
      artifactFinding,
    );
    return failedResult(state, commandResults, artifacts);
  }
  state = transitionPhase010SettingsObservabilityState(
    state.state,
    Phase010SettingsObservabilityEvent.ArtifactsValidated,
  );

  for (const stepId of selfCheckStepIds) {
    if (!commandResults[stepId]?.passed) {
      state = transitionPhase010SettingsObservabilityState(
        state.state,
        Phase010SettingsObservabilityEvent.Fail,
        {
          errorCode: Phase010SettingsObservabilityErrorCode.SelfCheckFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults, artifacts);
    }
  }
  state = transitionPhase010SettingsObservabilityState(
    state.state,
    Phase010SettingsObservabilityEvent.SelfCheckPassed,
  );
  state = transitionPhase010SettingsObservabilityState(
    state.state,
    Phase010SettingsObservabilityEvent.ResultWritten,
  );
  return { passed: true, state: state.state, commandResults, artifacts };
}

export async function runPhase010SettingsObservabilityGate({
  root = process.cwd(),
  writeArtifacts = true,
  runner = runCommandStep,
  steps = buildPhase010SettingsObservabilityCommandPlan(),
  artifacts = defaultArtifactTexts(),
} = {}) {
  let indexHealthText;
  try {
    indexHealthText = await readFile(
      join(root, ".tasks/phase010-index-health-repair-gate-result.md"),
      "utf8",
    );
  } catch (error) {
    return {
      passed: false,
      state: Phase010SettingsObservabilityState.Failed,
      errorCode: Phase010SettingsObservabilityErrorCode.IoFailed,
      findingId: error.path,
      commandResults: {},
      artifacts,
    };
  }
  if (!indexHealthText.includes("phase010_index_health_repair_gate=passed")) {
    return evaluatePhase010SettingsObservabilityGate({
      indexHealthText,
      commandResults: {},
      artifacts,
    });
  }
  const commandResults = {};
  for (const gateStep of steps) {
    const started = Date.now();
    const execution = await runner(gateStep, { root });
    commandResults[gateStep.id] = {
      command: gateStep.command.join(" "),
      passed: execution.exitCode === 0 && !execution.signal,
      exitCode: execution.exitCode,
      signal: execution.signal ?? null,
      durationMs: execution.durationMs ?? Date.now() - started,
    };
    const partial = evaluatePhase010SettingsObservabilityGate({
      indexHealthText,
      commandResults,
      artifacts,
    });
    if (!partial.passed && commandResults[gateStep.id].passed === false) {
      if (writeArtifacts) {
        await writeArtifactFiles(root, partial);
      }
      return partial;
    }
  }
  const result = evaluatePhase010SettingsObservabilityGate({
    indexHealthText,
    commandResults,
    artifacts,
  });
  if (writeArtifacts) {
    await writeArtifactFiles(root, result);
  }
  return result;
}

export function renderPhase010SettingsObservabilityArtifact(result) {
  const passed = result.passed === true;
  const lines = [
    "# Phase 010 Settings Observability Gate Result",
    "",
    passed
      ? "phase010_settings_observability_gate=passed"
      : "phase010_settings_observability_gate=failed",
    `validation_state=${result.state}`,
    `settings_scope=${passed ? "personal_local_desktop" : "not_verified"}`,
    `ai_provider_optional=${passed ? "verified" : "not_verified"}`,
    `field_debug_guard=${passed ? "verified" : "not_verified"}`,
    "",
    "- phase: `Phase 010.6`",
    "- gate: `Local Settings, AI Provider Readiness, Field Debug, and Observability Gate`",
    `- status: \`${passed ? "passed" : "failed"}\``,
    "- prerequisite: `.tasks/phase010-index-health-repair-gate-result.md` with `phase010_index_health_repair_gate=passed`",
    "- validation commands:",
    ...buildPhase010SettingsObservabilityCommandPlan().map(
      (gateStep) => `  - \`${gateStep.command.join(" ")}\``,
    ),
    "- changed layers: `usecase-validation`, `ui-validation`, `desktop-app-validation`, `release-tooling`, `task-tooling`.",
    "- settings sections: storage, backup-export, import, ai-provider, field-debug, workspace-health.",
    "- excluded current-scope settings: server URL, tenant, organization, billing, SSO, management console, team invite.",
    "- optional AI provider states: disabled, configured, configuration required, configuration failed.",
    "- Field Debug activation guard: scope, expiry, reason, and masking policy are required before active diagnostics.",
    "- release evidence:",
    "  - `.tasks/release/product-log-event-matrix-phase010.md`",
    "  - `.tasks/release/security-log-policy-manifest-phase010.json`",
    "  - `.tasks/release/local-desktop-runbook-phase010.md`",
    "- sensitive-data exclusion: this artifact records command ids, states, stable error codes, hashes, counts, and status only. It excludes provider raw key, authentication material, raw local path, raw prompt, raw answer, raw document body, and asset bytes.",
    "",
    "## Command Results",
    "",
    "| command | status | exit code | duration ms |",
    "| --- | --- | ---: | ---: |",
    ...Object.entries(result.commandResults ?? {}).map(
      ([id, entry]) =>
        `| \`${id}\` | ${entry.passed ? "passed" : "failed"} | ${entry.exitCode ?? "null"} | ${entry.durationMs ?? 0} |`,
    ),
  ];
  if (!passed) {
    lines.push("");
    lines.push(`- error_code: \`${result.errorCode}\``);
    lines.push(`- finding_id: \`${result.findingId ?? "unknown"}\``);
    if (result.failedStepId) {
      lines.push(`- failed_step: \`${result.failedStepId}\``);
    }
  }
  lines.push("");
  return lines.join("\n");
}

function validateArtifactTexts(artifacts) {
  const textByName = {
    productLogMatrix: artifacts.productLogMatrixText ?? "",
    securityManifest: artifacts.securityManifestText ?? "",
    runbook: artifacts.runbookText ?? "",
  };
  for (const [name, text] of Object.entries(textByName)) {
    const findingId = unsafeArtifactTerms.find((term) => text.includes(term));
    if (findingId) {
      return {
        errorCode: Phase010SettingsObservabilityErrorCode.UnsafeArtifactContent,
        findingId,
        artifactId: name,
      };
    }
  }
  const requiredMarkers = [
    ["productLogMatrix", "phase010_product_log_matrix=passed"],
    ["securityManifest", "phase010_security_log_manifest=passed"],
    ["runbook", "phase010_runbook=passed"],
  ];
  for (const [name, marker] of requiredMarkers) {
    if (!textByName[name].includes(marker)) {
      return {
        errorCode: Phase010SettingsObservabilityErrorCode.ArtifactInvalid,
        findingId: marker,
      };
    }
  }
  return null;
}

function defaultArtifactTexts() {
  return {
    productLogMatrixText: renderPhase010ProductLogMatrix(buildPhase010ProductLogMatrix()),
    securityManifestText: renderPhase010SecurityLogManifest(buildPhase010SecurityLogManifest()),
    runbookText: renderPhase010LocalDesktopRunbook(),
  };
}

async function writeArtifactFiles(root, result) {
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "phase010-settings-observability-gate-result.md"),
    renderPhase010SettingsObservabilityArtifact(result),
  );
  await writeFile(
    join(root, ".tasks", "release", "product-log-event-matrix-phase010.md"),
    result.artifacts.productLogMatrixText,
  );
  await writeFile(
    join(root, ".tasks", "release", "security-log-policy-manifest-phase010.json"),
    result.artifacts.securityManifestText,
  );
  await writeFile(
    join(root, ".tasks", "release", "local-desktop-runbook-phase010.md"),
    result.artifacts.runbookText,
  );
}

function failedResult(state, commandResults, artifacts) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    failedStepId: state.failedStepId,
    commandResults,
    artifacts,
  };
}

function event(name, logClass, allowedFields) {
  return { name, logClass, allowedFields };
}

function step(id, command) {
  return { id, command };
}

function runCommandStep(gateStep, { root }) {
  return new Promise((resolve, reject) => {
    const started = Date.now();
    const [command, ...args] = gateStep.command;
    const child = spawn(command, args, { cwd: root, stdio: "inherit" });
    child.on("error", reject);
    child.on("exit", (exitCode, signal) =>
      resolve({ exitCode, signal, durationMs: Date.now() - started }),
    );
  });
}

async function main() {
  const result = await runPhase010SettingsObservabilityGate({
    root: process.cwd(),
    writeArtifacts: true,
  });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase010_settings_observability_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
