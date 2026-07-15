import type {
  AccessibleDocumentView,
  AttachAssetCommand,
  CabinetHttpResponse,
  CabinetHttpTransport,
  CanvasView,
  CurrentDocumentView,
  DocumentAssetsPage,
  DocumentHistoryPage,
  HttpMethodName,
  KnowledgeGraphView,
  LinkOverviewView,
  LocalDesktopCommandClient,
  WorkspaceHomeQuery,
  PersonalLocalDesktopCapabilityProfile,
  SearchResultsPage,
  SelectedAssetDraft,
  SelfHostApiClientConfig,
} from "@sponzey-cabinet/client-core";
import {
  CabinetApiClientError,
  LocalDesktopCommandClientError,
  createAttachAssetClientCommand,
  createFetchHttpTransport,
  createKnowledgeGraphQuery,
  createPersonalLocalDesktopCapabilityProfile,
  createPlatformCapabilityMatrix,
  createSelfHostApiClient,
  createSelfHostApiClientConfig,
  withSelfHostSessionToken,
} from "@sponzey-cabinet/client-core";
import { createEditorBoundaryDescriptor } from "@sponzey-cabinet/editor";
import type {
  PersonalWorkspaceHealthState,
  PersonalWorkspaceHomeModel,
  PersonalWorkspaceShellModel,
  AiCitationSourceOpenAction,
  AiCitationSourceOpenInput,
  AiProviderSettingsSummaryView,
  AiProviderSettingsViewModel,
  BackupArtifactManifestSummaryView,
  BackupArtifactManifestViewModel,
  BackupSettingsInput,
  BackupSettingsViewModel,
  ImportPreviewInput,
  ImportPreviewViewModel,
  LocalAiToolScopeInput,
  LocalAiToolScopeViewModel,
  RestoreStagingValidationInput,
  RestoreStagingValidationViewModel,
  CanvasViewportPanelOptions,
  CanvasViewportPanelViewModel,
  DocumentReadingWorkspaceModel,
  DocumentAuthoringWorkspaceModel,
  GraphPanelOptions,
  GraphPanelViewModel,
  HistoryEntryViewModel,
  IndexFreshnessState,
  LocalDiscoveryPanelModel,
  RestoreApplyCommandResult,
  RestoreConfirmationInput,
  RestorePreviewModel,
  RestorePreviewModelInput,
  RestorePreviewRequest,
  ShellDescriptor,
} from "@sponzey-cabinet/ui";
import {
  createAiCitationSourceOpenAction,
  createAiProviderSettingsViewModel,
  createBackupArtifactManifestViewModel,
  createBackupSettingsViewModel,
  createCanvasViewportPanelModel,
  createDocumentReadingWorkspaceModel,
  createDocumentAuthoringWorkspaceModel,
  createGraphPanelViewModel,
  createImportPreviewViewModel,
  createLocalAiToolScopeViewModel,
  createLocalDiscoveryPanelModel,
  createPersonalWorkspaceHomeModel,
  createPersonalWorkspaceHomeModelFromResult,
  createPersonalWorkspaceHomeFailedModel,
  createPersonalWorkspaceShellModel,
  createRestoreApplyCommand,
  createRestoreStagingValidationModel,
  createRestorePreviewModel,
  createRestorePreviewRequestFromHistoryEntry,
  createShellDescriptor,
} from "@sponzey-cabinet/ui";

export interface DesktopSelectedAsset {
  readonly assetId: string;
  readonly label: string;
  readonly fileName: string;
  readonly mediaType: string;
  readonly byteSize: number;
}

export function mapDesktopAssetSelection(selection: DesktopSelectedAsset): SelectedAssetDraft {
  return {
    assetId: selection.assetId,
    label: selection.label,
    fileName: selection.fileName,
    mediaType: selection.mediaType,
    byteSize: selection.byteSize,
  };
}

