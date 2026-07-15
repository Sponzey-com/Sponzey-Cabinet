import assert from "node:assert/strict";
import test from "node:test";

import {
  DEFAULT_UI_AUDIT_POLICY,
  auditSemanticTextRecords,
  auditUiSource,
  buildRouteShellInventory,
  collectBaseline,
  createBaselineReport,
  renderBaselineArtifact,
  validateBaselineReport,
} from "./phase013_ui_baseline.mjs";

test("semantic audit accepts Korean product copy and approved technical terms", () => {
  const result = auditSemanticTextRecords([
    record("Home", "Ready", "visible_text", "문서를 저장했습니다"),
    record("Document", "Ready", "accessible_name", "Markdown 원문 편집기"),
    record("Assets", "Ready", "tooltip", "PDF 미리보기"),
    record("Assets", "Ready", "visible_text", "report.md"),
    record("Home", "Ready", "visible_text", "Cabinet에서 AI에게 질문"),
    record("Document", "Ready", "tooltip", "Cmd+S로 저장"),
  ], DEFAULT_UI_AUDIT_POLICY);

  assert.equal(result.state, "Passed");
  assert.deepEqual(result.findings, []);
});

test("semantic audit rejects English copy in visible, accessible, and tooltip channels", () => {
  const result = auditSemanticTextRecords([
    record("Search", "Loading", "visible_text", "Loading documents"),
    record("Backup", "Ready", "accessible_name", "Back to workspace"),
    record("Canvas", "Ready", "tooltip", "Current mode"),
  ], DEFAULT_UI_AUDIT_POLICY);

  assert.equal(result.state, "Failed");
  assert.deepEqual(
    result.findings.map(({ category, channel }) => [category, channel]),
    [
      ["mixed_language", "visible_text"],
      ["mixed_language", "accessible_name"],
      ["mixed_language", "tooltip"],
    ],
  );
});

test("semantic audit rejects internal identities, stable error codes, and absolute paths", () => {
  const result = auditSemanticTextRecords([
    record("Graph", "Ready", "visible_text", "doc-01J123456789ABCDEFGHJKMNPQ"),
    record("Document", "Failed", "visible_text", "DOCUMENT_SAVE_FAILED"),
    record("Assets", "Ready", "tooltip", "/Users/example/Cabinet/private.md"),
    record("Backup", "Ready", "accessible_name", "C:\\Users\\example\\Cabinet\\backup.zip"),
    record("Canvas", "Ready", "visible_text", "550e8400-e29b-41d4-a716-446655440000"),
  ], DEFAULT_UI_AUDIT_POLICY);

  assert.deepEqual(
    result.findings.map(({ category }) => category),
    ["internal_identity", "internal_error_code", "absolute_path", "absolute_path", "internal_identity"],
  );
});

test("semantic audit rejects malformed channel and never mutates its input", () => {
  const records = Object.freeze([
    Object.freeze({ route: "Home", state: "Ready", channel: "unknown", value: "홈", source: "synthetic", line: 1 }),
  ]);

  const result = auditSemanticTextRecords(records, DEFAULT_UI_AUDIT_POLICY);

  assert.equal(result.state, "Failed");
  assert.equal(result.findings[0]?.category, "invalid_record");
  assert.equal(records[0].channel, "unknown");
});

test("route shell inventory records owners and reports missing source, marker, and duplicate route", () => {
  const descriptors = [
    descriptor("Home", "home.ts", "cabinet-home-shell"),
    descriptor("Document", "document.ts", "document-shell"),
    descriptor("Document", "missing.ts", "document-shell"),
    descriptor("Backup", "backup.ts", "backup-shell"),
  ];
  const sources = {
    "home.ts": "desktop-shell cabinet-home-shell desktop-sidebar desktop-topbar",
    "document.ts": "desktop-shell document-shell desktop-sidebar desktop-topbar",
    "backup.ts": "desktop-shell desktop-sidebar desktop-topbar",
  };

  const result = buildRouteShellInventory(descriptors, sources);

  assert.equal(result.records.length, 3);
  assert.ok(result.findings.some((finding) => finding.category === "duplicate_route"));
  assert.ok(result.findings.some((finding) => finding.category === "source_missing"));
  assert.ok(result.findings.some((finding) => finding.category === "shell_marker_missing"));
  assert.equal(result.records[0]?.ownsSidebar, true);
  assert.equal(result.records[0]?.ownsTopbar, true);
});

