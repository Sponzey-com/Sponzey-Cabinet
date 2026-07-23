export interface DocumentRestoreRequestContext {
  readonly authoringGeneration: number;
  readonly restoreGeneration: number;
  readonly documentId: string;
}

export function createDocumentRestoreRequestContext(
  authoringGeneration: number,
  restoreGeneration: number,
  documentId: string,
): DocumentRestoreRequestContext {
  return Object.freeze({ authoringGeneration, restoreGeneration, documentId });
}

export function isCurrentDocumentRestoreRequest(
  request: DocumentRestoreRequestContext,
  current: DocumentRestoreRequestContext,
): boolean {
  return request.authoringGeneration === current.authoringGeneration
    && request.restoreGeneration === current.restoreGeneration
    && request.documentId === current.documentId;
}
