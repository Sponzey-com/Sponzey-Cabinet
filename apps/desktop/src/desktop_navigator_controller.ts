import {
  LocalDesktopCommandClientError,
  type LocalDesktopCommandClient,
} from "@sponzey-cabinet/client-core";
import {
  applyDocumentNavigatorResult,
  createDocumentNavigatorFailedModel,
  createDocumentNavigatorQuery,
  type DocumentNavigatorModel,
} from "@sponzey-cabinet/ui";

export async function loadDesktopDocumentNavigator(
  client: Pick<LocalDesktopCommandClient, "getDocumentNavigator">,
  model: DocumentNavigatorModel,
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
