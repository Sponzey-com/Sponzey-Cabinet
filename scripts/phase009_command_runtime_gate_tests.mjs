import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase009CommandRuntimeErrorCode,
  Phase009CommandRuntimeEvent,
  Phase009CommandRuntimeState,
  renderPhase009CommandRuntimeGateArtifact,
  runPhase009CommandRuntimeGate,
  transitionPhase009CommandRuntimeState,
  validatePhase009CommandRuntimeEvidence,
} from "./phase009_command_runtime_gate.mjs";

test("command runtime gate rejects missing desktop launch prerequisite marker", async () => {
  const root = await createCommandRuntimeFixtureRoot({
    desktopLaunchText: "phase009_desktop_launch_gate=failed\n",
  });

  const result = await runPhase009CommandRuntimeGate({ root, writeArtifact: false });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009CommandRuntimeErrorCode.DesktopLaunchMarkerMissing);
  assert.equal(result.findingId, ".tasks/phase009-desktop-launch-gate-result.md");
});

test("command runtime evidence validator rejects missing command coverage", () => {
  const evidence = completeEvidenceFixture();
  const result = validatePhase009CommandRuntimeEvidence({
    ...evidence,
    desktopLaunchText: "phase009_desktop_launch_gate=passed\n",
    platformRuntimeText: evidence.platformRuntimeText.replace("apply_restore", "applyrestore"),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, Phase009CommandRuntimeErrorCode.RequiredEvidenceMissing);
  assert.equal(result.findingId, "apply_restore");
});

test("command runtime gate passes fixture and renders safe artifact", async () => {
  const root = await createCommandRuntimeFixtureRoot();

  const result = await runPhase009CommandRuntimeGate({ root, writeArtifact: false });
  const artifact = renderPhase009CommandRuntimeGateArtifact(result);

  assert.equal(result.passed, true);
  assert.equal(result.state, Phase009CommandRuntimeState.Passed);
  assert.match(artifact, /phase009_command_runtime_gate=passed/);
  assert.match(artifact, /command_count=17/);
  assert.match(artifact, /LocalDesktopCommandState/);
  assert.match(artifact, /Product Log/);
  assert.match(artifact, /Field Debug Log/);
  assert.match(artifact, /Development Log/);
  assert.match(artifact, /sensitive data exclusion/);
  assert.doesNotMatch(artifact, /raw_document_body_fixture/);
  assert.doesNotMatch(artifact, /asset_content_fixture/);
  assert.doesNotMatch(artifact, /provider_api_key_fixture/);
  assert.doesNotMatch(artifact, /personal_absolute_path_fixture/);
});

test("command runtime gate writes marker artifact to explicit root", async () => {
  const root = await createCommandRuntimeFixtureRoot();

  const result = await runPhase009CommandRuntimeGate({ root, writeArtifact: true });
  const written = await readFile(join(root, ".tasks", "phase009-command-runtime-gate-result.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(written, /phase009_command_runtime_gate=passed/);
  assert.match(written, /validation_state=Passed/);
});

test("command runtime gate state machine exposes success, failure, and invalid transition", () => {
  const reading = transitionPhase009CommandRuntimeState(
    Phase009CommandRuntimeState.NotStarted,
    Phase009CommandRuntimeEvent.Start,
  );
  const validating = transitionPhase009CommandRuntimeState(
    reading.state,
    Phase009CommandRuntimeEvent.PrerequisitesRead,
  );
  const writing = transitionPhase009CommandRuntimeState(
    validating.state,
    Phase009CommandRuntimeEvent.EvidenceValidated,
  );
  const passed = transitionPhase009CommandRuntimeState(
    writing.state,
    Phase009CommandRuntimeEvent.ResultWritten,
  );
  const failed = transitionPhase009CommandRuntimeState(
    reading.state,
    Phase009CommandRuntimeEvent.Fail,
    { errorCode: Phase009CommandRuntimeErrorCode.DesktopLaunchMarkerMissing },
  );
  const invalid = transitionPhase009CommandRuntimeState(
    Phase009CommandRuntimeState.NotStarted,
    Phase009CommandRuntimeEvent.EvidenceValidated,
  );

  assert.equal(reading.state, Phase009CommandRuntimeState.ReadingPrerequisites);
  assert.equal(validating.state, Phase009CommandRuntimeState.ValidatingEvidence);
  assert.equal(writing.state, Phase009CommandRuntimeState.WritingResult);
  assert.equal(passed.state, Phase009CommandRuntimeState.Passed);
  assert.equal(failed.state, Phase009CommandRuntimeState.Failed);
  assert.equal(invalid.errorCode, Phase009CommandRuntimeErrorCode.InvalidTransition);
});

async function createCommandRuntimeFixtureRoot({
  desktopLaunchText = "phase009_desktop_launch_gate=passed\n",
  evidence = completeEvidenceFixture(),
} = {}) {
  const root = await mkdtemp(join(tmpdir(), "sponzey-phase009-command-runtime-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await mkdir(join(root, "packages", "client-core", "src"), { recursive: true });
  await mkdir(join(root, "packages", "client-core", "tests"), { recursive: true });
  await mkdir(join(root, "crates", "cabinet-platform", "src"), { recursive: true });
  await mkdir(join(root, "crates", "cabinet-platform", "tests"), { recursive: true });
  await mkdir(join(root, "apps", "desktop", "src-tauri", "src"), { recursive: true });

  await writeFile(join(root, ".tasks", "phase009-desktop-launch-gate-result.md"), desktopLaunchText);
  await writeFile(join(root, "packages", "client-core", "src", "index.ts"), evidence.clientCoreText);
  await writeFile(
    join(root, "packages", "client-core", "tests", "local_desktop_command_client_tests.ts"),
    evidence.clientCoreTestsText,
  );
  await writeFile(
    join(root, "crates", "cabinet-platform", "src", "local_desktop_runtime.rs"),
    evidence.platformRuntimeText,
  );
  await writeFile(
    join(root, "crates", "cabinet-platform", "tests", "local_desktop_command_runtime_tests.rs"),
    evidence.platformTestsText,
  );
  await writeFile(join(root, "apps", "desktop", "src-tauri", "src", "lib.rs"), evidence.desktopShellText);
  await writeFile(join(root, "apps", "desktop", "src-tauri", "src", "main.rs"), evidence.desktopShellMainText);
  return root;
}

function completeEvidenceFixture() {
  const commands = phase009CommandNames().join("\n");
  return {
    clientCoreText: [
      "export const PHASE009_LOCAL_DESKTOP_COMMAND_NAMES = [",
      commands,
      "] as const;",
      "createLocalDesktopCommandClient",
      "local_workspace_bootstrap",
      "update_current_document",
      "get_document_history",
      "list_document_assets",
    ].join("\n"),
    clientCoreTestsText: [
      "local desktop command registry matches the Phase 009 plan exactly",
      commands,
      "raw_document_body_fixture",
    ].join("\n"),
    platformRuntimeText: [
      "LocalDesktopCommandState",
      "Idle ValidatingInput ExecutingUsecase MappingResult Completed Failed",
      "LocalDesktopCommandPayload",
      "LocalDesktopUsecaseInput",
      "summarize_local_desktop_command_for_product_log",
      "asset_byte_len",
      commands,
    ].join("\n"),
    platformTestsText: [
      "local_desktop_command_mapper_covers_remaining_phase009_commands",
      "local_desktop_command_summary_hides_asset_import_and_restore_paths",
      commands,
      "asset_content_fixture",
    ].join("\n"),
    desktopShellText: [
      "DesktopLocalCommandRequestDto",
      "DesktopLocalCommandPayloadDto",
      "DesktopLocalCommandRuntimeResponse",
      "route_local_desktop_command_request",
      "COMMAND_INVALID_INPUT",
      "COMMAND_UNSUPPORTED",
      commands,
    ].join("\n"),
    desktopShellMainText: [
      "request: DesktopLocalCommandRequestDto",
      "route_local_desktop_command_request(request)",
      "provider_api_key_fixture",
    ].join("\n"),
  };
}

function phase009CommandNames() {
  return [
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
  ];
}