export function createDesktopAttachAssetCommand(
  workspaceId: string,
  documentId: string,
  selection: DesktopSelectedAsset,
): AttachAssetCommand {
  return createAttachAssetClientCommand(workspaceId, documentId, mapDesktopAssetSelection(selection));
}

export type DesktopWorkspaceDisplayState =
  | "LocalWorkspaceSelected"
  | "RemoteWorkspaceSelected"
  | "RemoteConnecting"
  | "RemoteConnected"
  | "RemoteError";

export interface DesktopWorkspaceCapabilities {
  readonly supportsLocalWorkspace: boolean;
  readonly supportsRemoteWorkspace: boolean;
}

export interface DesktopLocalWorkspaceSelection {
  readonly kind: "local";
  readonly workspaceId: string;
  readonly label: string;
  readonly localPath: string;
}

export interface DesktopRemoteWorkspaceSelection {
  readonly kind: "remote";
  readonly workspaceId: string;
  readonly label: string;
  readonly serverBaseUrl: string;
  readonly sessionToken: string;
}

export type DesktopWorkspaceSelection =
  | DesktopLocalWorkspaceSelection
  | DesktopRemoteWorkspaceSelection;

export interface DesktopWorkspaceErrorView {
  readonly code:
    | "DESKTOP_REMOTE_UNSUPPORTED"
    | "DESKTOP_LOCAL_UNSUPPORTED"
    | "DESKTOP_WORKSPACE_NOT_SELECTED"
    | "DESKTOP_REMOTE_UNAUTHORIZED"
    | "DESKTOP_REMOTE_SESSION_EXPIRED"
    | "DESKTOP_REMOTE_NETWORK_FAILURE"
    | "DESKTOP_REMOTE_CONNECTION_FAILED";
  readonly message: string;
}

export interface DesktopWorkspaceSelectorModel {
  readonly displayState: DesktopWorkspaceDisplayState;
  readonly capabilities: DesktopWorkspaceCapabilities;
  readonly selectedWorkspace?: DesktopWorkspaceSelection;
  readonly error?: DesktopWorkspaceErrorView;
}

export interface DesktopLocalWorkspaceDraft {
  readonly workspaceId: string;
  readonly displayName: string;
  readonly localPath: string;
}

export interface DesktopRemoteWorkspaceDraft {
  readonly workspaceId: string;
  readonly displayName: string;
  readonly serverBaseUrl: string;
  readonly sessionToken: string;
}

export interface DesktopDocumentEditCommand {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly title: string;
  readonly path: string;
  readonly body: string;
  readonly expectedVersionId: string;
}

export interface DesktopRemoteCurrentDocumentQuery {
  readonly workspaceId: string;
  readonly documentId: string;
}

export interface DesktopRemoteKnowledgeGraphQuery {
  readonly workspaceId: string;
  readonly documentId: string;
}

export interface DesktopRemoteDocumentReadResult {
  readonly status: "loaded-remote" | "not-loaded";
  readonly document?: AccessibleDocumentView;
  readonly error?: DesktopWorkspaceErrorView;
}

export interface DesktopRemoteKnowledgeGraphReadResult {
  readonly status: "loaded-remote" | "not-loaded";
  readonly graph?: KnowledgeGraphView;
  readonly error?: DesktopWorkspaceErrorView;
}

export interface DesktopDocumentSaveResult {
  readonly status: "saved-local" | "saved-remote" | "not-saved";
  readonly documentId?: string;
  readonly error?: DesktopWorkspaceErrorView;
}

export interface DesktopLocalDocumentSaveResult {
  readonly status: "saved-local";
  readonly documentId: string;
  readonly currentVersionId: string;
  readonly versionAppended: true;
}

export interface DesktopLocalCurrentDocumentQuery {
  readonly workspaceId: string;
  readonly documentId: string;
}

export interface DesktopLocalDocumentHistoryQuery {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly cursor?: string;
  readonly limit: number;
}

