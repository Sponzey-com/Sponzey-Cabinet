export interface ModalFocusable {
  readonly disabled?: boolean;
  focus(): void;
}

export interface ModalKeyboardEvent {
  readonly key: string;
  readonly shiftKey: boolean;
  readonly currentTarget: {
    querySelectorAll(selector: string): ArrayLike<unknown>;
  };
  readonly target: unknown;
  preventDefault(): void;
}

const FOCUSABLE_SELECTOR = "button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [href], [tabindex]:not([tabindex='-1'])";

export function handleModalKeyboard(event: ModalKeyboardEvent, dismiss: () => void): void {
  if (event.key === "Escape") {
    event.preventDefault();
    dismiss();
    return;
  }
  if (event.key !== "Tab") return;
  const focusable = Array.from(event.currentTarget.querySelectorAll(FOCUSABLE_SELECTOR))
    .filter(isModalFocusable)
    .filter((element) => element.disabled !== true);
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (!event.shiftKey && event.target === last) {
    event.preventDefault();
    first?.focus();
  } else if (event.shiftKey && event.target === first) {
    event.preventDefault();
    last?.focus();
  }
}

function isModalFocusable(value: unknown): value is ModalFocusable {
  return typeof value === "object" && value !== null && "focus" in value
    && typeof (value as { focus?: unknown }).focus === "function";
}
