import type { ClientCapabilities } from "@sponzey-cabinet/client-core";

export type EditorOperationKind =
  | "load-document"
  | "save-document"
  | "format-markdown"
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

export type MarkdownFormattingCommand = "heading" | "bold" | "italic" | "link" | "list" | "checklist" | "table";

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

export type WysiwygMarkdownInlineType = "text" | "wikilink" | "markdown_link" | "asset_reference";

export interface WysiwygMarkdownTextInline {
  readonly inlineType: "text";
  readonly text: string;
  readonly sourceRange: EditorSourceRange;
}

export interface WysiwygMarkdownWikilinkInline {
  readonly inlineType: "wikilink";
  readonly text: string;
  readonly target: string;
  readonly label?: string;
  readonly sourceRange: EditorSourceRange;
}

export interface WysiwygMarkdownMarkdownLinkInline {
  readonly inlineType: "markdown_link";
  readonly text: string;
  readonly target: string;
  readonly label: string;
  readonly sourceRange: EditorSourceRange;
}

export interface WysiwygMarkdownAssetReferenceInline {
  readonly inlineType: "asset_reference";
  readonly text: string;
  readonly assetId: string;
  readonly label: string;
  readonly sourceRange: EditorSourceRange;
}

export type WysiwygMarkdownInlineNode =
  | WysiwygMarkdownTextInline
  | WysiwygMarkdownWikilinkInline
  | WysiwygMarkdownMarkdownLinkInline
  | WysiwygMarkdownAssetReferenceInline;

export type WysiwygMarkdownBlockType =
  | "heading"
  | "paragraph"
  | "checklist"
  | "table"
  | "code_block"
  | "blockquote"
  | "fallback";

export type WysiwygMarkdownAlignment = "left" | "center" | "right" | "default";

export interface WysiwygMarkdownBlockBase {
  readonly blockId: string;
  readonly blockType: WysiwygMarkdownBlockType;
  readonly sourceRange: EditorSourceRange;
  readonly displayText: string;
  readonly editable: boolean;
  readonly fallbackReason?: string;
}

export interface WysiwygMarkdownHeadingBlock extends WysiwygMarkdownBlockBase {
  readonly blockType: "heading";
  readonly level: number;
  readonly inlines: readonly WysiwygMarkdownInlineNode[];
}

export interface WysiwygMarkdownParagraphBlock extends WysiwygMarkdownBlockBase {
  readonly blockType: "paragraph";
  readonly inlines: readonly WysiwygMarkdownInlineNode[];
}

export interface WysiwygMarkdownChecklistItem {
  readonly checked: boolean;
  readonly text: string;
}

export interface WysiwygMarkdownChecklistBlock extends WysiwygMarkdownBlockBase {
  readonly blockType: "checklist";
  readonly items: readonly WysiwygMarkdownChecklistItem[];
}

export interface WysiwygMarkdownTableBlock extends WysiwygMarkdownBlockBase {
  readonly blockType: "table";
  readonly headers: readonly string[];
  readonly alignments: readonly WysiwygMarkdownAlignment[];
  readonly rows: readonly (readonly string[])[];
}

export interface WysiwygMarkdownCodeBlock extends WysiwygMarkdownBlockBase {
  readonly blockType: "code_block";
  readonly editable: false;
  readonly language?: string;
}

export interface WysiwygMarkdownBlockquoteBlock extends WysiwygMarkdownBlockBase {
  readonly blockType: "blockquote";
  readonly editable: false;
  readonly calloutKind?: string;
}

export interface WysiwygMarkdownFallbackBlock extends WysiwygMarkdownBlockBase {
  readonly blockType: "fallback";
  readonly editable: false;
  readonly fallbackReason: string;
}

export type WysiwygMarkdownBlock =
  | WysiwygMarkdownHeadingBlock
  | WysiwygMarkdownParagraphBlock
  | WysiwygMarkdownChecklistBlock
  | WysiwygMarkdownTableBlock
  | WysiwygMarkdownCodeBlock
  | WysiwygMarkdownBlockquoteBlock
  | WysiwygMarkdownFallbackBlock;