export interface DesktopLocalWorkspaceFacade {
  saveCurrentDocument(command: DesktopDocumentEditCommand): Promise<DesktopLocalDocumentSaveResult>;
  getCurrentDocument(query: DesktopLocalCurrentDocumentQuery): Promise<CurrentDocumentView>;
  listDocumentHistory(query: DesktopLocalDocumentHistoryQuery): Promise<DocumentHistoryPage>;
}

export async function loadDesktopWorkspaceHome(
  client: LocalDesktopCommandClient,
  query: WorkspaceHomeQuery,
): Promise<PersonalWorkspaceHomeModel> {
  const profile = createPersonalLocalDesktopCapabilityProfile();
  try {
    const result = await client.getWorkspaceHome(query);
    return createPersonalWorkspaceHomeModelFromResult(profile, result);
  } catch (error) {
    if (error instanceof LocalDesktopCommandClientError) {
      return createPersonalWorkspaceHomeFailedModel(profile, error.code, error.retryable);
    }
    return createPersonalWorkspaceHomeFailedModel(profile, "COMMAND_BRIDGE_FAILED", false);
  }
}

export function createDesktopLocalCommandWorkspaceFacade(
  client: LocalDesktopCommandClient,
): DesktopLocalWorkspaceFacade {
  return {
    saveCurrentDocument(command) {
      return client.saveCurrentDocument(command);
    },

    getCurrentDocument(query) {
      return client.getCurrentDocument({ queryName: "get-current-document", ...query });
    },

    listDocumentHistory(query) {
      return client.listDocumentHistory({ queryName: "get-document-history", ...query });
    },
  };
}

export interface DesktopCurrentProductShell {
  readonly capability: PersonalLocalDesktopCapabilityProfile;
  readonly workspace: PersonalWorkspaceShellModel;
  readonly home: PersonalWorkspaceHomeModel;
}

export interface DesktopCurrentProductShellDescriptor extends DesktopCurrentProductShell {
  readonly shell: ShellDescriptor;
  readonly editor: string;
}

export function createDesktopDocumentReadingWorkspace(
  current: CurrentDocumentView,
  history: DocumentHistoryPage,
): DocumentReadingWorkspaceModel {
  return createDocumentReadingWorkspaceModel(current, history);
}

export function createDesktopDocumentAuthoringWorkspace(
  current: CurrentDocumentView,
  history: DocumentHistoryPage,
): DocumentAuthoringWorkspaceModel {
  return createDocumentAuthoringWorkspaceModel(current, history);
}

export function createDesktopRestorePreviewRequest(
  workspaceId: string,
  documentId: string,
  entry: HistoryEntryViewModel,
): RestorePreviewRequest {
  return createRestorePreviewRequestFromHistoryEntry(workspaceId, documentId, entry);
}

export function createDesktopRestorePreviewModel(
  input: RestorePreviewModelInput,
): RestorePreviewModel {
  return createRestorePreviewModel(input);
}

export function createDesktopRestoreApplyCommand(
  preview: RestorePreviewModel,
  confirmation: RestoreConfirmationInput,
): RestoreApplyCommandResult {
  return createRestoreApplyCommand(preview, confirmation);
}

export function createDesktopLocalDiscoveryPanel(input: {
  readonly search: SearchResultsPage;
  readonly links: LinkOverviewView;
  readonly assets: DocumentAssetsPage;
  readonly indexFreshness: IndexFreshnessState;
}): LocalDiscoveryPanelModel {
  return createLocalDiscoveryPanelModel(input);
}

export function createDesktopGraphPanel(
  graph: KnowledgeGraphView,
  options: GraphPanelOptions,
): GraphPanelViewModel {
  return createGraphPanelViewModel(graph, options);
}

export function createDesktopCanvasViewportPanel(
  canvas: CanvasView,
  options: CanvasViewportPanelOptions,
): CanvasViewportPanelViewModel {
  return createCanvasViewportPanelModel(canvas, options);
}

export function createDesktopAiCitationSourceOpenAction(
  input: AiCitationSourceOpenInput,
): AiCitationSourceOpenAction {
  return createAiCitationSourceOpenAction(input);
}

