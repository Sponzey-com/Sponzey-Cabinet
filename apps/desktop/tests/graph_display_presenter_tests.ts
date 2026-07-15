import assert from "node:assert/strict";
import test from "node:test";

import { filterGraphDisplayNodes, presentGraphNodes } from "../src/graph_display_presenter.ts";

test("graph presenter uses resolved titles and never falls back to node identity", () => {
  const nodes = [
    { id: "doc-secret-1", kind: "document" as const },
    { id: "doc-secret-missing", kind: "document" as const },
    { id: "asset-secret", kind: "attachment" as const },
    { id: "link-secret", kind: "unresolved_link" as const },
  ];
  const presented = presentGraphNodes(nodes, [{
    category: "document", identity: "doc-secret-1", label: "설계 문서", breadcrumbLabel: "프로젝트 / 설계", statusLabel: "", state: "resolved",
  }]);
  assert.deepEqual(presented.map((item) => [item.label, item.kindLabel, item.state]), [
    ["설계 문서", "문서", "resolved"],
    ["찾을 수 없는 문서", "문서", "missing"],
    ["첨부 파일", "첨부 파일", "missing"],
    ["미해결 링크", "미해결 링크", "missing"],
  ]);
  const visible = presented.map((item) => `${item.label} ${item.breadcrumbLabel} ${item.kindLabel}`).join(" ");
  for (const node of nodes) assert.equal(visible.includes(node.id), false);
});

test("graph filter matches title and breadcrumb but not internal identity", () => {
  const nodes = presentGraphNodes([{ id: "opaque-secret", kind: "document" }], [{
    category: "document", identity: "opaque-secret", label: "로컬 저장소 설계", breadcrumbLabel: "아키텍처 / 저장소", statusLabel: "", state: "resolved",
  }]);
  assert.equal(filterGraphDisplayNodes(nodes, "저장소").length, 1);
  assert.equal(filterGraphDisplayNodes(nodes, "아키텍처").length, 1);
  assert.equal(filterGraphDisplayNodes(nodes, "opaque-secret").length, 0);
});