export interface WysiwygMarkdownPresentationInput {
  readonly source: string;
}

export interface WysiwygMarkdownPresentationModel {
  readonly mode: "wysiwyg-markdown-presentation";
  readonly state: "Parsed";
  readonly blocks: readonly WysiwygMarkdownBlock[];
}

export interface WysiwygMarkdownBlockTextEditInput {
  readonly body: string;
  readonly sourceRange: EditorSourceRange;
  readonly expectedSourceText: string;
  readonly replacementSourceText: string;
}

export interface WysiwygMarkdownChecklistToggleInput {
  readonly body: string;
  readonly sourceRange: EditorSourceRange;
  readonly expectedSourceText: string;
  readonly itemIndex: number;
}

export interface WysiwygMarkdownTableCellEditInput {
  readonly body: string;
  readonly sourceRange: EditorSourceRange;
  readonly expectedSourceText: string;
  readonly rowIndex: number;
  readonly cellIndex: number;
  readonly replacementText: string;
}

export type WysiwygMarkdownBlockTextEditErrorCode =
  | "WYSIWYG_MARKDOWN_INVALID_RANGE"
  | "WYSIWYG_MARKDOWN_STALE_RANGE";

export type WysiwygMarkdownBlockTextEditResult =
  | {
      readonly status: "Applied";
      readonly nextBody: string;
      readonly changedRange: EditorSourceRange;
    }
  | {
      readonly status: "Rejected";
      readonly errorCode: WysiwygMarkdownBlockTextEditErrorCode;
    };

export type WysiwygPlainTextSyncEditorState =
  | "Idle"
  | "PlainTextEditing"
  | "WysiwygEditing"
  | "PatchRejected"
  | "RecoveryRequired";

export type WysiwygPlainTextSyncErrorCode =
  | "EDITOR_PATCH_STALE"
  | WysiwygMarkdownBlockTextEditErrorCode;

export interface WysiwygPlainTextSyncSessionInput {
  readonly documentId: string;
  readonly body: string;
  readonly revision?: number;
}

export interface WysiwygPlainTextSyncSession {
  readonly documentId: string;
  readonly body: string;
  readonly revision: number;
  readonly editorState: WysiwygPlainTextSyncEditorState;
  readonly errorCode?: WysiwygPlainTextSyncErrorCode;
}

export interface WysiwygPlainTextSyncChangeResult {
  readonly session: WysiwygPlainTextSyncSession;
  readonly changed: boolean;
}

export interface WysiwygPatchCommand {
  readonly baseRevision: number;
  readonly apply: () => WysiwygMarkdownBlockTextEditResult;
}

export type WysiwygPlainTextSyncPatchResult =
  | {
      readonly status: "Applied";
      readonly session: WysiwygPlainTextSyncSession;
      readonly changedRange: EditorSourceRange;
    }
  | {
      readonly status: "Rejected";
      readonly session: WysiwygPlainTextSyncSession;
      readonly errorCode: WysiwygPlainTextSyncErrorCode;
    };

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

export function createWysiwygPlainTextSyncSession(
  input: WysiwygPlainTextSyncSessionInput,
): WysiwygPlainTextSyncSession {
  return {
    documentId: input.documentId,
    body: input.body,
    revision: input.revision ?? 0,
    editorState: "Idle",
  };
}

export function applyPlainTextEditorChangeToSyncSession(
  session: WysiwygPlainTextSyncSession,
  nextBody: string,
): WysiwygPlainTextSyncChangeResult {
  if (nextBody === session.body) {
    return {
      session: {
        ...session,
        editorState: "PlainTextEditing",
        errorCode: undefined,
      },
      changed: false,
    };
  }

  return {
    session: {
      documentId: session.documentId,
      body: nextBody,
      revision: session.revision + 1,
      editorState: "PlainTextEditing",
    },
    changed: true,
  };
}

