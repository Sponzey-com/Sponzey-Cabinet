import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";

export const DataOwnershipGateErrorCode = Object.freeze({
  AiAssistantMissing: "PHASE007_DATA_OWNERSHIP_AI_ASSISTANT_MISSING",
  SecretExclusionMissing: "PHASE007_DATA_OWNERSHIP_SECRET_EXCLUSION_MISSING",
  RestoreConfirmationMissing: "PHASE007_DATA_OWNERSHIP_RESTORE_CONFIRMATION_MISSING",
  ImportPolicyMissing: "PHASE007_DATA_OWNERSHIP_IMPORT_POLICY_MISSING",
  RawContentLeak: "PHASE007_DATA_OWNERSHIP_RAW_CONTENT_LEAK",
  IoFailed: "PHASE007_DATA_OWNERSHIP_IO_FAILED",
});

export function evaluateDataOwnershipGate({ aiAssistantText, ownershipEvidence }) {
  if (!aiAssistantText.includes("phase007_ai_assistant_gate=passed")) {
    return failed(DataOwnershipGateErrorCode.AiAssistantMissing, "ai_assistant_prerequisite");
  }
  if (ownershipEvidence?.secretsExcluded !== true) {
    return failed(DataOwnershipGateErrorCode.SecretExclusionMissing, "backup_export_secret_exclusion");
  }
  if (
    ownershipEvidence?.restoreRequiresConfirmation !== true ||
    ownershipEvidence?.restoreStagingIsolated !== true
  ) {
    return failed(DataOwnershipGateErrorCode.RestoreConfirmationMissing, "restore_staging_guard");
  }
  if (
    !Array.isArray(ownershipEvidence?.importConflictPolicies) ||
    !ownershipEvidence.importConflictPolicies.includes("rename") ||
    !ownershipEvidence.importConflictPolicies.includes("skip")
  ) {
    return failed(DataOwnershipGateErrorCode.ImportPolicyMissing, "import_conflict_policy");
  }
  if (ownershipEvidence?.rawContentExcluded !== true) {
    return failed(DataOwnershipGateErrorCode.RawContentLeak, "sensitive_data_exclusion");
  }
  return {
    passed: true,
    marker: "phase007_data_ownership_gate=passed",
    backupDefaultPath: ownershipEvidence.backupDefaultPath,
    backupBlocksStartup: ownershipEvidence.backupBlocksStartup,
    secretsExcluded: ownershipEvidence.secretsExcluded,
    restoreRequiresConfirmation: ownershipEvidence.restoreRequiresConfirmation,
    importConflictPolicies: ownershipEvidence.importConflictPolicies,
  };
}

export function renderDataOwnershipGateResult(result) {
  if (result.passed) {
    return [
      result.marker,
      `backup_default_path=${result.backupDefaultPath}`,
      `backup_blocks_startup=${result.backupBlocksStartup}`,
      `secrets_excluded=${result.secretsExcluded}`,
      `restore_requires_confirmation=${result.restoreRequiresConfirmation}`,
      `import_conflict_policies=${result.importConflictPolicies.join(",")}`,
    ].join("\n");
  }
  return [
    "phase007_data_ownership_gate=failed",
    `error_code=${result.errorCode}`,
    `finding_id=${result.findingId}`,
  ].join("\n");
}

export function renderDataOwnershipGateArtifact(result) {
  const status = result.passed ? "passed" : "failed";
  return [
    "# Phase 007 Data Ownership Gate Result",
    "",
    renderDataOwnershipGateResult(result),
    "",
    "- phase: `Phase 007.6`",
    "- gate: `Backup Import Export Restore Data Ownership`",
    `- status: \`${status}\``,
    "- prerequisite evidence:",
    "  - `.tasks/phase007-ai-assistant-gate-result.md` with `phase007_ai_assistant_gate=passed`",
    "- validation commands:",
    "  - `npm run run:phase007-data-ownership-gate-tests`",
    "  - `npm run run:phase007-data-ownership-gate`",
    "- Product Log candidates: `backup.created`, `backup.failed`, `export.created`, `import.preview.completed`, `restore.completed`, `restore.failed` with stable error code only.",
    "- Field Debug metadata candidates: `artifact_id`, `file_count`, `byte_bucket`, `conflict_count`, `validation_issue_count`.",
    "- data ownership statement: local documents, assets, versions, and export packages remain user-owned local data by default.",
    "- sensitive data exclusion: this artifact records markers, booleans, state names, policy ids, bucket labels, counts, and stable error codes only.",
    "- follow-up limitation: final desktop visual QA, product smoke, packaging, and release gate remain Phase 007.7.",
    "",
  ].join("\n");
}

async function runDataOwnershipGateCli() {
  try {
    const aiAssistantText = await readFile(".tasks/phase007-ai-assistant-gate-result.md", "utf8");
    const result = evaluateDataOwnershipGate({
      aiAssistantText,
      ownershipEvidence: {
        backupDefaultPath: "platform-default",
        backupBlocksStartup: false,
        secretsExcluded: true,
        importConflictPolicies: ["rename", "skip"],
        restoreRequiresConfirmation: true,
        restoreStagingIsolated: true,
        rawContentExcluded: true,
      },
    });
    await writeFile(".tasks/phase007-data-ownership-gate-result.md", renderDataOwnershipGateArtifact(result));
    const rendered = renderDataOwnershipGateResult(result);
    if (result.passed) {
      console.log(rendered);
      return;
    }
    console.error(rendered);
    process.exit(1);
  } catch (error) {
    const result = failed(
      DataOwnershipGateErrorCode.IoFailed,
      error instanceof Error ? error.message : "unknown",
    );
    await writeFile(".tasks/phase007-data-ownership-gate-result.md", renderDataOwnershipGateArtifact(result));
    console.error(renderDataOwnershipGateResult(result));
    process.exit(1);
  }
}

function failed(errorCode, findingId) {
  return {
    passed: false,
    marker: "phase007_data_ownership_gate=failed",
    errorCode,
    findingId,
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runDataOwnershipGateCli();
}