export function createDesktopLocalAiToolScope(
  input: LocalAiToolScopeInput,
): LocalAiToolScopeViewModel {
  return createLocalAiToolScopeViewModel(input);
}

export function createDesktopAiProviderSettings(
  input: AiProviderSettingsSummaryView,
): AiProviderSettingsViewModel {
  return createAiProviderSettingsViewModel(input);
}

export function createDesktopBackupArtifactManifest(
  input: BackupArtifactManifestSummaryView,
): BackupArtifactManifestViewModel {
  return createBackupArtifactManifestViewModel(input);
}

export function createDesktopBackupSettings(input: BackupSettingsInput): BackupSettingsViewModel {
  return createBackupSettingsViewModel(input);
}

export function createDesktopRestoreStagingValidation(
  input: RestoreStagingValidationInput,
): RestoreStagingValidationViewModel {
  return createRestoreStagingValidationModel(input);
}

export * from "./desktop_backup_recovery_controller.ts";
export * from "./react_backup_recovery.ts";
export * from "./tauri_backup_recovery_transport.ts";

export function createDesktopImportPreview(input: ImportPreviewInput): ImportPreviewViewModel {
  return createImportPreviewViewModel(input);
}

export async function saveDesktopLocalCurrentDocument(
  command: DesktopDocumentEditCommand,
  facade: DesktopLocalWorkspaceFacade,
): Promise<DesktopLocalDocumentSaveResult> {
  return facade.saveCurrentDocument(command);
}

export async function getDesktopLocalCurrentDocument(
  query: DesktopLocalCurrentDocumentQuery,
  facade: DesktopLocalWorkspaceFacade,
): Promise<CurrentDocumentView> {
  return facade.getCurrentDocument(query);
}

export async function listDesktopLocalDocumentHistory(
  query: DesktopLocalDocumentHistoryQuery,
  facade: DesktopLocalWorkspaceFacade,
): Promise<DocumentHistoryPage> {
  return facade.listDocumentHistory(query);
}

export interface DesktopLocalWorkspaceRepository {
  openLocalWorkspace(selection: DesktopLocalWorkspaceSelection): Promise<void>;
  saveLocalDocument(command: DesktopDocumentEditCommand): Promise<void>;
}

export interface DesktopRemoteWorkspaceApiClient {
  openRemoteWorkspace(selection: DesktopRemoteWorkspaceSelection): Promise<void>;
  readRemoteCurrentDocument(query: DesktopRemoteCurrentDocumentQuery): Promise<AccessibleDocumentView>;
  readRemoteKnowledgeGraph(query: DesktopRemoteKnowledgeGraphQuery): Promise<KnowledgeGraphView>;
  saveRemoteDocument(command: DesktopDocumentEditCommand): Promise<{ readonly status: "saved-remote" }>;
}

export function createDesktopWorkspaceSelectorModel(
  capabilities: DesktopWorkspaceCapabilities = createDefaultDesktopWorkspaceCapabilities(),
): DesktopWorkspaceSelectorModel {
  return {
    displayState: "LocalWorkspaceSelected",
    capabilities,
  };
}

export async function selectDesktopLocalWorkspace(
  state: DesktopWorkspaceSelectorModel,
  draft: DesktopLocalWorkspaceDraft,
  repository: DesktopLocalWorkspaceRepository,
): Promise<DesktopWorkspaceSelectorModel> {
  if (!state.capabilities.supportsLocalWorkspace) {
    return desktopWorkspaceError(state, "DESKTOP_LOCAL_UNSUPPORTED", "Local workspace is not supported.");
  }

  const selection: DesktopLocalWorkspaceSelection = {
    kind: "local",
    workspaceId: draft.workspaceId,
    label: draft.displayName,
    localPath: draft.localPath,
  };
  await repository.openLocalWorkspace(selection);
  return {
    displayState: "LocalWorkspaceSelected",
    capabilities: state.capabilities,
    selectedWorkspace: selection,
  };
}

