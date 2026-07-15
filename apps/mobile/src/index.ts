import type {
  AccessibleDocumentView,
  CabinetHttpResponse,
  CabinetHttpTransport,
  CommentThreadPageView,
  DocumentHistoryPage,
  DocumentIdentity,
  HttpMethodName,
  MobileReadApiContract,
  PlatformFeatureSupport,
  PermissionDecisionView,
  ReviewWorkflowActionView,
  ReviewRequestPageView,
  SearchAccessibleDocumentsView,
  SelfHostApiClientConfig,
} from "@sponzey-cabinet/client-core";
import {
  CabinetApiClientError,
  createFetchHttpTransport,
  createMobileReadApiContract,
  createPlatformCapabilityMatrix,
  createSelfHostApiClient,
  createSelfHostApiClientConfig,
  validateMobileReadApiResponse,
  withSelfHostSessionToken,
} from "@sponzey-cabinet/client-core";

export type MobilePlatform = "ios" | "android";

export type MobileReadDisplayState =
  | "Idle"
  | "Loading"
  | "Loaded"
  | "UnsupportedAction"
  | "Error";

export type MobileReadEvent =
  | "LoadCurrent"
  | "LoadHistory"
  | "Search"
  | "LoadComments"
  | "LoadReviewRequests"
  | "LoadSucceeded"
  | "ApiFailure"
  | "EditRequested"
  | "CanvasEditRequested";

export type MobileReadErrorCode =
  | "MOBILE_UNAUTHORIZED"
  | "MOBILE_SESSION_EXPIRED"
  | "MOBILE_NETWORK_FAILURE"
  | "MOBILE_CONTRACT_VERSION_MISMATCH"
  | "MOBILE_PERMISSION_DECISION_MISSING"
  | "MOBILE_CONTRACT_RESPONSE_INVALID"
  | "MOBILE_UNSUPPORTED_EDIT"
  | "MOBILE_UNSUPPORTED_CANVAS_EDIT"
  | "MOBILE_API_ERROR";

export interface MobileReadSkeletonConfigInput {
  readonly platform: MobilePlatform;
  readonly apiBaseUrl: string;
  readonly sessionToken: string;
  readonly contractVersion: string;
}

export interface MobileReadSkeletonConfig {
  readonly platform: MobilePlatform;
  readonly apiBaseUrl: string;
  readonly sessionToken: string;
  readonly contractVersion: string;
}

export interface MobileReadCapabilities {
  readonly supportsMobileReadApi: boolean;
  readonly supportsRemoteEdit: false;
  readonly supportsOfflineRemoteEdit: false;
  readonly knowledgeGraphSupport: PlatformFeatureSupport;
  readonly canvasSupport: PlatformFeatureSupport;
  readonly realtimeCollaborationSupport: PlatformFeatureSupport;
  readonly supportsCanvasFullEdit: false;
}

export interface MobileReadErrorView {
  readonly code: MobileReadErrorCode;
  readonly message: string;
}

export interface MobilePermissionDecisionView {
  readonly result: PermissionDecisionView["result"];
  readonly reasonCode: string;
}

export interface MobileCurrentDocumentViewModel {
  readonly kind: "current-document";
  readonly workspaceId: string;
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
  readonly body: string;
  readonly versionId: string;
  readonly permissionDecision: MobilePermissionDecisionView;
  readonly canEdit: false;
}

export interface MobileHistoryEntryViewModel {
  readonly versionId: string;
  readonly summary: string;
  readonly author: string;
  readonly createdAt: string;
}

export interface MobileDocumentHistoryViewModel {
  readonly kind: "document-history";
  readonly workspaceId: string;
  readonly documentId: string;
  readonly entries: readonly MobileHistoryEntryViewModel[];
  readonly nextCursor?: string;
}

export interface MobileSearchResultViewModel {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
  readonly snippet: string;
}

export interface MobileSearchViewModel {
  readonly kind: "search-results";
  readonly workspaceId: string;
  readonly text: string;
  readonly results: readonly MobileSearchResultViewModel[];
  readonly permissionFilteredCount: number;
  readonly durationMs: number;
}

export interface MobileCommentThreadViewModel {
  readonly threadId: string;
  readonly documentId: string;
  readonly state: string;
  readonly commentCount: number;
  readonly anchorStatus?: string;
}

