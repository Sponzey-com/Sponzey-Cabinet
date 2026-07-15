import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const WorkspaceHomeGateErrorCode = Object.freeze({
  PlanValidationMissing: "PHASE007_WORKSPACE_HOME_PLAN_VALIDATION_MISSING",
  ProductScopeMismatch: "PHASE007_WORKSPACE_HOME_PRODUCT_SCOPE_MISMATCH",
  HomeModelMissing: "PHASE007_WORKSPACE_HOME_MODEL_MISSING",
  RequiredSectionMissing: "PHASE007_WORKSPACE_HOME_SECTION_MISSING",
  CommandSurfaceMismatch: "PHASE007_WORKSPACE_HOME_COMMAND_SURFACE_MISMATCH",
  ForbiddenCommandSurface: "PHASE007_WORKSPACE_HOME_FORBIDDEN_COMMAND_SURFACE",
  IoFailed: "PHASE007_WORKSPACE_HOME_IO_FAILED",
});

const requiredHomeSectionIds = [
  "recent-documents",
  "favorites",
  "tags",
  "recent-changes",
  "quick-search",
  "ai-entry",
  "backup-status",
  "workspace-health",
];

const allowedCommandActionIds = [
  "new-document",
  "quick-search",
  "open-graph",
  "ask-ai",
  "create-backup",
  "import-markdown",
  "export-package",
  "open-settings",
];

const forbiddenSurfaceTerms = [
  "serverBaseUrl",
  "sessionToken",
  "team-invite",
  "admin-console",
  "billing",
  "tenant-settings",
  "sso-settings",
  "server-workspace-connect",
  "provider_api_key_fixture",
];

export function evaluateWorkspaceHomeGate({ planValidationText, desktopShell }) {
  if (!planValidationText.includes("phase007_plan_validation=passed")) {
    return failed(
      WorkspaceHomeGateErrorCode.PlanValidationMissing,
      ".tasks/phase007-plan-validation-result.md",
    );
  }

  if (
    desktopShell?.capability?.productScope !== "personal_local_desktop" ||
    desktopShell?.workspace?.productScope !== "personal_local_desktop"
  ) {
    return failed(WorkspaceHomeGateErrorCode.ProductScopeMismatch, "personal_local_desktop");
  }

  if (desktopShell?.home?.mode !== "personal-workspace-home" || desktopShell.home.firstRoute !== "home") {
    return failed(WorkspaceHomeGateErrorCode.HomeModelMissing, "home");
  }

  const sectionIds = (desktopShell.home.sections ?? []).map((section) => section.id);
  for (const sectionId of requiredHomeSectionIds) {
    if (!sectionIds.includes(sectionId)) {
      return failed(WorkspaceHomeGateErrorCode.RequiredSectionMissing, sectionId);
    }
  }

  const actionIds = (desktopShell.workspace.commandActions ?? []).map((action) => action.id);
  const serialized = JSON.stringify(desktopShell);
  for (const term of forbiddenSurfaceTerms) {
    if (serialized.includes(term)) {
      return failed(WorkspaceHomeGateErrorCode.ForbiddenCommandSurface, term);
    }
  }

  if (JSON.stringify(actionIds) !== JSON.stringify(allowedCommandActionIds)) {
    return failed(WorkspaceHomeGateErrorCode.CommandSurfaceMismatch, actionIds.join(","));
  }

  return {
    passed: true,
    marker: "phase007_workspace_home_gate=passed",
    sectionCount: requiredHomeSectionIds.length,
    commandActionCount: allowedCommandActionIds.length,
    productScope: "personal_local_desktop",
  };
}

export function renderWorkspaceHomeGateResult(result) {
  if (result.passed) {
    return [
      "phase007_workspace_home_gate=passed",
      `product_scope=${result.productScope}`,
      `home_section_count=${result.sectionCount}`,
      `command_action_count=${result.commandActionCount}`,
    ].join("\n");
  }
  return [
    "phase007_workspace_home_gate=failed",
    `error_code=${result.errorCode}`,
    `finding_id=${result.findingId}`,
  ].join("\n");
}

export function renderWorkspaceHomeGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 007 Workspace Home Gate Result",
    "",
    renderWorkspaceHomeGateResult(result),
    "",
    "- phase: `Phase 007.1`",
    "- gate: `Personal Workspace Home`",
    `- status: \`${status}\``,
    "- commands:",
    "  - `npm run run:phase007-workspace-home-gate-tests`",
    "  - `npm run run:phase007-workspace-home-gate`",
    "- required sections: `recent-documents`, `favorites`, `tags`, `recent-changes`, `quick-search`, `ai-entry`, `backup-status`, `workspace-health`",
    "- allowed command actions: `new-document`, `quick-search`, `open-graph`, `ask-ai`, `create-backup`, `import-markdown`, `export-package`, `open-settings`",
    "- Product Log candidates: `workspace.home.ready`, `workspace.health.failed`",
    "- Field Debug metadata candidates: `section_count`, `health_state`, `missing_projection_summary`",
    "- sensitive data exclusion: this artifact records ids, counts, status, and stable error codes only.",
    "- follow-up limitation: document authoring and Markdown preview remain Phase 007.2.",
    "",
  ].join("\n");
}

async function runWorkspaceHomeGateCli() {
  try {
    const planValidationText = await readFile(".tasks/phase007-plan-validation-result.md", "utf8");
    const { desktopShell } = await import(
      pathToFileURL(join(process.cwd(), "apps/desktop/src/index.ts")).href
    );
    const result = evaluateWorkspaceHomeGate({ planValidationText, desktopShell });
    await writeFile(
      ".tasks/phase007-workspace-home-gate-result.md",
      renderWorkspaceHomeGateArtifact(result),
    );
    const rendered = renderWorkspaceHomeGateResult(result);
    if (result.passed) {
      console.log(rendered);
      return;
    }
    console.error(rendered);
    process.exit(1);
  } catch (error) {
    const result = failed(
      WorkspaceHomeGateErrorCode.IoFailed,
      error instanceof Error ? error.message : "unknown",
    );
    await writeFile(
      ".tasks/phase007-workspace-home-gate-result.md",
      renderWorkspaceHomeGateArtifact(result),
    );
    console.error(renderWorkspaceHomeGateResult(result));
    process.exit(1);
  }
}

function failed(errorCode, findingId) {
  return {
    passed: false,
    marker: "phase007_workspace_home_gate=failed",
    errorCode,
    findingId,
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runWorkspaceHomeGateCli();
}