export function selectDesktopRemoteWorkspace(
  state: DesktopWorkspaceSelectorModel,
  draft: DesktopRemoteWorkspaceDraft,
): DesktopWorkspaceSelectorModel {
  if (!state.capabilities.supportsRemoteWorkspace) {
    return desktopWorkspaceError(
      state,
      "DESKTOP_REMOTE_UNSUPPORTED",
      "Remote workspace is not supported by this platform.",
    );
  }

  return {
    displayState: "RemoteWorkspaceSelected",
    capabilities: state.capabilities,
    selectedWorkspace: {
      kind: "remote",
      workspaceId: draft.workspaceId,
      label: draft.displayName,
      serverBaseUrl: draft.serverBaseUrl,
      sessionToken: draft.sessionToken,
    },
  };
}

export function beginDesktopRemoteConnection(
  state: DesktopWorkspaceSelectorModel,
): DesktopWorkspaceSelectorModel {
  if (state.selectedWorkspace?.kind !== "remote") {
    return desktopWorkspaceError(
      state,
      "DESKTOP_WORKSPACE_NOT_SELECTED",
      "Select a remote workspace before connecting.",
    );
  }

  return {
    ...state,
    displayState: "RemoteConnecting",
    error: undefined,
  };
}

export async function connectDesktopRemoteWorkspace(
  state: DesktopWorkspaceSelectorModel,
  client: DesktopRemoteWorkspaceApiClient,
): Promise<DesktopWorkspaceSelectorModel> {
  if (state.selectedWorkspace?.kind !== "remote") {
    return desktopWorkspaceError(
      state,
      "DESKTOP_WORKSPACE_NOT_SELECTED",
      "Select a remote workspace before connecting.",
    );
  }

  try {
    await client.openRemoteWorkspace(state.selectedWorkspace);
    return {
      ...state,
      displayState: "RemoteConnected",
      error: undefined,
    };
  } catch (error) {
    return desktopWorkspaceErrorFromApiFailure(state, error);
  }
}

export async function readDesktopRemoteCurrentDocument(
  state: DesktopWorkspaceSelectorModel,
  query: DesktopRemoteCurrentDocumentQuery,
  remoteClient: DesktopRemoteWorkspaceApiClient,
): Promise<DesktopRemoteDocumentReadResult> {
  if (state.selectedWorkspace?.kind !== "remote") {
    return {
      status: "not-loaded",
      error: {
        code: "DESKTOP_WORKSPACE_NOT_SELECTED",
        message: "Select a remote workspace before reading a document.",
      },
    };
  }

  try {
    return {
      status: "loaded-remote",
      document: await remoteClient.readRemoteCurrentDocument(query),
    };
  } catch (error) {
    return {
      status: "not-loaded",
      error: desktopWorkspaceErrorViewFromApiFailure(error),
    };
  }
}

export async function readDesktopRemoteKnowledgeGraph(
  state: DesktopWorkspaceSelectorModel,
  query: DesktopRemoteKnowledgeGraphQuery,
  remoteClient: DesktopRemoteWorkspaceApiClient,
): Promise<DesktopRemoteKnowledgeGraphReadResult> {
  if (state.selectedWorkspace?.kind !== "remote") {
    return {
      status: "not-loaded",
      error: {
        code: "DESKTOP_WORKSPACE_NOT_SELECTED",
        message: "Select a remote workspace before reading a knowledge graph.",
      },
    };
  }

  try {
    return {
      status: "loaded-remote",
      graph: await remoteClient.readRemoteKnowledgeGraph(query),
    };
  } catch (error) {
    return {
      status: "not-loaded",
      error: desktopWorkspaceErrorViewFromApiFailure(error),
    };
  }
}