export interface MobileCommentsViewModel {
  readonly kind: "comments";
  readonly threads: readonly MobileCommentThreadViewModel[];
}

export interface MobileReviewRequestViewModel {
  readonly reviewRequestId: string;
  readonly documentId: string;
  readonly status: string;
}

export interface MobileReviewRequestsViewModel {
  readonly kind: "review-requests";
  readonly requests: readonly MobileReviewRequestViewModel[];
}

export type MobileReviewDecision = "approved" | "rejected";

export type MobilePushNotificationEventName =
  | "document.changed"
  | "comment.changed"
  | "review.state_changed"
  | "lock.state_changed"
  | "canvas.changed";

export type MobilePushNotificationTargetKind =
  | "document"
  | "comment_thread"
  | "review_request"
  | "document_lock"
  | "canvas";

export type MobileNotificationDeliveryState = "Queued" | "Sent" | "Failed" | "Retry";

export type MobileNotificationDeliveryEvent =
  | "Enqueue"
  | "SendSucceeded"
  | "SendFailed"
  | "RetryRequested"
  | "RetryScheduled"
  | "GiveUp";

export type MobileNotificationDeliveryErrorCode = "MOBILE_NOTIFICATION_INVALID_TRANSITION";

export interface MobilePushNotificationTarget {
  readonly kind: MobilePushNotificationTargetKind;
  readonly id: string;
}

export interface MobilePushNotificationInput {
  readonly eventName: MobilePushNotificationEventName;
  readonly target: MobilePushNotificationTarget;
  readonly title: string;
  readonly correlationId: string;
  readonly deliveryState: MobileNotificationDeliveryState;
  readonly unsafeDocumentBody?: string;
  readonly unsafeCommentBody?: string;
  readonly unsafeSessionToken?: string;
  readonly unsafeSessionId?: string;
  readonly unsafeRawCanvasState?: string;
}

export interface MobilePushNotificationPayload {
  readonly eventName: MobilePushNotificationEventName;
  readonly targetKind: MobilePushNotificationTargetKind;
  readonly targetId: string;
  readonly title: string;
  readonly correlationId: string;
  readonly deliveryState: MobileNotificationDeliveryState;
}

export interface MobileNotificationDeliveryTransitionResult {
  readonly state: MobileNotificationDeliveryState;
  readonly errorCode?: MobileNotificationDeliveryErrorCode;
}

export interface MobileReviewDecisionViewModel {
  readonly kind: "review-decision";
  readonly decision: MobileReviewDecision;
  readonly documentId: string;
  readonly reviewRequestId?: string;
  readonly previousState: ReviewWorkflowActionView["previousState"];
  readonly nextState: ReviewWorkflowActionView["nextState"];
}

export type MobileReadContentViewModel =
  | MobileCurrentDocumentViewModel
  | MobileDocumentHistoryViewModel
  | MobileSearchViewModel
  | MobileCommentsViewModel
  | MobileReviewRequestsViewModel
  | MobileReviewDecisionViewModel;

export interface MobileReadDisplayModel {
  readonly state: MobileReadDisplayState;
  readonly platform: MobilePlatform;
  readonly contractVersion: string;
  readonly capabilities: MobileReadCapabilities;
  readonly content?: MobileReadContentViewModel;
  readonly error?: MobileReadErrorView;
}

export interface MobileHistoryQuery extends DocumentIdentity {
  readonly limit: number;
  readonly cursor?: string;
}

export interface MobileSearchQuery {
  readonly workspaceId: string;
  readonly text: string;
  readonly limit: number;
}

export interface MobileReviewRequestsQuery {
  readonly workspaceId: string;
  readonly documentId?: string;
}

export interface MobileReviewDecisionCommand {
  readonly workspaceId: string;
  readonly reviewRequestId: string;
}

export interface MobileReadApiClient {
  getCurrentDocument(query: DocumentIdentity): Promise<AccessibleDocumentView>;
  getDocumentHistory(query: MobileHistoryQuery): Promise<DocumentHistoryPage>;
  searchDocuments(query: MobileSearchQuery): Promise<SearchAccessibleDocumentsView>;
  listDocumentComments(query: DocumentIdentity): Promise<CommentThreadPageView>;
  listReviewRequests(query: MobileReviewRequestsQuery): Promise<ReviewRequestPageView>;
  approveReviewRequest(command: MobileReviewDecisionCommand): Promise<ReviewWorkflowActionView>;
  rejectReviewRequest(command: MobileReviewDecisionCommand): Promise<ReviewWorkflowActionView>;
}

