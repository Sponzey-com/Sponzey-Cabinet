import type { RequirementEvidenceClass, RequirementEvidenceMapping } from "./requirement_evidence_mapping_contract.ts";

type TestReference = Readonly<{ source: string; testName: string }>;

const ref = (file: string, testName: string): TestReference => Object.freeze({
  source: `apps/desktop/tests/${file}`,
  testName,
});

const claim = (
  requirementId: string,
  evidence: TestReference,
  evidenceClass: RequirementEvidenceClass = "typescript_test",
): RequirementEvidenceMapping => Object.freeze({ requirementId, evidenceClass, ...evidence });

const claims = (namespace: string, numbers: readonly number[], evidence: TestReference): RequirementEvidenceMapping[] =>
  numbers.map((number) => claim(`${namespace}-${String(number).padStart(3, "0")}`, evidence));

const aggregate = (requirementId: string): RequirementEvidenceMapping => claim(
  requirementId,
  { source: "phase016.package", testName: "phase016.package.initial-restart" },
  "package",
);

export function createCurrentRequirementEvidenceMappings(): readonly RequirementEvidenceMapping[] {
  return Object.freeze([
    ...claims("NAV", [1, 2, 3, 4, 5], ref("all_route_shared_shell_tests.ts", "home search exploration and backup modules delegate shell rendering to the shared component")),
    ...claims("NAV", [6, 7], ref("workspace_shell_contract_tests.ts", "every route uses one Korean navigation order and exactly one active item")),
    aggregate("NAV-008"),
    claim("NAV-009", ref("seven_route_exposure_gate_tests.ts", "all seven ready routes expose no internal identity, error, path, or banned English action")),
    claim("NAV-010", ref("desktop_personal_workspace_home_tests.ts", "desktop current product descriptor exposes no hosted workspace command surface")),

    claim("HOME-001", ref("desktop_react_home_render_tests.ts", "React desktop home renders semantic loading and command-backed ready content")),
    ...claims("HOME", [2, 3, 8, 9, 10], ref("desktop_react_home_render_tests.ts", "React desktop home renders empty, degraded, failed, and retry states safely")),
    claim("HOME-004", ref("desktop_personal_workspace_home_tests.ts", "desktop workspace home facade calls local command once with explicit limits")),
    claim("HOME-005", ref("desktop_react_home_render_tests.ts", "home knowledge map renders authoritative graph labels and connection counts without internal identities")),
    ...claims("HOME", [6, 7], ref("desktop_react_home_render_tests.ts", "home knowledge map reports loading, empty, and failed graph states truthfully")),

    claim("SEARCH-001", ref("desktop_react_authoring_workbench_tests.ts", "React authoring keeps the global workspace search available")),
    claim("SEARCH-002", ref("desktop_search_escape_intent_tests.ts", "search Escape clears a non-empty query before returning")),
    ...claims("SEARCH", [3, 4, 5], ref("desktop_navigator_controller_tests.ts", "desktop navigator controller uses the full-text search port for a search query")),
    ...claims("SEARCH", [6, 7], ref("desktop_react_navigator_render_tests.ts", "React navigator presents a bounded escaped search snippet without path fallback")),
    claim("SEARCH-008", ref("desktop_react_navigator_render_tests.ts", "React navigator renders loading empty degraded failed and retry states")),
    claim("SEARCH-009", ref("desktop_navigator_controller_tests.ts", "desktop navigator controller result stays generation-bound for stale response guard")),
    claim("SEARCH-010", ref("desktop_search_escape_intent_tests.ts", "search Escape clears a non-empty query before returning")),
    claim("SEARCH-011", ref("desktop_search_return_context_tests.ts", "search return context follows Results DocumentOpen Results without flags")),
    claim("SEARCH-012", ref("desktop_react_navigator_render_tests.ts", "React navigator renders only the bounded result window and explicit range actions")),

    claim("DOC-001", ref("desktop_document_menu_target_tests.ts", "document menu resumes the last authoring document before recent documents")),
    claim("DOC-002", ref("desktop_document_menu_target_tests.ts", "document menu falls back when the last authoring document was deleted")),
    claim("DOC-003", ref("desktop_document_menu_target_tests.ts", "document menu ignores blank identities and reports an empty workspace")),
    claim("DOC-004", ref("desktop_document_menu_target_tests.ts", "document menu keeps last only when it exists in the current candidates")),
    claim("DOC-005", ref("desktop_react_authoring_workbench_tests.ts", "React authoring presents the derived title without a separate metadata editor")),
    claim("DOC-006", ref("desktop_react_authoring_workbench_tests.ts", "React authoring workbench exposes a WYSIWYG surface placeholder and plain text action contract")),
    ...claims("DOC", [7, 8], ref("desktop_react_authoring_workbench_tests.ts", "React authoring presents the derived title without a separate metadata editor")),
    aggregate("DOC-009"),
    ...claims("DOC", [10, 11, 12], ref("desktop_document_authoring_controller_tests.ts", "authoring controller verifies the durable document before reporting saved")),
    claim("DOC-013", ref("desktop_document_authoring_controller_tests.ts", "authoring controller reports a failed save when durable readback is stale")),
    claim("DOC-014", ref("desktop_react_authoring_workbench_tests.ts", "React authoring workbench keeps unsafe source out of the default WYSIWYG surface and source in the modal")),
    ...claims("DOC", [15, 16, 17, 18], ref("desktop_react_authoring_workbench_tests.ts", "React authoring workbench renders real backlink identities and bounded link state")),
    ...claims("DOC", [19, 20], ref("desktop_react_authoring_workbench_tests.ts", "React authoring attachment panel keeps per-file partial and recovery outcomes truthful")),
    aggregate("DOC-021"),
    claim("DOC-022", ref("desktop_react_authoring_workbench_tests.ts", "React authoring attachment panel presents a bounded safe existing-file chooser")),
    ...claims("DOC", [23, 24], ref("desktop_asset_controller_tests.ts", "Asset controller loads native detail and confirms unlink through list readback")),
    ...claims("DOC", [25, 26], ref("desktop_react_authoring_workbench_tests.ts", "React authoring history keeps entries during cursor load-more and exposes bounded retry")),
    claim("DOC-027", ref("desktop_react_authoring_workbench_tests.ts", "React authoring history enables version-pair compare only for two user-facing selections")),
    claim("DOC-028", {
      source: "crates/cabinet-usecases/tests/compare_document_versions_tests.rs",
      testName: "compare_middle_insertion_keeps_following_lines_equal",
    }, "rust_test"),
    claim("DOC-029", ref("desktop_document_diff_operation_controller_tests.ts", "diff operation controller follows Accepted Running Ready with explicit generation")),
    ...claims("DOC", [30, 31, 32, 33, 34], ref("desktop_react_authoring_workbench_tests.ts", "React authoring restore confirmation renders full diff and explicit actions without internal tokens")),

    ...claims("GRAPH", [1, 2, 3], ref("desktop_graph_controller_tests.ts", "Graph controller loads global scope without a center document or fake center")),
    aggregate("GRAPH-004"),
    ...claims("GRAPH", [5, 6, 7, 8, 9], ref("desktop_react_exploration_surfaces_tests.ts", "knowledge graph renders the Penpot 20260721 topology with workspace data")),
    claim("GRAPH-010", ref("desktop_graph_controller_tests.ts", "Graph controller accumulates global cursor pages without losing selection")),
    ...claims("GRAPH", [11, 12], ref("desktop_react_exploration_surfaces_tests.ts", "knowledge graph routes attachment activation and detail action to the exact asset")),
    claim("GRAPH-013", ref("topology_visual_orchestrator_tests.ts", "topology orchestrator owns camera resize and deterministic disposal")),
    claim("GRAPH-014", ref("desktop_graph_preference_controller_tests.ts", "graph preference save keeps validated session data when persistence fails")),
    ...claims("GRAPH", [15, 16], ref("desktop_graph_controller_tests.ts", "Graph repair runs reindex, worker, freshness, and graph reload in order")),
    claim("GRAPH-017", ref("react_topology_visual_host_accessibility_tests.ts", "topology semantic list exposes one roving tab stop and a safe selected summary")),

    ...claims("CANVAS", [1, 2, 3], ref("desktop_canvas_catalog_controller_tests.ts", "canvas catalog controller loads last-used data and represents an explicit empty catalog")),
    aggregate("CANVAS-004"),
    ...claims("CANVAS", [5, 6, 7, 8, 9, 10, 11], ref("desktop_react_exploration_surfaces_tests.ts", "canvas renders durable nodes, edges, revision and viewport controls without session fixtures")),
    ...claims("CANVAS", [12, 13, 14], ref("desktop_canvas_controller_tests.ts", "Canvas controller previews auto arrange without replacing durable state and applies from base revision")),
    ...claims("CANVAS", [15, 16, 17, 18], ref("desktop_react_exploration_surfaces_tests.ts", "canvas controls dispatch durable create, mutation, zoom and remove callbacks")),
    ...claims("CANVAS", [19, 20], ref("canvas_viewport_projection_tests.ts", "Canvas viewport projection filters geometry, edges and reports truncation")),
    ...claims("CANVAS", [21, 22], ref("desktop_canvas_controller_tests.ts", "Canvas controller acknowledges rename then archive lifecycle and blocks later mutation")),
    ...claims("CANVAS", [23, 24], ref("desktop_canvas_controller_tests.ts", "Canvas controller recovers with operation identity and returns to durable Ready state")),
    claim("CANVAS-025", ref("canvas_render_performance_tests.ts", "Canvas bounded React tree preparation p95 remains below the Phase 012 300ms budget")),

    ...claims("ASSET", [1, 2, 3], ref("desktop_asset_controller_tests.ts", "Asset controller imports opaque selections and completes only after durable readback")),
    ...claims("ASSET", [4, 5], ref("desktop_asset_controller_tests.ts", "Asset controller imports a prepared drop selection without reopening the picker")),
    ...claims("ASSET", [6, 7], ref("desktop_asset_controller_tests.ts", "Asset controller records deterministic partial success and continues after one file fails")),
    ...claims("ASSET", [8, 9, 10], ref("desktop_asset_controller_tests.ts", "Asset controller loads native detail and confirms unlink through list readback")),
    aggregate("ASSET-011"),
    ...claims("ASSET", [12, 13], ref("desktop_asset_controller_tests.ts", "Asset controller links a workspace asset and completes through document readback")),
    claim("ASSET-014", ref("desktop_asset_controller_tests.ts", "Asset controller appends a bounded workspace page by opaque cursor without duplicates")),
    claim("ASSET-015", ref("desktop_asset_controller_tests.ts", "Asset controller owns bounded-page query and media filter state without matching internal identity")),
    ...claims("ASSET", [16, 17, 18], ref("desktop_asset_controller_tests.ts", "Asset controller repairs projection then verifies attachment readback")),

    ...claims("BACKUP", [1, 2, 3], ref("desktop_backup_recovery_controller_tests.ts", "durable backup start remains creating until completed status is validated")),
    ...claims("BACKUP", [4, 5, 6], ref("desktop_backup_recovery_controller_tests.ts", "validated preview exposes all backup classes and confirmation state")),
    ...claims("BACKUP", [7, 8, 9, 10], ref("desktop_backup_recovery_controller_tests.ts", "confirmed restore starts durable staging and polls only to native terminal state")),
    aggregate("BACKUP-011"),
    ...claims("BACKUP", [12, 13], ref("desktop_backup_recovery_controller_tests.ts", "rollback failure remains recovery required and retryable")),
  ]);
}
