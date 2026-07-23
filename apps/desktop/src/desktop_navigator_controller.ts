import {
  createSearchAssetsQuery,
  createSearchDocumentsQuery,
  LocalDesktopCommandClientError,
  type LocalDesktopCommandClient,
  type AssetSearchResultsPage,
  type SearchResultsPage,
} from "@sponzey-cabinet/client-core";
import {
  applyDocumentNavigatorResult,
  createDocumentNavigatorFailedModel,
  createDocumentNavigatorQuery,
  type DocumentNavigatorModel,
} from "@sponzey-cabinet/ui";

export interface DesktopNavigatorClock {
  readonly nowMs: () => number;
}

const performanceClock: DesktopNavigatorClock = Object.freeze({
  nowMs: () => performance.now(),
});

export async function loadDesktopDocumentNavigator(
  client: Pick<LocalDesktopCommandClient, "getDocumentNavigator" | "searchDocuments" | "searchAssets">,
  model: DocumentNavigatorModel,
  clock: DesktopNavigatorClock = performanceClock,
): Promise<DocumentNavigatorModel> {
  const query = createDocumentNavigatorQuery({
    workspaceId: model.workspaceId,
    view: model.view,
    viewKey: model.viewKey,
    filter: model.filter,
    limit: 50,
  });
  if (!query) {
    return createDocumentNavigatorFailedModel({
      workspaceId: model.workspaceId,
      view: model.view,
      viewKey: model.viewKey,
      filter: model.filter,
      generation: model.generation,
      errorCode: "DOCUMENT_NAVIGATOR_INVALID_QUERY",
      retryable: false,
    });
  }
  try {
    if (model.filter) {
      const startedAt = clock.nowMs();
      const [page, assetPage] = await Promise.all([
        client.searchDocuments(
          createSearchDocumentsQuery(model.workspaceId, model.filter, 50),
        ),
        client.searchAssets(
          createSearchAssetsQuery(model.workspaceId, model.filter, 50),
        ),
      ]);
      const finishedAt = clock.nowMs();
      return applyDocumentNavigatorResult(
        model,
        model.generation,
        mapSearchPage(model, page, assetPage, Math.max(0, finishedAt - startedAt)),
      );
    }
    const result = await client.getDocumentNavigator(query);
    return applyDocumentNavigatorResult(model, model.generation, result);
  } catch (error) {
    const mapped = error instanceof LocalDesktopCommandClientError
      ? { code: error.code, retryable: error.retryable }
      : { code: "COMMAND_BRIDGE_FAILED", retryable: false };
    return createDocumentNavigatorFailedModel({
      workspaceId: model.workspaceId,
      view: model.view,
      viewKey: model.viewKey,
      filter: model.filter,
      generation: model.generation,
      errorCode: mapped.code,
      retryable: mapped.retryable,
    });
  }
}

function mapSearchPage(
  model: DocumentNavigatorModel,
  page: SearchResultsPage,
  assetPage: AssetSearchResultsPage,
  durationMs: number,
): Parameters<typeof applyDocumentNavigatorResult>[2] {
  return {
    workspaceId: page.workspaceId,
    view: model.view,
    state: page.results.length > 0 ? "Ready" : "EmptyResult",
    searchMetrics: { durationMs },
    assetResults: assetPage.results.map((asset) => ({ ...asset })),
    items: page.results.map((result) => ({
      documentId: result.documentId,
      title: result.title,
      path: result.path,
      snippet: result.snippet,
      collections: [],
      tags: [],
      favorite: false,
    })),
  };
}
