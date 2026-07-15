import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

import { validatePhase011AuthoringBrowserReport } from "./phase011_authoring_browser.mjs";

export function validatePhase011DocumentAuthoringGateInputs({
  inventoryText,
  authoringBrowserText,
}) {
  const sourceFingerprint = inventoryText.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
  if (!inventoryText.includes("phase011_current_inventory=passed") || !sourceFingerprint) {
    return failed("inventory");
  }
  if (!authoringBrowserText.includes("phase011_authoring_browser=passed")) {
    return failed("authoring_browser_marker");
  }
  if (containsSensitiveGateData(authoringBrowserText)) {
    return failed("sensitive_data");
  }

  let report;
  try {
    report = JSON.parse(authoringBrowserText);
  } catch {
    return failed("authoring_browser_json");
  }

  const validation = validatePhase011AuthoringBrowserReport(report, sourceFingerprint);
  if (!validation.passed) {
    const findingId = validation.findingIds[0] === "stale_source_fingerprint"
      ? "source_fingerprint"
      : validation.findingIds[0];
    return failed(findingId ?? "authoring_browser_report");
  }

  return {
    passed: true,
    marker: "phase011_document_authoring_gate=passed",
    sourceFingerprint,
    createDocumentCount: report.interactions.createDocumentCount,
    manualSaveCount: report.interactions.manualSaveCount,
    autosaveCount: report.interactions.autosaveCount,
    screenshotCount: report.runs.length,
  };
}

export function renderPhase011DocumentAuthoringGateArtifact(result) {
  if (!result.passed) {
    return [
      "phase011_document_authoring_gate=failed",
      `finding_id=${result.findingId}`,
      "",
    ].join("\n");
  }
  return [
    "phase011_document_authoring_gate=passed",
    "release_scope=personal_local_desktop",
    `source_fingerprint=${result.sourceFingerprint}`,
    `create_document_count=${result.createDocumentCount}`,
    `manual_save_count=${result.manualSaveCount}`,
    `autosave_count=${result.autosaveCount}`,
    `screenshot_count=${result.screenshotCount}`,
    "raw_body_excluded=true",
    "raw_path_excluded=true",
    "",
  ].join("\n");
}

export async function runPhase011DocumentAuthoringGate({ root = process.cwd() } = {}) {
  const inventoryText = await readFile(
    join(root, ".tasks", "phase011-current-implementation-inventory.md"),
    "utf8",
  );
  const authoringBrowserText = await readFile(
    join(root, ".tasks", "release", "authoring-browser-phase011.json"),
    "utf8",
  );
  const result = validatePhase011DocumentAuthoringGateInputs({
    inventoryText,
    authoringBrowserText,
  });
  const artifactPath = join(root, ".tasks", "phase011-document-authoring-gate-result.md");
  await mkdir(dirname(artifactPath), { recursive: true });
  await writeFile(artifactPath, renderPhase011DocumentAuthoringGateArtifact(result));
  return result;
}

function failed(findingId) {
  return {
    passed: false,
    findingId,
  };
}

function containsSensitiveGateData(text) {
  return [
    "/Users/",
    "C:\\Users\\",
    "raw markdown body",
    "Secret Browser Body",
    "notes/private.md",
    "provider_api_key",
    "sessionToken",
  ].some((token) => text.includes(token));
}

async function main() {
  const result = await runPhase011DocumentAuthoringGate({ root: process.cwd() });
  if (!result.passed) {
    console.error(`phase011_document_authoring_gate=failed ${result.findingId}`);
    process.exit(1);
  }
  console.log("phase011_document_authoring_gate=passed");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