export interface MobileReadSkeleton {
  loadCurrentDocument(query: DocumentIdentity): Promise<MobileReadDisplayModel>;
  loadDocumentHistory(query: MobileHistoryQuery): Promise<MobileReadDisplayModel>;
  searchDocuments(query: MobileSearchQuery): Promise<MobileReadDisplayModel>;
  loadDocumentComments(query: DocumentIdentity): Promise<MobileReadDisplayModel>;
  loadReviewRequests(query: MobileReviewRequestsQuery): Promise<MobileReadDisplayModel>;
  approveReviewRequest(command: MobileReviewDecisionCommand): Promise<MobileReadDisplayModel>;
  rejectReviewRequest(command: MobileReviewDecisionCommand): Promise<MobileReadDisplayModel>;
  requestEdit(): MobileReadDisplayModel;
  requestCanvasEdit(): MobileReadDisplayModel;
}

export function createMobileReadSkeletonConfig(
  input: MobileReadSkeletonConfigInput,
): MobileReadSkeletonConfig {
  if (input.platform !== "ios" && input.platform !== "android") {
    throw new MobileReadSkeletonError("MOBILE_CONTRACT_RESPONSE_INVALID", "Mobile platform is not supported.");
  }

  const apiConfig = withSelfHostSessionToken(
    createSelfHostApiClientConfig({ baseUrl: input.apiBaseUrl }),
    input.sessionToken,
  );

  return {
    platform: input.platform,
    apiBaseUrl: apiConfig.baseUrl,
    sessionToken: apiConfig.sessionToken ?? "",
    contractVersion: input.contractVersion,
  };
}

export function createMobileReadSkeleton(
  config: MobileReadSkeletonConfig,
  apiClient: MobileReadApiClient = createMobileReadSelfHostApiClient(config),
  contract: MobileReadApiContract = createMobileReadApiContract(),
): MobileReadSkeleton {
  return {
    loadCurrentDocument(query) {
      return runMobileReadLoad(config, async () => {
        ensureContractVersion(config, contract);
        const document = await apiClient.getCurrentDocument(query);
        ensureContractResponse(contract, "MobileCurrentDocumentResponse", document);
        if (!document.permissionDecision) {
          throw new MobileReadSkeletonError(
            "MOBILE_PERMISSION_DECISION_MISSING",
            "Mobile current document response is missing permission decision.",
          );
        }
        return mapLoaded(config, mapCurrentDocument(document));
      });
    },

    loadDocumentHistory(query) {
      return runMobileReadLoad(config, async () => {
        ensureContractVersion(config, contract);
        const history = await apiClient.getDocumentHistory(query);
        ensureContractResponse(contract, "MobileDocumentHistoryResponse", history);
        return mapLoaded(config, mapDocumentHistory(history));
      });
    },

    searchDocuments(query) {
      return runMobileReadLoad(config, async () => {
        ensureContractVersion(config, contract);
        const page = await apiClient.searchDocuments(query);
        ensureContractResponse(contract, "MobileSearchResponse", page);
        return mapLoaded(config, mapSearchResults(page));
      });
    },

    loadDocumentComments(query) {
      return runMobileReadLoad(config, async () => {
        ensureContractVersion(config, contract);
        const page = await apiClient.listDocumentComments(query);
        ensureContractResponse(contract, "MobileCommentThreadsResponse", page);
        return mapLoaded(config, mapComments(page));
      });
    },

    loadReviewRequests(query) {
      return runMobileReadLoad(config, async () => {
        ensureContractVersion(config, contract);
        const page = await apiClient.listReviewRequests(query);
        ensureContractResponse(contract, "MobileReviewRequestsResponse", page);
        return mapLoaded(config, mapReviewRequests(page));
      });
    },

    approveReviewRequest(command) {
      return runMobileReadLoad(config, async () => {
        ensureContractVersion(config, contract);
        const action = await apiClient.approveReviewRequest(command);
        return mapLoaded(config, mapReviewDecision("approved", action));
      });
    },

    rejectReviewRequest(command) {
      return runMobileReadLoad(config, async () => {
        ensureContractVersion(config, contract);
        const action = await apiClient.rejectReviewRequest(command);
        return mapLoaded(config, mapReviewDecision("rejected", action));
      });
    },

    requestEdit() {
      return {
        ...baseDisplayModel(config),
        state: transitionMobileReadState("Loaded", "EditRequested"),
        error: {
          code: "MOBILE_UNSUPPORTED_EDIT",
          message: "Mobile Phase 002 client is read-only.",
        },
      };
    },

    requestCanvasEdit() {
      return {
        ...baseDisplayModel(config),
        state: transitionMobileReadState("Loaded", "CanvasEditRequested"),
        error: {
          code: "MOBILE_UNSUPPORTED_CANVAS_EDIT",
          message: "Mobile Canvas full edit is not supported on this platform.",
        },
      };
    },
  };
}

