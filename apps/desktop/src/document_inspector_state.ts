export type DocumentInspectorTab = "links" | "attachments" | "history";

export type DocumentAttachmentUnlinkState =
  | { readonly status: "Closed" }
  | { readonly status: "Confirming"; readonly fileName: string }
  | { readonly status: "Submitting"; readonly fileName: string }
  | { readonly status: "Failed"; readonly fileName: string };

export interface DocumentInspectorState {
  readonly tab: DocumentInspectorTab;
  readonly unlink: DocumentAttachmentUnlinkState;
}

export type DocumentInspectorEvent =
  | { readonly type: "SelectTab"; readonly tab: DocumentInspectorTab }
  | { readonly type: "RequestUnlink"; readonly fileName: string }
  | { readonly type: "CancelUnlink" }
  | { readonly type: "ConfirmUnlink" }
  | { readonly type: "UnlinkSucceeded" }
  | { readonly type: "UnlinkFailed" };

export function createDocumentInspectorState(tab: DocumentInspectorTab = "links"): DocumentInspectorState {
  return Object.freeze({ tab, unlink: Object.freeze({ status: "Closed" as const }) });
}

export function transitionDocumentInspector(
  state: DocumentInspectorState,
  event: DocumentInspectorEvent,
): DocumentInspectorState {
  switch (event.type) {
    case "SelectTab":
      return event.tab === state.tab ? state : Object.freeze({ ...state, tab: event.tab });
    case "RequestUnlink": {
      const fileName = event.fileName.trim();
      if (!fileName || state.unlink.status === "Submitting") return state;
      return Object.freeze({ ...state, unlink: Object.freeze({ status: "Confirming" as const, fileName }) });
    }
    case "CancelUnlink":
      return state.unlink.status === "Closed"
        ? state
        : Object.freeze({ ...state, unlink: Object.freeze({ status: "Closed" as const }) });
    case "ConfirmUnlink":
      return state.unlink.status === "Confirming" || state.unlink.status === "Failed"
        ? Object.freeze({ ...state, unlink: Object.freeze({ status: "Submitting" as const, fileName: state.unlink.fileName }) })
        : state;
    case "UnlinkSucceeded":
      return state.unlink.status === "Submitting"
        ? Object.freeze({ ...state, unlink: Object.freeze({ status: "Closed" as const }) })
        : state;
    case "UnlinkFailed":
      return state.unlink.status === "Submitting"
        ? Object.freeze({ ...state, unlink: Object.freeze({ status: "Failed" as const, fileName: state.unlink.fileName }) })
        : state;
  }
}
