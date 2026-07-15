export interface ModalFocusTarget {
  readonly isConnected: boolean;
  focus(): void;
}

export interface ModalFocusEnvironment {
  activeElement(): ModalFocusTarget | undefined;
  defer(callback: () => void): void;
}

export function createFocusRestoringModalAction(
  action: () => void,
  environment: ModalFocusEnvironment = browserEnvironment(),
): () => void {
  const target = environment.activeElement();
  return () => {
    action();
    if (!target) return;
    environment.defer(() => {
      if (target.isConnected) target.focus();
    });
  };
}

export function browserModalFocusEnvironment(returnActionId?: string): ModalFocusEnvironment {
  return browserEnvironment(returnActionId);
}

function browserEnvironment(returnActionId?: string): ModalFocusEnvironment {
  return {
    activeElement() {
      if (typeof document === "undefined") return undefined;
      const selected = returnActionId
        ? document.querySelector(`[data-action="${returnActionId}"]`)
        : document.activeElement;
      return selected instanceof HTMLElement ? selected : undefined;
    },
    defer(callback) {
      globalThis.queueMicrotask(callback);
    },
  };
}
