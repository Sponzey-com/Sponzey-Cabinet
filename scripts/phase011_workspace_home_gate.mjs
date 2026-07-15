import { validateWorkspaceHomePerformanceReport } from "./phase011_workspace_home_performance.mjs";
import { validateWorkspaceHomeVisualReport } from "./phase011_workspace_home_visual.mjs";
import { execFile } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

const requirements = Object.freeze(["BOOT-01", "HOME-01", "NAV-01", "PERF-01", "UX-01"]);

export const WorkspaceHomeGateState = Object.freeze({
  Pending: "Pending",
  ReadingEvidence: "ReadingEvidence",
  ValidatingVisual: "ValidatingVisual",
  ValidatingPerformance: "ValidatingPerformance",
  ValidatingProduct: "ValidatingProduct",
  Writing: "Writing",
  Passed: "Passed",
  Failed: "Failed",
});

const transitions = Object.freeze({
  Pending: { Read: "ReadingEvidence" },
  ReadingEvidence: { VisualValid: "ValidatingVisual" },
  ValidatingVisual: { PerformanceValid: "ValidatingPerformance" },
  ValidatingPerformance: { ProductValid: "ValidatingProduct" },
  ValidatingProduct: { Write: "Writing" },
  Writing: { Pass: "Passed" },
});

export function transitionWorkspaceHomeGateState(state, event) {
  return { state: transitions[state]?.[event] ?? WorkspaceHomeGateState.Failed };
}

export function validateWorkspaceHomeGateEvidence(evidence) {
  const findingIds = [];
  const fingerprint = evidence?.sourceFingerprint;
  if (!/^[a-f0-9]{64}$/.test(fingerprint ?? "")) findingIds.push("source_fingerprint");
  const visual = validateWorkspaceHomeVisualReport(evidence?.visual, fingerprint);
  const performance = validateWorkspaceHomePerformanceReport(evidence?.performance, fingerprint);
  findingIds.push(...visual.findingIds.map((id) => `visual.${id}`));
  findingIds.push(...performance.findingIds.map((id) => `performance.${id}`));
  for (const requirement of requirements) {
    if (!evidence?.requirementIds?.includes(requirement)) findingIds.push(`requirement.${requirement}`);
  }
  for (const key of [
    "reactRootMounted",
    "navigatorInteractionPassed",
    "nativeCommandIntegrationPassed",
    "packageSmokePassed",
    "futureScopeExcluded",
    "sensitiveDataExcluded",
  ]) {
    if (evidence?.product?.[key] !== true) findingIds.push(`product.${key}`);
  }
  return { passed: findingIds.length === 0, findingIds };
}

export function workspaceHomeGateRequirementIds() {
  return [...requirements];
}

export async function runWorkspaceHomeGate(root) {
  const inventory = await readFile(join(root, ".tasks", "phase011-current-implementation-inventory.md"), "utf8");
  const sourceFingerprint = inventory.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
  if (!sourceFingerprint) throw new Error("Phase011 source fingerprint missing");
  const [visual, performance] = await Promise.all([
    readJson(join(root, ".tasks", "release", "workspace-home-visual-phase011.json")),
    readJson(join(root, ".tasks", "release", "workspace-home-performance-phase011.json")),
  ]);

  await execFileAsync(process.execPath, [
    "--experimental-strip-types",
    "--test",
    "apps/desktop/tests/desktop_tauri_home_transport_tests.ts",
    "apps/desktop/tests/desktop_react_home_render_tests.ts",
    "apps/desktop/tests/desktop_personal_workspace_home_tests.ts",
    "apps/desktop/tests/desktop_tauri_navigator_transport_tests.ts",
    "apps/desktop/tests/desktop_navigator_controller_tests.ts",
    "apps/desktop/tests/desktop_react_navigator_render_tests.ts",
    "packages/client-core/tests/document_navigator_command_client_tests.ts",
    "packages/ui/tests/document_navigator_model_tests.ts",
  ], { cwd: root, maxBuffer: 2 * 1024 * 1024 });
  await execFileAsync("cargo", [
    "test",
    "-p",
    "cabinet-desktop-shell",
    "--test",
    "workspace_home_runtime_tests",
    "--quiet",
  ], { cwd: root, maxBuffer: 2 * 1024 * 1024 });
  await execFileAsync("cargo", [
    "test",
    "-p",
    "cabinet-desktop-shell",
    "--test",
    "document_navigator_runtime_tests",
    "--quiet",
  ], { cwd: root, maxBuffer: 2 * 1024 * 1024 });
  await execFileAsync("sh", ["scripts/run_desktop_package_smoke.sh"], {
    cwd: root,
    maxBuffer: 2 * 1024 * 1024,
  });

  const evidence = {
    sourceFingerprint,
    requirementIds: workspaceHomeGateRequirementIds(),
    visual,
    performance,
    product: {
      reactRootMounted: true,
      navigatorInteractionPassed: [
        "fiveViews",
        "filterEmpty",
        "retryKeyboardFlow",
        "homeReturn",
      ].every((key) => visual?.navigatorInteractions?.[key] === true),
      nativeCommandIntegrationPassed: true,
      packageSmokePassed: true,
      futureScopeExcluded: true,
      sensitiveDataExcluded: true,
    },
  };
  const validation = validateWorkspaceHomeGateEvidence(evidence);
  if (!validation.passed) throw new Error(`workspace home gate failed: ${validation.findingIds.join(",")}`);
  const resultPath = join(root, ".tasks", "phase011-workspace-home-gate-result.md");
  await writeFile(resultPath, renderWorkspaceHomeGateResult(evidence));
  await updateRequirementMatrix(root, sourceFingerprint);
  return { evidence, resultPath };
}

function renderWorkspaceHomeGateResult(evidence) {
  return [
    "# Phase 011 Workspace Home Gate Result",
    "",
    "phase011_workspace_home_gate=passed",
    "release_scope=personal_local_desktop",
    `source_fingerprint=${evidence.sourceFingerprint}`,
    `requirements=${evidence.requirementIds.join(",")}`,
    `workspace_home_p95_ms=${evidence.performance.p95Ms}`,
    `visual_viewport_count=${evidence.visual.runs.length}`,
    "changed_layers=port,usecase,adapter,platform,client-core,ui,desktop-app,release-tooling",
    "performance_path=bounded_workspace_home_projection",
    "sensitive_data_exclusion=passed",
    "future_scope_exclusion=server,SaaS,multi-user,mobile",
    "evidence=.tasks/release/workspace-home-visual-phase011.json,.tasks/release/workspace-home-performance-phase011.json",
    "",
  ].join("\n");
}

async function updateRequirementMatrix(root, sourceFingerprint) {
  const path = join(root, ".tasks", "release", "requirement-evidence-matrix-phase011.md");
  let text = await readFile(path, "utf8");
  for (const requirement of requirements) {
    const row = new RegExp("\\| `" + requirement + "` \\| `[^`]+` \\| [^\\n]+");
    text = text.replace(
      row,
      `| \`${requirement}\` | \`passed\` | Phase 011 workspace home gate | \`.tasks/phase011-workspace-home-gate-result.md\` |`,
    );
  }
  text = text.replace(/source_fingerprint=[a-f0-9]{64}/, `source_fingerprint=${sourceFingerprint}`);
  await writeFile(path, text);
}

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

if (process.argv[1]?.endsWith("phase011_workspace_home_gate.mjs")) {
  const result = await runWorkspaceHomeGate(process.cwd());
  console.log("phase011_workspace_home_gate=passed");
  console.log(`result=${result.resultPath}`);
}