export function createMobileReadSelfHostApiClient(
  config: MobileReadSkeletonConfig,
  transport: CabinetHttpTransport = createFetchHttpTransport(),
): MobileReadApiClient {
  const apiConfig = toSelfHostConfig(config);
  const client = createSelfHostApiClient(apiConfig, transport);

  return {
    getCurrentDocument(query) {
      return client.getAccessibleDocument(query);
    },
    getDocumentHistory(query) {
      return mobileRequestJson<DocumentHistoryPage>(
        apiConfig,
        transport,
        "GET",
        `/api/workspaces/${encodePath(query.workspaceId)}/documents/${encodePath(query.documentId)}/history${queryString({
          limit: String(query.limit),
          cursor: query.cursor,
        })}`,
      );
    },
    searchDocuments(query) {
      return client.searchAccessibleDocuments(query);
    },
    listDocumentComments(query) {
      return client.listDocumentComments(query);
    },
    listReviewRequests(query) {
      return client.listReviewRequests(query);
    },
    approveReviewRequest(command) {
      return client.approveDocumentReview(command);
    },
    rejectReviewRequest(command) {
      return client.rejectDocumentReview(command);
    },
  };
}

export function createMobileReadInitialDisplayModel(
  config: MobileReadSkeletonConfig,
): MobileReadDisplayModel {
  return {
    ...baseDisplayModel(config),
    state: "Idle",
  };
}

export function transitionMobileReadState(
  currentState: MobileReadDisplayState,
  event: MobileReadEvent,
): MobileReadDisplayState {
  if (
    currentState === "Idle" &&
    ["LoadCurrent", "LoadHistory", "Search", "LoadComments", "LoadReviewRequests"].includes(event)
  ) {
    return "Loading";
  }
  if (currentState === "Loading" && event === "LoadSucceeded") {
    return "Loaded";
  }
  if (currentState === "Loading" && event === "ApiFailure") {
    return "Error";
  }
  if (event === "EditRequested" || event === "CanvasEditRequested") {
    return "UnsupportedAction";
  }
  return "Error";
}

export function createMobilePushNotificationPayload(
  input: MobilePushNotificationInput,
): MobilePushNotificationPayload {
  return {
    eventName: input.eventName,
    targetKind: input.target.kind,
    targetId: input.target.id,
    title: input.title,
    correlationId: input.correlationId,
    deliveryState: input.deliveryState,
  };
}

export function transitionMobileNotificationDeliveryState(
  currentState: MobileNotificationDeliveryState,
  event: MobileNotificationDeliveryEvent,
): MobileNotificationDeliveryTransitionResult {
  if (currentState === "Queued" && event === "SendSucceeded") {
    return { state: "Sent" };
  }
  if (currentState === "Queued" && event === "SendFailed") {
    return { state: "Failed" };
  }
  if (currentState === "Failed" && event === "RetryRequested") {
    return { state: "Retry" };
  }
  if (currentState === "Retry" && event === "RetryScheduled") {
    return { state: "Queued" };
  }
  if (currentState === "Failed" && event === "GiveUp") {
    return { state: "Failed" };
  }
  return {
    state: currentState,
    errorCode: "MOBILE_NOTIFICATION_INVALID_TRANSITION",
  };
}

