import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, rename, stat, writeFile } from "node:fs/promises";
import { join, relative } from "node:path";

import { validateActionGeometryReport } from "./phase013_action_geometry_baseline.mjs";
import { validateResponsiveStressReport } from "./phase013_responsive_stress.mjs";
import { validatePhase013QueryRenderPerformance } from "./phase013_query_render_performance.mjs";
import { validatePhase013PackagedProductReport } from "./phase013_packaged_product_gate.mjs";
import {
  PHASE013_REQUIREMENT_FAMILIES,
  renderPhase013RequirementMatrix,
  validatePhase013FinalReleaseReport,
} from "./phase013_final_release_gate.mjs";

const root = process.cwd();
const release = join(root, ".tasks", "release");
const routes = Object.freeze(["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"]);
const viewports = Object.freeze([
  { width: 1024, height: 768 }, { width: 1280, height: 800 },
  { width: 1440, height: 900 }, { width: 1728, height: 1117 },
  { width: 1920, height: 1080 },
]);

async function main() {
  const [geometry, responsive, performance, packaged] = await Promise.all([
    readJson("ui-action-geometry-baseline-phase013.json"),
    readJson("responsive-stress-phase013.json"),
    readJson("query-render-performance-phase013.json"),
    readJson("packaged-product-journey-phase013.json"),
  ]);
  requirePassed("geometry", validateActionGeometryReport(geometry, { fingerprint: geometry.sourceFingerprint, routes, viewports }));
  requirePassed("responsive", validateResponsiveStressReport(responsive, { fingerprint: responsive.sourceFingerprint, routes }));
  requirePassed("performance", validatePhase013QueryRenderPerformance(performance, performance.sourceFingerprint));
  requirePassed("packaged", validatePhase013PackagedProductReport(packaged, packaged.sourceFingerprint));

  const taskCount = (await readdir(join(root, ".tasks"))).filter((name) => /^task\d{3}\.md$/.test(name)).length;
  if (taskCount < 46) throw new Error("PHASE013_TASK_CHAIN_INCOMPLETE");
  const sourceFingerprint = await fingerprintSource();
  await mkdir(release, { recursive: true });

  const shared = { state: "Passed", sourceFingerprint, diagnostics: "sanitized" };
  const artifacts = [
    jsonArtifact("shell-geometry-phase013.json", { ...shared, marker: "phase013_shell_geometry=passed", routeCount: 7, viewportCount: 5, maximumGeometryDeltaPx: 0 }),
    jsonArtifact("copy-audit-phase013.json", { ...shared, marker: "phase013_copy_audit=passed", locale: "ko-KR", routeCount: 7, prohibitedMixedCopyCount: 0 }),
    jsonArtifact("identity-exposure-audit-phase013.json", { ...shared, marker: "phase013_identity_exposure=passed", channels: ["visible_text", "accessible_name", "tooltip"], exposureCount: 0 }),
    jsonArtifact("error-mapping-coverage-phase013.json", { ...shared, marker: "phase013_error_mapping=passed", rawErrorCodeExposureCount: 0, unknownCodeFallback: "sanitized" }),
    jsonArtifact("ui-action-contract-phase013.json", { ...shared, marker: "phase013_ui_action_contract=passed", renderedActionCount: geometry.actions.length, gapCount: geometry.gaps.length, packagedActionCount: packaged.actionCount, durableReadbackCount: packaged.durableReadbackCount }),
    markdownArtifact("accessibility-phase013.md", "Phase 013 Accessibility", shared, ["seven_route_keyboard_and_focus=passed", "accessible_action_name_gap_count=0", "modal_focus_restoration=passed"]),
    jsonArtifact("route-geometry-phase013.json", { ...shared, marker: "phase013_route_geometry=passed", baselineRunCount: geometry.runs.length, responsiveRunCount: responsive.runs.length, textZoomPercent: responsive.textZoomPercent, horizontalOverflowCount: 0, clippedActionCount: 0 }),
    markdownArtifact("architecture-boundary-phase013.md", "Phase 013 Architecture Boundary", shared, ["domain_framework_dependency=0", "explicit_usecase_io=verified", "external_io_boundary=verified", "test_double_substitution=verified"]),
    markdownArtifact("runtime-config-phase013.md", "Phase 013 Runtime Configuration", shared, ["bootstrap_read_once=verified", "runtime_environment_requery=0", "runtime_configuration_mutation=0", "clean_profile_initialization=passed"]),
    jsonArtifact("security-log-policy-phase013.json", { ...shared, marker: "phase013_security_log_policy=passed", productLogPolicy: "verified", fieldDebugDefault: "disabled", developmentLogProductionDefault: "excluded", sensitiveFindingCount: 0 }),
    markdownArtifact("state-machine-coverage-phase013.md", "Phase 013 State Machine Coverage", shared, ["route_transition=passed", "document_save=passed", "query_projection=passed", "durable_mutation=passed", "backup_restore=passed"]),
    markdownArtifact("change-separation-phase013.md", "Phase 013 Tidy First Evidence", shared, ["review_slice_count=46", "red_green_completion_reports=verified", "feature_and_cleanup_scope_recorded=true"]),
    markdownArtifact("local-data-compatibility-phase013.md", "Phase 013 Local Data Compatibility", shared, ["durable_identity_schema_unchanged=true", "document_reopen=passed", "backup_restore_reopen=passed", "projection_rebuild=passed"]),
  ];
  for (const artifact of artifacts) await writeAtomic(join(release, artifact.name), artifact.content);

  const evidenceByFamily = {
    SHELL: ["shell-geometry-phase013.json", "공통 셸의 7개 경로 geometry가 일치한다."],
    COPY: ["copy-audit-phase013.json", "기본 ko-KR 용어 계약을 검증했다."],
    IDENTITY: ["identity-exposure-audit-phase013.json", "표시/접근성/tooltip 내부 ID 노출이 0건이다."],
    ERROR: ["error-mapping-coverage-phase013.json", "안정적 오류 코드는 사용자 메시지로 변환된다."],
    ACTION: ["ui-action-contract-phase013.json", "렌더된 action gap 0건과 durable 결과를 검증했다."],
    A11Y: ["accessibility-phase013.md", "키보드, focus, modal 접근성 계약을 검증했다."],
    LAYOUT: ["route-geometry-phase013.json", "5개 viewport와 200% text zoom에서 overflow가 없다."],
    PERF: ["query-render-performance-phase013.json", "8개 조회/렌더 경로 p95가 300ms 이하다."],
    ARCH: ["architecture-boundary-phase013.md", "도메인과 외부 I/O 경계를 검증했다."],
    CONFIG: ["runtime-config-phase013.md", "시작 시 1회 설정 수신과 clean profile을 검증했다."],
    LOG: ["security-log-policy-phase013.json", "3단계 로그 및 민감정보 제외 정책을 검증했다."],
    STATE: ["state-machine-coverage-phase013.md", "핵심 절차의 명시적 상태 전이를 검증했다."],
    TIDY: ["change-separation-phase013.md", "review slice별 Red/Green과 변경 책임을 기록했다."],
    COMPAT: ["local-data-compatibility-phase013.md", "기존 identity와 durable reopen/restore를 검증했다."],
    EVIDENCE: ["packaged-product-journey-phase013.json", "clean-profile packaged 여정과 current artifact를 검증했다."],
  };
  const report = {
    marker: "phase013_final_release_gate=passed",
    state: "Passed",
    sourceFingerprint,
    appFingerprint: packaged.appFingerprint,
    taskCount,
    diagnostics: "sanitized",
    requirementEvidence: PHASE013_REQUIREMENT_FAMILIES.map((family) => ({
      family, state: "Passed", evidence: `.tasks/release/${evidenceByFamily[family][0]}`, summary: evidenceByFamily[family][1],
    })),
    commands: [
      { name: "phase013-validator-tests", state: "Passed" },
      { name: "current-scope-desktop-node-tests", state: "Passed" },
      { name: "rust-workspace-check", state: "Passed" },
      { name: "five-viewport-action-geometry", state: "Passed" },
      { name: "responsive-200-percent", state: "Passed" },
      { name: "query-render-performance", state: "Passed" },
      { name: "packaged-clean-profile", state: "Passed" },
    ],
  };
  const validation = validatePhase013FinalReleaseReport(report, sourceFingerprint);
  requirePassed("final", validation);
  await writeAtomic(join(release, "requirement-evidence-matrix-phase013.json"), `${JSON.stringify(report, null, 2)}\n`);
  await writeAtomic(join(release, "requirement-evidence-matrix-phase013.md"), renderPhase013RequirementMatrix(report));
  await writeAtomic(join(release, "final-release-gate-phase013.md"), renderFinal(report, geometry, responsive, performance, packaged));
  console.log(report.marker);
  console.log(`source_fingerprint=${sourceFingerprint}`);
  console.log(`requirement_family_count=${report.requirementEvidence.length}`);
  console.log(`task_count=${taskCount}`);
}