export async function saveDesktopDocumentEdit(
  state: DesktopWorkspaceSelectorModel,
  command: DesktopDocumentEditCommand,
  localRepository: DesktopLocalWorkspaceRepository,
  remoteClient: DesktopRemoteWorkspaceApiClient,
): Promise<DesktopDocumentSaveResult> {
  if (state.selectedWorkspace?.kind === "local") {
    await localRepository.saveLocalDocument(command);
    return {
      status: "saved-local",
      documentId: command.documentId,
    };
  }

  if (state.selectedWorkspace?.kind === "remote") {
    try {
      const result = await remoteClient.saveRemoteDocument(command);
      return {
        status: result.status,
        documentId: command.documentId,
      };
    } catch (error) {
      return {
        status: "not-saved",
        error: desktopWorkspaceErrorViewFromApiFailure(error),
      };
    }
  }

  return {
    status: "not-saved",
    error: {
      code: "DESKTOP_WORKSPACE_NOT_SELECTED",
      message: "Select a workspace before saving a document.",
    },
  };
}

export function createDesktopRemoteWorkspaceApiClient(
  transport: CabinetHttpTransport = createFetchHttpTransport(),
): DesktopRemoteWorkspaceApiClient {
  let activeConfig: SelfHostApiClientConfig | undefined;

  return {
    async openRemoteWorkspace(selection) {
      activeConfig = createDesktopRemoteConfig(selection);
      const client = createSelfHostApiClient(activeConfig, transport);
      await client.validateSession({ token: selection.sessionToken });
    },

    async readRemoteCurrentDocument(query) {
      const client = createSelfHostApiClient(requireActiveRemoteConfig(activeConfig), transport);
      return client.getAccessibleDocument(query);
    },

    async readRemoteKnowledgeGraph(query) {
      const client = createSelfHostApiClient(requireActiveRemoteConfig(activeConfig), transport);
      return client.getKnowledgeGraph(
        createKnowledgeGraphQuery(query.workspaceId, query.documentId),
      );
    },

    async saveRemoteDocument(command) {
      const config = requireActiveRemoteConfig(activeConfig);
      const saved = await desktopRequestJson<{
        readonly status: "saved-remote";
        readonly documentId: string;
        readonly versionId: string;
      }>(
        config,
        transport,
        "PUT",
        `/api/workspaces/${encodePath(command.workspaceId)}/documents/${encodePath(command.documentId)}/current`,
        {
          title: command.title,
          path: command.path,
          body: command.body,
          expectedVersionId: command.expectedVersionId,
        },
      );
      return { status: saved.status };
    },
  };
}

function createDefaultDesktopWorkspaceCapabilities(): DesktopWorkspaceCapabilities {
  const matrix = createPlatformCapabilityMatrix();
  return {
    supportsLocalWorkspace: matrix.desktop.supportsLocalWorkspace,
    supportsRemoteWorkspace: matrix.desktop.supportsRemoteWorkspace,
  };
}

function createDesktopRemoteConfig(
  selection: DesktopRemoteWorkspaceSelection,
): SelfHostApiClientConfig {
  return withSelfHostSessionToken(
    createSelfHostApiClientConfig({
      baseUrl: selection.serverBaseUrl,
    }),
    selection.sessionToken,
  );
}

function requireActiveRemoteConfig(
  config: SelfHostApiClientConfig | undefined,
): SelfHostApiClientConfig {
  if (!config) {
    throw new CabinetApiClientError(
      "INVALID_CLIENT_CONFIG",
      "remote workspace is not connected",
    );
  }
  return config;
}

