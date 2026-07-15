export interface DesktopKeyboardShortcutInput {
  readonly key: string;
  readonly metaKey: boolean;
  readonly altKey?: boolean;
  readonly ctrlKey?: boolean;
  readonly shiftKey?: boolean;
}

export function isMacDocumentSaveShortcut(input: DesktopKeyboardShortcutInput): boolean {
  return input.metaKey
    && !input.altKey
    && !input.ctrlKey
    && !input.shiftKey
    && input.key.toLowerCase() === "s";
}
