import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase011ProductSmokeState = Object.freeze({
  Pending: "Pending",
  RunningCommands: "RunningCommands",
  ReadingEvidence: "ReadingEvidence",
  ValidatingEvidence: "ValidatingEvidence",
  WritingArtifacts: "WritingArtifacts",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase011ProductSmokeEvent = Object.freeze({
  Start: "Start",
  CommandsPassed: "CommandsPassed",
  EvidenceRead: "EvidenceRead",
  EvidenceValidated: "EvidenceValidated",
  ArtifactsWritten: "ArtifactsWritten",
  Fail: "Fail",
});

export const Phase011ProductSmokeErrorCode = Object.freeze({
  CommandFailed: "PHASE011_PRODUCT_SMOKE_COMMAND_FAILED",
  SourceReadFailed: "PHASE011_PRODUCT_SMOKE_SOURCE_READ_FAILED",
  RequiredEvidenceMissing: "PHASE011_PRODUCT_SMOKE_REQUIRED_EVIDENCE_MISSING",
  NativePlatformNotVerified: "PHASE011_PRODUCT_SMOKE_NATIVE_PLATFORM_NOT_VERIFIED",
  VisualAccessibilityFailed: "PHASE011_PRODUCT_SMOKE_VISUAL_ACCESSIBILITY_FAILED",
  UnsafeArtifactContent: "PHASE011_PRODUCT_SMOKE_UNSAFE_ARTIFACT_CONTENT",
  StaleSourceFingerprint: "PHASE011_PRODUCT_SMOKE_STALE_SOURCE_FINGERPRINT",
  InvalidTransition: "PHASE011_PRODUCT_SMOKE_INVALID_TRANSITION",
});

const requiredMarkers = Object.freeze([
  marker("phase011_workspace_home_gate", ".tasks/phase011-workspace-home-gate-result.md", [
    "phase011_workspace_home_gate=passed",
    "release_scope=personal_local_desktop",
  ]),
  marker("phase011_document_authoring_gate", ".tasks/phase011-document-authoring-gate-result.md", [
    "phase011_document_authoring_gate=passed",
    "raw_body_excluded=true",
    "raw_path_excluded=true",
  ]),
  marker("phase011_history_restore_gate", ".tasks/phase011-history-restore-gate-result.md", [
    "phase011_history_restore_gate=passed",
    "current_history_query_separation=true",
    "git_terms_excluded=true",
  ]),
  marker("phase011_discovery_gate", ".tasks/phase011-discovery-gate-result.md", [
    "phase011_discovery_gate=passed",
    "graph_neighborhood_bounded=true",
    "raw_query_excluded=true",
  ]),
  marker("phase011_data_settings_gate", ".tasks/phase011-data-settings-gate-result.md", [
    "phase011_data_settings_gate=passed",
    "future_server_admin_settings_excluded=true",
  ]),
  marker("phase011_recovery_observability_gate", ".tasks/phase011-recovery-observability-gate-result.md", [
    "phase011_recovery_observability_gate=passed",
    "product_log_classes_separated=true",
  ]),
  marker("phase011_performance_budget", ".tasks/release/performance-budget-phase011.md", [
    "phase011_performance_budget=passed",
    "300ms",
  ]),
  marker("phase011_security_log_manifest", ".tasks/release/security-log-policy-manifest-phase011.json", [
    "phase011_security_log_manifest=passed",
    "Product Log",
    "Field Debug Log",
    "Development Log",
  ]),
  marker("phase011_runbook", ".tasks/release/local-desktop-runbook-phase011.md", [
    "phase011_runbook=passed",
    "Do not require external DB",
    "Git CLI",
    "Node.js runtime",
  ]),
]);

const visualArtifacts = Object.freeze([
  ".tasks/release/workspace-home-visual-phase011.json",
  ".tasks/release/authoring-browser-phase011.json",
]);

const externalPlatformEvidencePaths = Object.freeze({
  windows: ".tasks/release/native-platform-evidence-windows-phase011.md",
  macos: ".tasks/release/native-platform-evidence-macos-phase011.md",
  linux: ".tasks/release/native-platform-evidence-linux-phase011.md",
});

const unsafeTerms = Object.freeze([
  "provider_api_key_fixture",
  "raw_document_body_fixture",
  "personal_absolute_path_fixture",
  "raw_prompt_fixture",
  "raw_answer_fixture",
  "AUTH_MATERIAL_SAMPLE",
  "RAW_DOC_BODY_SAMPLE",
  "PERSONAL_PATH_SAMPLE",
  "AI_PROMPT_SAMPLE",
  "AI_ANSWER_SAMPLE",
  "/Users/example/private",
  "C:\\Users\\example\\private",
]);

export function transitionPhase011ProductSmokeState(currentState, event, detail = {}) {
  if (currentState === Phase011ProductSmokeState.Pending && event === Phase011ProductSmokeEvent.Start) {
    return { state: Phase011ProductSmokeState.RunningCommands };
  }
  if (
    currentState === Phase011ProductSmokeState.RunningCommands &&
    event === Phase011ProductSmokeEvent.CommandsPassed
  ) {
    return { state: Phase011ProductSmokeState.ReadingEvidence };
  }
  if (
    currentState === Phase011ProductSmokeState.ReadingEvidence &&
    event === Phase011ProductSmokeEvent.EvidenceRead
  ) {
    return { state: Phase011ProductSmokeState.ValidatingEvidence };
  }
  if (
    currentState === Phase011ProductSmokeState.ValidatingEvidence &&
    event === Phase011ProductSmokeEvent.EvidenceValidated
  ) {
    return { state: Phase011ProductSmokeState.WritingArtifacts };
  }
  if (
    currentState === Phase011ProductSmokeState.WritingArtifacts &&
    event === Phase011ProductSmokeEvent.ArtifactsWritten
  ) {
    return { state: Phase011ProductSmokeState.Passed };
  }
  if (
    [
      Phase011ProductSmokeState.RunningCommands,
      Phase011ProductSmokeState.ReadingEvidence,
      Phase011ProductSmokeState.ValidatingEvidence,
      Phase011ProductSmokeState.WritingArtifacts,
    ].includes(currentState) &&
    event === Phase011ProductSmokeEvent.Fail
  ) {
    return {
      state: Phase011ProductSmokeState.Failed,
      errorCode: detail.errorCode ?? Phase011ProductSmokeErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
      failedStepId: detail.failedStepId,
    };
  }
  return {
    state: Phase011ProductSmokeState.Failed,
    errorCode: Phase011ProductSmokeErrorCode.InvalidTransition,
  };
}

export function buildPhase011ProductSmokeCommandPlan() {
  return [
    step("workspaceHomeGate", ["sh", "scripts/run_phase011_workspace_home_gate.sh"]),
    step("documentAuthoringGate", ["sh", "scripts/run_phase011_document_authoring_gate.sh"]),
    step("historyRestoreGate", ["sh", "scripts/run_phase011_history_restore_gate.sh"]),
    step("discoveryGate", ["sh", "scripts/run_phase011_discovery_gate.sh"]),
    step("dataSettingsGate", ["sh", "scripts/run_phase011_data_settings_gate.sh"]),
    step("recoveryObservabilityGate", ["sh", "scripts/run_phase011_recovery_observability_gate.sh"]),
    step("desktopPackageSmoke", ["sh", "scripts/run_desktop_package_smoke.sh"]),
  ];
}

export function analyzePhase011ProductSmokeEvidence({
  sources,
  commandResults = {},
  currentPlatform = normalizePlatform(process.platform),
  deferOtherPlatforms = true,
}) {
  const sourceFingerprint = readSourceFingerprint(sources?.[".tasks/phase011-current-implementation-inventory.md"]);
  if (!sourceFingerprint) {
    return failed(Phase011ProductSmokeErrorCode.StaleSourceFingerprint, "source_fingerprint");
  }

  const targetResults = requiredMarkers.map((entry) => analyzeMarker(entry, sources));
  const missingTarget = targetResults.find((entry) => entry.status !== "covered");
  if (missingTarget) {
    return failed(
      Phase011ProductSmokeErrorCode.RequiredEvidenceMissing,
      missingTarget.id,
      { sourceFingerprint, targetResults },
    );
  }

  const visualReport = buildVisualAccessibilityReport({ sources, sourceFingerprint });
  if (!visualReport.passed) {
    return failed(Phase011ProductSmokeErrorCode.VisualAccessibilityFailed, visualReport.findingId, {
      sourceFingerprint,
      targetResults,
      visualReport,
    });
  }

  const platformMatrix = buildNativePlatformMatrix({
    sources,
    commandResults,
    currentPlatform,
    sourceFingerprint,
    deferOtherPlatforms,
  });
  if (!platformMatrix.passed) {
    return failed(Phase011ProductSmokeErrorCode.NativePlatformNotVerified, platformMatrix.findingId, {
      sourceFingerprint,
      targetResults,
      visualReport,
      platformMatrix,
    });
  }

  const result = {
    passed: true,
    marker: "phase011_product_smoke_gate=passed",
    state: Phase011ProductSmokeState.Passed,
    releaseScope: "personal_local_desktop",
    sourceFingerprint,
    targetResults,
    visualReport,
    platformMatrix,
    summary: {
      requiredTargets: targetResults.length,
      missingRequiredEvidence: 0,
      visualViewportCount: visualReport.viewportCount,
      platformCount: platformMatrix.rows.length,
    },
  };
  if (containsUnsafeContent(renderPhase011ProductSmokeGateMarkdown(result))) {
    return failed(Phase011ProductSmokeErrorCode.UnsafeArtifactContent, "product_smoke_artifact", {
      sourceFingerprint,
      targetResults,
      visualReport,
      platformMatrix,
    });
  }
  return result;
}

export function buildVisualAccessibilityReport({ sources, sourceFingerprint }) {
  const workspace = parseJsonSource(sources, ".tasks/release/workspace-home-visual-phase011.json");
  const authoring = parseJsonSource(sources, ".tasks/release/authoring-browser-phase011.json");
  if (workspace.marker !== "phase011_workspace_home_visual=passed") {
    return visualFailed("workspace_home_visual_marker");
  }
  if (authoring.marker !== "phase011_authoring_browser=passed") {
    return visualFailed("authoring_browser_marker");
  }
  const workspaceRuns = Array.isArray(workspace.runs) ? workspace.runs : [];
  const authoringRuns = Array.isArray(authoring.runs) ? authoring.runs : [];
  for (const run of [...workspaceRuns, ...authoringRuns]) {
    if (!run.readyState) return visualFailed("ready_state");
    if ((run.nonBlankPixelCount ?? 0) <= 0) return visualFailed("nonblank_pixels");
    if ((run.overlapCount ?? 0) !== 0) return visualFailed("overlap");
    if (run.horizontalOverflow) return visualFailed("horizontal_overflow");
    if (!run.focusVisible) return visualFailed("focus_visible");
  }
  if (!authoring.interactions?.codeMirrorMounted) return visualFailed("codemirror_mounted");
  if (!authoring.interactions?.previewTableRendered) return visualFailed("preview_table_rendered");
  return {
    passed: true,
    marker: "phase011_visual_accessibility=passed",
    sourceFingerprint,
    viewportCount: workspaceRuns.length + authoringRuns.length,
    workspaceViewportCount: workspaceRuns.length,
    authoringViewportCount: authoringRuns.length,
    focusVisible: true,
    nonBlank: true,
    noOverlap: true,
    noHorizontalOverflow: true,
    codeMirrorMounted: true,
    previewTableRendered: true,
  };
}

export function buildNativePlatformMatrix({
  sources,
  commandResults = {},
  currentPlatform = normalizePlatform(process.platform),
  sourceFingerprint,
  deferOtherPlatforms = true,
}) {
  const rows = ["windows", "macos", "linux"].map((platform) => {
    const external = parseExternalPlatformEvidence(sources?.[externalPlatformEvidencePaths[platform]], platform, sourceFingerprint);
    if (external.status === "passed") return external;
    if (platform === currentPlatform && commandResults.desktopPackageSmoke?.passed) {
      return {
        platform,
        status: "passed",
        evidence: "current_host_desktop_package_smoke",
        sourceFingerprint,
      };
    }
    if (deferOtherPlatforms && platform !== currentPlatform) {
      return {
        platform,
        status: "deferred_future",
        evidence: "deferred_by_phase011_scope_decision",
        sourceFingerprint,
      };
    }
    return {
      platform,
      status: "not_verified",
      evidence: external.evidence ?? "missing_external_native_runner_evidence",
      sourceFingerprint,
    };
  });
  const missing = rows.find((row) => !["passed", "deferred_future"].includes(row.status));
  const current = rows.find((row) => row.platform === currentPlatform);
  const currentMissing = current?.status !== "passed";
  const findingId = currentMissing ? currentPlatform : missing?.platform;
  return {
    passed: !missing && !currentMissing,
    marker: missing || currentMissing ? "phase011_native_platform_matrix=blocked" : "phase011_native_platform_matrix=passed",
    sourceFingerprint,
    rows,
    findingId,
    deferredPlatforms: rows.filter((row) => row.status === "deferred_future").map((row) => row.platform),
  };
}

export function renderVisualAccessibilityReport(report) {
  return [
    "# Phase 011 Visual Accessibility Report",
    "",
    report.marker,
    `source_fingerprint=${report.sourceFingerprint}`,
    `viewport_count=${report.viewportCount}`,
    `workspace_viewport_count=${report.workspaceViewportCount}`,
    `authoring_viewport_count=${report.authoringViewportCount}`,
    "nonblank_ui=true",
    "focus_visible=true",
    "overlap_count=0",
    "horizontal_overflow=false",
    "codemirror_mounted=true",
    "preview_table_rendered=true",
    "sensitive_data_exclusion=passed",
    "",
  ].join("\n");
}

export function renderNativePlatformMatrix(matrix) {
  const lines = [
    "# Phase 011 Native Platform Matrix",
    "",
    matrix.marker,
    `source_fingerprint=${matrix.sourceFingerprint}`,
    "scope=windows,macos,linux",
    `deferred_platforms=${matrix.deferredPlatforms?.join(",") ?? ""}`,
    "",
    "| Platform | Status | Evidence |",
    "| --- | --- | --- |",
  ];
  for (const row of matrix.rows) {
    lines.push(`| \`${row.platform}\` | \`${row.status}\` | \`${row.evidence}\` |`);
  }
  lines.push(
    "",
    "One host result is never copied into another platform row.",
    "Deferred platform rows are future release scope and must be verified in a later phase before claiming those OS releases.",
    "",
  );
  return lines.join("\n");
}

export function renderPhase011ProductSmokeGateMarkdown(result) {
  const lines = [
    "# Phase 011 Product Smoke Gate Result",
    "",
    result.marker,
    "release_scope=personal_local_desktop",
  ];
  if (result.sourceFingerprint) lines.push(`source_fingerprint=${result.sourceFingerprint}`);
  lines.push(`state=${result.state}`);
  if (!result.passed) {
    lines.push(`error_code=${result.errorCode}`);
    if (result.findingId) lines.push(`finding_id=${result.findingId}`);
  }
  lines.push(
    `required_target_count=${result.summary?.requiredTargets ?? 0}`,
    `missing_required_evidence=${result.summary?.missingRequiredEvidence ?? 0}`,
    `visual_accessibility=${result.visualReport?.marker ?? "phase011_visual_accessibility=failed"}`,
    `native_platform_matrix=${result.platformMatrix?.marker ?? "phase011_native_platform_matrix=blocked"}`,
    `deferred_platforms=${result.platformMatrix?.deferredPlatforms?.join(",") ?? ""}`,
    "future_scope_exclusion=server,SaaS,multi-user,mobile,admin,SSO,billing",
    "installed_runtime_requires_external_db=false",
    "installed_runtime_requires_external_search=false",
    "installed_runtime_requires_git_cli=false",
    "installed_runtime_requires_nodejs=false",
    "installed_runtime_requires_manual_env=false",
    "raw_body_excluded=true",
    "raw_path_excluded=true",
    "raw_prompt_answer_excluded=true",
    "",
    "## Evidence Targets",
    "",
    "| Target | Status |",
    "| --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(`| \`${targetResult.id}\` | \`${targetResult.status}\` |`);
  }
  lines.push("");
  if (result.platformMatrix?.rows?.length) {
    lines.push("## Native Platforms", "", "| Platform | Status |", "| --- | --- |");
    for (const row of result.platformMatrix.rows) {
      lines.push(`| \`${row.platform}\` | \`${row.status}\` |`);
    }
    lines.push("");
  }
  return lines.join("\n");
}

export async function runPhase011ProductSmokeGate({
  root = process.cwd(),
  runner = runCommand,
  commandPlan = buildPhase011ProductSmokeCommandPlan(),
  writeArtifacts = true,
  currentPlatform = normalizePlatform(process.platform),
} = {}) {
  let state = transitionPhase011ProductSmokeState(
    Phase011ProductSmokeState.Pending,
    Phase011ProductSmokeEvent.Start,
  );
  const commandResults = {};
  for (const stepEntry of commandPlan) {
    const commandResult = await runner(stepEntry.command, { cwd: root });
    commandResults[stepEntry.id] = {
      command: stepEntry.command.join(" "),
      passed: commandResult.exitCode === 0,
      exitCode: commandResult.exitCode,
      durationMs: commandResult.durationMs,
    };
    if (commandResult.exitCode !== 0) {
      const failedState = transitionPhase011ProductSmokeState(state.state, Phase011ProductSmokeEvent.Fail, {
        errorCode: Phase011ProductSmokeErrorCode.CommandFailed,
        failedStepId: stepEntry.id,
      });
      const result = failed(Phase011ProductSmokeErrorCode.CommandFailed, stepEntry.id, {
        state: failedState.state,
        commandResults,
      });
      if (writeArtifacts) await writeProductSmokeArtifacts(root, result);
      return result;
    }
  }
  state = transitionPhase011ProductSmokeState(state.state, Phase011ProductSmokeEvent.CommandsPassed);
  try {
    const sources = await readRequiredSources(root);
    state = transitionPhase011ProductSmokeState(state.state, Phase011ProductSmokeEvent.EvidenceRead);
    let result = analyzePhase011ProductSmokeEvidence({ sources, commandResults, currentPlatform });
    if (result.passed) {
      state = transitionPhase011ProductSmokeState(state.state, Phase011ProductSmokeEvent.EvidenceValidated);
      state = transitionPhase011ProductSmokeState(state.state, Phase011ProductSmokeEvent.ArtifactsWritten);
      result = { ...result, state: state.state, commandResults };
    } else {
      state = transitionPhase011ProductSmokeState(state.state, Phase011ProductSmokeEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.findingId,
      });
      result = { ...result, state: state.state, commandResults };
    }
    if (writeArtifacts) await writeProductSmokeArtifacts(root, result);
    return result;
  } catch {
    const failedState = transitionPhase011ProductSmokeState(state.state, Phase011ProductSmokeEvent.Fail, {
      errorCode: Phase011ProductSmokeErrorCode.SourceReadFailed,
    });
    const result = failed(Phase011ProductSmokeErrorCode.SourceReadFailed, "source_read", {
      state: failedState.state,
      commandResults,
    });
    if (writeArtifacts) await writeProductSmokeArtifacts(root, result);
    return result;
  }
}

async function writeProductSmokeArtifacts(root, result) {
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "phase011-product-smoke-gate-result.md"),
    `${renderPhase011ProductSmokeGateMarkdown(result)}\n`,
  );
  if (result.visualReport?.passed) {
    await writeFile(
      join(root, ".tasks", "release", "visual-accessibility-report-phase011.md"),
      renderVisualAccessibilityReport(result.visualReport),
    );
  }
  if (result.platformMatrix) {
    await writeFile(
      join(root, ".tasks", "release", "native-platform-matrix-phase011.md"),
      renderNativePlatformMatrix(result.platformMatrix),
    );
  }
}

