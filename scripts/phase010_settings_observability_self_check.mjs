import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import {
  buildPhase010ProductLogMatrix,
  buildPhase010SecurityLogManifest,
  renderPhase010LocalDesktopRunbook,
  renderPhase010ProductLogMatrix,
  renderPhase010SecurityLogManifest,
  renderPhase010SettingsObservabilityArtifact,
} from "./phase010_settings_observability_gate.mjs";
import { validateLogPolicyManifest } from "./security_log_scanner.mjs";
import { validateRunbookText } from "./runbook_validator.mjs";

const deniedTerms = [
  "provider_api_key_fixture",
  "raw_document_body_fixture",
  "personal_absolute_path_fixture",
  "token_fixture",
  "credential_fixture",
  "secret_fixture",
  "raw_prompt_fixture",
  "raw_answer_fixture",
  "/Users/",
  "C:\\Users\\",
];

const requiredSections = [
  "## Clean Install",
  "## Packaged Launch",
  "## Reinstall Preservation",
  "## Blank Screen Recovery",
  "## Index Repair",
  "## Export Import",
  "## Backup Restore",
  "## Field Debug",
  "## Data Export",
];

const requiredPhrases = [
  "phase010_runbook=passed",
  "No external database",
  "must not require a dev server",
  "Field Debug is disabled by default",
  "Activation requires scope, expiry, reason, and masking policy",
  "300ms",
];

export async function runPhase010SettingsObservabilitySelfCheck({ root = process.cwd() } = {}) {
  const securityManifest = buildPhase010SecurityLogManifest();
  validateLogPolicyManifest(securityManifest);

  const prospectiveRunbook = renderPhase010LocalDesktopRunbook();
  const runbookFindings = validateRunbookText({
    runbook: {
      id: "phase010_local_desktop_runbook",
      path: ".tasks/release/local-desktop-runbook-phase010.md",
    },
    text: prospectiveRunbook,
    manifest: {
      requiredSections,
      requiredPhrases,
      forbiddenText: deniedTerms.map((term) => ({ id: term, value: term })),
    },
  });
  if (runbookFindings.length > 0) {
    return {
      passed: false,
      errorCode: "PHASE010_SETTINGS_OBSERVABILITY_RUNBOOK_INVALID",
      findingId: runbookFindings[0].findingId,
    };
  }

  const texts = [
    renderPhase010ProductLogMatrix(buildPhase010ProductLogMatrix()),
    renderPhase010SecurityLogManifest(securityManifest),
    prospectiveRunbook,
    renderPhase010SettingsObservabilityArtifact({
      passed: true,
      state: "Passed",
      commandResults: {},
    }),
  ];

  for (const relativePath of [
    ".tasks/phase010-settings-observability-gate-result.md",
    ".tasks/release/product-log-event-matrix-phase010.md",
    ".tasks/release/security-log-policy-manifest-phase010.json",
    ".tasks/release/local-desktop-runbook-phase010.md",
  ]) {
    const path = join(root, relativePath);
    if (existsSync(path)) {
      texts.push(await readFile(path, "utf8"));
    }
  }

  for (const text of texts) {
    const finding = deniedTerms.find((term) => text.includes(term));
    if (finding) {
      return {
        passed: false,
        errorCode: "PHASE010_SETTINGS_OBSERVABILITY_SECURITY_FINDING",
        findingId: finding,
      };
    }
  }

  return {
    passed: true,
    marker: "phase010_settings_observability_self_check=passed",
  };
}

async function main() {
  const result = await runPhase010SettingsObservabilitySelfCheck({ root: process.cwd() });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId ?? ""}`.trim());
    process.exit(1);
  }
  console.log(result.marker);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error);
    process.exit(1);
  });
}
