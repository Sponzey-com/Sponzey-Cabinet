export interface TopologyEmptyState {
  readonly kind: "NoDocuments" | "NoFilterResults" | "NoRelations";
  readonly message: string;
}

export function presentTopologyEmptyState(input: {
  readonly sourceNodeCount: number;
  readonly sourceEdgeCount: number;
  readonly visibleNodeCount: number;
  readonly visualFilterActive: boolean;
}): TopologyEmptyState | undefined {
  if (input.sourceNodeCount === 0) {
    return Object.freeze({ kind: "NoDocuments", message: "아직 지도에 표시할 문서가 없습니다." });
  }
  if (input.visibleNodeCount === 0 && input.visualFilterActive) {
    return Object.freeze({ kind: "NoFilterResults", message: "현재 검색과 필터에 맞는 항목이 없습니다." });
  }
  if (input.sourceEdgeCount === 0) {
    return Object.freeze({ kind: "NoRelations", message: "문서 사이에 연결된 관계가 없습니다." });
  }
  return undefined;
}
