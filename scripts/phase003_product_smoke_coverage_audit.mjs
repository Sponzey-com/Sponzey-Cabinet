import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ProductSmokeCoverageAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const ProductSmokeCoverageAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const ProductSmokeCoverageAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_PRODUCT_SMOKE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE003_PRODUCT_SMOKE_SOURCE_SET_EMPTY",
  ReportWriteFailed: "PHASE003_PRODUCT_SMOKE_REPORT_WRITE_FAILED",
});

const STATUS = Object.freeze({
  ProductSmokeWired: "product smoke wired",
  ContractSmokeOnly: "contract smoke only",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("web_browser_product_smoke", "Web browser product smoke with built bundle", {
    productFiles: [
      "scripts/run_browser_smoke.sh",
      "scripts/run_browser_smoke.mjs",
      "scripts/build_web_app.mjs",
    ],
    productEvidence: [
      "node scripts/build_web_app.mjs",
      "run_web_app.mjs",
      "browser_smoke=passed",
      "CodeMirror editor mounted",
      "Markdown preview table rendered",
      "Restore flow completed",
    ],
    contractEvidence: [],
    contractFiles: [],
    priority: 100,
  }),
  target("self_host_e2e_product_smoke", "Self-host server E2E product smoke", {
    productFiles: [
      "scripts/run_self_host_e2e_smoke.sh",
      "scripts/run_self_host_e2e_smoke.mjs",
    ],
    productEvidence: [
      "scripts/run_self_host_server.sh",
      "waitForServer",
      "/api/auth/login",
      "search_under_300ms_target",
      "product_log_sensitive_exclusion",
      "self_host_e2e_smoke=passed",
    ],
    contractEvidence: [],
    contractFiles: [],
    priority: 98,
  }),
  target("desktop_remote_product_smoke", "Desktop remote workspace product smoke", {
    productFiles: [
      "scripts/run_desktop_remote_product_smoke.sh",
      "scripts/run_desktop_remote_product_smoke.mjs",
      "apps/desktop/tests/desktop_remote_product_smoke.ts",
    ],
    productEvidence: [
      "scripts/run_self_host_server.sh",
      "runDesktopSmoke",
      "desktop_remote_product_smoke=passed",
      "must not save remote document locally",
      "assertSensitiveOutputClean",
    ],
    contractEvidence: [],
    contractFiles: [],
    priority: 96,
  }),
  target("mobile_read_skeleton_smoke", "Mobile read-only skeleton smoke", {
    productFiles: [
      "scripts/run_mobile_read_product_smoke.sh",
      "scripts/run_mobile_read_product_smoke.mjs",
      "apps/mobile/tests/mobile_read_product_smoke.ts",
    ],
    contractFiles: [
      "scripts/run_mobile_read_contract_tests.sh",
      "scripts/run_mobile_read_contract_tests.mjs",
      "apps/mobile/tests/mobile_read_skeleton_tests.ts",
      "packages/client-core/tests/mobile_read_contract_tests.ts",
    ],
    productEvidence: [
      "mobile_read_product_smoke=passed",
      "scripts/run_self_host_server.sh",
    ],
    contractEvidence: [
      "mobile_read_boundary_scan=passed",
      "createMobileReadSelfHostApiClient",
      "MOBILE_UNSUPPORTED_EDIT",
      "supportsMobileReadApi",
      "supportsRemoteEdit",
      "ios",
      "android",
    ],
    priority: 94,
  }),
]);

class ProductSmokeCoverageAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "ProductSmokeCoverageAuditError";
    this.code = code;
  }
}

export function transitionProductSmokeCoverageAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${ProductSmokeCoverageAuditState.NotStarted}:${ProductSmokeCoverageAuditEvent.Start}`,
      ProductSmokeCoverageAuditState.ReadingSource,
    ],
    [
      `${ProductSmokeCoverageAuditState.ReadingSource}:${ProductSmokeCoverageAuditEvent.SourceLoaded}`,
      ProductSmokeCoverageAuditState.Auditing,
    ],
    [
      `${ProductSmokeCoverageAuditState.Auditing}:${ProductSmokeCoverageAuditEvent.AuditComplete}`,
      ProductSmokeCoverageAuditState.Reported,
    ],
    [
      `${ProductSmokeCoverageAuditState.Reported}:${ProductSmokeCoverageAuditEvent.ReportWritten}`,
      ProductSmokeCoverageAuditState.Reported,
    ],
    [
      `${ProductSmokeCoverageAuditState.ReadingSource}:${ProductSmokeCoverageAuditEvent.Fail}`,
      ProductSmokeCoverageAuditState.Failed,
    ],
    [
      `${ProductSmokeCoverageAuditState.Auditing}:${ProductSmokeCoverageAuditEvent.Fail}`,
      ProductSmokeCoverageAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new ProductSmokeCoverageAuditError(
      ProductSmokeCoverageAuditErrorCode.InvalidTransition,
      `invalid product smoke coverage audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeProductSmokeCoverageSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new ProductSmokeCoverageAuditError(
      ProductSmokeCoverageAuditErrorCode.SourceSetEmpty,
      "phase003 product smoke coverage audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter(
    (entry) => entry.status !== STATUS.ProductSmokeWired,
  );

  return {
    phase: "Phase 003.3",
    sourceFiles: Object.keys(sources).sort(),
    summary: {
      totalTargets: targets.length,
      productSmokeWired: countStatus(targets, STATUS.ProductSmokeWired),
      contractSmokeOnly: countStatus(targets, STATUS.ContractSmokeOnly),
      missing: countStatus(targets, STATUS.Missing),
      targetsNeedingWork: targetsNeedingWork.length,
    },
    findings:
      targetsNeedingWork.length === 0
        ? []
        : [
            {
              id: "PHASE003_PRODUCT_SMOKE_COVERAGE_GAP",
              message:
                "Some Phase 003.3 client product smoke targets are missing or contract-only.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ],
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderProductSmokeCoverageAuditMarkdown(audit) {
  const lines = [
    "# Phase 003 Product Smoke Coverage Audit",
    "",
    "현재 단계: Phase 003.3 - Product Smoke for Web, Desktop, and Mobile Baselines",
    "",
    "## Purpose",
    "",
    "- client product smoke 범위를 코드 evidence 기준으로 고정한다.",
    "- contract smoke only 상태를 product smoke passed 상태로 오판하지 않는다.",
    "- 다음 task는 모든 product smoke gap을 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| total targets | ${audit.summary.totalTargets} |`,
    `| product smoke wired | ${audit.summary.productSmokeWired} |`,
    `| contract smoke only | ${audit.summary.contractSmokeOnly} |`,
    `| missing | ${audit.summary.missing} |`,
    `| targets needing work | ${audit.summary.targetsNeedingWork} |`,
    "",
    "## Target Status",
    "",
    "| Target | Label | Status | Missing Files | Missing Product Evidence | Missing Contract Evidence |",
    "| --- | --- | --- | --- | --- | --- |",
    ...audit.targets.map((entry) => {
      const missingFiles =
        entry.missingFiles.length > 0 ? entry.missingFiles.map(code).join(", ") : "none";
      const missingProductEvidence =
        entry.missingProductEvidence.length > 0
          ? entry.missingProductEvidence.map(code).join(", ")
          : "none";
      const missingContractEvidence =
        entry.missingContractEvidence.length > 0
          ? entry.missingContractEvidence.map(code).join(", ")
          : "none";
      return `| \`${entry.id}\` | ${entry.label} | ${entry.status} | ${missingFiles} | ${missingProductEvidence} | ${missingContractEvidence} |`;
    }),
    "",
    "## Findings",
    "",
  ];

  if (audit.findings.length === 0) {
    lines.push("- No product smoke coverage gap was detected.");
  } else {
    for (const finding of audit.findings) {
      lines.push(`- ${finding.id}: ${finding.message}`);
      lines.push(`- affected target count: ${finding.targetIds.length}`);
      lines.push(`- affected targets: ${finding.targetIds.map(code).join(", ")}`);
    }
  }

  lines.push("", "## Next implementation target", "");
  if (audit.nextImplementationTarget) {
    lines.push(
      `- target id: \`${audit.nextImplementationTarget.id}\``,
      `- label: ${audit.nextImplementationTarget.label}`,
      `- current status: ${audit.nextImplementationTarget.status}`,
      "- selected reason: highest priority Phase 003.3 product smoke target that is not product smoke wired.",
      "- next task should implement or harden this selected target only.",
    );
  } else {
    lines.push("- No next product smoke target remains.");
  }

  lines.push(
    "",
    "## Review Notes",
    "",
    "- Product smoke must run through actual process/client boundaries, not just static contract fixtures.",
    "- Web smoke must build the Web bundle and exercise the browser runtime.",
    "- Desktop remote smoke must connect to the self-host server and must not write remote documents through local repositories.",
    "- Mobile smoke must keep iOS/Android read-only behavior explicit and must not leak platform SDK types into shared domain/usecase code.",
    "- Product smoke output must not contain document bodies, tokens, secrets, credentials, or asset content.",
  );

  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const reportPath = path.join(repoRoot, ".tasks/phase003/product-smoke-coverage-audit.md");
  let state = ProductSmokeCoverageAuditState.NotStarted;
  try {
    state = transitionProductSmokeCoverageAuditState(
      state,
      ProductSmokeCoverageAuditEvent.Start,
    );
    const sources = await readProjectSources(repoRoot);
    state = transitionProductSmokeCoverageAuditState(
      state,
      ProductSmokeCoverageAuditEvent.SourceLoaded,
    );
    const audit = analyzeProductSmokeCoverageSources({ sources });
    state = transitionProductSmokeCoverageAuditState(
      state,
      ProductSmokeCoverageAuditEvent.AuditComplete,
    );
    await mkdir(path.dirname(reportPath), { recursive: true });
    await writeFile(reportPath, renderProductSmokeCoverageAuditMarkdown(audit), "utf8");
    console.log("phase003_product_smoke_coverage_audit=passed");
    console.log(`total_targets=${audit.summary.totalTargets}`);
    console.log(`product_smoke_wired=${audit.summary.productSmokeWired}`);
    console.log(`contract_smoke_only=${audit.summary.contractSmokeOnly}`);
    console.log(`missing_targets=${audit.summary.missing}`);
    console.log(`targets_needing_work=${audit.summary.targetsNeedingWork}`);
    console.log(`next_target_id=${audit.nextImplementationTarget?.id ?? "none"}`);
    console.log(`report_path=${reportPath}`);
    return transitionProductSmokeCoverageAuditState(
      state,
      ProductSmokeCoverageAuditEvent.ReportWritten,
    );
  } catch (error) {
    if (
      state === ProductSmokeCoverageAuditState.ReadingSource ||
      state === ProductSmokeCoverageAuditState.Auditing
    ) {
      transitionProductSmokeCoverageAuditState(
        state,
        ProductSmokeCoverageAuditEvent.Fail,
      );
    }
    const code = error?.code ?? ProductSmokeCoverageAuditErrorCode.ReportWriteFailed;
    console.error(`phase003_product_smoke_coverage_audit=failed code=${code}`);
    throw error;
  }
}

function target(id, label, options) {
  return {
    id,
    label,
    productFiles: options.productFiles,
    contractFiles: options.contractFiles,
    productEvidence: options.productEvidence,
    contractEvidence: options.contractEvidence,
    priority: options.priority,
  };
}

function analyzeTarget(entry, sources) {
  const missingProductFiles = entry.productFiles.filter((file) => !hasSource(sources, file));
  const missingContractFiles = entry.contractFiles.filter((file) => !hasSource(sources, file));
  const allFiles = [...entry.productFiles, ...entry.contractFiles];
  const combinedSource = allFiles.map((file) => sources[file] ?? "").join("\n");
  const missingProductEvidence = entry.productEvidence.filter(
    (evidence) => !combinedSource.includes(evidence),
  );
  const missingContractEvidence = entry.contractEvidence.filter(
    (evidence) => !combinedSource.includes(evidence),
  );
  return {
    id: entry.id,
    label: entry.label,
    status: classifyStatus({
      entry,
      missingProductFiles,
      missingContractFiles,
      missingProductEvidence,
      missingContractEvidence,
    }),
    missingFiles: [...missingProductFiles, ...missingContractFiles],
    missingProductFiles,
    missingContractFiles,
    missingProductEvidence,
    missingContractEvidence,
    priority: entry.priority,
  };
}

function classifyStatus({
  entry,
  missingProductFiles,
  missingContractFiles,
  missingProductEvidence,
  missingContractEvidence,
}) {
  if (missingProductFiles.length === 0 && missingProductEvidence.length === 0) {
    return STATUS.ProductSmokeWired;
  }
  if (
    missingContractFiles.length === 0 &&
    entry.contractEvidence.length > 0 &&
    missingContractEvidence.length === 0
  ) {
    return STATUS.ContractSmokeOnly;
  }
  return STATUS.Missing;
}

function pickNextImplementationTarget(targetsNeedingWork) {
  return [...targetsNeedingWork].sort((left, right) => right.priority - left.priority)[0] ?? null;
}

function countStatus(targets, status) {
  return targets.filter((target) => target.status === status).length;
}

function code(value) {
  return `\`${value}\``;
}

function hasSource(sources, file) {
  return Object.prototype.hasOwnProperty.call(sources, file);
}

async function readProjectSources(repoRoot) {
  const files = [
    ...new Set(TARGETS.flatMap((entry) => [...entry.productFiles, ...entry.contractFiles])),
  ].sort();
  const entries = await Promise.all(
    files.map(async (file) => {
      try {
        return [file, await readFile(path.join(repoRoot, file), "utf8")];
      } catch {
        return null;
      }
    }),
  );
  return Object.fromEntries(entries.filter(Boolean));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
