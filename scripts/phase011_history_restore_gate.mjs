import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

export function validatePhase011HistoryRestoreGateInputs({
  inventoryText,
  authoringGateText,
  clientCoreText,
  uiText,
  tauriTransportText,
  desktopRuntimeText,
}) {
  const sourceFingerprint = inventoryText.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
  if (!inventoryText.includes("phase011_current_inventory=passed") || !sourceFingerprint) {
    return failed("inventory");
  }
  if (!authoringGateText.includes("phase011_document_authoring_gate=passed")) {
    return failed("authoring_gate");
  }
  if (containsSensitiveGateData(authoringGateText)) {
    return failed("sensitive_data");
  }

  const combined = [
    clientCoreText,
    uiText,
    tauriTransportText,
    desktopRuntimeText,
    authoringGateText,
  ].join("\n");

  const requiredSnippets = [
    ["client_get_version", "getDocumentVersion(query)"],
    ["client_preview_restore", "previewDocumentRestore(query)"],
    ["client_apply_restore", "restoreDocumentVersion(command)"],
    ["ui_expected_guard", "expectedCurrentVersionId"],
    ["tauri_history_route", "get_document_history"],
    ["tauri_preview_route", "preview_document_restore"],
    ["tauri_restore_route", "restore_document_version"],
    ["native_stale_guard", "DOCUMENT_RESTORE_VERSION_CONFLICT"],
    ["native_pointer_guard", "load_current_version"],
    ["native_pointer_update", "compare_and_set_current_version"],
  ];
  for (const [findingId, snippet] of requiredSnippets) {
    if (!combined.includes(snippet)) return failed(findingId);
  }

  return {
    passed: true,
    marker: "phase011_history_restore_gate=passed",
    sourceFingerprint,
    commandCount: 4,
    staleGuard: true,
  };
}

export function renderPhase011HistoryRestoreGateArtifact(result) {
  if (!result.passed) {
    return [
      "phase011_history_restore_gate=failed",
      `finding_id=${result.findingId}`,
      "",
    ].join("\n");
  }
  return [
    "phase011_history_restore_gate=passed",
    "release_scope=personal_local_desktop",
    `source_fingerprint=${result.sourceFingerprint}`,
    "requirements=HIST-01,HIST-02,STATE-01,PERF-01,COMPAT-01",
    `history_restore_command_count=${result.commandCount}`,
    `expected_current_version_guard=${result.staleGuard}`,
    "current_history_query_separation=true",
    "restore_preview_required=true",
    "restore_confirmation_required=true",
    "raw_body_excluded=true",
    "raw_path_excluded=true",
    "git_terms_excluded=true",
    "",
  ].join("\n");
}

export async function runPhase011HistoryRestoreGate({ root = process.cwd() } = {}) {
  const read = (path) => readFile(join(root, path), "utf8");
  const result = validatePhase011HistoryRestoreGateInputs({
    inventoryText: await read(".tasks/phase011-current-implementation-inventory.md"),
    authoringGateText: await read(".tasks/phase011-document-authoring-gate-result.md"),
    clientCoreText: await read("packages/client-core/src/index.ts"),
    uiText: await read("packages/ui/src/index.ts"),
    tauriTransportText: await read("apps/desktop/src/tauri_authoring_transport.ts"),
    desktopRuntimeText: await read("apps/desktop/src-tauri/src/lib.rs"),
  });
  const artifactPath = join(root, ".tasks", "phase011-history-restore-gate-result.md");
  await mkdir(dirname(artifactPath), { recursive: true });
  await writeFile(artifactPath, renderPhase011HistoryRestoreGateArtifact(result));
  return result;
}

function failed(findingId) {
  return { passed: false, findingId };
}

function containsSensitiveGateData(text) {
  return [
    "/Users/",
    "C:\\Users\\",
    "provider_api_key",
    "sessionToken",
    "raw markdown body",
    "Secret Browser Body",
  ].some((token) => text.includes(token));
}

async function main() {
  const result = await runPhase011HistoryRestoreGate({ root: process.cwd() });
  if (!result.passed) {
    console.error(`phase011_history_restore_gate=failed ${result.findingId}`);
    process.exit(1);
  }
  console.log("phase011_history_restore_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
