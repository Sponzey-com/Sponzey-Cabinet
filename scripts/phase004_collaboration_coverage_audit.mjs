import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const CollaborationCoverageAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const CollaborationCoverageAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const CollaborationCoverageAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE004_COLLABORATION_COVERAGE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE004_COLLABORATION_COVERAGE_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target(
    "collaboration_domain_usecase_contract",
    "Collaborative edit operation, session, usecase, and port contract",
    {
      files: [
        "crates/cabinet-domain/src/collaboration.rs",
        "crates/cabinet-domain/tests/collaboration_tests.rs",
        "crates/cabinet-usecases/src/collaboration.rs",
        "crates/cabinet-usecases/tests/collaboration_usecase_tests.rs",
        "crates/cabinet-ports/src/collaboration.rs",
        "crates/cabinet-ports/tests/collaboration_contract_tests.rs",
      ],
      evidence: [
        "DocumentOperation",
        "EditSessionState",
        "transition_edit_session_state",
        "detect_collaboration_conflict",
        "StartEditSessionUsecase",
        "ApplyCollaborativeEditUsecase",
        "UpdatePresenceUsecase",
        "CollaborationSessionStore",
        "CollaborationEventLog",
        "stale_base_revision_returns_conflict_without_event_append",
        "presence_update_is_saved_without_durable_operation_append",
      ],
      priority: 100,
    },
  ),
  target(
    "collaboration_realtime_transport_contract",
    "Realtime room, owner policy, transport, and local adapter contract",
    {
      files: [
        "crates/cabinet-domain/src/realtime.rs",
        "crates/cabinet-domain/tests/realtime_tests.rs",
        "crates/cabinet-ports/src/realtime.rs",
        "crates/cabinet-ports/tests/realtime_contract_tests.rs",
        "crates/cabinet-adapters/src/local_realtime.rs",
        "crates/cabinet-adapters/tests/local_realtime_adapter_tests.rs",
      ],
      evidence: [
        "RealtimeConnectionState",
        "transition_realtime_connection_state",
        "DocumentRoomOwnerPolicy",
        "RealtimeTransport",
        "LocalDocumentRoomOwnerPolicy",
        "LocalRealtimeTransport",
        "RoomNotJoined",
        "realtime_connection_transition_supports_join_sync_conflict_and_replay_flow",
        "realtime_transport_contract_keeps_join_operation_presence_and_replay_separate",
        "local_realtime_transport_rejects_unjoined_room_broadcasts_with_stable_error",
      ],
      priority: 98,
    },
  ),
  target(
    "collaboration_server_runtime_boundary",
    "Server realtime route, command mapper, executor, runtime target, and split dispatch",
    {
      files: [
        "crates/cabinet-server/src/composition.rs",
        "crates/cabinet-server/src/runtime.rs",
        "crates/cabinet-server/src/collaboration_realtime.rs",
        "crates/cabinet-server/tests/collaboration_realtime_command_mapper_tests.rs",
        "crates/cabinet-server/tests/collaboration_realtime_executor_tests.rs",
        "crates/cabinet-server/tests/collaboration_realtime_runtime_target_tests.rs",
        "crates/cabinet-server/tests/split_realtime_server_target_tests.rs",
        "crates/cabinet-server/tests/server_runtime_wiring_tests.rs",
        "crates/cabinet-server/tests/server_dependency_manifest_tests.rs",
      ],
      evidence: [
        "collaboration.join_document_room",
        "collaboration.broadcast_operation",
        "collaboration.broadcast_presence",
        "collaboration.request_replay",
        "command_from_input",
        "execute_realtime_command",
        "CollaborationRealtimeRuntimeTarget",
        "SplitRealtimeServerTarget",
        "LocalDocumentRoomOwnerPolicy",
        "LocalRealtimeTransport",
        "executor_appends_operation_before_transport_broadcast",
        "split_target_sends_collaboration_realtime_routes_to_realtime_target",
      ],
      priority: 96,
    },
  ),
  target(
    "collaboration_client_editor_contract",
    "Editor adapter and client-core realtime collaboration contract",
    {
      files: [
        "packages/editor/src/index.ts",
        "packages/editor/tests/collaboration_adapter_tests.ts",
        "packages/client-core/src/index.ts",
        "packages/client-core/tests/realtime_client_tests.ts",
      ],
      evidence: [
        "createCollaborativeEditInputFromEditorTransaction",
        "createPresenceInputFromEditorSelection",
        "CollaborationRealtimeClient",
        "createCollaborationRealtimeClient",
        "selectedText",
        "documentBody",
        "token",
        "editor collaboration adapter does not import CodeMirror runtime types",
        "collaboration realtime client strips sensitive presence draft fields",
        "collaboration realtime client does not import runtime transport or editor types",
      ],
      priority: 94,
    },
  ),
  target(
    "collaboration_smoke_security_evidence",
    "Realtime collaboration smoke result and security log scanner evidence",
    {
      files: [
        "scripts/run_phase004_realtime_collaboration_smoke.sh",
        "scripts/run_phase004_realtime_collaboration_smoke_tests.mjs",
        ".tasks/realtime-collaboration-smoke-result.md",
        ".tasks/release/security-log-policy-manifest.json",
        "scripts/security_log_scanner_tests.mjs",
        "package.json",
      ],
      evidence: [
        "phase004_realtime_collaboration_smoke=passed",
        "server_split_dispatch=passed",
        "run:phase004-realtime-collaboration-smoke",
        "phase004_realtime_collaboration_smoke_result",
        "realtime_operation_text_fixture",
        "realtime_selection_text_fixture",
        "operation_text",
        "selection_text",
        "codemirror_transaction_dump",
      ],
      priority: 92,
    },
  ),
]);

class CollaborationCoverageAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "CollaborationCoverageAuditError";
    this.code = code;
  }
}

export function transitionCollaborationCoverageAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${CollaborationCoverageAuditState.NotStarted}:${CollaborationCoverageAuditEvent.Start}`,
      CollaborationCoverageAuditState.ReadingSource,
    ],
    [
      `${CollaborationCoverageAuditState.ReadingSource}:${CollaborationCoverageAuditEvent.SourceLoaded}`,
      CollaborationCoverageAuditState.Auditing,
    ],
    [
      `${CollaborationCoverageAuditState.Auditing}:${CollaborationCoverageAuditEvent.AuditComplete}`,
      CollaborationCoverageAuditState.Reported,
    ],
    [
      `${CollaborationCoverageAuditState.Reported}:${CollaborationCoverageAuditEvent.ReportWritten}`,
      CollaborationCoverageAuditState.Reported,
    ],
    [
      `${CollaborationCoverageAuditState.ReadingSource}:${CollaborationCoverageAuditEvent.Fail}`,
      CollaborationCoverageAuditState.Failed,
    ],
    [
      `${CollaborationCoverageAuditState.Auditing}:${CollaborationCoverageAuditEvent.Fail}`,
      CollaborationCoverageAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new CollaborationCoverageAuditError(
      CollaborationCoverageAuditErrorCode.InvalidTransition,
      `invalid collaboration coverage audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeCollaborationCoverageSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new CollaborationCoverageAuditError(
      CollaborationCoverageAuditErrorCode.SourceSetEmpty,
      "phase004 collaboration coverage audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);

  return {
    phase: "Phase 004.4",
    sourceFiles: Object.keys(sources).sort(),
    summary: {
      totalTargets: targets.length,
      covered: targets.filter((entry) => entry.status === STATUS.Covered).length,
      missing: targets.filter((entry) => entry.status === STATUS.Missing).length,
      targetsNeedingWork: targetsNeedingWork.length,
    },
    findings:
      targetsNeedingWork.length === 0
        ? []
        : [
            {
              id: "PHASE004_COLLABORATION_COVERAGE_GAP",
              message: "Some Phase 004.4 realtime collaboration coverage targets are missing evidence.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ],
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderCollaborationCoverageAuditMarkdown(audit) {
  const lines = [
    "# Phase 004 Collaboration Coverage Audit",
    "",
    "현재 단계: Phase 004.4 - Realtime Collaboration Gateway Runtime and Product Smoke",
    "",
    "## Purpose",
    "",
    "- collaborative edit domain/usecase/port, realtime runtime, client/editor adapter, smoke/security evidence를 코드 기준으로 고정한다.",
    "- static contract만 있는 상태를 realtime collaboration coverage complete로 오판하지 않는다.",
    "- 다음 task는 모든 collaboration gap을 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| total targets | ${audit.summary.totalTargets} |`,
    `| covered | ${audit.summary.covered} |`,
    `| missing | ${audit.summary.missing} |`,
    `| targets needing work | ${audit.summary.targetsNeedingWork} |`,
    "",
    "## Target Status",
    "",
    "| Target | Label | Status | Missing Files | Missing Evidence |",
    "| --- | --- | --- | --- | --- |",
    ...audit.targets.map((entry) => {
      const missingFiles =
        entry.missingFiles.length > 0 ? entry.missingFiles.map(code).join(", ") : "none";
      const missingEvidence =
        entry.missingEvidence.length > 0 ? entry.missingEvidence.map(code).join(", ") : "none";
      return `| \`${entry.id}\` | ${entry.label} | ${entry.status} | ${missingFiles} | ${missingEvidence} |`;
    }),
    "",
    "## Findings",
    "",
  ];

  if (audit.findings.length === 0) {
    lines.push("- No collaboration coverage gap was detected.");
  } else {
    for (const finding of audit.findings) {
      lines.push(`- ${finding.id}: ${finding.message}`);
      lines.push(`- affected target count: ${finding.targetIds.length}`);
      lines.push(`- affected targets: ${finding.targetIds.map(code).join(", ")}`);
    }
  }

  lines.push("", "## Next Implementation Target", "");
  if (audit.nextImplementationTarget) {
    lines.push(
      `- target id: \`${audit.nextImplementationTarget.id}\``,
      `- label: ${audit.nextImplementationTarget.label}`,
      `- current status: ${audit.nextImplementationTarget.status}`,
      "- selected reason: highest priority Phase 004.4 collaboration target that is not covered.",
      "- next task should implement or harden this selected target only.",
    );
  } else {
    lines.push("- No next collaboration coverage target remains.");
  }

  lines.push(
    "",
    "## Review Notes",
    "",
    "- Collaboration operation and session rules must remain in domain/usecase logic, not editor, Web, desktop, or transport code.",
    "- CodeMirror and realtime transport runtime types must not enter domain/usecase contracts.",
    "- Realtime room ownership must remain behind policy/port abstractions so horizontal scale is not blocked by a hard-coded singleton.",
    "- Audit output must not include document bodies, operation text, selection text, clipboard content, tokens, secrets, credentials, or CodeMirror transaction dumps.",
  );

  return `${lines.join("\n")}\n`;
}

async function runCli() {
  let state = transitionCollaborationCoverageAuditState(
    CollaborationCoverageAuditState.NotStarted,
    CollaborationCoverageAuditEvent.Start,
  );
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const resultPath = path.join(repoRoot, ".tasks/collaboration-coverage-audit.md");
  const sources = await readAuditSources(repoRoot);
  state = transitionCollaborationCoverageAuditState(
    state,
    CollaborationCoverageAuditEvent.SourceLoaded,
  );
  const audit = analyzeCollaborationCoverageSources({ sources });
  state = transitionCollaborationCoverageAuditState(
    state,
    CollaborationCoverageAuditEvent.AuditComplete,
  );
  await mkdir(path.dirname(resultPath), { recursive: true });
  await writeFile(resultPath, renderCollaborationCoverageAuditMarkdown(audit));
  state = transitionCollaborationCoverageAuditState(
    state,
    CollaborationCoverageAuditEvent.ReportWritten,
  );

  if (audit.summary.targetsNeedingWork === 0) {
    console.log("phase004_collaboration_coverage_audit=passed");
    console.log(`state=${state}`);
    console.log(`target_count=${audit.summary.totalTargets}`);
    console.log(`result_path=${resultPath}`);
    return;
  }

  console.error("phase004_collaboration_coverage_audit=failed");
  console.error(`state=${state}`);
  console.error(`targets_needing_work=${audit.summary.targetsNeedingWork}`);
  console.error(`result_path=${resultPath}`);
  process.exitCode = 1;
}

async function readAuditSources(repoRoot) {
  const paths = new Set(TARGETS.flatMap((entry) => entry.files));
  const sources = {};
  for (const relativePath of paths) {
    try {
      sources[relativePath] = await readFile(path.join(repoRoot, relativePath), "utf8");
    } catch {
      // Missing files are represented by absence from the source map.
    }
  }
  return sources;
}

function analyzeTarget(entry, sources) {
  const combined = entry.files.map((filePath) => sources[filePath] ?? "").join("\n");
  const missingFiles = entry.files.filter((filePath) => !(filePath in sources));
  const missingEvidence = entry.evidence.filter((needle) => !combined.includes(needle));
  const status =
    missingFiles.length === 0 && missingEvidence.length === 0
      ? STATUS.Covered
      : STATUS.Missing;
  return {
    id: entry.id,
    label: entry.label,
    priority: entry.priority,
    status,
    missingFiles,
    missingEvidence,
  };
}

function pickNextImplementationTarget(targetsNeedingWork) {
  if (targetsNeedingWork.length === 0) {
    return null;
  }
  return [...targetsNeedingWork].sort((a, b) => b.priority - a.priority)[0];
}

function target(id, label, options) {
  return {
    id,
    label,
    files: options.files,
    evidence: options.evidence,
    priority: options.priority,
  };
}

function code(value) {
  return `\`${value}\``;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
