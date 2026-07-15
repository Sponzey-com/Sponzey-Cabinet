import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { markdown } from "@codemirror/lang-markdown";
import { EditorState } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";

export interface CodeMirrorDocumentEditorOptions {
  readonly parent: HTMLElement;
  readonly body: string;
  readonly onChange: (body: string) => void;
}

export interface CodeMirrorDocumentEditor {
  setDocument(body: string): void;
  focus(): void;
  destroy(): void;
}

export const CODEMIRROR_REPLACE_DOCUMENT_EVENT = "cabinet:replace-codemirror-document";

export interface CodeMirrorReplaceDocumentDetail {
  readonly body: string;
}

export function codeMirrorDocumentContentAttributes(): Readonly<Record<string, string>> {
  return Object.freeze({ "data-action": "edit-document-body", "aria-label": "문서 본문 편집" });
}

export function requestCodeMirrorDocumentReplacement(
  target: EventTarget,
  body: string,
  createEvent: (detail: CodeMirrorReplaceDocumentDetail) => Event = (detail) => new CustomEvent(CODEMIRROR_REPLACE_DOCUMENT_EVENT, { detail }),
): boolean {
  return target.dispatchEvent(createEvent({ body }));
}

export function mountCodeMirrorDocumentEditor(
  options: CodeMirrorDocumentEditorOptions,
): CodeMirrorDocumentEditor {
  let applyingExternalDocument = false;
  const view = new EditorView({
    parent: options.parent,
    state: EditorState.create({
      doc: options.body,
      extensions: [
        history(),
        markdown(),
        keymap.of([
          ...defaultKeymap,
          ...historyKeymap,
        ]),
        EditorView.lineWrapping,
        EditorView.contentAttributes.of(codeMirrorDocumentContentAttributes()),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !applyingExternalDocument) {
            options.onChange(update.state.doc.toString());
          }
        }),
        EditorView.theme({
          "&": { height: "100%", fontSize: "14px" },
          ".cm-scroller": {
            overflow: "auto",
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
          },
          ".cm-content": { minHeight: "100%", padding: "18px 20px" },
          ".cm-gutters": { backgroundColor: "#f7f9f7", borderRight: "1px solid #d9ded9" },
          "&.cm-focused": { outline: "2px solid #76aee8", outlineOffset: "-2px" },
        }),
      ],
    }),
  });

  const replaceDocument = (event: Event) => {
    const detail = (event as CustomEvent<CodeMirrorReplaceDocumentDetail>).detail;
    if (!detail || typeof detail.body !== "string") return;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: detail.body },
    });
  };
  options.parent.addEventListener(CODEMIRROR_REPLACE_DOCUMENT_EVENT, replaceDocument);

  return {
    setDocument(body) {
      if (body === view.state.doc.toString()) return;
      applyingExternalDocument = true;
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: body },
      });
      applyingExternalDocument = false;
    },
    focus() {
      view.focus();
    },
    destroy() {
      options.parent.removeEventListener(CODEMIRROR_REPLACE_DOCUMENT_EVENT, replaceDocument);
      view.destroy();
    },
  };
}
