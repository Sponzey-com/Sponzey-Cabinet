import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const DurableDependencyManifestAuditState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingSource: "ReadingSource",
  Auditing: "Auditing",
  Reported: "Reported",
  Failed: "Failed",
});

export const DurableDependencyManifestAuditEvent = Object.freeze({
  Start: "Start",
  SourceLoaded: "SourceLoaded",
  AuditComplete: "AuditComplete",
  ReportWritten: "ReportWritten",
  Fail: "Fail",
});

export const DurableDependencyManifestAuditErrorCode = Object.freeze({
  InvalidTransition: "PHASE003_DURABLE_DEPENDENCY_INVALID_TRANSITION",
  ManifestEmpty: "PHASE003_DURABLE_DEPENDENCY_MANIFEST_EMPTY",
  SourceReadFailed: "PHASE003_DURABLE_DEPENDENCY_SOURCE_READ_FAILED",
  ReportWriteFailed: "PHASE003_DURABLE_DEPENDENCY_REPORT_WRITE_FAILED",
});

const REQUIRED_DURABLE_DEPENDENCIES = Object.freeze([
  required("document_repository", "LocalDocumentRepository"),
  required("version_store", "LocalVersionStore"),
  required("document_asset_metadata_store", "LocalDocumentAssetRepository"),
  required("object_storage", "LocalObjectStorage"),
  required("search_index", "LocalSearchIndex"),
  required("link_index", "LocalLinkIndex"),
  required("session_store", "LocalSessionStore"),
  required("user_repository", "LocalUserRepository"),
  required("group_repository", "LocalGroupRepository"),
  required("permission_policy_repository", "LocalPermissionPolicyRepository"),
  required("comment_repository", "LocalCommentRepository"),
  required("review_workflow_repository", "LocalReviewWorkflowRepository"),
  required("document_lock_repository", "LocalDocumentLockRepository"),
  required("audit_store", "LocalAuditLogStore"),
  required("backup_store", "LocalBackupStore"),
]);

class DurableDependencyManifestAuditError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "DurableDependencyManifestAuditError";
    this.code = code;
  }
}

export function transitionDurableDependencyManifestAuditState(state, event) {
  const key = `${state}:${event}`;
  const transitions = new Map([
    [
      `${DurableDependencyManifestAuditState.NotStarted}:${DurableDependencyManifestAuditEvent.Start}`,
      DurableDependencyManifestAuditState.ReadingSource,
    ],
    [
      `${DurableDependencyManifestAuditState.ReadingSource}:${DurableDependencyManifestAuditEvent.SourceLoaded}`,
      DurableDependencyManifestAuditState.Auditing,
    ],
    [
      `${DurableDependencyManifestAuditState.Auditing}:${DurableDependencyManifestAuditEvent.AuditComplete}`,
      DurableDependencyManifestAuditState.Reported,
    ],
    [
      `${DurableDependencyManifestAuditState.Reported}:${DurableDependencyManifestAuditEvent.ReportWritten}`,
      DurableDependencyManifestAuditState.Reported,
    ],
    [
      `${DurableDependencyManifestAuditState.ReadingSource}:${DurableDependencyManifestAuditEvent.Fail}`,
      DurableDependencyManifestAuditState.Failed,
    ],
    [
      `${DurableDependencyManifestAuditState.Auditing}:${DurableDependencyManifestAuditEvent.Fail}`,
      DurableDependencyManifestAuditState.Failed,
    ],
  ]);
  const nextState = transitions.get(key);
  if (!nextState) {
    throw new DurableDependencyManifestAuditError(
      DurableDependencyManifestAuditErrorCode.InvalidTransition,
      `invalid durable dependency manifest audit transition: ${key}`,
    );
  }
  return nextState;
}