export function applyWysiwygPatchToSyncSession(
  session: WysiwygPlainTextSyncSession,
  command: WysiwygPatchCommand,
): WysiwygPlainTextSyncPatchResult {
  if (command.baseRevision !== session.revision) {
    return {
      status: "Rejected",
      session: {
        ...session,
        editorState: "PatchRejected",
        errorCode: "EDITOR_PATCH_STALE",
      },
      errorCode: "EDITOR_PATCH_STALE",
    };
  }

  const patch = command.apply();
  if (patch.status === "Rejected") {
    return {
      status: "Rejected",
      session: {
        ...session,
        editorState: "PatchRejected",
        errorCode: patch.errorCode,
      },
      errorCode: patch.errorCode,
    };
  }

  return {
    status: "Applied",
    session: {
      documentId: session.documentId,
      body: patch.nextBody,
      revision: patch.nextBody === session.body ? session.revision : session.revision + 1,
      editorState: "WysiwygEditing",
    },
    changedRange: patch.changedRange,
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

export function applyMarkdownFormattingCommand(
  body: string,
  command: MarkdownFormattingCommand,
): string {
  const value = createMarkdownFormattingOperation(command).value ?? "";
  if (!body) return value;
  return `${body}${body.endsWith("\n") ? "" : "\n\n"}${value}`;
}

export function createMarkdownFormattingOperation(command: MarkdownFormattingCommand): EditorOperation {
  if (command === "table") {
    return createInsertMarkdownTableOperation({
      headers: ["항목", "내용", "상태"],
      alignments: ["left", "center", "right"],
      rowCount: 2,
    });
  }
  const snippets: Record<Exclude<MarkdownFormattingCommand, "table">, string> = {
    heading: "# 제목",
    bold: "**굵은 텍스트**",
    italic: "_기울임 텍스트_",
    link: "[링크 텍스트](https://example.com)",
    list: "- 목록 항목",
    checklist: "- [ ] 할 일",
  };
  return {
    kind: "format-markdown",
    value: snippets[command],
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

export function createWysiwygMarkdownPresentationModel(
  input: WysiwygMarkdownPresentationInput,
): WysiwygMarkdownPresentationModel {
  return {
    mode: "wysiwyg-markdown-presentation",
    state: "Parsed",
    blocks: parseWysiwygMarkdownBlocks(input.source),
  };
}

export function applyWysiwygMarkdownBlockTextEdit(
  input: WysiwygMarkdownBlockTextEditInput,
): WysiwygMarkdownBlockTextEditResult {
  const { body, sourceRange, expectedSourceText, replacementSourceText } = input;
  if (
    !Number.isInteger(sourceRange.start) ||
    !Number.isInteger(sourceRange.end) ||
    sourceRange.start < 0 ||
    sourceRange.end < sourceRange.start ||
    sourceRange.end > body.length
  ) {
    return { status: "Rejected", errorCode: "WYSIWYG_MARKDOWN_INVALID_RANGE" };
  }
  if (body.slice(sourceRange.start, sourceRange.end) !== expectedSourceText) {
    return { status: "Rejected", errorCode: "WYSIWYG_MARKDOWN_STALE_RANGE" };
  }
  const nextBody = `${body.slice(0, sourceRange.start)}${replacementSourceText}${body.slice(sourceRange.end)}`;
  return {
    status: "Applied",
    nextBody,
    changedRange: {
      start: sourceRange.start,
      end: sourceRange.start + replacementSourceText.length,
    },
  };
}

export function applyWysiwygMarkdownChecklistItemToggle(
  input: WysiwygMarkdownChecklistToggleInput,
): WysiwygMarkdownBlockTextEditResult {
  const { body, sourceRange, expectedSourceText, itemIndex } = input;
  if (
    !Number.isInteger(itemIndex) ||
    itemIndex < 0 ||
    !Number.isInteger(sourceRange.start) ||
    !Number.isInteger(sourceRange.end) ||
    sourceRange.start < 0 ||
    sourceRange.end < sourceRange.start ||
    sourceRange.end > body.length
  ) {
    return { status: "Rejected", errorCode: "WYSIWYG_MARKDOWN_INVALID_RANGE" };
  }
  if (body.slice(sourceRange.start, sourceRange.end) !== expectedSourceText) {
    return { status: "Rejected", errorCode: "WYSIWYG_MARKDOWN_STALE_RANGE" };
  }
  const lines = expectedSourceText.split("\n");
  if (itemIndex >= lines.length || !isChecklistLine(lines[itemIndex])) {
    return { status: "Rejected", errorCode: "WYSIWYG_MARKDOWN_INVALID_RANGE" };
  }
  const replacementLines = lines.map((line, index) => {
    if (index !== itemIndex) return line;
    return line.replace(/^(-\s+\[)([ xX])(\]\s*)/, (_, prefix: string, marker: string, suffix: string) =>
      `${prefix}${marker.toLowerCase() === "x" ? " " : "x"}${suffix}`,
    );
  });
  return applyWysiwygMarkdownBlockTextEdit({
    body,
    sourceRange,
    expectedSourceText,
    replacementSourceText: replacementLines.join("\n"),
  });
}

export function applyWysiwygMarkdownTableCellEdit(
  input: WysiwygMarkdownTableCellEditInput,
): WysiwygMarkdownBlockTextEditResult {
  const { body, sourceRange, expectedSourceText, rowIndex, cellIndex, replacementText } = input;
  if (
    !Number.isInteger(rowIndex) ||
    !Number.isInteger(cellIndex) ||
    rowIndex < 0 ||
    cellIndex < 0 ||
    !Number.isInteger(sourceRange.start) ||
    !Number.isInteger(sourceRange.end) ||
    sourceRange.start < 0 ||
    sourceRange.end < sourceRange.start ||
    sourceRange.end > body.length
  ) {
    return { status: "Rejected", errorCode: "WYSIWYG_MARKDOWN_INVALID_RANGE" };
  }
  if (body.slice(sourceRange.start, sourceRange.end) !== expectedSourceText) {
    return { status: "Rejected", errorCode: "WYSIWYG_MARKDOWN_STALE_RANGE" };
  }
  const lines = expectedSourceText.split("\n");
  const bodyRowIndex = rowIndex + 2;
  if (lines.length < 3 || bodyRowIndex >= lines.length || !isPipeRow(lines[bodyRowIndex])) {
    return { status: "Rejected", errorCode: "WYSIWYG_MARKDOWN_INVALID_RANGE" };
  }
  const cells = [...parseMarkdownPipeRow(lines[bodyRowIndex])];
  if (cellIndex >= cells.length) {
    return { status: "Rejected", errorCode: "WYSIWYG_MARKDOWN_INVALID_RANGE" };
  }
  cells[cellIndex] = sanitizeMarkdownTableCell(replacementText);
  const replacementLines = lines.map((line, index) =>
    index === bodyRowIndex ? `| ${cells.join(" | ")} |` : line,
  );
  return applyWysiwygMarkdownBlockTextEdit({
    body,
    sourceRange,
    expectedSourceText,
    replacementSourceText: replacementLines.join("\n"),
  });
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

interface MarkdownSourceLine {
  readonly text: string;
  readonly start: number;
  readonly end: number;
}

function parseWysiwygMarkdownBlocks(source: string): readonly WysiwygMarkdownBlock[] {
  const lines = splitMarkdownSourceLines(source);
  const blocks: WysiwygMarkdownBlock[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];
    if (line.text.trim() === "") {
      index += 1;
      continue;
    }

    const heading = /^(#{1,6})(\s+)(.+)$/.exec(line.text);
    if (heading) {
      const headingText = heading[3].trim();
      const headingTextStart = line.start + heading[1].length + heading[2].length + heading[3].indexOf(headingText);
      blocks.push({
        blockId: createWysiwygBlockId(blocks.length),
        blockType: "heading",
        level: heading[1].length,
        sourceRange: { start: line.start, end: line.end },
        displayText: headingText,
        editable: true,
        inlines: parseWysiwygInlineNodes(headingText, headingTextStart),
      });
      index += 1;
      continue;
    }

    if (isCodeFenceLine(line.text)) {
      const startIndex = index;
      const language = parseCodeFenceLanguage(line.text);
      index += 1;
      const contentStartIndex = index;
      while (index < lines.length && !isCodeFenceLine(lines[index].text)) {
        index += 1;
      }
      const contentEndIndex = index - 1;
      const closed = index < lines.length && isCodeFenceLine(lines[index].text);
      const sourceEndIndex = closed ? index : Math.max(startIndex, index - 1);
      if (closed) index += 1;
      const contentLines = contentEndIndex >= contentStartIndex
        ? lines.slice(contentStartIndex, contentEndIndex + 1).map((contentLine) => contentLine.text)
        : [];
      blocks.push({
        blockId: createWysiwygBlockId(blocks.length),
        blockType: "code_block",
        sourceRange: rangeFromLines(lines, startIndex, sourceEndIndex),
        displayText: contentLines.join("\n"),
        editable: false,
        ...(language ? { language } : {}),
      });
      continue;
    }

    if (isBlockquoteLine(line.text)) {
      const startIndex = index;
      const quoteLines: string[] = [];
      let calloutKind: string | undefined;
      while (index < lines.length && isBlockquoteLine(lines[index].text)) {
        const stripped = stripBlockquotePrefix(lines[index].text);
        const callout = /^\[!([A-Z][A-Z0-9_-]*)]\s*(.*)$/.exec(stripped.trim());
        if (callout && calloutKind === undefined) {
          calloutKind = callout[1];
          quoteLines.push(callout[2].trim());
        } else {
          quoteLines.push(stripped);
        }
        index += 1;
      }
      blocks.push({
        blockId: createWysiwygBlockId(blocks.length),
        blockType: "blockquote",
        sourceRange: rangeFromLines(lines, startIndex, index - 1),
        displayText: quoteLines.join("\n").trim(),
        editable: false,
        ...(calloutKind ? { calloutKind } : {}),
      });
      continue;
    }

    if (isChecklistLine(line.text)) {
      const startIndex = index;
      const items: WysiwygMarkdownChecklistItem[] = [];
      while (index < lines.length && isChecklistLine(lines[index].text)) {
        const item = /^-\s+\[([ xX])]\s*(.*)$/.exec(lines[index].text);
        items.push({ checked: item?.[1].toLowerCase() === "x", text: item?.[2].trim() ?? "" });
        index += 1;
      }
      const sourceRange = rangeFromLines(lines, startIndex, index - 1);
      blocks.push({
        blockId: createWysiwygBlockId(blocks.length),
        blockType: "checklist",
        sourceRange,
        displayText: items.map((item) => item.text).join("\n"),
        editable: true,
        items,
      });
      continue;
    }

    if (isTableStart(lines, index)) {
      const startIndex = index;
      index += 2;
      while (index < lines.length && isPipeRow(lines[index].text)) {
        index += 1;
      }
      const tableLines = lines.slice(startIndex, index);
      const headers = parseMarkdownPipeRow(tableLines[0].text);
      const alignments = parseMarkdownAlignmentRow(tableLines[1].text, headers.length);
      const rows = tableLines.slice(2).map((row) => parseMarkdownPipeRow(row.text));
      blocks.push({
        blockId: createWysiwygBlockId(blocks.length),
        blockType: "table",
        sourceRange: rangeFromLines(lines, startIndex, index - 1),
        displayText: headers.join(" | "),
        editable: true,
        headers,
        alignments,
        rows,
      });
      continue;
    }

    if (isRawHtmlLine(line.text)) {
      blocks.push({
        blockId: createWysiwygBlockId(blocks.length),
        blockType: "fallback",
        sourceRange: { start: line.start, end: line.end },
        displayText: "원문 편집에서 확인할 수 있는 HTML 블록",
        editable: false,
        fallbackReason: "raw_html",
      });
      index += 1;
      continue;
    }

    const startIndex = index;
    while (
      index < lines.length &&
      lines[index].text.trim() !== "" &&
      !/^(#{1,6})\s+(.+)$/.test(lines[index].text) &&
      !isCodeFenceLine(lines[index].text) &&
      !isBlockquoteLine(lines[index].text) &&
      !isChecklistLine(lines[index].text) &&
      !isTableStart(lines, index) &&
      !isRawHtmlLine(lines[index].text)
    ) {
      index += 1;
    }
    const paragraphLines = lines.slice(startIndex, index);
    const displayText = paragraphLines.map((paragraphLine) => paragraphLine.text).join("\n");
    blocks.push({
      blockId: createWysiwygBlockId(blocks.length),
      blockType: "paragraph",
      sourceRange: rangeFromLines(lines, startIndex, index - 1),
      displayText,
      editable: true,
      inlines: parseWysiwygInlineNodes(displayText, lines[startIndex].start),
    });
  }

  return blocks;
}

function parseWysiwygInlineNodes(text: string, baseOffset: number): readonly WysiwygMarkdownInlineNode[] {
  const nodes: WysiwygMarkdownInlineNode[] = [];
  let cursor = 0;

  while (cursor < text.length) {
    const next = findNextWysiwygInlineCandidate(text, cursor);
    if (!next) {
      pushWysiwygTextInline(nodes, text.slice(cursor), baseOffset + cursor);
      break;
    }
    if (next.start > cursor) {
      pushWysiwygTextInline(nodes, text.slice(cursor, next.start), baseOffset + cursor);
    }
    nodes.push(offsetWysiwygInlineNode(next.node, baseOffset));
    cursor = next.end;
  }

  return nodes.length > 0 ? nodes : [{ inlineType: "text", text, sourceRange: { start: baseOffset, end: baseOffset + text.length } }];
}

function findNextWysiwygInlineCandidate(
  text: string,
  cursor: number,
): { readonly start: number; readonly end: number; readonly node: WysiwygMarkdownInlineNode } | undefined {
  const candidates: Array<{ readonly start: number; readonly end: number; readonly node: WysiwygMarkdownInlineNode }> = [];
  const asset = parseNextWysiwygAssetInline(text, cursor);
  if (asset) candidates.push(asset);
  const wikilink = parseNextWysiwygWikilinkInline(text, cursor);
  if (wikilink) candidates.push(wikilink);
  const markdownLink = parseNextWysiwygMarkdownLinkInline(text, cursor);
  if (markdownLink) candidates.push(markdownLink);
  candidates.sort((left, right) => left.start - right.start || left.end - right.end);
  return candidates[0];
}

function parseNextWysiwygAssetInline(
  text: string,
  cursor: number,
): { readonly start: number; readonly end: number; readonly node: WysiwygMarkdownInlineNode } | undefined {
  const open = text.indexOf("![[asset:", cursor);
  if (open === -1) return undefined;
  const contentStart = open + "![[asset:".length;
  const close = text.indexOf("]]", contentStart);
  if (close === -1) return undefined;
  const parsed = parseAssetReferenceContent(text.slice(contentStart, close));
  if (!parsed) return undefined;
  const end = close + 2;
  return {
    start: open,
    end,
    node: {
      inlineType: "asset_reference",
      text: parsed.label,
      assetId: parsed.assetId,
      label: parsed.label,
      sourceRange: { start: open, end },
    },
  };
}

function parseNextWysiwygWikilinkInline(
  text: string,
  cursor: number,
): { readonly start: number; readonly end: number; readonly node: WysiwygMarkdownInlineNode } | undefined {
  const open = text.indexOf("[[", cursor);
  if (open === -1) return undefined;
  if (open > 0 && text.startsWith("![[", open - 1)) return parseNextWysiwygWikilinkInline(text, open + 2);
  const contentStart = open + 2;
  const close = text.indexOf("]]", contentStart);
  if (close === -1) return undefined;
  const parsed = parseWikilinkContent(text.slice(contentStart, close));
  if (!parsed) return undefined;
  const end = close + 2;
  return {
    start: open,
    end,
    node: {
      inlineType: "wikilink",
      text: parsed.label ?? parsed.target,
      target: parsed.target,
      ...(parsed.label ? { label: parsed.label } : {}),
      sourceRange: { start: open, end },
    },
  };
}

function parseNextWysiwygMarkdownLinkInline(
  text: string,
  cursor: number,
): { readonly start: number; readonly end: number; readonly node: WysiwygMarkdownInlineNode } | undefined {
  const linkPattern = /\[([^\]\n]+)]\(([^)\n]+)\)/g;
  linkPattern.lastIndex = cursor;
  const match = linkPattern.exec(text);
  if (!match) return undefined;
  const start = match.index;
  const end = start + match[0].length;
  return {
    start,
    end,
    node: {
      inlineType: "markdown_link",
      text: match[1].trim(),
      label: match[1].trim(),
      target: match[2].trim(),
      sourceRange: { start, end },
    },
  };
}

function pushWysiwygTextInline(nodes: WysiwygMarkdownInlineNode[], value: string, start: number): void {
  if (!value) return;
  nodes.push({ inlineType: "text", text: value, sourceRange: { start, end: start + value.length } });
}

function offsetWysiwygInlineNode(
  node: WysiwygMarkdownInlineNode,
  offset: number,
): WysiwygMarkdownInlineNode {
  const sourceRange = {
    start: node.sourceRange.start + offset,
    end: node.sourceRange.end + offset,
  };
  if (node.inlineType === "text") return { ...node, sourceRange };
  if (node.inlineType === "wikilink") return { ...node, sourceRange };
  if (node.inlineType === "markdown_link") return { ...node, sourceRange };
  return { ...node, sourceRange };
}

function splitMarkdownSourceLines(source: string): readonly MarkdownSourceLine[] {
  if (!source) return [];
  const lines: MarkdownSourceLine[] = [];
  let start = 0;
  for (const text of source.split("\n")) {
    const end = start + text.length;
    lines.push({ text, start, end });
    start = end + 1;
  }
  return lines;
}

function createWysiwygBlockId(index: number): string {
  return `block-${index + 1}`;
}

function rangeFromLines(lines: readonly MarkdownSourceLine[], startIndex: number, endIndex: number): EditorSourceRange {
  return {
    start: lines[startIndex].start,
    end: lines[endIndex].end,
  };
}

function isCodeFenceLine(line: string): boolean {
  return /^```[A-Za-z0-9_-]*\s*$/.test(line.trim());
}

function parseCodeFenceLanguage(line: string): string | undefined {
  const language = /^```([A-Za-z0-9_-]*)\s*$/.exec(line.trim())?.[1]?.trim();
  return language ? language : undefined;
}

function isBlockquoteLine(line: string): boolean {
  return /^>\s?/.test(line);
}

function stripBlockquotePrefix(line: string): string {
  return line.replace(/^>\s?/, "");
}

function isChecklistLine(line: string): boolean {
  return /^-\s+\[[ xX]]\s*/.test(line);
}

function isTableStart(lines: readonly MarkdownSourceLine[], index: number): boolean {
  return index + 1 < lines.length && isPipeRow(lines[index].text) && isMarkdownAlignmentRow(lines[index + 1].text);
}

function isPipeRow(line: string): boolean {
  return /^\s*\|.*\|\s*$/.test(line);
}

function isMarkdownAlignmentRow(line: string): boolean {
  const cells = parseMarkdownPipeRow(line);
  return cells.length > 0 && cells.every((cell) => /^:?-{3,}:?$/.test(cell.trim()));
}

function isRawHtmlLine(line: string): boolean {
  return /^<\/?[A-Za-z][^>]*>/.test(line.trim());
}

function parseMarkdownPipeRow(line: string): readonly string[] {
  return line.trim().replace(/^\|/, "").replace(/\|$/, "").split("|").map((cell) => cell.trim());
}

function parseMarkdownAlignmentRow(line: string, expectedLength: number): readonly WysiwygMarkdownAlignment[] {
  const alignments = parseMarkdownPipeRow(line).map((cell) => {
    const value = cell.trim();
    if (value.startsWith(":") && value.endsWith(":")) return "center";
    if (value.startsWith(":")) return "left";
    if (value.endsWith(":")) return "right";
    return "default";
  });
  return Array.from({ length: expectedLength }, (_, index) => alignments[index] ?? "default");
}