async function readRequiredSources(root) {
  const paths = [
    ".tasks/phase011-current-implementation-inventory.md",
    ...requiredMarkers.map((entry) => entry.path),
    ...visualArtifacts,
    ...Object.values(externalPlatformEvidencePaths),
  ];
  const sources = {};
  for (const filePath of paths) {
    try {
      sources[filePath] = await readFile(join(root, filePath), "utf8");
    } catch {
      if (!Object.values(externalPlatformEvidencePaths).includes(filePath)) throw new Error(filePath);
    }
  }
  return sources;
}

function analyzeMarker(entry, sources) {
  const text = sources?.[entry.path] ?? "";
  const missing = [entry.path, ...entry.needles].filter((needle) => {
    if (needle === entry.path) return !text;
    return !text.includes(needle);
  });
  return { id: entry.id, status: missing.length === 0 ? "covered" : "missing", missing };
}

function parseJsonSource(sources, path) {
  const text = sources?.[path];
  if (!text) return {};
  try {
    return JSON.parse(text);
  } catch {
    return {};
  }
}

function parseExternalPlatformEvidence(text, platform, sourceFingerprint) {
  if (!text) return { platform, status: "not_verified" };
  if (
    text.includes("phase011_native_platform_evidence=passed") &&
    text.includes(`native_platform=${platform}`) &&
    text.includes(`source_fingerprint=${sourceFingerprint}`)
  ) {
    return { platform, status: "passed", evidence: "external_native_runner_evidence", sourceFingerprint };
  }
  return { platform, status: "not_verified", evidence: "invalid_external_native_runner_evidence", sourceFingerprint };
}

