import type { ClientCapabilities } from "@sponzey-cabinet/client-core";

export type EditorOperationKind =
  | "load-document"
  | "save-document"
  | "insert-wikilink"
  | "insert-asset-reference"
  | "insert-markdown-table";

export interface EditorOperation {
  readonly kind: EditorOperationKind;
  readonly documentId?: string;
  readonly value?: string;
}

export interface EditorSourceRange {
  readonly start: number;
  readonly end: number;
}

export type EditorDirtyState = "clean" | "dirty";

export interface EditorDocumentSnapshot {
  readonly documentId: string;
  readonly body: string;
  readonly versionId?: string;
}

export interface EditorSessionModel {
  readonly documentId: string;
  readonly loadedBody: string;
  readonly currentBody: string;
  readonly dirtyState: EditorDirtyState;
}

export interface EditorSaveCommand {
  readonly kind: "save-document";
  readonly documentId: string;
  readonly body: string;
  readonly dirtyState: EditorDirtyState;
}

export type MarkdownTableSourceAlignment = "left" | "center" | "right" | "default";

export interface InsertMarkdownTableInput {
  readonly headers: readonly string[];
  readonly alignments?: readonly MarkdownTableSourceAlignment[];
  readonly rowCount: number;
}

export interface WikilinkDecoration {
  readonly target: string;
  readonly label?: string;
  readonly text: string;
  readonly range: EditorSourceRange;
}

export interface WikilinkOpenCommand {
  readonly kind: "open-wikilink";
  readonly target: string;
  readonly label?: string;
  readonly range: EditorSourceRange;
}

export interface AssetReferenceDecoration {
  readonly assetId: string;
  readonly label: string;
  readonly text: string;
  readonly range: EditorSourceRange;
}

export interface AssetReferenceOpenCommand {
  readonly kind: "open-asset-reference";
  readonly assetId: string;
  readonly label: string;
  readonly range: EditorSourceRange;
}

export interface EditorTextChangeDraft {
  readonly start: number;
  readonly end: number;
  readonly insertedText: string;
}

export interface EditorTransactionDraft {
  readonly documentId: string;
  readonly actorUserId: string;
  readonly operationId: string;
  readonly baseRevision: number;
  readonly currentRevision: number;
  readonly changes: readonly EditorTextChangeDraft[];
}

export interface EditorCollaborativeEditInput {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly actorUserId: string;
  readonly operationId: string;
  readonly baseRevision: number;
  readonly currentRevision: number;
  readonly startOffset: number;
  readonly endOffset: number;
  readonly insertedText: string;
}

export interface EditorSelectionDraft {
  readonly documentId: string;
  readonly actorUserId: string;
  readonly cursorStart: number;
  readonly cursorEnd: number;
  readonly selectedText?: string;
  readonly documentBody?: string;
  readonly token?: string;
}

export interface EditorPresenceInput {
  readonly workspaceId: string;
  readonly documentId: string;
  readonly actorUserId: string;
  readonly cursorStart: number;
  readonly cursorEnd: number;
}

export type EditorCollaborationAdapterErrorCode =
  | "EDITOR_COLLABORATION_MULTI_CHANGE_UNSUPPORTED"
  | "EDITOR_COLLABORATION_INVALID_RANGE";

export class EditorCollaborationAdapterError extends Error {
  readonly code: EditorCollaborationAdapterErrorCode;

  constructor(code: EditorCollaborationAdapterErrorCode, message: string) {
    super(message);
    this.name = "EditorCollaborationAdapterError";
    this.code = code;
  }
}

export function createEditorBoundaryDescriptor(capabilities: ClientCapabilities): string {
  return `editor:${capabilities.runtime}`;
}

export function createEditorSession(snapshot: EditorDocumentSnapshot): EditorSessionModel {
  return {
    documentId: snapshot.documentId,
    loadedBody: snapshot.body,
    currentBody: snapshot.body,
    dirtyState: "clean",
  };
}

export function applyEditorContentChange(session: EditorSessionModel, nextBody: string): EditorSessionModel {
  return {
    documentId: session.documentId,
    loadedBody: session.loadedBody,
    currentBody: nextBody,
    dirtyState: nextBody === session.loadedBody ? "clean" : "dirty",
  };
}

export function createEditorLoadOperation(snapshot: EditorDocumentSnapshot): EditorOperation {
  return {
    kind: "load-document",
    documentId: snapshot.documentId,
    value: snapshot.body,
  };
}

export function createEditorSaveCommand(session: EditorSessionModel): EditorSaveCommand {
  return {
    kind: "save-document",
    documentId: session.documentId,
    body: session.currentBody,
    dirtyState: session.dirtyState,
  };
}

