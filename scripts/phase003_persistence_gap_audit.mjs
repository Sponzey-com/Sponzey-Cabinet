import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const PersistenceGapAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const PersistenceGapAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const PersistenceGapAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_PERSISTENCE_INVALID_TRANSITION",
  SourceSetEmpty: "PHASE003_PERSISTENCE_SOURCE_SET_EMPTY",
  SourceReadFailed: "PHASE003_PERSISTENCE_SOURCE_READ_FAILED",
  ReportWriteFailed: "PHASE003_PERSISTENCE_REPORT_WRITE_FAILED",
});

const STATUS = Object.freeze({
  DurableAdapterWired: "durable adapter wired",
  VolatileAdapterOnly: "volatile adapter only",
  ContractCompleteOnly: "contract complete only",
  PortDefinedOnly: "port defined only",
  Missing: "missing",
});

const TARGETS = [
  target("document_current_store", "Document current store", {
    portFiles: ["crates/cabinet-ports/src/document_repository.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_document_repository.rs"],
    contractTestFiles: ["crates/cabinet-adapters/tests/local_document_repository_tests.rs"],
    priority: 90,
  }),
  target("version_history_store", "Version history store", {
    portFiles: ["crates/cabinet-ports/src/version_store.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_version_store.rs"],
    contractTestFiles: ["crates/cabinet-adapters/tests/local_version_store_tests.rs"],
    priority: 88,
  }),
  target("document_asset_metadata_store", "Document asset metadata store", {
    portFiles: ["crates/cabinet-ports/src/document_asset_repository.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_document_asset_repository.rs"],
    contractTestFiles: [
      "crates/cabinet-adapters/tests/local_document_asset_repository_tests.rs",
    ],
    priority: 84,
  }),
  target("object_storage", "Object storage", {
    portFiles: ["crates/cabinet-ports/src/object_storage.rs"],
    adapterFiles: [
      "crates/cabinet-adapters/src/local_object_storage.rs",
      "crates/cabinet-adapters/src/fake_s3_object_storage.rs",
    ],
    contractTestFiles: [
      "crates/cabinet-adapters/tests/object_storage_adapter_contract_tests.rs",
      "crates/cabinet-adapters/tests/object_storage_contract_tests.rs",
    ],
    priority: 80,
  }),
  target("search_index", "Search index", {
    portFiles: ["crates/cabinet-ports/src/search_index.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_search_index.rs"],
    contractTestFiles: ["crates/cabinet-adapters/tests/local_search_index_tests.rs"],
    priority: 78,
  }),
  target("link_index", "Link index", {
    portFiles: ["crates/cabinet-ports/src/link_index.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_link_index.rs"],
    contractTestFiles: ["crates/cabinet-adapters/tests/local_link_index_tests.rs"],
    priority: 76,
  }),
  target("session_store", "Session store", {
    portFiles: ["crates/cabinet-ports/src/auth.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_auth.rs"],
    contractTestFiles: ["crates/cabinet-adapters/tests/local_auth_adapter_tests.rs"],
    durableEvidence: ["LocalSessionStore"],
    volatileEvidence: ["InMemorySessionStore"],
    priority: 100,
  }),
  target("user_repository", "User repository", {
    portFiles: ["crates/cabinet-ports/src/user_repository.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_user_repository.rs"],
    contractTestFiles: ["crates/cabinet-ports/tests/user_repository_contract_tests.rs"],
    priority: 98,
  }),
  target("group_repository", "Group repository", {
    portFiles: ["crates/cabinet-ports/src/group_repository.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_group_repository.rs"],
    contractTestFiles: ["crates/cabinet-ports/tests/group_repository_contract_tests.rs"],
    priority: 96,
  }),
  target("permission_policy_repository", "Permission policy repository", {
    portFiles: ["crates/cabinet-ports/src/permission_policy_repository.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_permission_policy_repository.rs"],
    contractTestFiles: [
      "crates/cabinet-ports/tests/permission_policy_repository_contract_tests.rs",
    ],
    priority: 94,
  }),
  target("comment_repository", "Comment repository", {
    portFiles: ["crates/cabinet-ports/src/comment_repository.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_comment_repository.rs"],
    contractTestFiles: ["crates/cabinet-ports/tests/comment_repository_contract_tests.rs"],
    priority: 92,
  }),
  target("review_workflow_repository", "Review workflow repository", {
    portFiles: ["crates/cabinet-ports/src/review_workflow.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_review_workflow_repository.rs"],
    contractTestFiles: ["crates/cabinet-ports/tests/review_workflow_contract_tests.rs"],
    priority: 90,
  }),
  target("document_lock_repository", "Document lock repository", {
    portFiles: ["crates/cabinet-ports/src/document_lock.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_document_lock_repository.rs"],
    contractTestFiles: ["crates/cabinet-ports/tests/document_lock_contract_tests.rs"],
    priority: 88,
  }),
  target("audit_log_store", "Audit log store", {
    portFiles: ["crates/cabinet-ports/src/audit_log.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_audit_log_store.rs"],
    contractTestFiles: ["crates/cabinet-ports/tests/audit_log_store_contract_tests.rs"],
    priority: 86,
  }),
  target("backup_job_store", "Backup job store", {
    portFiles: ["crates/cabinet-ports/src/backup_store.rs"],
    adapterFiles: ["crates/cabinet-adapters/src/local_backup_store.rs"],
    contractTestFiles: ["crates/cabinet-ports/tests/backup_store_contract_tests.rs"],
    priority: 84,
  }),
];

class PersistenceGapAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "PersistenceGapAuditError";
    this.code = code;
  }
}

export function transitionPersistenceGapAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${PersistenceGapAuditState.NotStarted}:${PersistenceGapAuditEvent.Start}`,
      PersistenceGapAuditState.ReadingSource,
    ],
    [
      `${PersistenceGapAuditState.ReadingSource}:${PersistenceGapAuditEvent.SourceLoaded}`,
      PersistenceGapAuditState.Auditing,
    ],
    [
      `${PersistenceGapAuditState.Auditing}:${PersistenceGapAuditEvent.AuditComplete}`,
      PersistenceGapAuditState.Reported,
    ],
    [
      `${PersistenceGapAuditState.Reported}:${PersistenceGapAuditEvent.ReportWritten}`,
      PersistenceGapAuditState.Reported,
    ],
    [
      `${PersistenceGapAuditState.ReadingSource}:${PersistenceGapAuditEvent.Fail}`,
      PersistenceGapAuditState.Failed,
    ],
    [
      `${PersistenceGapAuditState.Auditing}:${PersistenceGapAuditEvent.Fail}`,
      PersistenceGapAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new PersistenceGapAuditError(
      PersistenceGapAuditErrorCode.InvalidTransition,
      `invalid persistence gap audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzePersistenceSources({ sources }) {
  if (!sources || Object.keys(sources).length === 0) {
    throw new PersistenceGapAuditError(
      PersistenceGapAuditErrorCode.SourceSetEmpty,
      "phase003 persistence audit source set is empty",
    );
  }

  const targets = TARGETS.map((entry) => analyzeTarget(entry, sources));
  const targetsNeedingWork = targets.filter((entry) => entry.needsPersistenceWork);
  const migrationEvidence = analyzeMigrationEvidence(sources);

  return {
    phase: "Phase 003.2",
    sourceFiles: Object.keys(sources).sort(),
    summary: {
      totalTargets: targets.length,
      durableAdapterWired: countStatus(targets, STATUS.DurableAdapterWired),
      volatileAdapterOnly: countStatus(targets, STATUS.VolatileAdapterOnly),
      contractCompleteOnly: countStatus(targets, STATUS.ContractCompleteOnly),
      portDefinedOnly: countStatus(targets, STATUS.PortDefinedOnly),
      missing: countStatus(targets, STATUS.Missing),
      targetsNeedingWork: targetsNeedingWork.length,
    },
    findings:
      targetsNeedingWork.length > 0
        ? [
            {
              id: "PHASE003_PERSISTENCE_GAP",
              message:
                "Some Phase 003.2 persistence targets are not durable local adapters with contract evidence.",
              targetIds: targetsNeedingWork.map((entry) => entry.id),
            },
          ]
        : [],
    migrationEvidence,
    nextImplementationTarget: pickNextImplementationTarget(targetsNeedingWork),
    targets,
  };
}

