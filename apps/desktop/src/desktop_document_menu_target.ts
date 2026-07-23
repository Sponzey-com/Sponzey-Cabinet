export type DesktopDocumentMenuTarget =
  | { readonly kind: "LastDocument"; readonly documentId: string }
  | { readonly kind: "RecentDocument"; readonly documentId: string }
  | { readonly kind: "EmptyWorkspace" };

export function resolveDesktopDocumentMenuTarget(
  lastAuthoringDocumentId: string | undefined,
  recentDocumentIds: readonly string[],
): DesktopDocumentMenuTarget {
  const currentDocumentIds = [...new Set(
    recentDocumentIds.map(normalizeDocumentId).filter((documentId): documentId is string => documentId !== undefined),
  )];
  const lastAuthoring = normalizeDocumentId(lastAuthoringDocumentId);
  if (lastAuthoring && currentDocumentIds.includes(lastAuthoring)) {
    return { kind: "LastDocument", documentId: lastAuthoring };
  }
  const recentDocument = currentDocumentIds[0];
  return recentDocument
    ? { kind: "RecentDocument", documentId: recentDocument }
    : { kind: "EmptyWorkspace" };
}

function normalizeDocumentId(documentId: string | undefined): string | undefined {
  const normalized = documentId?.trim();
  return normalized ? normalized : undefined;
}