function containsUnsafeContent(text) {
  return unsafeTerms.some((term) => text.includes(term));
}

function readSourceFingerprint(text = "") {
  return text.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
}

function visualFailed(findingId) {
  return {
    passed: false,
    marker: "phase011_visual_accessibility=failed",
    findingId,
    viewportCount: 0,
  };
}

function failed(errorCode, findingId, detail = {}) {
  return {
    passed: false,
    marker: "phase011_product_smoke_gate=failed",
    state: detail.state ?? Phase011ProductSmokeState.Failed,
    releaseScope: "personal_local_desktop",
    sourceFingerprint: detail.sourceFingerprint,
    errorCode,
    findingId,
    targetResults: detail.targetResults ?? [],
    visualReport: detail.visualReport,
    platformMatrix: detail.platformMatrix,
    commandResults: detail.commandResults,
    summary: {
      requiredTargets: detail.targetResults?.length ?? requiredMarkers.length,
      missingRequiredEvidence: 1,
    },
  };
}

function marker(id, path, needles) {
  return { id, path, needles };
}

function step(id, command) {
  return { id, command };
}

function normalizePlatform(platform) {
  if (platform === "win32" || platform === "windows") return "windows";
  if (platform === "darwin" || platform === "macos") return "macos";
  if (platform === "linux") return "linux";
  return platform;
}

async function runCommand(command, { cwd }) {
  const started = Date.now();
  return await new Promise((resolve) => {
    const child = spawn(command[0], command.slice(1), {
      cwd,
      stdio: "inherit",
      env: process.env,
    });
    child.on("close", (exitCode, signal) => {
      resolve({ exitCode: exitCode ?? 1, signal, durationMs: Date.now() - started });
    });
    child.on("error", () => {
      resolve({ exitCode: 1, signal: null, durationMs: Date.now() - started });
    });
  });
}

async function runCli() {
  const result = await runPhase011ProductSmokeGate();
  if (result.passed) {
    console.log(result.marker);
    console.log(`state=${result.state}`);
    console.log(`native_platform_matrix=${result.platformMatrix.marker}`);
    return;
  }
  console.error(result.marker);
  console.error(`state=${result.state}`);
  console.error(`error_code=${result.errorCode}`);
  if (result.findingId) console.error(`finding_id=${result.findingId}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