async function runMobileReadLoad(
  config: MobileReadSkeletonConfig,
  load: () => Promise<MobileReadDisplayModel>,
): Promise<MobileReadDisplayModel> {
  transitionMobileReadState("Idle", "LoadCurrent");
  try {
    return await load();
  } catch (error) {
    return {
      ...baseDisplayModel(config),
      state: transitionMobileReadState("Loading", "ApiFailure"),
      error: mapMobileReadError(error),
    };
  }
}

function ensureContractVersion(
  config: MobileReadSkeletonConfig,
  contract: MobileReadApiContract,
): void {
  if (config.contractVersion !== contract.version) {
    throw new MobileReadSkeletonError(
      "MOBILE_CONTRACT_VERSION_MISMATCH",
      "Mobile read API contract version mismatch.",
    );
  }
}

function ensureContractResponse(
  contract: MobileReadApiContract,
  responseName: Parameters<typeof validateMobileReadApiResponse>[1],
  response: unknown,
): void {
  const validation = validateMobileReadApiResponse(contract, responseName, response);
  if (!validation.valid) {
    const code = validation.missingFields.includes("permissionDecision")
      ? "MOBILE_PERMISSION_DECISION_MISSING"
      : "MOBILE_CONTRACT_RESPONSE_INVALID";
    throw new MobileReadSkeletonError(code, "Mobile read API response does not match contract.");
  }
}

function mapLoaded(
  config: MobileReadSkeletonConfig,
  content: MobileReadContentViewModel,
): MobileReadDisplayModel {
  return {
    ...baseDisplayModel(config),
    state: transitionMobileReadState("Loading", "LoadSucceeded"),
    content,
  };
}

function baseDisplayModel(config: MobileReadSkeletonConfig): Omit<MobileReadDisplayModel, "state"> {
  return {
    platform: config.platform,
    contractVersion: config.contractVersion,
    capabilities: mobileCapabilities(config.platform),
  };
}

function mobileCapabilities(platform: MobilePlatform): MobileReadCapabilities {
  const matrix = createPlatformCapabilityMatrix();
  const profile = platform === "ios" ? matrix.ios : matrix.android;
  return {
    supportsMobileReadApi: profile.supportsMobileReadApi,
    supportsRemoteEdit: false,
    supportsOfflineRemoteEdit: false,
    knowledgeGraphSupport: profile.knowledgeGraphSupport,
    canvasSupport: profile.canvasSupport,
    realtimeCollaborationSupport: profile.realtimeCollaborationSupport,
    supportsCanvasFullEdit: false,
  };
}

function mapCurrentDocument(document: AccessibleDocumentView): MobileCurrentDocumentViewModel {
  return {
    kind: "current-document",
    workspaceId: document.workspaceId,
    documentId: document.documentId,
    title: document.title,
    path: document.path,
    body: document.body,
    versionId: document.versionId,
    permissionDecision: {
      result: document.permissionDecision.result,
      reasonCode: document.permissionDecision.reasonCode,
    },
    canEdit: false,
  };
}

function mapDocumentHistory(history: DocumentHistoryPage): MobileDocumentHistoryViewModel {
  return {
    kind: "document-history",
    workspaceId: history.workspaceId,
    documentId: history.documentId,
    entries: history.entries.map((entry) => ({
      versionId: entry.versionId,
      summary: entry.summary,
      author: entry.author,
      createdAt: entry.createdAt,
    })),
    nextCursor: history.nextCursor,
  };
}

function mapSearchResults(page: SearchAccessibleDocumentsView): MobileSearchViewModel {
  return {
    kind: "search-results",
    workspaceId: page.workspaceId,
    text: page.text,
    results: page.results.map((result) => ({
      workspaceId: result.workspaceId,
      documentId: result.documentId,
      title: result.title,
      path: result.path,
      snippet: result.snippet,
    })),
    permissionFilteredCount: page.permissionFilteredCount,
    durationMs: page.durationMs,
  };
}

function mapComments(page: CommentThreadPageView): MobileCommentsViewModel {
  return {
    kind: "comments",
    threads: page.threads.map((thread) => ({
      threadId: thread.threadId,
      documentId: thread.documentId,
      state: thread.state,
      commentCount: thread.comments.length,
      anchorStatus: thread.anchor?.status,
    })),
  };
}