export interface RevisionSafeEditorSession {
  readonly documentId: string;
  readonly currentBody: string;
  readonly revision: number;
  readonly persistedRevision: number;
  readonly expectedVersionId?: string;
  readonly inFlightRevision?: number;
  readonly dirtyState: EditorDirtyState;
  readonly errorCode?: string;
}

export interface RevisionSafeSaveCommand {
  readonly kind: "save-document-revision";
  readonly documentId: string;
  readonly body: string;
  readonly revision: number;
  readonly expectedVersionId?: string;
}

export interface RevisionSafeSaveStartResult {
  readonly started: boolean;
  readonly session: RevisionSafeEditorSession;
  readonly command?: RevisionSafeSaveCommand;
}

export type RevisionSafeSaveCompletion =
  | {
      readonly revision: number;
      readonly status: "succeeded";
      readonly savedVersionId: string;
    }
  | {
      readonly revision: number;
      readonly status: "failed";
      readonly errorCode: string;
    };

export interface RevisionSafeSaveCompletionResult {
  readonly ignored: boolean;
  readonly session: RevisionSafeEditorSession;
}

export function createRevisionSafeEditorSession(
  snapshot: EditorDocumentSnapshot,
): RevisionSafeEditorSession {
  return {
    documentId: snapshot.documentId,
    currentBody: snapshot.body,
    revision: 0,
    persistedRevision: 0,
    expectedVersionId: snapshot.versionId,
    dirtyState: "clean",
  };
}

export function applyRevisionSafeEditorContentChange(
  session: RevisionSafeEditorSession,
  nextBody: string,
): RevisionSafeEditorSession {
  if (nextBody === session.currentBody) return session;
  return {
    ...session,
    currentBody: nextBody,
    revision: session.revision + 1,
    dirtyState: "dirty",
    errorCode: undefined,
  };
}

export function startRevisionSafeEditorSave(
  session: RevisionSafeEditorSession,
): RevisionSafeSaveStartResult {
  if (session.inFlightRevision !== undefined || session.revision <= session.persistedRevision) {
    return { started: false, session };
  }
  const command: RevisionSafeSaveCommand = {
    kind: "save-document-revision",
    documentId: session.documentId,
    body: session.currentBody,
    revision: session.revision,
    expectedVersionId: session.expectedVersionId,
  };
  return {
    started: true,
    session: {
      ...session,
      inFlightRevision: session.revision,
      errorCode: undefined,
    },
    command,
  };
}

export function completeRevisionSafeEditorSave(
  session: RevisionSafeEditorSession,
  completion: RevisionSafeSaveCompletion,
): RevisionSafeSaveCompletionResult {
  if (session.inFlightRevision !== completion.revision) {
    return { ignored: true, session };
  }
  if (completion.status === "failed") {
    return {
      ignored: false,
      session: {
        ...session,
        inFlightRevision: undefined,
        dirtyState: "dirty",
        errorCode: completion.errorCode,
      },
    };
  }
  return {
    ignored: false,
    session: {
      ...session,
      persistedRevision: completion.revision,
      expectedVersionId: completion.savedVersionId,
      inFlightRevision: undefined,
      dirtyState: session.revision > completion.revision ? "dirty" : "clean",
      errorCode: undefined,
    },
  };
}

export function createInsertMarkdownTableOperation(input: InsertMarkdownTableInput): EditorOperation {
  const headers = input.headers.map((header) => sanitizeMarkdownTableCell(header));
  const alignments = headers.map((_, index) =>
    markdownTableAlignmentSource(input.alignments?.[index] ?? "default"),
  );
  const emptyRow = `| ${headers.map(() => "").join(" | ")} |`;
  const rows = Array.from({ length: Math.max(0, input.rowCount) }, () => emptyRow);
  return {
    kind: "insert-markdown-table",
    value: [
      `| ${headers.join(" | ")} |`,
      `| ${alignments.join(" | ")} |`,
      ...rows,
    ].join("\n"),
  };
}

export function findWikilinkDecorations(source: string): readonly WikilinkDecoration[] {
  const decorations: WikilinkDecoration[] = [];
  let cursor = 0;

  while (cursor < source.length) {
    const open = source.indexOf("[[", cursor);
    if (open === -1) {
      break;
    }

    const isAssetReference = open > 0 && source.startsWith("![[", open - 1);
    const contentStart = open + 2;
    const close = source.indexOf("]]", contentStart);
    if (close === -1) {
      break;
    }

    if (!isAssetReference) {
      const content = source.slice(contentStart, close);
      const parsed = parseWikilinkContent(content);
      if (parsed) {
        decorations.push({
          target: parsed.target,
          label: parsed.label,
          text: source.slice(open, close + 2),
          range: { start: open, end: close + 2 },
        });
      }
    }

    cursor = close + 2;
  }

  return decorations;
}

