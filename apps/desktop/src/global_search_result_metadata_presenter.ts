export interface GlobalSearchResultMetadataInput {
  readonly documentCount: number;
  readonly assetCount?: number;
  readonly durationMs?: number;
}

export function presentGlobalSearchResultMetadata(
  input: GlobalSearchResultMetadataInput,
): string {
  const totalCount = clampCount(input.documentCount) + clampCount(input.assetCount ?? 0);
  const duration = presentDuration(input.durationMs);
  return duration ? `${totalCount}개 결과 · ${duration}` : `${totalCount}개 결과`;
}

function clampCount(value: number): number {
  if (!Number.isFinite(value)) return 0;
  if (value < 0) return 0;
  return Math.floor(value);
}

function presentDuration(durationMs: number | undefined): string | undefined {
  if (durationMs === undefined || !Number.isFinite(durationMs) || durationMs < 0) {
    return undefined;
  }
  return `${Math.round(durationMs)}ms`;
}