export function analyzeDurableDependencyManifestSources({ runtimeSource }) {
  const manifestEntries = parseManifestEntries(runtimeSource);
  const byDependency = new Map(
    manifestEntries.map((entry) => [entry.dependency, entry]),
  );
  const missing = [];
  const wrongImplementation = [];
  const wrongDurability = [];
  const durableLocalWired = [];

  for (const expected of REQUIRED_DURABLE_DEPENDENCIES) {
    const actual = byDependency.get(expected.dependency);
    if (!actual) {
      missing.push(expected);
      continue;
    }
    if (actual.implementation !== expected.implementation) {
      wrongImplementation.push({ expected, actual });
      continue;
    }
    if (actual.durability !== "DurableLocal") {
      wrongDurability.push({ expected, actual });
      continue;
    }
    durableLocalWired.push(actual);
  }

  return {
    phase: "Phase 003.2",
    sourceFiles: ["crates/cabinet-server/src/runtime.rs"],
    summary: {
      manifestEntries: manifestEntries.length,
      requiredDurableDependencies: REQUIRED_DURABLE_DEPENDENCIES.length,
      durableLocalWired: durableLocalWired.length,
      missingDurableDependencies: missing.length,
      wrongImplementation: wrongImplementation.length,
      wrongDurability: wrongDurability.length,
    },
    findings: buildFindings({ missing, wrongImplementation, wrongDurability }),
    nextImplementationTarget: pickNextImplementationTarget({
      missing,
      wrongImplementation,
      wrongDurability,
    }),
    requiredDependencies: REQUIRED_DURABLE_DEPENDENCIES,
    manifestEntries,
    durableLocalWired,
    missing,
    wrongImplementation,
    wrongDurability,
  };
}

export function renderDurableDependencyManifestAuditMarkdown(audit) {
  const lines = [
    "# Phase 003 Durable Dependency Manifest Audit",
    "",
    "현재 단계: Phase 003.2 - Persistence, Migration, and Recovery Hardening",
    "",
    "## Purpose",
    "",
    "- self-host runtime composition이 요구하는 durable local dependency contract를 코드 기준으로 고정한다.",
    "- durable adapter 구현 완료와 runtime composition dependency contract를 분리해서 검증한다.",
    "- RuntimeDependencyManifest drift를 release/audit gate에서 발견할 수 있게 한다.",
    "",
    "## Summary",
    "",
    "| Metric | Count |",
    "| --- | ---: |",
    `| manifest entries | ${audit.summary.manifestEntries} |`,
    `| required durable dependencies | ${audit.summary.requiredDurableDependencies} |`,
    `| DurableLocal wired | ${audit.summary.durableLocalWired} |`,
    `| missing durable dependencies | ${audit.summary.missingDurableDependencies} |`,
    `| wrong implementation | ${audit.summary.wrongImplementation} |`,
    `| wrong durability | ${audit.summary.wrongDurability} |`,
    "",
    "## Required Durable Dependencies",
    "",
    "| Dependency | Expected Implementation | Actual Implementation | Actual Durability | Status |",
    "| --- | --- | --- | --- | --- |",
    ...audit.requiredDependencies.map((expected) => {
      const actual = audit.manifestEntries.find(
        (entry) => entry.dependency === expected.dependency,
      );
      return `| \`${expected.dependency}\` | ${expected.implementation} | ${actual?.implementation ?? "missing"} | ${actual?.durability ?? "missing"} | ${dependencyStatus(expected, actual)} |`;
    }),
    "",
    "## Findings",
    "",
  ];

  if (audit.findings.length === 0) {
    lines.push("- No durable dependency manifest gap was detected.");
  } else {
    for (const finding of audit.findings) {
      lines.push(`- ${finding.id}: ${finding.message}`);
      lines.push(`- affected dependencies: ${finding.dependencies.join(", ")}`);
    }
  }

  lines.push(
    "",
    "## Next Dependency",
    "",
    `- next dependency: \`${audit.nextImplementationTarget?.dependency ?? "none"}\``,
    "",
    "## Review Notes",
    "",
    "- RuntimeDependencyManifest must not replace adapter contract tests.",
    "- DurableLocal means the self-host composition contract requires a durable local adapter implementation.",
    "- Policy, RuntimeUtility, External, and VolatileLocal dependencies must not be counted as durable stores.",
  );

  return `${lines.join("\n")}\n`;
}

