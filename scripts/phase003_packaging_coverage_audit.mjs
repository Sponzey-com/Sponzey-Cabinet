import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const PackagingCoverageAuditState = Object.freeze({
  Pending: "Pending",
  ReadingSources: "ReadingSources",
  Classifying: "Classifying",
  Reported: "Reported",
  Failed: "Failed",
});

export const PackagingCoverageAuditEvent = Object.freeze({
  Start: "Start",
  SourcesRead: "SourcesRead",
  Classified: "Classified",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const PackagingCoverageAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_PACKAGING_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE003_PACKAGING_SOURCE_SET_EMPTY",
  ReportWriteFailed: "PHASE003_PACKAGING_REPORT_WRITE_FAILED",
});

const STATUS = Object.freeze({
  Covered: "covered",
  Partial: "partial",
  Missing: "missing",
});

const TARGETS = Object.freeze([
  target("server_package_smoke", "Self-host server package smoke", {
    requiredFiles: [
      "scripts/run_self_host_server_package_smoke.sh",
      "scripts/run_self_host_server_package_smoke.mjs",
    ],
    requiredEvidence: [
      "cargo build -p cabinet-server",
      "cabinet-server",
      "--self-host-package-smoke",
      "server_package_smoke=passed",
      "assertSensitiveOutputClean",
    ],
    supportFiles: [],
    supportEvidence: [],
    priority: 100,
  }),
  target("web_asset_serving_verification", "Web asset build and serving verification", {
    requiredFiles: [
      "scripts/build_web_app.mjs",
      "scripts/run_browser_smoke.sh",
      "scripts/run_browser_smoke.mjs",
      "scripts/run_web_app.mjs",
    ],
    requiredEvidence: [
      "node scripts/build_web_app.mjs",
      "scripts/run_web_app.mjs",
      "waitForHttp",
      "browser_smoke=passed",
      "CodeMirror editor mounted",
    ],
    supportFiles: [],
    supportEvidence: [],
    priority: 96,
  }),
  target("desktop_packaging_smoke", "Desktop packaging and bundled asset smoke", {
    requiredFiles: [
      "scripts/build_desktop_assets.mjs",
      "scripts/run_desktop_package_smoke.sh",
      "scripts/run_desktop_tauri_build.sh",
      "scripts/run_desktop_packaged_app_smoke.sh",
      "scripts/run_desktop_dist_browser_smoke.sh",
      "apps/desktop/src-tauri/src/main.rs",
      "apps/desktop/src-tauri/src/lib.rs",
    ],
    requiredEvidence: [
      "node scripts/build_desktop_assets.mjs",
      "tauri build",
      "--bundles app",
      "--packaged-smoke",
      "create_desktop_package_smoke_report",
      "packaged_runtime_smoke_does_not_require_node",
      "SPONZEY_CABINET_WEB_PUBLIC_DIR=apps/desktop/dist",
      "packaged_app_binary_found=true",
    ],
    supportFiles: [],
    supportEvidence: [],
    priority: 94,
  }),
  target("install_once_clean_start", "Install once clean start smoke", {
    requiredFiles: [
      "scripts/run_local_app.sh",
      "crates/cabinet-platform/src/bin/cabinet_local.rs",
      "crates/cabinet-platform/src/release_smoke.rs",
      "crates/cabinet-platform/tests/clean_install_smoke.rs",
    ],
    requiredEvidence: [
      "cargo run --quiet -p cabinet-platform --bin cabinet-local",
      "run_clean_install_smoke",
      "first_run_completed",
      "setup_healthy",
      "already_present_directories",
      "without_external_services",
      "created_directories",
    ],
    supportFiles: [],
    supportEvidence: [],
    priority: 92,
  }),
  target("upgrade_migration_command_flow", "Upgrade and migration command flow", {
    requiredFiles: [
      "scripts/run_self_host_upgrade_smoke.sh",
      "scripts/run_self_host_upgrade_smoke.mjs",
    ],
    requiredEvidence: [
      "run_self_host_upgrade_smoke",
      "migration_state_machine",
      "upgrade_migration_smoke=passed",
      "assertSensitiveOutputClean",
    ],
    supportFiles: [
      "crates/cabinet-core/src/migration.rs",
      "crates/cabinet-core/tests/migration_tests.rs",
      "crates/cabinet-platform/src/release_smoke.rs",
      "crates/cabinet-platform/tests/data_preservation_smoke.rs",
      "crates/cabinet-platform/tests/phase002_migration_fixture_smoke.rs",
    ],
    supportEvidence: [
      "MigrationState",
      "MigrationEvent",
      "MigrationRunner",
      "migration_transitions_to_completed_through_explicit_events",
      "migration_runner_is_idempotent_when_initial_version_is_already_recorded",
      "run_data_preservation_smoke",
      "run_phase002_migration_fixture_smoke",
      "migration_idempotent",
      "migration_failure_preserved_current_fixture",
    ],
    priority: 98,
  }),
]);

class PackagingCoverageAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "PackagingCoverageAuditError";
    this.code = code;
  }
}

export function transitionPackagingCoverageAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${PackagingCoverageAuditState.Pending}:${PackagingCoverageAuditEvent.Start}`,
      PackagingCoverageAuditState.ReadingSources,
    ],
    [
      `${PackagingCoverageAuditState.ReadingSources}:${PackagingCoverageAuditEvent.SourcesRead}`,
      PackagingCoverageAuditState.Classifying,
    ],
    [
      `${PackagingCoverageAuditState.Classifying}:${PackagingCoverageAuditEvent.Classified}`,
      PackagingCoverageAuditState.Reported,
    ],
    [
      `${PackagingCoverageAuditState.Reported}:${PackagingCoverageAuditEvent.ReportWritten}`,
      PackagingCoverageAuditState.Reported,
    ],
    [
      `${PackagingCoverageAuditState.ReadingSources}:${PackagingCoverageAuditEvent.Fail}`,
      PackagingCoverageAuditState.Failed,
    ],
    [
      `${PackagingCoverageAuditState.Classifying}:${PackagingCoverageAuditEvent.Fail}`,
      PackagingCoverageAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new PackagingCoverageAuditError(
      PackagingCoverageAuditErrorCode.InvalidTransition,
      `invalid packaging coverage audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzePackagingCoverageSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new PackagingCoverageAuditError(
      PackagingCoverageAuditErrorCode.SourceSetEmpty,
      "phase003 packaging coverage audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.status !== STATUS.Covered);

  return {
    phase: "Phase 003.4",
    sourceFiles: Object.keys(sources).sort(),
    summary: {
      totalTargets: targets.length,
      covered: countStatus(targets, STATUS.Covered),
      partial: countStatus(targets, STATUS.Partial),
      missing: countStatus(targets, STATUS.Missing),
      targetsNeedingWork: targetsNeedingWork.length,
    },
    findings:
      targetsNeedingWork.length === 0
        ? []
        : [
            {
              id: "PHASE003_PACKAGING_COVERAGE_GAP",
              message:
                "Some Phase 003.4 packaging, install, or upgrade targets are missing or partial.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ],
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderPackagingCoverageAuditMarkdown(audit) {
  const lines = [
    "# Phase 003 Packaging Coverage Audit",
    "",
    "현재 단계: Phase 003.4 - Packaging, Install Once, and Upgrade Flow",
    "",
    "## Purpose",
    "",
    "- Phase 003.4 packaging/install/upgrade 범위를 코드 evidence 기준으로 고정한다.",
    "- install once와 upgrade flow를 product smoke passed 상태로 오판하지 않는다.",
    "- 다음 task는 모든 packaging gap을 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| total targets | ${audit.summary.totalTargets} |`,
    `| covered | ${audit.summary.covered} |`,
    `| partial | ${audit.summary.partial} |`,
    `| missing | ${audit.summary.missing} |`,
    `| targets needing work | ${audit.summary.targetsNeedingWork} |`,
    "",
    "## Target Status",
    "",
    "| Target | Label | Status | Missing Files | Missing Required Evidence | Missing Support Evidence |",
    "| --- | --- | --- | --- | --- | --- |",
    ...audit.targets.map((entry) => {
      const missingFiles =
        entry.missingFiles.length > 0 ? entry.missingFiles.map(code).join(", ") : "none";
      const missingRequiredEvidence =
        entry.missingRequiredEvidence.length > 0
          ? entry.missingRequiredEvidence.map(code).join(", ")
          : "none";
      const missingSupportEvidence =
        entry.missingSupportEvidence.length > 0
          ? entry.missingSupportEvidence.map(code).join(", ")
          : "none";
      return `| \`${entry.id}\` | ${entry.label} | ${entry.status} | ${missingFiles} | ${missingRequiredEvidence} | ${missingSupportEvidence} |`;
    }),
    "",
    "## Findings",
    "",
  ];

  if (audit.findings.length === 0) {
    lines.push("- No packaging coverage gap was detected.");
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
      "- selected reason: highest priority Phase 003.4 packaging target that is not covered.",
      "- next task should implement or harden this selected target only.",
    );
  } else {
    lines.push("- No next packaging target remains.");
  }

  lines.push(
    "",
    "## Review Notes",
    "",
    "- Server package smoke must exercise a built self-host server artifact rather than only a source-level unit test.",
    "- Web asset verification must build assets and serve them through the product runtime boundary.",
    "- Desktop packaging smoke must verify bundled assets without relying on Node.js at packaged runtime.",
    "- Install once clean start must not require external DB, external search server, Git CLI, manual environment variables, or manual config file edits.",
    "- Upgrade/migration command flow must be executable and must preserve existing data through explicit migration state machine evidence.",
    "- Audit output must not contain document bodies, tokens, secrets, credentials, or asset content.",
  );

  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const reportPath = path.join(repoRoot, ".tasks/phase003/packaging-coverage-audit.md");
  let state = PackagingCoverageAuditState.Pending;
  try {
    state = transitionPackagingCoverageAuditState(state, PackagingCoverageAuditEvent.Start);
    const sources = await readProjectSources(repoRoot);
    state = transitionPackagingCoverageAuditState(
      state,
      PackagingCoverageAuditEvent.SourcesRead,
    );
    const audit = analyzePackagingCoverageSources({ sources });
    state = transitionPackagingCoverageAuditState(
      state,
      PackagingCoverageAuditEvent.Classified,
    );
    await mkdir(path.dirname(reportPath), { recursive: true });
    await writeFile(reportPath, renderPackagingCoverageAuditMarkdown(audit), "utf8");
    console.log("phase003_packaging_coverage_audit=passed");
    console.log(`total_targets=${audit.summary.totalTargets}`);
    console.log(`covered_targets=${audit.summary.covered}`);
    console.log(`partial_targets=${audit.summary.partial}`);
    console.log(`missing_targets=${audit.summary.missing}`);
    console.log(`targets_needing_work=${audit.summary.targetsNeedingWork}`);
    console.log(`next_target_id=${audit.nextImplementationTarget?.id ?? "none"}`);
    console.log(`report_path=${reportPath}`);
    return transitionPackagingCoverageAuditState(
      state,
      PackagingCoverageAuditEvent.ReportWritten,
    );
  } catch (error) {
    if (
      state === PackagingCoverageAuditState.ReadingSources ||
      state === PackagingCoverageAuditState.Classifying
    ) {
      transitionPackagingCoverageAuditState(state, PackagingCoverageAuditEvent.Fail);
    }
    const code = error?.code ?? PackagingCoverageAuditErrorCode.ReportWriteFailed;
    console.error(`phase003_packaging_coverage_audit=failed code=${code}`);
    throw error;
  }
}

function target(id, label, options) {
  return {
    id,
    label,
    requiredFiles: options.requiredFiles,
    requiredEvidence: options.requiredEvidence,
    supportFiles: options.supportFiles,
    supportEvidence: options.supportEvidence,
    priority: options.priority,
  };
}

function analyzeTarget(entry, sources) {
  const missingRequiredFiles = entry.requiredFiles.filter((file) => !hasSource(sources, file));
  const missingSupportFiles = entry.supportFiles.filter((file) => !hasSource(sources, file));
  const combinedSource = [...entry.requiredFiles, ...entry.supportFiles]
    .map((file) => sources[file] ?? "")
    .join("\n");
  const missingRequiredEvidence = entry.requiredEvidence.filter(
    (evidence) => !combinedSource.includes(evidence),
  );
  const missingSupportEvidence = entry.supportEvidence.filter(
    (evidence) => !combinedSource.includes(evidence),
  );

  return {
    id: entry.id,
    label: entry.label,
    status: classifyStatus({
      entry,
      missingRequiredFiles,
      missingRequiredEvidence,
      missingSupportFiles,
      missingSupportEvidence,
    }),
    missingFiles: [...missingRequiredFiles, ...missingSupportFiles],
    missingRequiredFiles,
    missingSupportFiles,
    missingRequiredEvidence,
    missingSupportEvidence,
    priority: entry.priority,
  };
}

function classifyStatus({
  entry,
  missingRequiredFiles,
  missingRequiredEvidence,
  missingSupportFiles,
  missingSupportEvidence,
}) {
  if (missingRequiredFiles.length === 0 && missingRequiredEvidence.length === 0) {
    return STATUS.Covered;
  }

  if (
    entry.supportFiles.length > 0 &&
    missingSupportFiles.length === 0 &&
    missingSupportEvidence.length === 0
  ) {
    return STATUS.Partial;
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
    ...new Set(TARGETS.flatMap((entry) => [...entry.requiredFiles, ...entry.supportFiles])),
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