export function renderPersistenceGapAuditMarkdown(audit) {
  const lines = [
    "# Phase 003 Persistence Gap Audit",
    "",
    "현재 단계: Phase 003.2 - Persistence, Migration, and Recovery Hardening",
    "",
    "## Purpose",
    "",
    "- self-host metadata store coverage를 코드 기준으로 고정한다.",
    "- durable adapter wired, volatile adapter only, contract complete only, port defined only, missing 상태를 구분한다.",
    "- runtime handler wired 상태를 product-ready persistence로 오판하지 않는다.",
    "- 다음 task는 모든 gap을 한 번에 구현하지 않고 가장 높은 우선순위 target 하나를 선택한다.",
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| total targets | ${audit.summary.totalTargets} |`,
    `| durable adapter wired | ${audit.summary.durableAdapterWired} |`,
    `| volatile adapter only | ${audit.summary.volatileAdapterOnly} |`,
    `| contract complete only | ${audit.summary.contractCompleteOnly} |`,
    `| port defined only | ${audit.summary.portDefinedOnly} |`,
    `| missing | ${audit.summary.missing} |`,
    `| targets needing work | ${audit.summary.targetsNeedingWork} |`,
    "",
    "## Target Status",
    "",
    "| Target | Label | Port | Adapter | Contract Test | Status |",
    "| --- | --- | --- | --- | --- | --- |",
    ...audit.targets.map(
      (entry) =>
        `| \`${entry.id}\` | ${entry.label} | ${yesNo(entry.portPresent)} | ${yesNo(entry.adapterPresent)} | ${yesNo(entry.contractTestPresent)} | ${entry.status} |`,
    ),
    "",
    "## Migration and Recovery Evidence",
    "",
    `- migration state machine present: ${yesNo(audit.migrationEvidence.stateMachinePresent)}`,
    `- migration tests present: ${yesNo(audit.migrationEvidence.testsPresent)}`,
    `- Phase 002 fixture smoke present: ${yesNo(audit.migrationEvidence.phase002FixtureSmokePresent)}`,
    "",
    "## Findings",
    "",
  ];

  if (audit.findings.length === 0) {
    lines.push("- No persistence gap was detected.");
  } else {
    for (const finding of audit.findings) {
      lines.push(
        `- ${finding.id}: ${finding.message}`,
        `- affected target count: ${finding.targetIds.length}`,
      );
    }
  }

  lines.push("", "## Next implementation target", "");
  if (audit.nextImplementationTarget) {
    lines.push(
      `- target id: \`${audit.nextImplementationTarget.id}\``,
      `- label: ${audit.nextImplementationTarget.label}`,
      `- current status: ${audit.nextImplementationTarget.status}`,
      `- selected reason: highest priority Phase 003.2 target that is not durable adapter wired.`,
      "- next task should implement or harden this selected target only.",
    );
  } else {
    lines.push("- No next persistence target remains.");
  }

  lines.push(
    "",
    "## Review Notes",
    "",
    "- Persistence implementations must stay behind port interfaces.",
    "- Domain and usecase code must not access filesystem, database, network, or environment directly.",
    "- Runtime config must still be read once at bootstrap and injected explicitly.",
    "- Migration and recovery flows with failure states must use explicit state transitions.",
    "- Current document lookup must not scan full version history.",
    "- User-facing reads and searches must preserve the p95 300ms target under indexed/projection state.",
  );

  return `${lines.join("\n")}\n`;
}

async function runCli() {
  let state = PersistenceGapAuditState.NotStarted;
  try {
    state = transitionPersistenceGapAuditState(state, PersistenceGapAuditEvent.Start);
    const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
    const sources = await readProjectSources(repoRoot);
    state = transitionPersistenceGapAuditState(state, PersistenceGapAuditEvent.SourceLoaded);
    const audit = analyzePersistenceSources({ sources });
    state = transitionPersistenceGapAuditState(state, PersistenceGapAuditEvent.AuditComplete);
    const reportPath = path.join(repoRoot, ".tasks/phase003/persistence-gap-audit.md");
    await writeFile(reportPath, renderPersistenceGapAuditMarkdown(audit), "utf8");
    console.log("phase003_persistence_gap_audit=passed");
    console.log(`total_targets=${audit.summary.totalTargets}`);
    console.log(`durable_adapter_wired=${audit.summary.durableAdapterWired}`);
    console.log(`volatile_adapter_only=${audit.summary.volatileAdapterOnly}`);
    console.log(`contract_complete_only=${audit.summary.contractCompleteOnly}`);
    console.log(`port_defined_only=${audit.summary.portDefinedOnly}`);
    console.log(`targets_needing_work=${audit.summary.targetsNeedingWork}`);
    console.log(`next_target_id=${audit.nextImplementationTarget?.id ?? "none"}`);
    console.log(`report_path=${reportPath}`);
    return transitionPersistenceGapAuditState(state, PersistenceGapAuditEvent.ReportWritten);
  } catch (error) {
    if (state === PersistenceGapAuditState.ReadingSource || state === PersistenceGapAuditState.Auditing) {
      transitionPersistenceGapAuditState(state, PersistenceGapAuditEvent.Fail);
    }
    const code = error?.code ?? PersistenceGapAuditErrorCode.ReportWriteFailed;
    console.error(`phase003_persistence_gap_audit=failed code=${code}`);
    throw error;
  }
}

