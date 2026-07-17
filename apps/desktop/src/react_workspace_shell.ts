import React from "react";
import { ChevronDown, ChevronRight, Plus, Search } from "lucide-react";

import type { WorkspaceShellModel, WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";
import type { MessageCatalog } from "./ko_kr_catalog.ts";

export interface WorkspaceShellDocumentShortcut {
  readonly label: string;
  readonly actionId?: string;
  readonly onOpen?: () => void;
}

export interface WorkspaceShellElementOptions {
  readonly model: WorkspaceShellModel;
  readonly messages: MessageCatalog;
  readonly routeActions: Readonly<Partial<Record<WorkspaceShellRouteKind, () => void>>>;
  readonly rootAttributes?: Readonly<Record<string, string>>;
  readonly rootClassName?: string;
  readonly onCreateDocument?: () => void;
  readonly onSearch?: () => void;
  readonly searchActionId?: string;
  readonly documentShortcuts?: readonly WorkspaceShellDocumentShortcut[];
  readonly savedStatus?: string;
  readonly mainClassName?: string;
  readonly topbarContent?: React.ReactNode;
  readonly topbarClassName?: string;
  readonly globalLayer?: React.ReactNode;
  readonly content: React.ReactNode;
}

const SHELL_ICON_PROPS = Object.freeze({ size: 15, strokeWidth: 2, "aria-hidden": true });

export function createWorkspaceShellElement(options: WorkspaceShellElementOptions): React.ReactElement {
  const e = React.createElement;
  const message = options.messages.message;
  const outlet = React.isValidElement(options.content) && options.content.type === "main"
    ? React.cloneElement(options.content as React.ReactElement<{ className?: string }>, {
        className: options.mainClassName ?? (options.content.props as { className?: string }).className ?? "desktop-main",
        tabIndex: -1,
        "data-workspace-route-main": "true",
      })
    : e("main", { className: options.mainClassName ?? "desktop-main", tabIndex: -1, "data-workspace-route-main": "true" }, options.content);
  return e(
    "div",
    {
      className: `desktop-shell cabinet-home-shell workspace-shell-frame${options.rootClassName ? ` ${options.rootClassName}` : ""}`,
      "data-cabinet-react-root": "mounted",
      "data-design-reference": "penpot-20260713",
      "data-shell-route": options.model.route,
      "data-shell-variant": options.model.variant,
      ...options.rootAttributes,
    },
    e(
      "aside",
      { className: "desktop-sidebar" },
      e("div", { className: "sidebar-brand-row" }, e("strong", { className: "desktop-wordmark" }, message("shell.brand")), e("span", { className: "local-badge" }, message("shell.local"))),
      e("button", { type: "button", className: "sidebar-new-document", "data-action": "new-document", onClick: options.onCreateDocument, disabled: !options.onCreateDocument }, e(Plus, SHELL_ICON_PROPS), message("shell.newDocument")),
      e("section", { className: "cabinet-summary", "aria-label": message("shell.cabinet") }, e("span", { className: "cabinet-summary-mark", "aria-hidden": "true" }), e("div", null, e("strong", null, message("shell.cabinet")), e("small", null, message("shell.cabinetStorage")))),
      e("nav", { className: "primary-navigation", "aria-label": message("shell.navigationLabel") }, options.model.navigation.map((item) => {
        const action = options.routeActions[item.route];
        return e("button", {
          key: item.route,
          type: "button",
          className: item.active ? "nav-item active" : "nav-item",
          "data-action": `navigate-${item.route.toLowerCase()}`,
          "aria-current": item.active ? "page" : undefined,
          disabled: item.active || !action,
          onClick: action,
        }, e("span", { className: "nav-marker", "aria-hidden": "true" }), item.label);
      })),
      e("section", { className: "sidebar-document-tree", "aria-label": message("shell.documentTreeLabel") }, e("p", { className: "sidebar-section-label" }, message("shell.documentTreeLabel")), e("strong", { className: "tree-section-heading" }, e(ChevronDown, SHELL_ICON_PROPS), message("shell.gettingStarted")), e("button", { type: "button", "data-action": "navigate-search", onClick: options.onSearch, disabled: !options.onSearch }, message("shell.welcomeDocument")), e("strong", { className: "tree-section-heading" }, e(ChevronDown, SHELL_ICON_PROPS), message("shell.projects")), ...(options.documentShortcuts ?? []).map((shortcut, index) => shortcut.onOpen
        ? e("button", { key: `${shortcut.label}-${index}`, type: "button", "data-action": shortcut.actionId ?? "open-sidebar-document", onClick: shortcut.onOpen }, shortcut.label)
        : e("span", { key: `${shortcut.label}-${index}`, className: "sidebar-current-document", "aria-current": "page" }, shortcut.label)), e("strong", { className: "tree-section-heading" }, e(ChevronRight, SHELL_ICON_PROPS), message("shell.reading"))),
      e("div", { className: "sidebar-footer" }, e("span", { className: "saved-indicator" }, e("i", { "aria-hidden": "true" }), options.savedStatus ?? message("shell.saved"))),
    ),
    e("header", { className: `desktop-topbar${options.topbarClassName ? ` ${options.topbarClassName}` : ""}` }, options.topbarContent ?? e("button", { type: "button", className: "topbar-search", "data-action": options.searchActionId ?? "navigate-search", onClick: options.onSearch, disabled: !options.onSearch, "aria-label": message("shell.searchPrompt"), title: options.onSearch ? message("shell.searchPrompt") : message("shell.searchUnavailable") }, e(Search, SHELL_ICON_PROPS), e("span", null, message("shell.searchPlaceholder")))),
    outlet,
    e("div", { className: "workspace-global-host", "data-workspace-global-host": "mounted" }, options.globalLayer),
  );
}
