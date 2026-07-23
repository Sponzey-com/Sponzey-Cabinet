import assert from "node:assert/strict";
import test from "node:test";
import { presentTopologyEmptyState } from "../src/topology_empty_state_presenter.ts";

test("topology empty presenter distinguishes documents filters and relations", () => {
  assert.equal(presentTopologyEmptyState({ sourceNodeCount: 0, sourceEdgeCount: 0, visibleNodeCount: 0, visualFilterActive: false })?.kind, "NoDocuments");
  assert.equal(presentTopologyEmptyState({ sourceNodeCount: 2, sourceEdgeCount: 1, visibleNodeCount: 0, visualFilterActive: true })?.kind, "NoFilterResults");
  assert.equal(presentTopologyEmptyState({ sourceNodeCount: 1, sourceEdgeCount: 0, visibleNodeCount: 1, visualFilterActive: false })?.kind, "NoRelations");
  assert.equal(presentTopologyEmptyState({ sourceNodeCount: 2, sourceEdgeCount: 1, visibleNodeCount: 2, visualFilterActive: false }), undefined);
});

test("topology empty messages are stable Korean copy without counts or identity", () => {
  const states = [
    presentTopologyEmptyState({ sourceNodeCount: 0, sourceEdgeCount: 0, visibleNodeCount: 0, visualFilterActive: false }),
    presentTopologyEmptyState({ sourceNodeCount: 2, sourceEdgeCount: 1, visibleNodeCount: 0, visualFilterActive: true }),
    presentTopologyEmptyState({ sourceNodeCount: 1, sourceEdgeCount: 0, visibleNodeCount: 1, visualFilterActive: false }),
  ];
  assert.deepEqual(states.map((state) => state?.message), [
    "아직 지도에 표시할 문서가 없습니다.",
    "현재 검색과 필터에 맞는 항목이 없습니다.",
    "문서 사이에 연결된 관계가 없습니다.",
  ]);
  assert.doesNotMatch(JSON.stringify(states), /doc-|asset-|\d+개/);
});

