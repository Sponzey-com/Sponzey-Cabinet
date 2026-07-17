import {
  LocalDesktopCommandClientError,
  type CurrentDocumentQuery,
  type CurrentDocumentView,
  type LocalDesktopCommandClient,
  type SaveDocumentRevisionCommand,
} from "@sponzey-cabinet/client-core";
import {
  applyRevisionSafeEditorContentChange,
  completeRevisionSafeEditorSave,
  createRevisionSafeEditorSession,
  startRevisionSafeEditorSave,
  type RevisionSafeEditorSession,
} from "@sponzey-cabinet/editor";
import {
  DocumentSaveCoordinatorEvent,
  DocumentSaveCoordinatorState,
  createDocumentSaveCoordinator,
  transitionDocumentSaveCoordinator,
  type DocumentSaveCoordinatorSnapshot,
  type DocumentSaveSideEffectRequest,
} from "@sponzey-cabinet/ui";

export interface DesktopDocumentAuthoringControllerOptions {
  readonly client: Pick<
    LocalDesktopCommandClient,
    "getCurrentDocument" | "saveDocumentRevision"
  >;
  readonly operationIdSource: () => string;
  readonly author: string;
  readonly summary: string;
  readonly autosaveDelayMs?: number;
}

export interface DesktopDocumentAuthoringSnapshot {
  readonly workspaceId?: string;
  readonly documentId?: string;
  readonly title?: string;
  readonly path?: string;
  readonly body?: string;
  readonly revision: number;
  readonly persistedRevision: number;
  readonly expectedVersionId?: string;
  readonly saveState: DocumentSaveCoordinatorSnapshot["state"];
  readonly errorCode?: string;
  readonly retryable?: boolean;
  readonly repairRequired?: boolean;
}

export interface DesktopDocumentCloseDecision {
  readonly canClose: boolean;
  readonly choices?: readonly ["RetrySave", "Discard", "Cancel"];
}

export interface DesktopDocumentMetadataReadbackResult {
  readonly accepted: boolean;
  readonly snapshot: DesktopDocumentAuthoringSnapshot;
  readonly errorCode?: "DOCUMENT_AUTHORING_READBACK_MISMATCH";
}

export class DesktopDocumentAuthoringController {
  readonly #client: DesktopDocumentAuthoringControllerOptions["client"];
  readonly #operationIdSource: () => string;
  readonly #author: string;
  readonly #summary: string;
  #workspaceId?: string;
  #title?: string;
  #path?: string;
  #editor?: RevisionSafeEditorSession;
  #coordinator: DocumentSaveCoordinatorSnapshot;
  #retryable?: boolean;
  #repairRequired?: boolean;
  #saveOperation?: { readonly revision: number; readonly operationId: string };

