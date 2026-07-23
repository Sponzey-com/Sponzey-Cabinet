import type { WorkspaceShellRouteKind } from "./workspace_shell_contract.ts";

export type Penpot20260721Surface =
  | "design-direction"
  | "home"
  | "document"
  | "global-search"
  | "graph"
  | "canvas"
  | "attachments"
  | "backup"
  | "interaction-specs";

export interface Penpot20260721BoardContract {
  readonly id: string;
  readonly name: string;
  readonly surface: Penpot20260721Surface;
  readonly route?: WorkspaceShellRouteKind;
  readonly primarySidebarRoute: boolean;
  readonly role: string;
}

export interface Penpot20260721DesignContract {
  readonly pageId: string;
  readonly pageName: "20260721";
  readonly boards: readonly Penpot20260721BoardContract[];
  readonly primaryRoutes: readonly WorkspaceShellRouteKind[];
  readonly acceptanceRules: readonly string[];
}

export const PENPOT_20260721_PRIMARY_ROUTES = Object.freeze([
  "Home",
  "Document",
  "Graph",
  "Canvas",
  "Assets",
  "Backup",
] as const satisfies readonly WorkspaceShellRouteKind[]);

export const PENPOT_20260721_PALETTE = Object.freeze({
  cabinetTeal: "#0F8F83",
  knowledgeBlue: "#4E72E6",
  decisionAmber: "#D89A20",
  referenceRose: "#D85C7B",
  ink: "#18212B",
  canvas: "#F4F7F8",
});

export const PENPOT_20260721_TYPOGRAPHY = Object.freeze({
  fontFamily: "Noto Sans KR",
  bodyFontSizePx: 15,
  bodyLineHeight: 1.65,
  uiFontSizePx: 13,
  uiLineHeight: 1.5,
});

export const PENPOT_20260721_ACCEPTANCE_RULES = Object.freeze([
  "do_not_transform_penpot_layout",
  "search_only_topbar_or_command_k",
  "left_recent_documents_are_root_owned",
  "document_menu_resumes_last_working_document",
  "document_title_is_first_physical_line",
  "single_contextual_inspector",
  "canvas_toolbar_visible_during_auto_arrange",
  "minimum_hit_target_44px",
  "no_internal_identity_or_git_terms",
] as const);

export const PENPOT_20260721_FORBIDDEN_USER_EXPOSURE_PATTERNS = Object.freeze([
  /documentId/i,
  /versionId/i,
  /assetId/i,
  /\.md\b/i,
  /snapshot/i,
  /git|commit|branch|repository/i,
  /\/Users\//i,
] as const);

export const PENPOT_20260721_BOARDS = Object.freeze([
  board(
    "0b53b828-083e-80f9-8008-5beb8e88f2b4",
    "00 Design Direction / 20260721",
    "design-direction",
    undefined,
    false,
    "Shared product direction, visual language, workspace structure, and interaction rules.",
  ),
  board(
    "0b53b828-083e-80f9-8008-5beb8ff4f246",
    "01 Home / Unified Workspace",
    "home",
    "Home",
    true,
    "Home route layout, recent work, knowledge summary, and local workspace state.",
  ),
  board(
    "0b53b828-083e-80f9-8008-5beb91f114ee",
    "02 Document / Focused Authoring",
    "document",
    "Document",
    true,
    "Document route authoring, first-line title, editor, inspector, attachments, and history.",
  ),
  board(
    "0b53b828-083e-80f9-8008-5bebe6dedbbe",
    "03 Global Search / One Entry Point",
    "global-search",
    "Search",
    false,
    "Topbar and Command-K search overlay, result grouping, keyboard flow, and return context.",
  ),
  board(
    "0b53b828-083e-80f9-8008-5bebe89ba91b",
    "04 Knowledge Map / Explore in Context",
    "graph",
    "Graph",
    true,
    "Knowledge Map route, graph toolbar, projection rendering, legend, and selected document inspector.",
  ),
  board(
    "0b53b828-083e-80f9-8008-5bebec2ea5d1",
    "05 Canvas / Stable Tooling",
    "canvas",
    "Canvas",
    true,
    "Canvas route, fixed toolbar, auto arrange feedback, document/file/note cards, and viewport controls.",
  ),
  board(
    "0b53b828-083e-80f9-8008-5bec61a6bc5d",
    "06 Attachments / Library and Context",
    "attachments",
    "Assets",
    true,
    "Attachment library route, file add action, search, filters, preview, and linked document context.",
  ),
  board(
    "0b53b828-083e-80f9-8008-5bec65f6f145",
    "07 Backup and Restore / Guided Safety",
    "backup",
    "Backup",
    true,
    "Backup and Restore route, safety status, package history, restore preview, and guided actions.",
  ),
  board(
    "0b53b828-083e-80f9-8008-5bec699e2314",
    "08 Interaction Specs / Shared States",
    "interaction-specs",
    undefined,
    false,
    "Shared states, navigation rules, feedback copy, keyboard flow, responsive behavior, and accessibility.",
  ),
] as const);

export const PENPOT_20260721_CONTRACT: Penpot20260721DesignContract = Object.freeze({
  pageId: "0b53b828-083e-80f9-8008-5bea9a88ee0b",
  pageName: "20260721",
  boards: PENPOT_20260721_BOARDS,
  primaryRoutes: PENPOT_20260721_PRIMARY_ROUTES,
  acceptanceRules: PENPOT_20260721_ACCEPTANCE_RULES,
});

export function validatePenpot20260721DesignContract(
  contract: Penpot20260721DesignContract,
): readonly string[] {
  const issues: string[] = [];
  if (contract.pageId !== PENPOT_20260721_CONTRACT.pageId) issues.push("PENPOT_20260721_PAGE_ID_MISMATCH");
  if (contract.pageName !== "20260721") issues.push("PENPOT_20260721_PAGE_NAME_MISMATCH");
  if (contract.boards.length !== 9) issues.push("PENPOT_20260721_BOARD_COUNT_MISMATCH");
  if (new Set(contract.boards.map((board) => board.id)).size !== contract.boards.length) issues.push("PENPOT_20260721_BOARD_ID_DUPLICATE");
  if (new Set(contract.boards.map((board) => board.name)).size !== contract.boards.length) issues.push("PENPOT_20260721_BOARD_NAME_DUPLICATE");
  if (contract.primaryRoutes.includes("Search")) issues.push("PENPOT_20260721_SEARCH_PRIMARY_ROUTE_FORBIDDEN");
  for (const route of PENPOT_20260721_PRIMARY_ROUTES) {
    if (!contract.boards.some((board) => board.route === route && board.primarySidebarRoute)) {
      issues.push(`PENPOT_20260721_PRIMARY_ROUTE_BOARD_MISSING:${route}`);
    }
  }
  if (!contract.boards.some((board) => board.surface === "global-search" && board.route === "Search" && !board.primarySidebarRoute)) {
    issues.push("PENPOT_20260721_GLOBAL_SEARCH_BOARD_MISSING");
  }
  return Object.freeze(issues);
}

function board(
  id: string,
  name: string,
  surface: Penpot20260721Surface,
  route: WorkspaceShellRouteKind | undefined,
  primarySidebarRoute: boolean,
  role: string,
): Readonly<Penpot20260721BoardContract> {
  return Object.freeze({ id, name, surface, route, primarySidebarRoute, role });
}
