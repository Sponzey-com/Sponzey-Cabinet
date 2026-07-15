import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import {
  buildPhase010DataPortabilityCommandPlan,
  buildPhase010DataPortabilityManifest,
  renderPhase010DataPortabilityArtifact,
  renderPhase010DataPortabilityManifest,
} from "./phase010_data_portability_gate.mjs";

const deniedTerms = [
  "raw_document_body_fixture",
  "provider_api_key_fixture",
  "personal_absolute_path_fixture",
  "token_fixture",
  "credential_fixture",
  "secret_fixture",
  "/Users/",
  "C:\\Users\\",
];

export async function runPhase010DataPortabilitySecurityScan({ root = process.cwd() } = {}) {
  const texts = [
    renderPhase010DataPortabilityArtifact(passingProspectiveResult()),
    renderPhase010DataPortabilityManifest(buildPhase010DataPortabilityManifest()),
  ];

  for (const relativePath of [
    ".tasks/phase010-durable-authoring-gate-result.md",
    ".tasks/phase010-data-portability-gate-result.md",
    ".tasks/release/data-portability-manifest-phase010.json",
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
        errorCode: "PHASE010_DATA_PORTABILITY_SECURITY_FINDING",
        findingId: finding,
      };
    }
  }

  return { passed: true, marker: "phase010_data_portability_security_scan=passed" };
}

function passingProspectiveResult() {
  const commandResults = Object.fromEntries(
    buildPhase010DataPortabilityCommandPlan().map((gateStep) => [
      gateStep.id,
      { command: gateStep.command.join(" "), passed: true, exitCode: 0, durationMs: 0 },
    ]),
  );
  return {
    passed: true,
    state: "Passed",
    commandResults,
    manifest: buildPhase010DataPortabilityManifest(),
  };
}

async function main() {
  const result = await runPhase010DataPortabilitySecurityScan({ root: process.cwd() });
  if (!result.passed) {
    console.error(`${result.errorCode} ${result.findingId}`.trim());
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