test("source audit finds hard-coded copy and direct identity/error rendering without copying source lines", () => {
  const source = [
    'e("button", { title: "Current mode" }, "Save")',
    'e("strong", null, node.id)',
    'e("p", null, snapshot.errorCode)',
    'e("button", { onClick: () => open(node.id) }, "문서 열기")',
  ].join("\n");

  const result = auditUiSource({ route: "Graph", sourceFile: "react_graph.ts", source }, DEFAULT_UI_AUDIT_POLICY);

  assert.ok(result.findings.some((finding) => finding.category === "mixed_language"));
  assert.ok(result.findings.some((finding) => finding.category === "internal_identity"));
  assert.ok(result.findings.some((finding) => finding.category === "internal_error_code"));
  assert.equal(result.findings.some((finding) => finding.evidence?.includes("onClick")), false);
  assert.equal(result.findings.some((finding) => finding.sourceText), false);
});

test("source audit ignores tag, class, action, callback, and internal enum strings", () => {
  const source = [
    'e("button", { type: "button", className: "graph-save", "data-action": "save-document", onClick: () => open(node.id) }, "저장")',
    'const state = snapshot.state === "Ready" ? "준비됨" : "대기 중";',
  ].join("\n");

  const result = auditUiSource({ route: "Graph", sourceFile: "react_graph.ts", source }, DEFAULT_UI_AUDIT_POLICY);

  assert.deepEqual(result.findings, []);
});

test("source audit preserves the original line offset for a route-specific source segment", () => {
  const result = auditUiSource({
    route: "Canvas",
    sourceFile: "react_exploration.ts",
    source: 'e("button", null, "Retry")',
    lineOffset: 249,
  }, DEFAULT_UI_AUDIT_POLICY);

  assert.equal(result.findings[0]?.line, 250);
});

test("baseline artifact is sanitized and validation rejects stale or unsafe reports", () => {
  const report = createBaselineReport({
    sourceFingerprint: "a".repeat(64),
    fixtureHash: "b".repeat(64),
    inventory: {
      records: [{ route: "Home", sourceFile: "apps/desktop/src/home.ts", shellMarker: "home-shell", ownsSidebar: true, ownsTopbar: true, sourceHash: "c".repeat(64) }],
      findings: [],
    },
    findings: [{ category: "mixed_language", route: "Home", channel: "visible_text", source: "apps/desktop/src/home.ts", line: 12, evidence: "Save" }],
  });
  const artifact = renderBaselineArtifact(report);

  assert.match(artifact, /phase013_ui_baseline=recorded/);
  assert.match(artifact, /source_fingerprint=a{64}/);
  assert.match(artifact, /finding_count=1/);
  assert.doesNotMatch(artifact, /\/Users\/dongwooshin/);
  assert.doesNotMatch(artifact, /sourceText/);
  assert.deepEqual(validateBaselineReport(report, { sourceFingerprint: "a".repeat(64), fixtureHash: "b".repeat(64) }), []);
  assert.ok(validateBaselineReport(report, { sourceFingerprint: "d".repeat(64), fixtureHash: "b".repeat(64) }).includes("stale_source_fingerprint"));

  const unsafe = { ...report, findings: [{ ...report.findings[0], source: "/Users/example/private.ts" }] };
  assert.ok(validateBaselineReport(unsafe, { sourceFingerprint: "a".repeat(64), fixtureHash: "b".repeat(64) }).includes("unsafe_source_path"));
});

test("baseline collection reads source only through the injected boundary", async () => {
  const descriptors = [
    descriptor("Home", "apps/desktop/src/home.ts", "home-shell"),
    descriptor("Backup", "apps/desktop/src/backup.ts", "backup-surface"),
  ];
  const sourceByAbsolutePath = new Map([
    ["/workspace/apps/desktop/src/home.ts", 'e("div", { className: "desktop-shell home-shell desktop-sidebar desktop-topbar" }, "Home")'],
    ["/workspace/apps/desktop/src/backup.ts", 'e("section", { className: "backup-surface" }, "Create backup")'],
  ]);
  const reads = [];

  const report = await collectBaseline({
    rootDir: "/workspace",
    descriptors,
    readText: async (path) => {
      reads.push(path);
      const source = sourceByAbsolutePath.get(path);
      if (!source) throw new Error("SOURCE_NOT_FOUND");
      return source;
    },
    policy: DEFAULT_UI_AUDIT_POLICY,
  });

  assert.deepEqual(reads, ["/workspace/apps/desktop/src/home.ts", "/workspace/apps/desktop/src/backup.ts"]);
  assert.equal(report.routeCount, 2);
  assert.equal(report.shellOwnerCount, 1);
  assert.ok(report.findings.some((finding) => finding.category === "mixed_language"));
  assert.ok(/^[a-f0-9]{64}$/.test(report.sourceFingerprint));
  assert.ok(/^[a-f0-9]{64}$/.test(report.fixtureHash));
});

function record(route, state, channel, value) {
  return { route, state, channel, value, source: "synthetic", line: 1 };
}

function descriptor(route, sourceFile, shellMarker) {
  return { route, sourceFile, shellMarker, sidebarMarker: "desktop-sidebar", topbarMarker: "desktop-topbar" };
}
