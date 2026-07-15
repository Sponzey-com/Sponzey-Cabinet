import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const WorkspaceShellGateState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  ValidatingEvidence: "ValidatingEvidence",
  WritingReport: "WritingReport",
  Passed: "Passed",
  Failed: "Failed",
});

export const WorkspaceShellGateEvent = Object.freeze({
  Start: "Start",
  SourcesLoaded: "SourcesLoaded",
  EvidenceValidated: "EvidenceValidated",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const WorkspaceShellGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE006_WORKSPACE_SHELL_REQUIRED_EVIDENCE_MISSING",
  ForbiddenProductText: "PHASE006_WORKSPACE_SHELL_FORBIDDEN_PRODUCT_TEXT",
  SourceReadFailed: "PHASE006_WORKSPACE_SHELL_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE006_WORKSPACE_SHELL_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("local_runtime_prerequisite", "Phase 006 local desktop runtime prerequisite", {
    requiredFiles: [".tasks/phase006-local-runtime-gate-result.md"],
    evidence: ["phase006_local_runtime_gate=passed"],
  }),
  target("client_core_personal_profile", "personal local desktop capability profile", {
    requiredFiles: [
      "packages/client-core/src/index.ts",
      "packages/client-core/tests/personal_local_desktop_capability_tests.ts",
    ],
    evidence: [
      "PersonalLocalDesktopCapabilityProfile",
      "createPersonalLocalDesktopCapabilityProfile",
      "personal_local_desktop",
      "supportsRemoteWorkspace: false",
      "isForbiddenPersonalLocalDesktopAction",
      "personal local desktop profile exposes local-first actions",
    ],
  }),
  target("ui_personal_workspace_shell", "personal workspace shell and recovery action model", {
    requiredFiles: [
      "packages/ui/src/index.ts",
      "packages/ui/tests/personal_workspace_shell_model_tests.ts",
    ],
    evidence: [
      "PersonalWorkspaceShellModel",
      "createPersonalWorkspaceShellModel",
      "WorkspaceHealthActionModel",
      "ReadOnlyRecovery",
      "open-recovery",
      "personal workspace shell exposes local productivity navigation",
      "workspace health action model limits writes during read-only recovery",
    ],
  }),
  target("desktop_current_product_shell", "personal local desktop shell entry", {
    requiredFiles: [
      "apps/desktop/src/index.ts",
      "apps/desktop/tests/desktop_personal_workspace_shell_tests.ts",
    ],
    evidence: [
      "export function createDesktopCurrentProductShell(",
      "createDesktopCurrentProductShellDescriptor",
      "createPersonalLocalDesktopCapabilityProfile",
      "createPersonalWorkspaceShellModel",
      "desktopShell",
      "personal_local_desktop",
      "desktop current product shell uses personal local workspace profile",
      "desktop shell default export does not expose server administration actions",
    ],
    forbiddenText: [
      "server-url",
      "tenant-admin",
      "organization-admin",
      "team-invite",
      "sso-settings",
      "admin-console",
    ],
    forbiddenFiles: ["apps/desktop/src/index.ts"],
  }),
]);

export function transitionWorkspaceShellGateState(currentState, event, detail = {}) {
  if (currentState === WorkspaceShellGateState.Pending && event === WorkspaceShellGateEvent.Start) {
    return { state: WorkspaceShellGateState.ReadingSources };
  }
  if (
    currentState === WorkspaceShellGateState.ReadingSources &&
    event === WorkspaceShellGateEvent.SourcesLoaded
  ) {
    return { state: WorkspaceShellGateState.ValidatingEvidence };
  }
  if (
    currentState === WorkspaceShellGateState.ValidatingEvidence &&
    event === WorkspaceShellGateEvent.EvidenceValidated
  ) {
    return { state: WorkspaceShellGateState.WritingReport };
  }
  if (
    currentState === WorkspaceShellGateState.WritingReport &&
    event === WorkspaceShellGateEvent.ReportWritten
  ) {
    return { state: WorkspaceShellGateState.Passed };
  }
  if (
    [
      WorkspaceShellGateState.ReadingSources,
      WorkspaceShellGateState.ValidatingEvidence,
      WorkspaceShellGateState.WritingReport,
    ].includes(currentState) &&
    event === WorkspaceShellGateEvent.Fail
  ) {
    return {
      state: WorkspaceShellGateState.Failed,
      errorCode: detail.errorCode ?? WorkspaceShellGateErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
    };
  }
  return {
    state: WorkspaceShellGateState.Failed,
    errorCode: WorkspaceShellGateErrorCode.InvalidTransition,
  };
}

export function analyzeWorkspaceShellEvidence({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    return failedResult({
      errorCode: WorkspaceShellGateErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_set", missing: ["sources"] }],
    });
  }

  const targetResults = requiredTargets.map((entry) => analyzeTarget(entry, sources));
  const forbidden = targetResults.find((entry) => entry.forbiddenFound.length > 0);
  if (forbidden) {
    return failedResult({
      errorCode: WorkspaceShellGateErrorCode.ForbiddenProductText,
      missingEvidence: [
        {
          targetId: forbidden.id,
          missing: forbidden.forbiddenFound,
        },
      ],
      targetResults,
    });
  }

  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));
  if (missingEvidence.length > 0) {
    return failedResult({
      errorCode: WorkspaceShellGateErrorCode.RequiredEvidenceMissing,
      missingEvidence,
      targetResults,
    });
  }

  return {
    passed: true,
    marker: "phase006_workspace_shell_gate=passed",
    state: WorkspaceShellGateState.Passed,
    summary: {
      requiredTargets: requiredTargets.length,
      missingRequiredEvidence: 0,
    },
    targetResults,
    missingEvidence: [],
  };
}

