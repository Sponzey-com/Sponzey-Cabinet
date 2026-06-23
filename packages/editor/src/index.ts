import type { ClientCapabilities } from "@sponzey-cabinet/client-core";

export type EditorOperationKind = "load-document" | "save-document" | "insert-wikilink" | "insert-asset-reference";

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

function parseAssetReferenceContent(content: string): { assetId: string; label: string } | undefined {
  const [assetIdValue, ...labelValues] = content.split("|");
  const assetId = assetIdValue.trim();
  const label = labelValues.join("|").trim();
  if (!assetId || !label) {
    return undefined;
  }
  return { assetId, label };
}
