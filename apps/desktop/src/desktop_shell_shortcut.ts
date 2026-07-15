export interface WorkspaceSearchShortcutEvent {
  readonly key: string;
  readonly metaKey: boolean;
  readonly ctrlKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
  readonly repeat: boolean;
}

export function isMacWorkspaceSearchShortcut(event: WorkspaceSearchShortcutEvent): boolean {
  return event.key.toLowerCase() === "k"
    && event.metaKey
    && !event.ctrlKey
    && !event.altKey
    && !event.shiftKey
    && !event.repeat;
}
