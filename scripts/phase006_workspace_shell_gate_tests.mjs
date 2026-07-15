import assert from "node:assert/strict";
import test from "node:test";

import {
  WorkspaceShellGateErrorCode,
  WorkspaceShellGateEvent,
  WorkspaceShellGateState,
  analyzeWorkspaceShellEvidence,
  renderWorkspaceShellGateMarkdown,
  transitionWorkspaceShellGateState,
} from "./phase006_workspace_shell_gate.mjs";

test("workspace shell gate reports complete personal desktop evidence as passed", () => {
  const result = analyzeWorkspaceShellEvidence({
    sources: completeSources(),
  });
  const markdown = renderWorkspaceShellGateMarkdown(result);

  assert.equal(result.passed, true);
  assert.equal(result.marker, "phase006_workspace_shell_gate=passed");
  assert.equal(result.summary.missingRequiredEvidence, 0);
  assert.match(markdown, /phase006_workspace_shell_gate=passed/);
  assert.match(markdown, /personal local desktop shell/);
  assert.match(markdown, /server\/admin action absence/);
  assert.doesNotMatch(markdown, /phase006-raw-document-body-should-not-log/);
  assert.doesNotMatch(markdown, /provider_api_key_fixture/);
});

test("workspace shell gate fails when local runtime prerequisite is missing", () => {
  const sources = completeSources();
  sources[".tasks/phase006-local-runtime-gate-result.md"] =
    "phase006_local_runtime_gate=failed";

  const result = analyzeWorkspaceShellEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, WorkspaceShellGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "local_runtime_prerequisite");
  assert.equal(result.marker, "phase006_workspace_shell_gate=failed");
});

test("workspace shell gate fails when forbidden product action appears", () => {
  const sources = completeSources();
  sources["apps/desktop/src/index.ts"] += "\nconst forbidden = 'team-invite';\n";

  const result = analyzeWorkspaceShellEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, WorkspaceShellGateErrorCode.ForbiddenProductText);
  assert.equal(result.missingEvidence[0].targetId, "desktop_current_product_shell");
});

test("workspace shell gate fails when desktop entry does not use personal shell", () => {
  const sources = completeSources();
  sources["apps/desktop/src/index.ts"] = sources["apps/desktop/src/index.ts"].replace(
    "export function createDesktopCurrentProductShell(",
    "export function createDesktopGenericShell(",
  );

  const result = analyzeWorkspaceShellEvidence({ sources });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, WorkspaceShellGateErrorCode.RequiredEvidenceMissing);
  assert.equal(result.missingEvidence[0].targetId, "desktop_current_product_shell");
});

test("workspace shell gate state machine exposes explicit transitions and invalid transition", () => {
  const readingSources = transitionWorkspaceShellGateState(
    WorkspaceShellGateState.Pending,
    WorkspaceShellGateEvent.Start,
  );
  const validatingEvidence = transitionWorkspaceShellGateState(
    readingSources.state,
    WorkspaceShellGateEvent.SourcesLoaded,
  );
  const writingReport = transitionWorkspaceShellGateState(
    validatingEvidence.state,
    WorkspaceShellGateEvent.EvidenceValidated,
  );
  const passed = transitionWorkspaceShellGateState(
    writingReport.state,
    WorkspaceShellGateEvent.ReportWritten,
  );
  const failed = transitionWorkspaceShellGateState(validatingEvidence.state, WorkspaceShellGateEvent.Fail, {
    errorCode: WorkspaceShellGateErrorCode.RequiredEvidenceMissing,
    findingId: "desktop_current_product_shell",
  });
  const invalid = transitionWorkspaceShellGateState(
    WorkspaceShellGateState.Pending,
    WorkspaceShellGateEvent.ReportWritten,
  );

  assert.equal(readingSources.state, WorkspaceShellGateState.ReadingSources);
  assert.equal(validatingEvidence.state, WorkspaceShellGateState.ValidatingEvidence);
  assert.equal(writingReport.state, WorkspaceShellGateState.WritingReport);
  assert.equal(passed.state, WorkspaceShellGateState.Passed);
  assert.equal(failed.state, WorkspaceShellGateState.Failed);
  assert.equal(failed.findingId, "desktop_current_product_shell");
  assert.equal(invalid.errorCode, WorkspaceShellGateErrorCode.InvalidTransition);
});

function completeSources() {
  return {
    ".tasks/phase006-local-runtime-gate-result.md": "phase006_local_runtime_gate=passed",
    "packages/client-core/src/index.ts": [
      "PersonalLocalDesktopCapabilityProfile",
      "createPersonalLocalDesktopCapabilityProfile",
      "personal_local_desktop",
      "supportsRemoteWorkspace: false",
      "isForbiddenPersonalLocalDesktopAction",
    ].join("\n"),
    "packages/client-core/tests/personal_local_desktop_capability_tests.ts": [
      "personal local desktop profile exposes local-first actions",
      "server-url",
      "tenant-admin",
      "team-invite",
      "sso-settings",
      "billing",
      "admin-console",
    ].join("\n"),
    "packages/ui/src/index.ts": [
      "PersonalWorkspaceShellModel",
      "createPersonalWorkspaceShellModel",
      "WorkspaceHealthActionModel",
      "ReadOnlyRecovery",
      "open-recovery",
    ].join("\n"),
    "packages/ui/tests/personal_workspace_shell_model_tests.ts": [
      "personal workspace shell exposes local productivity navigation",
      "workspace health action model limits writes during read-only recovery",
      "server-url",
      "billing",
      "admin-console",
    ].join("\n"),
    "apps/desktop/src/index.ts": [
      "export function createDesktopCurrentProductShell(",
      "createPersonalLocalDesktopCapabilityProfile",
      "createPersonalWorkspaceShellModel",
      "desktopShell",
      "personal_local_desktop",
    ].join("\n"),
    "apps/desktop/tests/desktop_personal_workspace_shell_tests.ts": [
      "desktop current product shell uses personal local workspace profile",
      "desktop shell default export does not expose server administration actions",
      "ReadOnlyRecovery",
      "createDesktopCurrentProductShellDescriptor",
    ].join("\n"),
  };
}