export function renderWorkspaceShellGateMarkdown(result) {
  const lines = [
    "# Phase 006 Workspace Shell Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 006`",
    "- gate: `Personal Workspace Shell and Navigation UX`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    `- state: \`${result.state}\``,
  ];
  if (!result.passed) {
    lines.push(`- error code: \`${result.errorCode}\``);
  }
  lines.push(
    `- required targets: \`${result.summary.requiredTargets}\``,
    `- missing required evidence: \`${result.summary.missingRequiredEvidence}\``,
    "",
    "## Evidence",
    "",
    "- personal local desktop shell",
    "- server/admin action absence",
    "",
    "| Target | Status | Description |",
    "| --- | --- | --- |",
  );
  for (const targetResult of result.targetResults ?? []) {
    lines.push(
      `| \`${targetResult.id}\` | \`${targetResult.status}\` | ${targetResult.description} |`,
    );
  }
  lines.push(
    "",
    "## Commands",
    "",
    "- `npm run run:phase006-workspace-shell-gate-tests`",
    "- `npm run run:phase006-workspace-shell-gate`",
    "- `node --test packages/client-core/tests/personal_local_desktop_capability_tests.ts packages/ui/tests/personal_workspace_shell_model_tests.ts apps/desktop/tests/desktop_personal_workspace_shell_tests.ts`",
    "- `npm run run:security-log-scanner`",
    "",
    "## Sensitive Data Exclusion",
    "",
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record document body, asset content, AI prompt, AI answer, provider key, token, credential, server URL, or personal absolute path.",
    "",
    "## Follow-Up Limitation",
    "",
    "React rendered shell layout, viewport overflow smoke, and keyboard navigation smoke remain for later Phase 006 document/UI tasks.",
    "",
  );
  return lines.join("\n");
}

export async function runWorkspaceShellGate({ root = process.cwd() } = {}) {
  let state = transitionWorkspaceShellGateState(
    WorkspaceShellGateState.Pending,
    WorkspaceShellGateEvent.Start,
  );
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    state = transitionWorkspaceShellGateState(state.state, WorkspaceShellGateEvent.SourcesLoaded);

    const result = analyzeWorkspaceShellEvidence({ sources });
    if (!result.passed) {
      state = transitionWorkspaceShellGateState(state.state, WorkspaceShellGateEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.missingEvidence[0]?.targetId,
      });
      return { ...result, state: state.state };
    }
    state = transitionWorkspaceShellGateState(state.state, WorkspaceShellGateEvent.EvidenceValidated);
    state = transitionWorkspaceShellGateState(state.state, WorkspaceShellGateEvent.ReportWritten);
    return { ...result, state: state.state };
  } catch {
    state = transitionWorkspaceShellGateState(state.state, WorkspaceShellGateEvent.Fail, {
      errorCode: WorkspaceShellGateErrorCode.SourceReadFailed,
    });
    return failedResult({
      errorCode: state.errorCode,
      state: state.state,
      missingEvidence: [{ targetId: "source_read", missing: ["required source file"] }],
    });
  }
}

function analyzeTarget(entry, sources) {
  const texts = entry.requiredFiles.map((filePath) => sources[filePath] ?? "");
  const missingFiles = entry.requiredFiles.filter((filePath) => !(filePath in sources));
  const missingEvidence = entry.evidence.filter(
    (needle) => !texts.some((text) => text.includes(needle)),
  );
  const forbiddenTexts = entry.forbiddenFiles.map((filePath) => sources[filePath] ?? "");
  const forbiddenFound = entry.forbiddenText.filter((needle) =>
    forbiddenTexts.some((text) => text.includes(needle)),
  );
  const missing = [...missingFiles, ...missingEvidence];
  return {
    id: entry.id,
    description: entry.description,
    status: missing.length === 0 && forbiddenFound.length === 0 ? "covered" : "missing",
    missing,
    forbiddenFound,
  };
}

function failedResult({
  errorCode,
  state = WorkspaceShellGateState.Failed,
  missingEvidence,
  targetResults = [],
}) {
  return {
    passed: false,
    marker: "phase006_workspace_shell_gate=failed",
    state,
    errorCode,
    summary: {
      requiredTargets: requiredTargets.length,
      missingRequiredEvidence: missingEvidence.length,
    },
    targetResults,
    missingEvidence,
  };
}

function target(id, description, { requiredFiles, evidence, forbiddenText = [], forbiddenFiles = requiredFiles }) {
  return {
    id,
    description,
    requiredFiles,
    evidence,
    forbiddenText,
    forbiddenFiles,
  };
}

function collectRequiredFiles() {
  return [...new Set(requiredTargets.flatMap((entry) => entry.requiredFiles))];
}

async function runCli() {
  const result = await runWorkspaceShellGate();
  const markdown = renderWorkspaceShellGateMarkdown(result);
  await writeFile(".tasks/phase006-workspace-shell-gate-result.md", markdown);
  if (result.passed) {
    console.log(result.marker);
    console.log(`gate_state=${result.state}`);
    console.log(`required_targets=${result.summary.requiredTargets}`);
    return;
  }
  console.error(result.marker);
  console.error(`gate_state=${result.state}`);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
