import React from "react";

import { KO_KR_MESSAGES } from "./ko_kr_catalog.ts";
import {
  createWorkspaceShellElement,
  type WorkspaceShellDocumentShortcut,
} from "./react_workspace_shell.ts";
import { createWorkspaceShellModel, type WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";

export interface DesktopDocumentEmptyStateCallbacks {
  readonly onCreateDocument: () => void;
  readonly onHome: () => void;
  readonly onSearchOpen?: () => void;
  readonly onSearch: (query?: string) => void;
  readonly onGraph?: () => void;
  readonly onCanvas?: () => void;
  readonly onAssets?: () => void;
  readonly onBackup?: () => void;
}

export interface DesktopDocumentEmptyStateOptions {
  readonly documentShortcuts?: readonly WorkspaceShellDocumentShortcut[];
}

const shellRoutes: readonly WorkspaceShellRouteKind[] = [
  "Home",
  "Search",
  "Document",
  "Graph",
  "Canvas",
  "Assets",
  "Backup",
];

export function createDesktopDocumentEmptyStateElement(
  callbacks: DesktopDocumentEmptyStateCallbacks,
  options: DesktopDocumentEmptyStateOptions = {},
): React.ReactElement {
  const e = React.createElement;
  const message = KO_KR_MESSAGES.message;
  return createWorkspaceShellElement({
    model: createWorkspaceShellModel({
      route: "Document",
      availableActions: shellRoutes,
      messages: KO_KR_MESSAGES,
    }),
    messages: KO_KR_MESSAGES,
    routeActions: {
      Home: callbacks.onHome,
      Search: callbacks.onSearch,
      Graph: callbacks.onGraph,
      Canvas: callbacks.onCanvas,
      Assets: callbacks.onAssets,
      Backup: callbacks.onBackup,
    },
    rootAttributes: { "data-document-empty-state": "true" },
    onCreateDocument: callbacks.onCreateDocument,
    onSearchOpen: callbacks.onSearchOpen,
    onSearch: callbacks.onSearch,
    documentShortcuts: options.documentShortcuts,
    content: e(
      "main",
      { className: "document-empty-main" },
      e(
        "section",
        { className: "document-empty-state", "aria-labelledby": "document-empty-title" },
        e("h1", { id: "document-empty-title" }, message("document.emptyTitle")),
        e("p", null, message("document.emptyDescription")),
        e(
          "button",
          {
            type: "button",
            className: "primary-action",
            "data-action": "create-document-empty-state",
            onClick: callbacks.onCreateDocument,
          },
          message("action.createDocument"),
        ),
      ),
    ),
  });
}