  constructor(options: DesktopDocumentAuthoringControllerOptions) {
    this.#client = options.client;
    this.#operationIdSource = options.operationIdSource;
    this.#author = options.author;
    this.#summary = options.summary;
    this.#coordinator = createDocumentSaveCoordinator({
      autosaveDelayMs: options.autosaveDelayMs ?? 800,
    });
  }

  async open(query: CurrentDocumentQuery): Promise<DesktopDocumentAuthoringSnapshot> {
    const current = await this.#client.getCurrentDocument(query);
    this.#workspaceId = current.workspaceId;
    this.#title = current.title;
    this.#path = current.path;
    this.#editor = createRevisionSafeEditorSession({
      documentId: current.documentId,
      body: current.body,
      versionId: current.versionId,
    });
    this.#coordinator = transitionDocumentSaveCoordinator(
      createDocumentSaveCoordinator({ autosaveDelayMs: this.#coordinator.autosaveDelayMs }),
      {
        type: DocumentSaveCoordinatorEvent.DocumentOpened,
        revision: this.#editor.revision,
        versionId: current.versionId,
      },
    ).snapshot;
    this.#retryable = undefined;
    this.#repairRequired = undefined;
    this.#saveOperation = undefined;
    return this.snapshot();
  }

  changeContent(nextBody: string): DesktopDocumentAuthoringSnapshot {
    if (
      !this.#editor ||
      [
        DocumentSaveCoordinatorState.NoDocument,
        DocumentSaveCoordinatorState.CloseBlocked,
        DocumentSaveCoordinatorState.ReadOnlyRecovery,
      ].includes(this.#coordinator.state)
    ) {
      return this.snapshot();
    }
    const changed = applyRevisionSafeEditorContentChange(this.#editor, nextBody);
    if (changed === this.#editor) return this.snapshot();
    this.#editor = changed;
    this.#coordinator = transitionDocumentSaveCoordinator(this.#coordinator, {
      type: DocumentSaveCoordinatorEvent.ContentChanged,
      revision: changed.revision,
      contentRef: `revision:${changed.revision}`,
    }).snapshot;
    return this.snapshot();
  }

  applyMetadataReadback(
    current: CurrentDocumentView,
  ): DesktopDocumentMetadataReadbackResult {
    if (
      !this.#editor ||
      current.workspaceId !== this.#workspaceId ||
      current.documentId !== this.#editor.documentId ||
      current.versionId !== this.#editor.expectedVersionId
    ) {
      return {
        accepted: false,
        snapshot: this.snapshot(),
        errorCode: "DOCUMENT_AUTHORING_READBACK_MISMATCH",
      };
    }
    this.#title = current.title;
    this.#path = current.path;
    return { accepted: true, snapshot: this.snapshot() };
  }

  async autosaveElapsed(elapsedMs: number): Promise<DesktopDocumentAuthoringSnapshot> {
    const transition = transitionDocumentSaveCoordinator(this.#coordinator, {
      type: DocumentSaveCoordinatorEvent.AutosaveElapsed,
      elapsedMs,
    });
    this.#coordinator = transition.snapshot;
    await this.#executeSideEffect(transition.sideEffect);
    return this.snapshot();
  }

  async manualSave(): Promise<DesktopDocumentAuthoringSnapshot> {
    const transition = transitionDocumentSaveCoordinator(this.#coordinator, {
      type: DocumentSaveCoordinatorEvent.SaveRequested,
    });
    this.#coordinator = transition.snapshot;
    await this.#executeSideEffect(transition.sideEffect);
    return this.snapshot();
  }

  async retrySave(): Promise<DesktopDocumentAuthoringSnapshot> {
    const transition = transitionDocumentSaveCoordinator(this.#coordinator, {
      type: DocumentSaveCoordinatorEvent.RetryRequested,
    });
    this.#coordinator = transition.snapshot;
    await this.#executeSideEffect(transition.sideEffect);
    return this.snapshot();
  }

  requestClose(): DesktopDocumentCloseDecision {
    if (
      [
        DocumentSaveCoordinatorState.NoDocument,
        DocumentSaveCoordinatorState.Clean,
        DocumentSaveCoordinatorState.Saved,
      ].includes(this.#coordinator.state)
    ) {
      return { canClose: true };
    }
    const transition = transitionDocumentSaveCoordinator(this.#coordinator, {
      type: DocumentSaveCoordinatorEvent.CloseRequested,
    });
    this.#coordinator = transition.snapshot;
    return {
      canClose: false,
      choices: transition.recoveryChoices,
    };
  }

  cancelClose(): DesktopDocumentAuthoringSnapshot {
    this.#coordinator = transitionDocumentSaveCoordinator(this.#coordinator, {
      type: DocumentSaveCoordinatorEvent.CloseCancelled,
    }).snapshot;
    return this.snapshot();
  }

  discard(): DesktopDocumentAuthoringSnapshot {
    this.#coordinator = transitionDocumentSaveCoordinator(this.#coordinator, {
      type: DocumentSaveCoordinatorEvent.DiscardConfirmed,
    }).snapshot;
    if (this.#coordinator.state === DocumentSaveCoordinatorState.NoDocument) {
      this.#workspaceId = undefined;
      this.#title = undefined;
      this.#path = undefined;
      this.#editor = undefined;
      this.#retryable = undefined;
      this.#repairRequired = undefined;
      this.#saveOperation = undefined;
    }
    return this.snapshot();
  }

  snapshot(): DesktopDocumentAuthoringSnapshot {
    return {
      workspaceId: this.#workspaceId,
      documentId: this.#editor?.documentId,
      title: this.#title,
      path: this.#path,
      body: this.#editor?.currentBody,
      revision: this.#editor?.revision ?? 0,
      persistedRevision: this.#editor?.persistedRevision ?? 0,
      expectedVersionId: this.#editor?.expectedVersionId,
      saveState: this.#coordinator.state,
      errorCode: this.#coordinator.errorCode ?? this.#editor?.errorCode,
      retryable: this.#retryable,
      repairRequired: this.#repairRequired,
    };
  }

  async #executeSideEffect(
    effect: DocumentSaveSideEffectRequest | undefined,
  ): Promise<void> {
    if (!effect || !this.#editor || !this.#workspaceId) return;
    const start = startRevisionSafeEditorSave(this.#editor);
    if (!start.started || !start.command || start.command.revision !== effect.revision) return;
    this.#editor = start.session;
    this.#coordinator = transitionDocumentSaveCoordinator(this.#coordinator, {
      type: DocumentSaveCoordinatorEvent.SaveStarted,
      revision: start.command.revision,
    }).snapshot;
    try {
      const operationId = this.#operationIdForRevision(start.command.revision);
      const command: SaveDocumentRevisionCommand = {
        operationId,
        workspaceId: this.#workspaceId,
        documentId: start.command.documentId,
        body: start.command.body,
        expectedVersionId: start.command.expectedVersionId ?? "",
        author: this.#author,
        summary: this.#summary,
        revision: start.command.revision,
      };
      const result = await this.#client.saveDocumentRevision(command);
      if (result.revision === start.command.revision) {
        const persisted = await this.#client.getCurrentDocument({
          queryName: "get-current-document",
          workspaceId: this.#workspaceId,
          documentId: start.command.documentId,
        });
        if (
          persisted.versionId !== result.currentVersionId ||
          persisted.body !== start.command.body
        ) {
          throw new LocalDesktopCommandClientError(
            "DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE",
            true,
          );
        }
        this.#title = persisted.title;
        this.#path = persisted.path;
      }
      const editorCompletion = completeRevisionSafeEditorSave(this.#editor, {
        revision: result.revision,
        status: "succeeded",
        savedVersionId: result.currentVersionId,
      });
      this.#editor = editorCompletion.session;
      const transition = transitionDocumentSaveCoordinator(this.#coordinator, {
        type: DocumentSaveCoordinatorEvent.SaveSucceeded,
        revision: result.revision,
        savedVersionId: result.currentVersionId,
      });
      this.#coordinator = transition.snapshot;
      this.#retryable = undefined;
      this.#repairRequired = undefined;
      if (result.revision === start.command.revision) {
        this.#saveOperation = undefined;
      }
      await this.#executeSideEffect(transition.sideEffect);
    } catch (error) {
      const mapped = error instanceof LocalDesktopCommandClientError
        ? {
            code: error.code,
            retryable: error.retryable,
            repairRequired: error.repairRequired,
          }
        : { code: "COMMAND_BRIDGE_FAILED", retryable: false, repairRequired: false };
      const editorCompletion = completeRevisionSafeEditorSave(this.#editor, {
        revision: start.command.revision,
        status: "failed",
        errorCode: mapped.code,
      });
      this.#editor = editorCompletion.session;
      this.#coordinator = transitionDocumentSaveCoordinator(this.#coordinator, {
        type: DocumentSaveCoordinatorEvent.SaveFailed,
        revision: start.command.revision,
        errorCode: mapped.code,
      }).snapshot;
      this.#retryable = mapped.retryable;
      this.#repairRequired = mapped.repairRequired;
      if (mapped.repairRequired) {
        this.#coordinator = transitionDocumentSaveCoordinator(this.#coordinator, {
          type: DocumentSaveCoordinatorEvent.ReadOnlyEntered,
        }).snapshot;
      }
    }
  }

  #operationIdForRevision(revision: number): string {
    if (this.#saveOperation?.revision === revision) {
      return this.#saveOperation.operationId;
    }
    const operationId = this.#operationIdSource().trim();
    if (!operationId) {
      throw new LocalDesktopCommandClientError("DOCUMENT_REVISION_INVALID_INPUT", false);
    }
    this.#saveOperation = { revision, operationId };
    return operationId;
  }
}

export function createDesktopDocumentAuthoringController(
  options: DesktopDocumentAuthoringControllerOptions,
): DesktopDocumentAuthoringController {
  return new DesktopDocumentAuthoringController(options);
}