function target(id, label, options) {
  return {
    id,
    label,
    portFiles: options.portFiles,
    adapterFiles: options.adapterFiles,
    contractTestFiles: options.contractTestFiles,
    durableEvidence: options.durableEvidence ?? [],
    volatileEvidence: options.volatileEvidence ?? [],
    priority: options.priority,
  };
}

function analyzeTarget(entry, sources) {
  const portPresent = entry.portFiles.some((file) => hasSource(sources, file));
  const adapterPresent = entry.adapterFiles.some((file) => hasSource(sources, file));
  const contractTestPresent = entry.contractTestFiles.some((file) => hasSource(sources, file));
  const durableAdapter = adapterPresent && entry.durableEvidence.some((evidence) =>
    entry.adapterFiles.some((file) => (sources[file] ?? "").includes(evidence)),
  );
  const volatileAdapter = !durableAdapter && adapterPresent && entry.volatileEvidence.some((evidence) =>
    entry.adapterFiles.some((file) => (sources[file] ?? "").includes(evidence)),
  );
  const status = classifyStatus({
    portPresent,
    adapterPresent,
    contractTestPresent,
    volatileAdapter,
  });
  return {
    id: entry.id,
    label: entry.label,
    portPresent,
    adapterPresent,
    contractTestPresent,
    volatileAdapter,
    status,
    priority: entry.priority,
    needsPersistenceWork: status !== STATUS.DurableAdapterWired,
  };
}

function classifyStatus({ portPresent, adapterPresent, contractTestPresent, volatileAdapter }) {
  if (!portPresent) {
    return STATUS.Missing;
  }
  if (adapterPresent && contractTestPresent && !volatileAdapter) {
    return STATUS.DurableAdapterWired;
  }
  if (adapterPresent && volatileAdapter) {
    return STATUS.VolatileAdapterOnly;
  }
  if (contractTestPresent) {
    return STATUS.ContractCompleteOnly;
  }
  return STATUS.PortDefinedOnly;
}

function analyzeMigrationEvidence(sources) {
  const migrationSource = sources["crates/cabinet-core/src/migration.rs"] ?? "";
  return {
    stateMachinePresent:
      migrationSource.includes("MigrationState") && migrationSource.includes("MigrationEvent"),
    testsPresent: hasSource(sources, "crates/cabinet-core/tests/migration_tests.rs"),
    phase002FixtureSmokePresent: hasSource(
      sources,
      "crates/cabinet-platform/tests/phase002_migration_fixture_smoke.rs",
    ),
  };
}

function pickNextImplementationTarget(targetsNeedingWork) {
  return [...targetsNeedingWork].sort((left, right) => right.priority - left.priority)[0] ?? null;
}

function countStatus(targets, status) {
  return targets.filter((target) => target.status === status).length;
}

function hasSource(sources, file) {
  return Object.prototype.hasOwnProperty.call(sources, file);
}

function yesNo(value) {
  return value ? "yes" : "no";
}

async function readProjectSources(repoRoot) {
  const files = new Set([
    ...TARGETS.flatMap((entry) => [
      ...entry.portFiles,
      ...entry.adapterFiles,
      ...entry.contractTestFiles,
    ]),
    "crates/cabinet-core/src/migration.rs",
    "crates/cabinet-core/tests/migration_tests.rs",
    "crates/cabinet-platform/tests/phase002_migration_fixture_smoke.rs",
  ]);
  for (const dir of [
    "crates/cabinet-adapters/src",
    "crates/cabinet-adapters/tests",
    "crates/cabinet-ports/tests",
  ]) {
    for (const file of await listRustFiles(repoRoot, dir)) {
      files.add(file);
    }
  }

  const entries = await Promise.all(
    [...files].sort().map(async (file) => {
      try {
        return [file, await readFile(path.join(repoRoot, file), "utf8")];
      } catch {
        return null;
      }
    }),
  );
  return Object.fromEntries(entries.filter(Boolean));
}

async function listRustFiles(repoRoot, relativeDir) {
  try {
    const names = await readdir(path.join(repoRoot, relativeDir));
    return names.filter((name) => name.endsWith(".rs")).map((name) => `${relativeDir}/${name}`);
  } catch {
    return [];
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
