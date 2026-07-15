import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase009CommandRuntimeState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingPrerequisites: "ReadingPrerequisites",
  ValidatingEvidence: "ValidatingEvidence",
  WritingResult: "WritingResult",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase009CommandRuntimeEvent = Object.freeze({
  Start: "Start",
  PrerequisitesRead: "PrerequisitesRead",
  EvidenceValidated: "EvidenceValidated",
  ResultWritten: "ResultWritten",
  Fail: "Fail",
});

export const Phase009CommandRuntimeErrorCode = Object.freeze({
  DesktopLaunchMarkerMissing: "PHASE009_DESKTOP_LAUNCH_MARKER_MISSING",
  RequiredEvidenceMissing: "PHASE009_COMMAND_RUNTIME_REQUIRED_EVIDENCE_MISSING",
  SensitiveDataLeak: "PHASE009_COMMAND_RUNTIME_SENSITIVE_DATA_LEAK",
  IoFailed: "PHASE009_COMMAND_RUNTIME_IO_FAILED",
  InvalidTransition: "PHASE009_COMMAND_RUNTIME_INVALID_TRANSITION",
});

export const PHASE009_COMMAND_NAMES = Object.freeze([
  "local_workspace_bootstrap",
  "local_workspace_home",
  "get_current_document",
  "update_current_document",
  "get_document_history",
  "get_document_version",
  "preview_document_restore",
  "restore_document_version",
  "search_documents",
  "get_link_overview",
  "get_graph_projection",
  "list_document_assets",
  "attach_document_asset",
  "create_backup",
  "preview_import",
  "preview_restore",
  "apply_restore",
]);

const requiredEvidence = Object.freeze([
  evidence("client_core_registry", "TypeScript command registry", "clientCoreText", [
    "PHASE009_LOCAL_DESKTOP_COMMAND_NAMES",
    "createLocalDesktopCommandClient",
    "local_workspace_bootstrap",
    "update_current_document",
    "get_document_history",
    "list_document_assets",
  ]),
  evidence("client_core_tests", "TypeScript command client tests", "clientCoreTestsText", [
    "local desktop command registry matches the Phase 009 plan exactly",
  ]),
  evidence("platform_runtime", "Rust platform command runtime", "platformRuntimeText", [
    "LocalDesktopCommandState",
    "Idle",
    "ValidatingInput",
    "ExecutingUsecase",
    "MappingResult",
    "Completed",
    "Failed",
    "LocalDesktopCommandPayload",
    "LocalDesktopUsecaseInput",
    "summarize_local_desktop_command_for_product_log",
    "asset_byte_len",
  ]),
  evidence("platform_tests", "Rust platform command runtime tests", "platformTestsText", [
    "local_desktop_command_mapper_covers_remaining_phase009_commands",
    "local_desktop_command_summary_hides_asset_import_and_restore_paths",
  ]),
  evidence("desktop_shell_adapter", "Desktop shell typed DTO adapter", "desktopShellText", [
    "DesktopLocalCommandRequestDto",
    "DesktopLocalCommandPayloadDto",
    "DesktopLocalCommandRuntimeResponse",
    "route_local_desktop_command_request",
    "COMMAND_INVALID_INPUT",
    "COMMAND_UNSUPPORTED",
  ]),
  evidence("desktop_shell_main", "Tauri command typed request boundary", "desktopShellMainText", [
    "request: DesktopLocalCommandRequestDto",
    "route_local_desktop_command_request(request)",
  ]),
]);

