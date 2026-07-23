import assert from "node:assert/strict";
import test from "node:test";

import { filterTopologyVisualGraph } from "../src/topology_visual_filter.ts";

const nodes = [
  { identity: "secret-architecture", kind: "document" as const, label: "설계 개요", breadcrumbLabel: "프로젝트", kindLabel: "문서", state: "resolved" as const, canNavigate: true },
  { identity: "secret-attachment", kind: "attachment" as const, label: "회의 자료", breadcrumbLabel: "", kindLabel: "첨부 파일", state: "resolved" as const, canNavigate: true },
  { identity: "secret-link", kind: "external_link" as const, label: "참고 링크", breadcrumbLabel: "외부", kindLabel: "외부 링크", state: "resolved" as const, canNavigate: false },
];
const edges = [
  { id: "edge-1", sourceId: "secret-architecture", targetId: "secret-attachment", kind: "attachment_reference" as const },
  { id: "edge-2", sourceId: "secret-architecture", targetId: "secret-link", kind: "external_reference" as const },
];

test("topology visual filter searches safe labels and keeps only complete edges", () => {
  const byLabel = filterTopologyVisualGraph(nodes, edges, "설계");
  assert.deepEqual(byLabel.nodes.map((node) => node.label), ["설계 개요"]);
  assert.deepEqual(byLabel.edges, []);

  const byKind = filterTopologyVisualGraph(nodes, edges, "첨부 파일");
  assert.deepEqual(byKind.nodes.map((node) => node.label), ["회의 자료"]);

  const byBreadcrumb = filterTopologyVisualGraph(nodes, edges, "외부", { includeExternal: true });
  assert.deepEqual(byBreadcrumb.nodes.map((node) => node.label), ["참고 링크"]);
});

test("topology visual filter never searches identity and preserves empty-query ordering", () => {
  assert.deepEqual(filterTopologyVisualGraph(nodes, edges, "secret-architecture"), { nodes: [], edges: [] });
  const all = filterTopologyVisualGraph(nodes, edges, "   ", { includeExternal: true });
  assert.deepEqual(all.nodes, nodes);
  assert.deepEqual(all.edges, edges);
});

test("topology visual filter hides external nodes by default and keeps complete edges when enabled", () => {
  const nodes = [
    { identity: "doc-1", kind: "document" as const, label: "문서", breadcrumbLabel: "", kindLabel: "문서", state: "resolved" as const, canNavigate: true },
    { identity: "external-1", kind: "external_link" as const, label: "외부 자료", breadcrumbLabel: "", kindLabel: "외부 링크", state: "resolved" as const, canNavigate: false },
  ];
  const edges = [{ id: "edge", sourceId: "doc-1", targetId: "external-1" }];
  assert.deepEqual(filterTopologyVisualGraph(nodes, edges, "").nodes.map((item) => item.identity), ["doc-1"]);
  const enabled = filterTopologyVisualGraph(nodes, edges, "", { includeExternal: true });
  assert.equal(enabled.nodes.length, 2);
  assert.equal(enabled.edges.length, 1);
});