export function createInsertWikilinkOperation(target: string, label?: string): EditorOperation {
  const normalizedTarget = target.trim();
  const normalizedLabel = label?.trim();
  const suffix = normalizedLabel ? `|${normalizedLabel}` : "";

  return {
    kind: "insert-wikilink",
    value: `[[${normalizedTarget}${suffix}]]`,
  };
}

export function createOpenWikilinkCommand(decoration: WikilinkDecoration): WikilinkOpenCommand {
  return {
    kind: "open-wikilink",
    target: decoration.target,
    label: decoration.label,
    range: decoration.range,
  };
}

function parseWikilinkContent(content: string): { target: string; label?: string } | undefined {
  const [targetValue, ...labelValues] = content.split("|");
  const target = targetValue.trim();
  const label = labelValues.join("|").trim();
  if (!target) {
    return undefined;
  }
  return label ? { target, label } : { target };
}

function markdownTableAlignmentSource(alignment: MarkdownTableSourceAlignment): string {
  if (alignment === "left") {
    return ":---";
  }
  if (alignment === "center") {
    return ":---:";
  }
  if (alignment === "right") {
    return "---:";
  }
  return "---";
}

function sanitizeMarkdownTableCell(value: string): string {
  return value.replace(/\|/g, "\\|").trim();
}

export function findAssetReferenceDecorations(source: string): readonly AssetReferenceDecoration[] {
  const decorations: AssetReferenceDecoration[] = [];
  let cursor = 0;

  while (cursor < source.length) {
    const open = source.indexOf("![[asset:", cursor);
    if (open === -1) {
      break;
    }

    const contentStart = open + "![[asset:".length;
    const close = source.indexOf("]]", contentStart);
    if (close === -1) {
      break;
    }

    const content = source.slice(contentStart, close);
    const parsed = parseAssetReferenceContent(content);
    if (parsed) {
      decorations.push({
        assetId: parsed.assetId,
        label: parsed.label,
        text: source.slice(open, close + 2),
        range: { start: open, end: close + 2 },
      });
    }

    cursor = close + 2;
  }

  return decorations;
}

export function createInsertAssetReferenceOperation(assetId: string, label: string): EditorOperation {
  return {
    kind: "insert-asset-reference",
    value: `![[asset:${assetId.trim()}|${label.trim()}]]`,
  };
}

export function createOpenAssetReferenceCommand(
  decoration: AssetReferenceDecoration,
): AssetReferenceOpenCommand {
  return {
    kind: "open-asset-reference",
    assetId: decoration.assetId,
    label: decoration.label,
    range: decoration.range,
  };
}

export function createCollaborativeEditInputFromEditorTransaction(
  workspaceId: string,
  transaction: EditorTransactionDraft,
): EditorCollaborativeEditInput {
  if (transaction.changes.length !== 1) {
    throw new EditorCollaborationAdapterError(
      "EDITOR_COLLABORATION_MULTI_CHANGE_UNSUPPORTED",
      "Only single text change drafts can be converted to collaborative edit input.",
    );
  }

  const change = transaction.changes[0];
  assertEditorRange(change.start, change.end);

  return {
    workspaceId,
    documentId: transaction.documentId,
    actorUserId: transaction.actorUserId,
    operationId: transaction.operationId,
    baseRevision: transaction.baseRevision,
    currentRevision: transaction.currentRevision,
    startOffset: change.start,
    endOffset: change.end,
    insertedText: change.insertedText,
  };
}

export function createPresenceInputFromEditorSelection(
  workspaceId: string,
  selection: EditorSelectionDraft,
): EditorPresenceInput {
  assertEditorRange(selection.cursorStart, selection.cursorEnd);

  return {
    workspaceId,
    documentId: selection.documentId,
    actorUserId: selection.actorUserId,
    cursorStart: selection.cursorStart,
    cursorEnd: selection.cursorEnd,
  };
}

function assertEditorRange(start: number, end: number): void {
  if (!Number.isInteger(start) || !Number.isInteger(end) || start < 0 || end < start) {
    throw new EditorCollaborationAdapterError(
      "EDITOR_COLLABORATION_INVALID_RANGE",
      "Editor collaboration ranges must use non-negative integer offsets with end greater than or equal to start.",
    );
  }
}

function parseAssetReferenceContent(content: string): { assetId: string; label: string } | undefined {
  const [assetIdValue, ...labelValues] = content.split("|");
  const assetId = assetIdValue.trim();
  const label = labelValues.join("|").trim();
  if (!assetId || !label) {
    return undefined;
  }
  return { assetId, label };
}