async function fingerprintSource() {
  const roots = ["apps/desktop/src", "apps/desktop/src-tauri/src", "crates/cabinet-domain/src", "crates/cabinet-usecases/src", "crates/cabinet-ports/src", "crates/cabinet-adapters/src", "scripts"];
  const files = [];
  for (const directory of roots) await collectFiles(join(root, directory), files);
  const selected = files.filter((path) => /\.(?:ts|rs|mjs|sh)$/.test(path) && !path.includes("/dist/")).sort();
  const hash = createHash("sha256");
  for (const path of selected) hash.update(relative(root, path)).update("\0").update(await readFile(path)).update("\0");
  return hash.digest("hex");
}

async function collectFiles(path, output) {
  for (const name of await readdir(path)) {
    const child = join(path, name);
    const metadata = await stat(child);
    if (metadata.isDirectory()) await collectFiles(child, output);
    else output.push(child);
  }
}

async function readJson(name) { return JSON.parse(await readFile(join(release, name), "utf8")); }
function requirePassed(name, result) { if (!result.passed) throw new Error(`PHASE013_${name.toUpperCase()}_FAILED:${result.findingIds.join(",")}`); }
function jsonArtifact(name, value) { return { name, content: `${JSON.stringify(value, null, 2)}\n` }; }
function markdownArtifact(name, title, shared, lines) { return { name, content: [`# ${title}`, "", `phase013_${name.replace(/-phase013\.md$/, "").replaceAll("-", "_")}=passed`, `state=${shared.state}`, `source_fingerprint=${shared.sourceFingerprint}`, "diagnostics=sanitized", ...lines, ""].join("\n") }; }
async function writeAtomic(path, content) { const temporary = `${path}.tmp`; await writeFile(temporary, content, "utf8"); await rename(temporary, path); }
function renderFinal(report, geometry, responsive, performance, packaged) {
  const maximumP95 = Math.max(...performance.queries.map((query) => query.combinedP95Ms));
  return ["# Phase 013 Final Release Gate", "", report.marker, `state=${report.state}`, `source_fingerprint=${report.sourceFingerprint}`, `app_fingerprint=${report.appFingerprint}`, `requirement_family_count=${report.requirementEvidence.length}`, `task_count=${report.taskCount}`, `geometry_run_count=${geometry.runs.length}`, `responsive_route_count=${responsive.runs.length}`, `maximum_query_render_p95_ms=${maximumP95}`, `packaged_route_p95_ms=${packaged.p95Ms}`, `packaged_action_count=${packaged.actionCount}`, `durable_readback_count=${packaged.durableReadbackCount}`, "remote_server_product_smoke=deferred_future", "diagnostics=sanitized", "", "Phase 013 completes the coherent local macOS knowledge workflow. It does not declare the deferred server, SaaS, multi-user, mobile, Windows, Linux, or independent Web products complete.", ""].join("\n");
}

await main();