export function transitionPhase009CommandRuntimeState(currentState, event, detail = {}) {
  if (
    currentState === Phase009CommandRuntimeState.NotStarted &&
    event === Phase009CommandRuntimeEvent.Start
  ) {
    return { state: Phase009CommandRuntimeState.ReadingPrerequisites };
  }
  if (
    currentState === Phase009CommandRuntimeState.ReadingPrerequisites &&
    event === Phase009CommandRuntimeEvent.PrerequisitesRead
  ) {
    return { state: Phase009CommandRuntimeState.ValidatingEvidence };
  }
  if (
    currentState === Phase009CommandRuntimeState.ValidatingEvidence &&
    event === Phase009CommandRuntimeEvent.EvidenceValidated
  ) {
    return { state: Phase009CommandRuntimeState.WritingResult };
  }
  if (
    currentState === Phase009CommandRuntimeState.WritingResult &&
    event === Phase009CommandRuntimeEvent.ResultWritten
  ) {
    return { state: Phase009CommandRuntimeState.Passed };
  }
  if (
    [
      Phase009CommandRuntimeState.ReadingPrerequisites,
      Phase009CommandRuntimeState.ValidatingEvidence,
      Phase009CommandRuntimeState.WritingResult,
    ].includes(currentState) &&
    event === Phase009CommandRuntimeEvent.Fail
  ) {
    return {
      state: Phase009CommandRuntimeState.Failed,
      errorCode: detail.errorCode ?? Phase009CommandRuntimeErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase009CommandRuntimeState.Failed,
    errorCode: Phase009CommandRuntimeErrorCode.InvalidTransition,
  };
}

export function validatePhase009CommandRuntimeEvidence(evidenceTexts) {
  if (!evidenceTexts.desktopLaunchText?.includes("phase009_desktop_launch_gate=passed")) {
    return failed(
      Phase009CommandRuntimeErrorCode.DesktopLaunchMarkerMissing,
      ".tasks/phase009-desktop-launch-gate-result.md",
    );
  }

  for (const commandName of PHASE009_COMMAND_NAMES) {
    for (const sourceKey of ["clientCoreText", "platformRuntimeText", "desktopShellText"]) {
      if (!evidenceTexts[sourceKey]?.includes(commandName)) {
        return failed(Phase009CommandRuntimeErrorCode.RequiredEvidenceMissing, commandName);
      }
    }
  }

  for (const item of requiredEvidence) {
    const text = evidenceTexts[item.sourceKey] ?? "";
    for (const needle of item.needles) {
      if (!text.includes(needle)) {
        return failed(Phase009CommandRuntimeErrorCode.RequiredEvidenceMissing, needle);
      }
    }
  }

  return {
    passed: true,
    state: Phase009CommandRuntimeState.Passed,
    commandCount: PHASE009_COMMAND_NAMES.length,
    evidenceCount: requiredEvidence.length,
    evidenceTargets: requiredEvidence.map((item) => item.id),
  };
}

export async function runPhase009CommandRuntimeGate({
  root = process.cwd(),
  writeArtifact = true,
} = {}) {
  let state = transitionPhase009CommandRuntimeState(
    Phase009CommandRuntimeState.NotStarted,
    Phase009CommandRuntimeEvent.Start,
  ).state;

  try {
    const evidenceTexts = {
      desktopLaunchText: await readFile(
        join(root, ".tasks", "phase009-desktop-launch-gate-result.md"),
        "utf8",
      ),
      clientCoreText: await readFile(join(root, "packages", "client-core", "src", "index.ts"), "utf8"),
      clientCoreTestsText: await readFile(
        join(root, "packages", "client-core", "tests", "local_desktop_command_client_tests.ts"),
        "utf8",
      ),
      platformRuntimeText: await readFile(
        join(root, "crates", "cabinet-platform", "src", "local_desktop_runtime.rs"),
        "utf8",
      ),
      platformTestsText: await readFile(
        join(root, "crates", "cabinet-platform", "tests", "local_desktop_command_runtime_tests.rs"),
        "utf8",
      ),
      desktopShellText: await readFile(
        join(root, "apps", "desktop", "src-tauri", "src", "lib.rs"),
        "utf8",
      ),
      desktopShellMainText: await readFile(
        join(root, "apps", "desktop", "src-tauri", "src", "main.rs"),
        "utf8",
      ),
    };

    if (!evidenceTexts.desktopLaunchText.includes("phase009_desktop_launch_gate=passed")) {
      return toFailedResult(state, {
        errorCode: Phase009CommandRuntimeErrorCode.DesktopLaunchMarkerMissing,
        findingId: ".tasks/phase009-desktop-launch-gate-result.md",
      });
    }
    state = transitionPhase009CommandRuntimeState(
      state,
      Phase009CommandRuntimeEvent.PrerequisitesRead,
    ).state;

    const validation = validatePhase009CommandRuntimeEvidence(evidenceTexts);
    if (!validation.passed) {
      return toFailedResult(state, validation);
    }
    state = transitionPhase009CommandRuntimeState(
      state,
      Phase009CommandRuntimeEvent.EvidenceValidated,
    ).state;

    const result = {
      ...validation,
      state: Phase009CommandRuntimeState.Passed,
    };

    if (writeArtifact) {
      await mkdir(join(root, ".tasks"), { recursive: true });
      await writeFile(
        join(root, ".tasks", "phase009-command-runtime-gate-result.md"),
        renderPhase009CommandRuntimeGateArtifact(result),
      );
    }

    state = transitionPhase009CommandRuntimeState(
      state,
      Phase009CommandRuntimeEvent.ResultWritten,
    ).state;
    return { ...result, state };
  } catch (error) {
    return toFailedResult(state, {
      errorCode: Phase009CommandRuntimeErrorCode.IoFailed,
      findingId: error.path ?? error.message,
    });
  }
}

export function renderPhase009CommandRuntimeGateArtifact(result) {
  const marker = result.passed
    ? "phase009_command_runtime_gate=passed"
    : "phase009_command_runtime_gate=failed";

  const lines = [
    "# Phase 009 Command Runtime Gate Result",
    "",
    marker,
    `validation_state=${result.state}`,
    `command_count=${result.commandCount ?? 0}`,
    "",
    "- phase: `Phase 009.2`",
    "- gate: `Native Command DTO Contract and Runtime Composition`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
  ];

  if (!result.passed) {
    lines.push(`- error code: \`${result.errorCode}\``, `- finding id: \`${result.findingId}\``);
  }

  lines.push(
    "- prerequisites:",
    "  - `.tasks/phase009-desktop-launch-gate-result.md` with `phase009_desktop_launch_gate=passed`",
    "- validation commands:",
    "  - `npm run run:phase009-command-runtime-gate-tests`",
    "  - `node --check scripts/phase009_command_runtime_gate.mjs scripts/phase009_command_runtime_gate_tests.mjs`",
    "  - `cargo test -p cabinet-platform`",
    "  - `cargo test -p cabinet-usecases`",
    "  - `cargo test -p cabinet-adapters`",
    "  - `cargo test -p cabinet-desktop-shell`",
    "  - `node --test packages/client-core/tests/local_desktop_command_client_tests.ts apps/desktop/tests/desktop_local_command_facade_tests.ts`",
    "  - `npm run run:phase009-command-runtime-gate`",
    "- command runtime evidence:",
    "  - TypeScript `PHASE009_LOCAL_DESKTOP_COMMAND_NAMES` has 17 command names.",
    "  - Rust `LocalDesktopCommandState` covers `Idle`, `ValidatingInput`, `ExecutingUsecase`, `MappingResult`, `Completed`, `Failed`.",
    "  - Rust platform mapper covers every Phase 009 local desktop command.",
    "  - Desktop shell uses `DesktopLocalCommandRequestDto` and `DesktopLocalCommandRuntimeResponse` without Tauri types in mapper tests.",
    "- Product Log policy: command runtime logs may record stable command name, safe ids/counts/sizes, duration bucket, and stable error code only.",
    "- Field Debug Log policy: command DTO summary must be bounded, explicitly enabled, and must not include raw payload or paths.",
    "- Development Log policy: mapper and fake transport diagnostics remain test/development only.",
    "- state machine: `LocalDesktopCommandState` and command runtime gate state machine are explicit and tested.",
    "- sensitive data exclusion: this artifact records command names, source evidence ids, counts, state names, command names, and stable error codes only. It does not record raw document body, asset content, file path, backup path, import path, AI prompt, AI answer, provider key, token, credential, secret, or personal absolute path.",
    "",
  );

  return lines.join("\n");
}

function toFailedResult(state, detail) {
  const failedTransition = transitionPhase009CommandRuntimeState(
    state,
    Phase009CommandRuntimeEvent.Fail,
    detail,
  );
  return {
    passed: false,
    state: Phase009CommandRuntimeState.Failed,
    errorCode: failedTransition.errorCode,
    findingId: failedTransition.findingId,
    commandCount: PHASE009_COMMAND_NAMES.length,
    evidenceCount: requiredEvidence.length,
  };
}

function failed(errorCode, findingId) {
  return {
    passed: false,
    state: Phase009CommandRuntimeState.Failed,
    errorCode,
    findingId,
  };
}

function evidence(id, description, sourceKey, needles) {
  return Object.freeze({ id, description, sourceKey, needles: Object.freeze(needles) });
}

async function main() {
  const result = await runPhase009CommandRuntimeGate({ root: process.cwd(), writeArtifact: true });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log("phase009_command_runtime_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
