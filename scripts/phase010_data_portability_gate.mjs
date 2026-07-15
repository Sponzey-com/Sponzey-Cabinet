import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase010DataPortabilityState = Object.freeze({
  Pending: "Pending",
  ReadingPrerequisites: "ReadingPrerequisites",
  RunningPackageTests: "RunningPackageTests",
  RunningUiTests: "RunningUiTests",
  RunningSecurityScan: "RunningSecurityScan",
  WritingManifest: "WritingManifest",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase010DataPortabilityEvent = Object.freeze({
  Start: "Start",
  PrerequisitesRead: "PrerequisitesRead",
  PackageTestsPassed: "PackageTestsPassed",
  UiTestsPassed: "UiTestsPassed",
  SecurityScanPassed: "SecurityScanPassed",
  ManifestWritten: "ManifestWritten",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase010DataPortabilityErrorCode = Object.freeze({
  DurableAuthoringMarkerMissing: "PHASE010_DURABLE_AUTHORING_MARKER_MISSING",
  PackageTestsFailed: "PHASE010_DATA_PORTABILITY_PACKAGE_TESTS_FAILED",
  UiTestsFailed: "PHASE010_DATA_PORTABILITY_UI_TESTS_FAILED",
  SecurityScanFailed: "PHASE010_DATA_PORTABILITY_SECURITY_SCAN_FAILED",
  ManifestUnsafeContent: "PHASE010_DATA_PORTABILITY_MANIFEST_UNSAFE_CONTENT",
  ManifestInvalid: "PHASE010_DATA_PORTABILITY_MANIFEST_INVALID",
  IoFailed: "PHASE010_DATA_PORTABILITY_IO_FAILED",
  InvalidTransition: "PHASE010_DATA_PORTABILITY_INVALID_TRANSITION",
});

const packageStepIds = ["exportMarkdown", "importMarkdownFolder", "backupUsecases", "localBackupStore"];
const uiStepIds = ["uiPortabilityModels", "desktopPortabilitySmoke"];
const securityStepIds = ["securityScan"];
const unsafeManifestTerms = [
  "raw_document_body_fixture",
  "provider_api_key_fixture",
  "personal_absolute_path_fixture",
  "token_fixture",
  "credential_fixture",
  "secret_fixture",
  "/Users/",
  "C:\\Users\\",
];

export function transitionPhase010DataPortabilityState(currentState, event, detail = {}) {
  if (
    currentState === Phase010DataPortabilityState.Pending &&
    event === Phase010DataPortabilityEvent.Start
  ) {
    return { state: Phase010DataPortabilityState.ReadingPrerequisites };
  }
  if (
    currentState === Phase010DataPortabilityState.ReadingPrerequisites &&
    event === Phase010DataPortabilityEvent.PrerequisitesRead
  ) {
    return { state: Phase010DataPortabilityState.RunningPackageTests };
  }
  if (
    currentState === Phase010DataPortabilityState.RunningPackageTests &&
    event === Phase010DataPortabilityEvent.PackageTestsPassed
  ) {
    return { state: Phase010DataPortabilityState.RunningUiTests };
  }
  if (
    currentState === Phase010DataPortabilityState.RunningUiTests &&
    event === Phase010DataPortabilityEvent.UiTestsPassed
  ) {
    return { state: Phase010DataPortabilityState.RunningSecurityScan };
  }
  if (
    currentState === Phase010DataPortabilityState.RunningSecurityScan &&
    event === Phase010DataPortabilityEvent.SecurityScanPassed
  ) {
    return { state: Phase010DataPortabilityState.WritingManifest };
  }
  if (
    currentState === Phase010DataPortabilityState.WritingManifest &&
    event === Phase010DataPortabilityEvent.ManifestWritten
  ) {
    return { state: Phase010DataPortabilityState.WritingResult };
  }
  if (
    currentState === Phase010DataPortabilityState.WritingResult &&
    event === Phase010DataPortabilityEvent.ResultWritten
  ) {
    return { state: Phase010DataPortabilityState.Passed };
  }
  if (
    [
      Phase010DataPortabilityState.ReadingPrerequisites,
      Phase010DataPortabilityState.RunningPackageTests,
      Phase010DataPortabilityState.RunningUiTests,
      Phase010DataPortabilityState.RunningSecurityScan,
      Phase010DataPortabilityState.WritingManifest,
      Phase010DataPortabilityState.WritingResult,
    ].includes(currentState) &&
    event === Phase010DataPortabilityEvent.Fail
  ) {
    return {
      state: Phase010DataPortabilityState.Failed,
      errorCode: detail.errorCode ?? Phase010DataPortabilityErrorCode.IoFailed,
      findingId: detail.findingId,
      failedStepId: detail.failedStepId,
    };
  }
  return {
    state: Phase010DataPortabilityState.Failed,
    errorCode: Phase010DataPortabilityErrorCode.InvalidTransition,
  };
}

export function buildPhase010DataPortabilityCommandPlan() {
  return [
    step("exportMarkdown", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "export_markdown_tests",
    ]),
    step("importMarkdownFolder", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "import_markdown_folder_tests",
    ]),
    step("backupUsecases", [
      "cargo",
      "test",
      "-p",
      "cabinet-usecases",
      "--test",
      "backup_usecase_tests",
    ]),
    step("localBackupStore", [
      "cargo",
      "test",
      "-p",
      "cabinet-adapters",
      "--test",
      "local_backup_store_tests",
    ]),
    step("uiPortabilityModels", [
      "node",
      "--test",
      "packages/ui/tests/backup_restore_staging_model_tests.ts",
      "packages/ui/tests/import_preview_model_tests.ts",
      "packages/ui/tests/restore_flow_model_tests.ts",
    ]),
    step("desktopPortabilitySmoke", [
      "node",
      "--test",
      "apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts",
      "apps/desktop/tests/desktop_import_preview_smoke_tests.ts",
    ]),
    step("securityScan", ["node", "scripts/phase010_data_portability_security_scan.mjs"]),
  ];
}

export function buildPhase010DataPortabilityManifest(input = defaultManifestInput()) {
  return {
    marker: "phase010_data_portability_manifest=passed",
    schemaVersion: input.schemaVersion,
    productScope: input.productScope,
    workspaceScope: input.workspaceScope,
    createdAt: "2026-07-10T00:00:00Z",
    counts: {
      documents: input.documentCount,
      versions: input.versionCount,
      assetMetadata: input.assetMetadataCount,
      graphProjections: input.graphProjectionCount,
    },
    exportFormats: input.exportFormats,
    importSources: input.importSources,
    capabilities: input.capabilities,
    safeWarningIds: input.safeWarningIds,
    validationCommands: buildPhase010DataPortabilityCommandPlan().map((gateStep) =>
      gateStep.command.join(" "),
    ),
    sensitiveDataPolicy: {
      recordsRawPackageContents: false,
      recordsRawDocumentBody: false,
      recordsAssetBytes: false,
      recordsRawAbsolutePath: false,
      recordsAuthenticationMaterial: false,
    },
    currentScopeExclusions: [
      "server hosting",
      "SaaS runtime",
      "multi-user collaboration",
      "mobile product implementation",
      "remote workspace runtime",
    ],
  };
}

export function evaluatePhase010DataPortabilityGate({
  durableAuthoringText,
  commandResults,
  manifest = defaultManifestInput(),
}) {
  let state = transitionPhase010DataPortabilityState(
    Phase010DataPortabilityState.Pending,
    Phase010DataPortabilityEvent.Start,
  );

  if (!durableAuthoringText.includes("phase010_durable_authoring_gate=passed")) {
    state = transitionPhase010DataPortabilityState(
      state.state,
      Phase010DataPortabilityEvent.Fail,
      {
        errorCode: Phase010DataPortabilityErrorCode.DurableAuthoringMarkerMissing,
        findingId: ".tasks/phase010-durable-authoring-gate-result.md",
      },
    );
    return failedResult(state, commandResults, manifest);
  }

  state = transitionPhase010DataPortabilityState(
    state.state,
    Phase010DataPortabilityEvent.PrerequisitesRead,
  );

  for (const stepId of packageStepIds) {
    const result = commandResults[stepId];
    if (!result?.passed) {
      state = transitionPhase010DataPortabilityState(
        state.state,
        Phase010DataPortabilityEvent.Fail,
        {
          errorCode: Phase010DataPortabilityErrorCode.PackageTestsFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults, manifest);
    }
  }

  state = transitionPhase010DataPortabilityState(
    state.state,
    Phase010DataPortabilityEvent.PackageTestsPassed,
  );

  for (const stepId of uiStepIds) {
    const result = commandResults[stepId];
    if (!result?.passed) {
      state = transitionPhase010DataPortabilityState(
        state.state,
        Phase010DataPortabilityEvent.Fail,
        {
          errorCode: Phase010DataPortabilityErrorCode.UiTestsFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults, manifest);
    }
  }

  state = transitionPhase010DataPortabilityState(
    state.state,
    Phase010DataPortabilityEvent.UiTestsPassed,
  );

  for (const stepId of securityStepIds) {
    const result = commandResults[stepId];
    if (!result?.passed) {
      state = transitionPhase010DataPortabilityState(
        state.state,
        Phase010DataPortabilityEvent.Fail,
        {
          errorCode: Phase010DataPortabilityErrorCode.SecurityScanFailed,
          findingId: stepId,
          failedStepId: stepId,
        },
      );
      return failedResult(state, commandResults, manifest);
    }
  }

  state = transitionPhase010DataPortabilityState(
    state.state,
    Phase010DataPortabilityEvent.SecurityScanPassed,
  );

  const manifestSafety = validateManifestSafety(manifest);
  if (!manifestSafety.passed) {
    state = transitionPhase010DataPortabilityState(
      state.state,
      Phase010DataPortabilityEvent.Fail,
      {
        errorCode: manifestSafety.errorCode,
        findingId: manifestSafety.findingId,
      },
    );
    return failedResult(state, commandResults, manifest);
  }

  state = transitionPhase010DataPortabilityState(
    state.state,
    Phase010DataPortabilityEvent.ManifestWritten,
  );
  state = transitionPhase010DataPortabilityState(
    state.state,
    Phase010DataPortabilityEvent.ResultWritten,
  );

  return {
    passed: true,
    state: state.state,
    commandCount: Object.keys(commandResults).length,
    commandResults,
    manifest,
  };
}

export async function runPhase010DataPortabilityGate({
  root = process.cwd(),
  writeArtifacts = true,
  runner = runCommandStep,
  steps = buildPhase010DataPortabilityCommandPlan(),
  manifest = defaultManifestInput(),
} = {}) {
  const durablePath = ".tasks/phase010-durable-authoring-gate-result.md";
  let durableAuthoringText;
  try {
    durableAuthoringText = await readFile(join(root, durablePath), "utf8");
  } catch (error) {
    return {
      passed: false,
      state: Phase010DataPortabilityState.Failed,
      errorCode: Phase010DataPortabilityErrorCode.IoFailed,
      findingId: error.path ?? durablePath,
      commandResults: {},
      manifest,
    };
  }

  if (!durableAuthoringText.includes("phase010_durable_authoring_gate=passed")) {
    return evaluatePhase010DataPortabilityGate({
      durableAuthoringText,
      commandResults: {},
      manifest,
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
    const partial = evaluatePhase010DataPortabilityGate({
      durableAuthoringText,
      commandResults,
      manifest,
    });
    if (!partial.passed && commandResults[gateStep.id].passed === false) {
      if (writeArtifacts) {
        await writeArtifactFiles(root, partial);
      }
      return partial;
    }
  }

  const result = evaluatePhase010DataPortabilityGate({
    durableAuthoringText,
    commandResults,
    manifest,
  });

  if (writeArtifacts) {
    await writeArtifactFiles(root, result);
  }

  return result;
}

export function renderPhase010DataPortabilityArtifact(result) {
  const passed = result.passed === true;
  const lines = [
    "# Phase 010 Data Portability Gate Result",
    "",
    passed ? "phase010_data_portability_gate=passed" : "phase010_data_portability_gate=failed",
    `validation_state=${result.state}`,
    `import_preview_no_mutation=${passed ? "verified" : "not_verified"}`,
    `restore_validation_required=${passed ? "verified" : "not_verified"}`,
    `data_portability_manifest=${passed ? "passed" : "not_written"}`,
    "",
    "- phase: `Phase 010.4`",
    "- gate: `Data Portability, Import, Export, Backup, and Restore Package Gate`",
    `- status: \`${passed ? "passed" : "failed"}\``,
    "- prerequisites:",
    "  - `.tasks/phase010-durable-authoring-gate-result.md` with `phase010_durable_authoring_gate=passed`",
    "- validation commands:",
    ...buildPhase010DataPortabilityCommandPlan().map(
      (gateStep) => `  - \`${gateStep.command.join(" ")}\``,
    ),
    "- changed layers: `usecase-validation`, `adapter-validation`, `ui-validation`, `desktop-app-validation`, `release-tooling`, `task-tooling`.",
    "- p95 300ms path impact: none directly. Data portability status reads remain covered by later index/settings release checks.",
    "- Product Log candidates: `export.package.created`, `backup.created`, `import.preview.completed`, `restore.validation.completed`, `restore.apply.completed`, `restore.apply.failed`.",
    "- Field Debug candidates: package hash, warning ids, count, state, stable error code, and duration bucket with explicit scope/expiry only.",
    "- data ownership evidence: export markdown, import markdown folder, backup job lifecycle, local backup store staging, import preview, and restore staging are validated by active local tests.",
    "- import evidence: preview model and desktop smoke expose markdown folder and Obsidian vault sources without raw local path exposure.",
    "- restore evidence: restore staging blocks apply before validation and confirmation.",
    "- sensitive-data exclusion: this artifact records command ids, counts, states, and stable error codes only. It excludes raw package contents, document body, asset bytes, authentication material, and personal absolute paths.",
    "- current scope: personal local desktop only. Server, SaaS, multi-user, collaboration, mobile, and remote workspace tests are not release prerequisites for this gate.",
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

export function renderPhase010DataPortabilityManifest(manifest) {
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

async function writeArtifactFiles(root, result) {
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "phase010-data-portability-gate-result.md"),
    renderPhase010DataPortabilityArtifact(result),
  );
  if (result.passed) {
    await writeFile(
      join(root, ".tasks", "release", "data-portability-manifest-phase010.json"),
      renderPhase010DataPortabilityManifest(
        buildPhase010DataPortabilityManifest(result.manifest),
      ),
    );
  }
}

function validateManifestSafety(manifest) {
  if (
    manifest.productScope !== "personal_local_desktop" ||
    manifest.workspaceScope !== "single_user_local_workspace" ||
    manifest.capabilities?.importPreviewNoMutation !== true ||
    manifest.capabilities?.restoreRequiresValidation !== true
  ) {
    return {
      passed: false,
      errorCode: Phase010DataPortabilityErrorCode.ManifestInvalid,
      findingId: "manifest_scope_or_capabilities",
    };
  }
  const manifestText = JSON.stringify(manifest);
  const found = unsafeManifestTerms.find((term) => manifestText.includes(term));
  if (found) {
    return {
      passed: false,
      errorCode: Phase010DataPortabilityErrorCode.ManifestUnsafeContent,
      findingId: found,
    };
  }
  return { passed: true };
}

function failedResult(state, commandResults, manifest) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    findingId: state.findingId,
    failedStepId: state.failedStepId,
    commandResults,
    manifest,
  };
}

function defaultManifestInput() {
  return {
    schemaVersion: "phase010.data_portability.v1",
    productScope: "personal_local_desktop",
    workspaceScope: "single_user_local_workspace",
    documentCount: 12,
    versionCount: 36,
    assetMetadataCount: 4,
    graphProjectionCount: 8,
    exportFormats: ["markdown_folder", "workspace_backup_package"],
    importSources: ["markdown_folder", "obsidian_vault"],
    capabilities: {
      exportPackage: true,
      importPreviewNoMutation: true,
      backupCreate: true,
      restoreRequiresValidation: true,
    },
    safeWarningIds: ["IMPORT_CONFLICT_DETECTED", "RESTORE_REQUIRES_CONFIRMATION"],
  };
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
  const result = await runPhase010DataPortabilityGate({
    root: process.cwd(),
    writeArtifacts: true,
  });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase010_data_portability_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