async function runCli() {
  const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const reportPath = path.join(
    repoRoot,
    ".tasks/phase003/durable-dependency-manifest-audit.md",
  );
  let state = DurableDependencyManifestAuditState.NotStarted;
  try {
    state = transitionDurableDependencyManifestAuditState(
      state,
      DurableDependencyManifestAuditEvent.Start,
    );
    const runtimeSource = await readFile(
      path.join(repoRoot, "crates/cabinet-server/src/runtime.rs"),
      "utf8",
    );
    state = transitionDurableDependencyManifestAuditState(
      state,
      DurableDependencyManifestAuditEvent.SourceLoaded,
    );
    const audit = analyzeDurableDependencyManifestSources({ runtimeSource });
    state = transitionDurableDependencyManifestAuditState(
      state,
      DurableDependencyManifestAuditEvent.AuditComplete,
    );
    await writeFile(reportPath, renderDurableDependencyManifestAuditMarkdown(audit));
    state = transitionDurableDependencyManifestAuditState(
      state,
      DurableDependencyManifestAuditEvent.ReportWritten,
    );

    console.log("phase003_durable_dependency_manifest_audit=passed");
    console.log(
      `required_durable_dependencies=${audit.summary.requiredDurableDependencies}`,
    );
    console.log(`durable_local_wired=${audit.summary.durableLocalWired}`);
    console.log(
      `missing_durable_dependencies=${audit.summary.missingDurableDependencies}`,
    );
    console.log(`wrong_implementation=${audit.summary.wrongImplementation}`);
    console.log(`wrong_durability=${audit.summary.wrongDurability}`);
    console.log(
      `next_dependency=${audit.nextImplementationTarget?.dependency ?? "none"}`,
    );
    console.log(`report_path=${reportPath}`);
  } catch (error) {
    if (state === DurableDependencyManifestAuditState.ReadingSource) {
      state = transitionDurableDependencyManifestAuditState(
        state,
        DurableDependencyManifestAuditEvent.Fail,
      );
      console.error("phase003_durable_dependency_manifest_audit=failed");
      console.error(
        `error_code=${error.code ?? DurableDependencyManifestAuditErrorCode.SourceReadFailed}`,
      );
      throw error;
    }
    if (state === DurableDependencyManifestAuditState.Auditing) {
      state = transitionDurableDependencyManifestAuditState(
        state,
        DurableDependencyManifestAuditEvent.Fail,
      );
      console.error("phase003_durable_dependency_manifest_audit=failed");
      console.error(
        `error_code=${error.code ?? DurableDependencyManifestAuditErrorCode.ReportWriteFailed}`,
      );
      throw error;
    }
    throw error;
  }
}

function parseManifestEntries(runtimeSource) {
  if (!runtimeSource || runtimeSource.trim().length === 0) {
    throw new DurableDependencyManifestAuditError(
      DurableDependencyManifestAuditErrorCode.ManifestEmpty,
      "runtime dependency manifest source is empty",
    );
  }
  const entries = [];
  const pattern =
    /dependency\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*([A-Za-z]+)\s*,?\s*\)/g;
  for (const match of runtimeSource.matchAll(pattern)) {
    entries.push({
      dependency: match[1],
      implementation: match[2],
      durability: match[3],
    });
  }
  if (entries.length === 0) {
    throw new DurableDependencyManifestAuditError(
      DurableDependencyManifestAuditErrorCode.ManifestEmpty,
      "runtime dependency manifest contains no dependency entries",
    );
  }
  return entries;
}

function buildFindings({ missing, wrongImplementation, wrongDurability }) {
  const findings = [];
  if (missing.length > 0) {
    findings.push({
      id: "PHASE003_DURABLE_DEPENDENCY_MISSING",
      message: "Required durable local dependencies are missing from RuntimeDependencyManifest.",
      dependencies: missing.map((entry) => entry.dependency),
    });
  }
  if (wrongImplementation.length > 0) {
    findings.push({
      id: "PHASE003_DURABLE_DEPENDENCY_WRONG_IMPLEMENTATION",
      message:
        "Required durable local dependencies point at the wrong implementation name.",
      dependencies: wrongImplementation.map((entry) => entry.expected.dependency),
    });
  }
  if (wrongDurability.length > 0) {
    findings.push({
      id: "PHASE003_DURABLE_DEPENDENCY_WRONG_DURABILITY",
      message:
        "Required durable local dependencies are not classified as DurableLocal.",
      dependencies: wrongDurability.map((entry) => entry.expected.dependency),
    });
  }
  return findings;
}

function pickNextImplementationTarget({
  missing,
  wrongImplementation,
  wrongDurability,
}) {
  return (
    missing[0] ??
    wrongImplementation[0]?.expected ??
    wrongDurability[0]?.expected ??
    null
  );
}

function dependencyStatus(expected, actual) {
  if (!actual) {
    return "missing";
  }
  if (actual.implementation !== expected.implementation) {
    return "wrong implementation";
  }
  if (actual.durability !== "DurableLocal") {
    return "wrong durability";
  }
  return "DurableLocal wired";
}

function required(dependency, implementation) {
  return Object.freeze({ dependency, implementation });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  await runCli();
}