function mapReviewRequests(page: ReviewRequestPageView): MobileReviewRequestsViewModel {
  return {
    kind: "review-requests",
    requests: page.requests.map((request) => ({
      reviewRequestId: request.reviewRequestId,
      documentId: request.documentId,
      status: request.status,
    })),
  };
}

function mapReviewDecision(
  decision: MobileReviewDecision,
  action: ReviewWorkflowActionView,
): MobileReviewDecisionViewModel {
  return {
    kind: "review-decision",
    decision,
    documentId: action.documentId,
    reviewRequestId: action.reviewRequestId,
    previousState: action.previousState,
    nextState: action.nextState,
  };
}

function mapMobileReadError(error: unknown): MobileReadErrorView {
  if (error instanceof MobileReadSkeletonError) {
    return {
      code: error.code,
      message: error.message,
    };
  }

  if (error instanceof CabinetApiClientError) {
    if (error.code === "SESSION_EXPIRED") {
      return {
        code: "MOBILE_SESSION_EXPIRED",
        message: "Mobile read session expired.",
      };
    }
    if (error.code === "UNAUTHORIZED") {
      return {
        code: "MOBILE_UNAUTHORIZED",
        message: "Mobile read authorization failed.",
      };
    }
    if (error.code === "NETWORK_FAILURE") {
      return {
        code: "MOBILE_NETWORK_FAILURE",
        message: "Mobile read network request failed.",
      };
    }
  }

  return {
    code: "MOBILE_API_ERROR",
    message: "Mobile read request failed.",
  };
}

async function mobileRequestJson<T>(
  config: SelfHostApiClientConfig,
  transport: CabinetHttpTransport,
  method: HttpMethodName,
  path: string,
): Promise<T> {
  let response: CabinetHttpResponse;
  try {
    response = await transport({
      method,
      url: `${config.baseUrl}${path}`,
      headers: requestHeaders(config),
    });
  } catch (error) {
    if (error instanceof CabinetApiClientError) {
      throw error;
    }
    throw new CabinetApiClientError("NETWORK_FAILURE", "network request failed");
  }

  if (response.status >= 200 && response.status < 300) {
    return parseJson<T>(response.body, response.status);
  }

  throw new CabinetApiClientError(mapHttpStatusToApiError(response.status, response.body), "mobile read API request failed", response.status);
}

function toSelfHostConfig(config: MobileReadSkeletonConfig): SelfHostApiClientConfig {
  return withSelfHostSessionToken(
    createSelfHostApiClientConfig({ baseUrl: config.apiBaseUrl }),
    config.sessionToken,
  );
}

function requestHeaders(config: SelfHostApiClientConfig): Record<string, string> {
  const headers: Record<string, string> = {
    accept: "application/json",
  };
  if (config.sessionToken) {
    headers.authorization = `Bearer ${config.sessionToken}`;
  }
  return headers;
}

function parseJson<T>(body: string, status: number): T {
  try {
    return (body.trim() ? JSON.parse(body) : {}) as T;
  } catch {
    throw new CabinetApiClientError("API_ERROR", "mobile read API returned invalid JSON", status);
  }
}

function mapHttpStatusToApiError(status: number, body: string): string {
  const parsed = parseUnknownJson(body);
  if (isRecord(parsed)) {
    const errorCode = parsed.errorCode ?? parsed.code;
    if (typeof errorCode === "string" && errorCode.trim()) {
      return errorCode;
    }
  }
  if (status === 401) {
    return "SESSION_EXPIRED";
  }
  if (status === 403) {
    return "UNAUTHORIZED";
  }
  return "API_ERROR";
}

function parseUnknownJson(body: string): unknown {
  try {
    return body.trim() ? JSON.parse(body) : undefined;
  } catch {
    return undefined;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function encodePath(value: string): string {
  return encodeURIComponent(value);
}

function queryString(params: Readonly<Record<string, string | undefined>>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) {
      search.set(key, value);
    }
  }
  const serialized = search.toString();
  return serialized ? `?${serialized}` : "";
}

class MobileReadSkeletonError extends Error {
  readonly code: MobileReadErrorCode;

  constructor(code: MobileReadErrorCode, message: string) {
    super(message);
    this.name = "MobileReadSkeletonError";
    this.code = code;
  }
}
