const SEARCH_SNIPPET_LIMIT = 160;

export function presentSearchResultSnippet(snippet: string | undefined): string | undefined {
  const normalized = snippet?.replace(/\s+/gu, " ").trim();
  if (!normalized) return undefined;
  return normalized.slice(0, SEARCH_SNIPPET_LIMIT);
}
