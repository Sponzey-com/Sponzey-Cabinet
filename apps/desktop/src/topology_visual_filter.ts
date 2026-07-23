import type { GraphDisplayNode } from "./graph_display_presenter.ts";

export interface TopologyVisualEdge {
  readonly sourceId: string;
  readonly targetId: string;
}

export function filterTopologyVisualGraph<
  Node extends GraphDisplayNode,
  Edge extends TopologyVisualEdge,
>(
  nodes: readonly Node[],
  edges: readonly Edge[],
  query: string,
  options: Readonly<{ readonly includeExternal?: boolean }> = {},
): Readonly<{ readonly nodes: readonly Node[]; readonly edges: readonly Edge[] }> {
  const normalized = query.trim().toLocaleLowerCase("ko-KR");
  const kindFiltered = options.includeExternal === true
    ? nodes
    : nodes.filter((node) => node.kind !== "external_link");
  if (!normalized) {
    const visibleIds = new Set(kindFiltered.map((node) => node.identity));
    return Object.freeze({
      nodes: kindFiltered,
      edges: Object.freeze(edges.filter((edge) => visibleIds.has(edge.sourceId) && visibleIds.has(edge.targetId))),
    });
  }
  const visibleNodes = Object.freeze(kindFiltered.filter((node) =>
    `${node.label} ${node.kindLabel} ${node.breadcrumbLabel}`
      .toLocaleLowerCase("ko-KR")
      .includes(normalized),
  ));
  const visibleIds = new Set(visibleNodes.map((node) => node.identity));
  const visibleEdges = Object.freeze(edges.filter((edge) =>
    visibleIds.has(edge.sourceId) && visibleIds.has(edge.targetId),
  ));
  return Object.freeze({ nodes: visibleNodes, edges: visibleEdges });
}
