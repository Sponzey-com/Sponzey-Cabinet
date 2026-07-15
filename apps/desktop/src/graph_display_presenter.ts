import type { KnowledgeGraphNodeView } from "@sponzey-cabinet/client-core";
import type { DisplayReference, DisplayReferenceState } from "./display_reference_resolver.ts";

export interface GraphDisplayNode {
  readonly identity: string;
  readonly kind: KnowledgeGraphNodeView["kind"];
  readonly label: string;
  readonly breadcrumbLabel: string;
  readonly kindLabel: string;
  readonly state: DisplayReferenceState;
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
  references: readonly DisplayReference[],
): readonly GraphDisplayNode[] {
  const byIdentity = new Map(references.map((reference) => [reference.identity, reference]));
  return Object.freeze(nodes.map((node) => {
    const reference = node.kind === "document" ? byIdentity.get(node.id) : undefined;
    return Object.freeze({
      identity: node.id,
      kind: node.kind,
      label: reference?.label ?? missingLabels[node.kind],
      breadcrumbLabel: reference?.breadcrumbLabel ?? "",
      kindLabel: kindLabels[node.kind],
      state: reference?.state ?? "missing",
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
