import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const DocumentUxGateState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  ValidatingEvidence: "ValidatingEvidence",
  WritingReport: "WritingReport",
  Passed: "Passed",
  Failed: "Failed",
});

export const DocumentUxGateEvent = Object.freeze({
  Start: "Start",
  SourcesLoaded: "SourcesLoaded",
  EvidenceValidated: "EvidenceValidated",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const DocumentUxGateErrorCode = Object.freeze({
  RequiredEvidenceMissing: "PHASE006_DOCUMENT_UX_REQUIRED_EVIDENCE_MISSING",
  SourceReadFailed: "PHASE006_DOCUMENT_UX_SOURCE_READ_FAILED",
  InvalidTransition: "PHASE006_DOCUMENT_UX_INVALID_TRANSITION",
});

const requiredTargets = Object.freeze([
  target("workspace_shell_prerequisite", "Phase 006 workspace shell gate prerequisite", {
    requiredFiles: [".tasks/phase006-workspace-shell-gate-result.md"],
    evidence: ["phase006_workspace_shell_gate=passed"],
  }),
  target("document_query_performance_budget", "current/history p95 300ms performance budget", {
    requiredFiles: [".tasks/release/performance-budget-phase006.md"],
    evidence: [
      "phase006_document_query_budget=passed",
      "current_document_read_p95_ms=",
      "history_read_p95_ms=",
      "threshold_ms=300",
    ],
  }),
  target("ui_markdown_preview_and_read_split", "Markdown preview and current/history split UI model", {
    requiredFiles: [
      "packages/ui/src/index.ts",
      "packages/ui/tests/markdown_preview_model_tests.ts",
    ],
    evidence: [
      "createMarkdownPreviewModel",
      "createDocumentReadingWorkspaceModel",
      "MarkdownTablePreviewBlock",
      "markdown preview renders table grid while source remains markdown text",
      "document reading workspace keeps current and history query paths separated",
    ],
  }),
  target("ui_restore_flow", "restore preview and apply state model", {
    requiredFiles: [
      "packages/ui/src/index.ts",
      "packages/ui/tests/restore_flow_model_tests.ts",
    ],
    evidence: [
      "transitionRestoreFlowState",
      "createRestorePreviewRequestFromHistoryEntry",
      "createRestoreApplyCommand",
      "restore flow state machine accepts only preview before apply",
      "restore preview model creates confirmed command only when restore is allowed",
    ],
  }),
  target("desktop_document_ux_smoke", "desktop document UX product smoke", {
    requiredFiles: [
      "apps/desktop/src/index.ts",
      "apps/desktop/tests/desktop_document_ux_smoke_tests.ts",
    ],
    evidence: [
      "createDesktopDocumentReadingWorkspace",
      "createDesktopRestorePreviewRequest",
      "createDesktopRestoreApplyCommand",
      "desktop document UX smoke renders markdown preview table and keeps read paths split",
      "desktop restore smoke creates preview request and confirmed command without body payload",
    ],
  }),
  target("rust_restore_and_compare_usecases", "Rust restore preview/restore/compare usecase evidence", {
    requiredFiles: [
      "crates/cabinet-usecases/tests/preview_document_restore_tests.rs",
      "crates/cabinet-usecases/tests/restore_document_version_tests.rs",
      "crates/cabinet-usecases/tests/compare_document_versions_tests.rs",
    ],
    evidence: [
      "preview_document_restore_returns_diff_without_writes",
      "restore_document_version_appends_restore_version_updates_current_and_emits_events",
      "compare_current_to_version_uses_current_and_specific_snapshot_without_history_list",
      "compare_two_versions_uses_specific_snapshots_without_current_or_history_list",
    ],
  }),
]);

export function transitionDocumentUxGateState(currentState, event, detail = {}) {
  if (currentState === DocumentUxGateState.Pending && event === DocumentUxGateEvent.Start) {
    return { state: DocumentUxGateState.ReadingSources };
  }
  if (
    currentState === DocumentUxGateState.ReadingSources &&
    event === DocumentUxGateEvent.SourcesLoaded
  ) {
    return { state: DocumentUxGateState.ValidatingEvidence };
  }
  if (
    currentState === DocumentUxGateState.ValidatingEvidence &&
    event === DocumentUxGateEvent.EvidenceValidated
  ) {
    return { state: DocumentUxGateState.WritingReport };
  }
  if (
    currentState === DocumentUxGateState.WritingReport &&
    event === DocumentUxGateEvent.ReportWritten
  ) {
    return { state: DocumentUxGateState.Passed };
  }
  if (
    [
      DocumentUxGateState.ReadingSources,
      DocumentUxGateState.ValidatingEvidence,
      DocumentUxGateState.WritingReport,
    ].includes(currentState) &&
    event === DocumentUxGateEvent.Fail
  ) {
    return {
      state: DocumentUxGateState.Failed,
      errorCode: detail.errorCode ?? DocumentUxGateErrorCode.RequiredEvidenceMissing,
      findingId: detail.findingId,
    };
  }
  return {
    state: DocumentUxGateState.Failed,
    errorCode: DocumentUxGateErrorCode.InvalidTransition,
  };
}

export function analyzeDocumentUxEvidence({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    return failedResult({
      errorCode: DocumentUxGateErrorCode.SourceReadFailed,
      missingEvidence: [{ targetId: "source_set", missing: ["sources"] }],
    });
  }

  const targetResults = requiredTargets.map((entry) => analyzeTarget(entry, sources));
  const missingEvidence = targetResults
    .filter((entry) => entry.missing.length > 0)
    .map((entry) => ({ targetId: entry.id, missing: entry.missing }));

  if (missingEvidence.length > 0) {
    return failedResult({
      errorCode: DocumentUxGateErrorCode.RequiredEvidenceMissing,
      missingEvidence,
      targetResults,
    });
  }

  return {
    passed: true,
    marker: "phase006_document_ux_gate=passed",
    state: DocumentUxGateState.Passed,
    summary: {
      requiredTargets: requiredTargets.length,
      missingRequiredEvidence: 0,
    },
    targetResults,
    missingEvidence: [],
  };
}

export function renderDocumentUxGateMarkdown(result) {
  const lines = [
    "# Phase 006 Document UX Gate Result",
    "",
    result.marker,
    "",
    "- phase: `Phase 006`",
    "- gate: `Document Editor, Markdown Preview, History, and Restore UX`",
    "- scope: `Markdown preview and restore UX`",
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
    "- `npm run run:phase006-document-query-budget-tests`",
    "- `npm run run:phase006-document-query-budget`",
    "- `npm run run:phase006-document-ux-gate-tests`",
    "- `npm run run:phase006-document-ux-gate`",
    "- `node --test packages/ui/tests/markdown_preview_model_tests.ts packages/ui/tests/restore_flow_model_tests.ts apps/desktop/tests/desktop_document_ux_smoke_tests.ts`",
    "- `cargo test -p cabinet-usecases preview_document_restore`",
    "- `cargo test -p cabinet-usecases restore_document_version`",
    "- `cargo test -p cabinet-usecases compare_document_versions`",
    "- `npm run run:security-log-scanner`",
    "",
    "## Sensitive Data Exclusion",
    "",
    "This artifact records target ids, marker status, counts, and stable error codes only. It does not record document body, raw markdown, rendered HTML dump, asset content, AI prompt, AI answer, provider key, token, credential, or personal absolute path.",
    "",
    "## Follow-Up Limitation",
    "",
    "React DOM screenshot/layout smoke and keyboard navigation smoke remain for a later Phase 006 UI hardening task.",
    "",
  );
  return lines.join("\n");
}

export async function runDocumentUxGate({ root = process.cwd() } = {}) {
  let state = transitionDocumentUxGateState(DocumentUxGateState.Pending, DocumentUxGateEvent.Start);
  try {
    const sources = {};
    for (const filePath of collectRequiredFiles()) {
      sources[filePath] = await readFile(`${root}/${filePath}`, "utf8");
    }
    state = transitionDocumentUxGateState(state.state, DocumentUxGateEvent.SourcesLoaded);

    const result = analyzeDocumentUxEvidence({ sources });
    if (!result.passed) {
      state = transitionDocumentUxGateState(state.state, DocumentUxGateEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.missingEvidence[0]?.targetId,
      });
      return { ...result, state: state.state };
    }
    state = transitionDocumentUxGateState(state.state, DocumentUxGateEvent.EvidenceValidated);
    state = transitionDocumentUxGateState(state.state, DocumentUxGateEvent.ReportWritten);
    return { ...result, state: state.state };
  } catch {
    state = transitionDocumentUxGateState(state.state, DocumentUxGateEvent.Fail, {
      errorCode: DocumentUxGateErrorCode.SourceReadFailed,
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
  const missing = [...missingFiles, ...missingEvidence];
  return {
    id: entry.id,
    description: entry.description,
    status: missing.length === 0 ? "covered" : "missing",
    missing,
  };
}

function failedResult({ errorCode, state = DocumentUxGateState.Failed, missingEvidence, targetResults = [] }) {
  return {
    passed: false,
    marker: "phase006_document_ux_gate=failed",
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

function target(id, description, { requiredFiles, evidence }) {
  return {
    id,
    description,
    requiredFiles,
    evidence,
  };
}

function collectRequiredFiles() {
  return [...new Set(requiredTargets.flatMap((entry) => entry.requiredFiles))];
}

async function runCli() {
  const result = await runDocumentUxGate();
  const markdown = renderDocumentUxGateMarkdown(result);
  await writeFile(".tasks/phase006-document-ux-gate-result.md", markdown);
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

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
