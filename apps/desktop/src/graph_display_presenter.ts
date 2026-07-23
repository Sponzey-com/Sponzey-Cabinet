import type { KnowledgeGraphNodeView } from "@sponzey-cabinet/client-core";
import type { DisplayReferenceState } from "./display_reference_resolver.ts";

export interface GraphDisplayNode {
  readonly identity: string;
  readonly kind: KnowledgeGraphNodeView["kind"];
  readonly label: string;
  readonly breadcrumbLabel: string;
  readonly kindLabel: string;
  readonly state: DisplayReferenceState;
  readonly canNavigate: boolean;
}

const kindLabels = Object.freeze({
  document: "문서",
  unresolved_link: "미해결 링크",
  attachment: "첨부 파일",
  external_link: "외부 링크",
});

const missingLabels = Object.freeze({
  document: "찾을 수 없는 문서",
  unresolved_link: "미해결 링크",
  attachment: "첨부 파일",
  external_link: "외부 링크",
});

export function presentGraphNodes(
  nodes: readonly KnowledgeGraphNodeView[],
): readonly GraphDisplayNode[] {
  return Object.freeze(nodes.map((node) => {
    const available = node.availability === "available";
    return Object.freeze({
      identity: node.id,
      kind: node.kind,
      label: node.label?.trim() || missingLabels[node.kind],
      breadcrumbLabel: node.breadcrumbLabel?.trim() || "",
      kindLabel: kindLabels[node.kind],
      state: available ? "resolved" : "missing",
      canNavigate: available && node.canNavigate === true,
    });
  }));
}

export function filterGraphDisplayNodes(
  nodes: readonly GraphDisplayNode[],
  query: string,
): readonly GraphDisplayNode[] {
  const normalized = query.trim().toLocaleLowerCase("ko-KR");
  if (!normalized) return nodes;
  return Object.freeze(nodes.filter((node) =>
    `${node.label} ${node.breadcrumbLabel}`.toLocaleLowerCase("ko-KR").includes(normalized),
  ));
}
