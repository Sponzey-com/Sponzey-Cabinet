import { mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const MobileCapabilityAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const MobileCapabilityAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const MobileCapabilityAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE004_MOBILE_CAPABILITY_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE004_MOBILE_CAPABILITY_SOURCE_SET_EMPTY",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("mobile_capability_matrix_contract", "Mobile platform capability matrix contract", {
    files: [
      "packages/client-core/src/index.ts",
      "packages/client-core/tests/mobile_read_contract_tests.ts",
    ],
    evidence: [
      "PlatformFeatureSupport",
      "knowledgeGraphSupport",
      "canvasSupport",
      "realtimeCollaborationSupport",
      "supportsCanvasFullEdit",
      "createPlatformCapabilityMatrix",
      "platform capability matrix documents web desktop and mobile differences without domain rules",
    ],
    priority: 100,
  }),
  target("mobile_skeleton_action_and_push_boundary", "Mobile skeleton actions and push boundary", {
    files: [
      "apps/mobile/src/index.ts",
      "apps/mobile/tests/mobile_read_skeleton_tests.ts",
      "apps/mobile/tests/mobile_push_notification_tests.ts",
    ],
    evidence: [
      "requestCanvasEdit",
      "approveReviewRequest",
      "rejectReviewRequest",
      "createMobilePushNotificationPayload",
      "transitionMobileNotificationDeliveryState",
      "MOBILE_UNSUPPORTED_CANVAS_EDIT",
      "MOBILE_NOTIFICATION_INVALID_TRANSITION",
      "mobile skeleton maps review approve and reject decisions without raw body data",
      "mobile push payload excludes sensitive document comment token and canvas data",
      "mobile notification delivery state machine exposes queued sent failed and retry transitions",
    ],
    priority: 98,
  }),
  target("mobile_phase004_product_smoke", "Mobile Phase004 product smoke evidence", {
    files: [
      "apps/mobile/tests/mobile_read_product_smoke.ts",
      "scripts/run_mobile_read_product_smoke.mjs",
      ".tmp/mobile-read-product-smoke-output.txt",
    ],
    evidence: [
      "requiredSmokeMarkers",
      "assertRequiredMarkersPresent",
      "mobile_review_decision_product_smoke=passed",
      "mobile_canvas_unsupported_product_smoke=passed",
      "mobile_push_payload_product_smoke=passed",
      "mobile_read_product_smoke=passed",
    ],
    priority: 96,
  }),
  target("mobile_push_security_policy", "Mobile push security log policy evidence", {
    files: [
      ".tasks/release/security-log-policy-manifest.json",
      "scripts/security_log_scanner_tests.mjs",
    ],
    evidence: [
      "phase004_mobile_push_payload_contract",
      "mobile_push_document_body_fixture",
      "mobile_push_comment_body_fixture",
      "mobile_push_session_token_fixture",
      "mobile_push_raw_canvas_state_fixture",
      "push_document_body",
      "push_comment_body",
      "push_session_token",
      "push_raw_canvas_state",
      "active security manifest includes Phase 004 mobile push artifacts and denied fixtures",
    ],
    priority: 94,
  }),
]);

class MobileCapabilityAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "MobileCapabilityAuditError";
    this.code = code;
  }
}

