import assert from "node:assert/strict";
import test from "node:test";

import {
  evaluateWorkspaceHomeGate,
  renderWorkspaceHomeGateResult,
  WorkspaceHomeGateErrorCode,
} from "./phase007_workspace_home_gate.mjs";

test("workspace home gate rejects missing plan validation prerequisite", () => {
  const result = evaluateWorkspaceHomeGate({
    planValidationText: "phase007_plan_validation=failed",
    desktopShell: completeDesktopShell(),
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, WorkspaceHomeGateErrorCode.PlanValidationMissing);
});

test("workspace home gate rejects forbidden hosted command surface", () => {
  const shell = {
    ...completeDesktopShell(),
    workspace: {
      ...completeDesktopShell().workspace,
      commandActions: [
        ...completeDesktopShell().workspace.commandActions,
        { id: "server-workspace-connect", label: "Connect server" },
      ],
    },
  };

  const result = evaluateWorkspaceHomeGate({
    planValidationText: "phase007_plan_validation=passed",
    desktopShell: shell,
  });

  assert.equal(result.passed, false);
  assert.equal(result.errorCode, WorkspaceHomeGateErrorCode.ForbiddenCommandSurface);
  assert.equal(result.findingId, "server-workspace-connect");
});

test("workspace home gate passes complete desktop shell and renders safe marker", () => {
  const result = evaluateWorkspaceHomeGate({
    planValidationText: "phase007_plan_validation=passed",
    desktopShell: completeDesktopShell(),
  });
  const rendered = renderWorkspaceHomeGateResult(result);

  assert.equal(result.passed, true);
  assert.match(rendered, /phase007_workspace_home_gate=passed/);
  assert.doesNotMatch(rendered, /provider_api_key_fixture/);
});

function completeDesktopShell() {
  return {
    capability: {
      productScope: "personal_local_desktop",
      runtime: "desktop-local",
      supportsLocalWorkspace: true,
      supportsRemoteWorkspace: false,
    },
    workspace: {
      mode: "personal-workspace-shell",
      productScope: "personal_local_desktop",
      commandActions: [
        { id: "new-document", label: "New document" },
        { id: "quick-search", label: "Search" },
        { id: "open-graph", label: "Graph" },
        { id: "ask-ai", label: "Ask AI" },
        { id: "create-backup", label: "Backup" },
        { id: "import-markdown", label: "Import" },
        { id: "export-package", label: "Export" },
        { id: "open-settings", label: "Settings" },
      ],
    },
    home: {
      mode: "personal-workspace-home",
      productScope: "personal_local_desktop",
      firstRoute: "home",
      sections: [
        { id: "recent-documents" },
        { id: "favorites" },
        { id: "tags" },
        { id: "recent-changes" },
        { id: "quick-search" },
        { id: "ai-entry" },
        { id: "backup-status" },
        { id: "workspace-health" },
      ],
    },
  };
}