async function desktopRequestJson<T>(
  config: SelfHostApiClientConfig,
  transport: CabinetHttpTransport,
  method: HttpMethodName,
  path: string,
  body?: unknown,
): Promise<T> {
  let response: CabinetHttpResponse;
  try {
    response = await transport({
      method,
      url: `${config.baseUrl}${path}`,
      headers: desktopRequestHeaders(config, body !== undefined),
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch (error) {
    if (error instanceof CabinetApiClientError) {
      throw error;
    }
    throw new CabinetApiClientError("NETWORK_FAILURE", "network request failed");
  }

  if (response.status >= 200 && response.status < 300) {
    return parseDesktopJsonResponse<T>(response.body);
  }

  const errorBody = parseDesktopUnknownJson(response.body);
  throw new CabinetApiClientError(
    desktopApiErrorCode(response.status, errorBody),
    "remote workspace API request failed",
    response.status,
  );
}

function desktopRequestHeaders(
  config: SelfHostApiClientConfig,
  includesJsonBody: boolean,
): Record<string, string> {
  const headers: Record<string, string> = {
    accept: "application/json",
  };
  if (includesJsonBody) {
    headers["content-type"] = "application/json";
  }
  if (config.sessionToken) {
    headers.authorization = `Bearer ${config.sessionToken}`;
  }
  return headers;
}

function parseDesktopJsonResponse<T>(body: string): T {
  if (!body.trim()) {
    return {} as T;
  }
  try {
    return JSON.parse(body) as T;
  } catch {
    throw new CabinetApiClientError("API_ERROR", "remote workspace returned invalid JSON");
  }
}

function parseDesktopUnknownJson(body: string): unknown {
  if (!body.trim()) {
    return undefined;
  }
  try {
    return JSON.parse(body) as unknown;
  } catch {
    return undefined;
  }
}

function desktopApiErrorCode(status: number, body: unknown): string {
  if (isRecord(body)) {
    const code = body.errorCode ?? body.code;
    if (typeof code === "string" && code.trim()) {
      return code;
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function encodePath(value: string): string {
  return encodeURIComponent(value);
}

function desktopWorkspaceError(
  state: DesktopWorkspaceSelectorModel,
  code: DesktopWorkspaceErrorView["code"],
  message: string,
): DesktopWorkspaceSelectorModel {
  return {
    ...state,
    displayState: "RemoteError",
    error: {
      code,
      message,
    },
  };
}

function desktopWorkspaceErrorFromApiFailure(
  state: DesktopWorkspaceSelectorModel,
  error: unknown,
): DesktopWorkspaceSelectorModel {
  return {
    ...state,
    displayState: "RemoteError",
    error: desktopWorkspaceErrorViewFromApiFailure(error),
  };
}

function desktopWorkspaceErrorViewFromApiFailure(error: unknown): DesktopWorkspaceErrorView {
  if (error instanceof CabinetApiClientError) {
    if (error.code === "SESSION_EXPIRED") {
      return {
        code: "DESKTOP_REMOTE_SESSION_EXPIRED",
        message: "Remote workspace session expired.",
      };
    }
    if (error.code === "UNAUTHORIZED") {
      return {
        code: "DESKTOP_REMOTE_UNAUTHORIZED",
        message: "Remote workspace authorization failed.",
      };
    }
    if (error.code === "NETWORK_FAILURE") {
      return {
        code: "DESKTOP_REMOTE_NETWORK_FAILURE",
        message: "Remote workspace network request failed.",
      };
    }
  }

  return {
    code: "DESKTOP_REMOTE_CONNECTION_FAILED",
    message: "Remote workspace connection failed.",
  };
}

export function createDesktopCurrentProductShell(
  healthState: PersonalWorkspaceHealthState = "Ready",
): DesktopCurrentProductShell {
  const capability = createPersonalLocalDesktopCapabilityProfile();
  return {
    capability,
    workspace: createPersonalWorkspaceShellModel({
      profile: capability,
      healthState,
    }),
    home: createPersonalWorkspaceHomeModel({
      profile: capability,
      healthState,
    }),
  };
}

export function createDesktopCurrentProductShellDescriptor(
  healthState: PersonalWorkspaceHealthState = "Ready",
): DesktopCurrentProductShellDescriptor {
  const current = createDesktopCurrentProductShell(healthState);
  return {
    ...current,
    shell: createShellDescriptor(current.capability),
    editor: createEditorBoundaryDescriptor(current.capability),
  };
}

export const desktopShell = createDesktopCurrentProductShellDescriptor();