export function transitionMobileCapabilityAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${MobileCapabilityAuditState.NotStarted}:${MobileCapabilityAuditEvent.Start}`,
      MobileCapabilityAuditState.ReadingSource,
    ],
    [
      `${MobileCapabilityAuditState.ReadingSource}:${MobileCapabilityAuditEvent.SourceLoaded}`,
      MobileCapabilityAuditState.Auditing,
    ],
    [
      `${MobileCapabilityAuditState.Auditing}:${MobileCapabilityAuditEvent.AuditComplete}`,
      MobileCapabilityAuditState.Reported,
    ],
    [
      `${MobileCapabilityAuditState.Reported}:${MobileCapabilityAuditEvent.ReportWritten}`,
      MobileCapabilityAuditState.Reported,
    ],
    [
      `${MobileCapabilityAuditState.ReadingSource}:${MobileCapabilityAuditEvent.Fail}`,
      MobileCapabilityAuditState.Failed,
    ],
    [
      `${MobileCapabilityAuditState.Auditing}:${MobileCapabilityAuditEvent.Fail}`,
      MobileCapabilityAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new MobileCapabilityAuditError(
      MobileCapabilityAuditErrorCode.InvalidTransition,
      `invalid mobile capability audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeMobileCapabilitySources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new MobileCapabilityAuditError(
      MobileCapabilityAuditErrorCode.SourceSetEmpty,
      "phase004 mobile capability audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);

  return {
    phase: "Phase 004.6",
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
              id: "PHASE004_MOBILE_CAPABILITY_COVERAGE_GAP",
              message: "Some Phase 004.6 mobile capability targets are missing evidence.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ],
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderMobileCapabilityAuditMarkdown(audit) {
  const lines = [
    "# Phase 004 Mobile Capability Coverage Audit",
    "",
    "현재 단계: Phase 004.6 - Mobile Collaboration Baseline and Platform Capability Matrix",
    "",
    "## Purpose",
    "",
    "- mobile capability, action mapping, push boundary, product smoke evidence를 코드와 smoke output 기준으로 고정한다.",
    "- product smoke output이 없는 상태를 mobile baseline complete로 오판하지 않는다.",
    "- 다음 task는 모든 mobile gap을 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
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
    lines.push("- No mobile capability coverage gap was detected.");
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
      "- selected reason: highest priority Phase 004.6 mobile target that is not covered.",
      "- next task should implement or harden this selected target only.",
    );
  } else {
    lines.push("- No next mobile capability target remains.");
  }

  lines.push(
    "",
    "## Validation Commands",
    "",
    "- `npm run run:phase004-mobile-capability-audit-tests`",
    "- `npm run run:phase004-mobile-capability-audit`",
    "- `npm run run:mobile-read-product-smoke`",
    "- `npm run run:mobile-read-contract-tests`",
    "- `npm run run:security-log-scanner`",
    "",
    "## Log and Configuration Notes",
    "",
    "- 이 audit는 Product Log 또는 Field Debug Log writer를 추가하지 않는다.",
    "- smoke output은 Development/Release validation output으로만 사용한다.",
    "- 외부 notification provider, native SDK, environment variable, hidden config를 요구하지 않는다.",
    "- product smoke output이 없으면 `mobile_phase004_product_smoke`는 missing으로 판정한다.",
  );

  return `${lines.join("\n")}\n`;
}

export async function runMobileCapabilityAudit({
  root = process.cwd(),
  outputPath = ".tasks/mobile-capability-audit.md",
} = {}) {
  let state = transitionMobileCapabilityAuditState(
    MobileCapabilityAuditState.NotStarted,
    MobileCapabilityAuditEvent.Start,
  );
  const sources = await readActualSources(root);
  state = transitionMobileCapabilityAuditState(state, MobileCapabilityAuditEvent.SourceLoaded);
  const audit = analyzeMobileCapabilitySources({ sources });
  state = transitionMobileCapabilityAuditState(state, MobileCapabilityAuditEvent.AuditComplete);
  const markdown = renderMobileCapabilityAuditMarkdown(audit);
  const absoluteOutputPath = path.join(root, outputPath);
  await mkdir(path.dirname(absoluteOutputPath), { recursive: true });
  await writeFile(absoluteOutputPath, markdown);
  state = transitionMobileCapabilityAuditState(state, MobileCapabilityAuditEvent.ReportWritten);
  return { ...audit, state, outputPath };
}

async function readActualSources(root) {
  const filePaths = [...new Set(TARGETS.flatMap((entry) => entry.files))];
  const sources = {};
  for (const relativePath of filePaths) {
    const absolutePath = path.join(root, relativePath);
    if (!existsSync(absolutePath)) {
      continue;
    }
    sources[relativePath] = await readFile(absolutePath, "utf8");
  }
  return sources;
}

function target(id, label, { files, evidence, priority }) {
  return Object.freeze({ id, label, files, evidence, priority });
}

function analyzeTarget(entry, sources) {
  const missingFiles = entry.files.filter((file) => !Object.hasOwn(sources, file));
  const combined = entry.files.map((file) => sources[file] ?? "").join("\n");
  const missingEvidence = entry.evidence.filter((needle) => !combined.includes(needle));
  return {
    id: entry.id,
    label: entry.label,
    status:
      missingFiles.length === 0 && missingEvidence.length === 0 ? STATUS.Covered : STATUS.Missing,
    missingFiles,
    missingEvidence,
    priority: entry.priority,
  };
}

function pickNextImplementationTarget(targetsNeedingWork) {
  if (targetsNeedingWork.length === 0) {
    return null;
  }
  return [...targetsNeedingWork].sort((left, right) => right.priority - left.priority)[0];
}

function code(value) {
  return `\`${value}\``;
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1]);

if (isMain) {
  runMobileCapabilityAudit()
    .then((audit) => {
      console.log("phase004_mobile_capability_audit=completed");
      console.log(`audit_state=${audit.state}`);
      console.log(`covered=${audit.summary.covered}`);
      console.log(`missing=${audit.summary.missing}`);
      console.log(`targets_needing_work=${audit.summary.targetsNeedingWork}`);
      console.log(`output_path=${audit.outputPath}`);
      process.exitCode = audit.summary.targetsNeedingWork === 0 ? 0 : 1;
    })
    .catch((error) => {
      console.error("phase004_mobile_capability_audit=failed");
      console.error(
        `error_code=${error?.code ?? "PHASE004_MOBILE_CAPABILITY_UNEXPECTED_FAILURE"}`,
      );
      process.exitCode = 1;
    });
}
