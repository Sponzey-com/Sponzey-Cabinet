import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  PENPOT_20260721_ACCEPTANCE_RULES,
  PENPOT_20260721_BOARDS,
  PENPOT_20260721_CONTRACT,
  PENPOT_20260721_FORBIDDEN_USER_EXPOSURE_PATTERNS,
  PENPOT_20260721_PALETTE,
  PENPOT_20260721_PRIMARY_ROUTES,
  validatePenpot20260721DesignContract,
} from "../src/penpot_20260721_design_contract.ts";

test("Penpot 20260721 design contract pins the connected page and all nine boards", () => {
  assert.equal(PENPOT_20260721_CONTRACT.pageName, "20260721");
  assert.equal(PENPOT_20260721_CONTRACT.pageId, "0b53b828-083e-80f9-8008-5bea9a88ee0b");
  assert.equal(PENPOT_20260721_BOARDS.length, 9);
  assert.deepEqual(PENPOT_20260721_BOARDS.map((board) => board.name), [
    "00 Design Direction / 20260721",
    "01 Home / Unified Workspace",
    "02 Document / Focused Authoring",
    "03 Global Search / One Entry Point",
    "04 Knowledge Map / Explore in Context",
    "05 Canvas / Stable Tooling",
    "06 Attachments / Library and Context",
    "07 Backup and Restore / Guided Safety",
    "08 Interaction Specs / Shared States",
  ]);
  assert.deepEqual(validatePenpot20260721DesignContract(PENPOT_20260721_CONTRACT), []);
});

test("Penpot 20260721 contract keeps Search out of primary sidebar navigation", () => {
  assert.deepEqual(PENPOT_20260721_PRIMARY_ROUTES, ["Home", "Document", "Graph", "Canvas", "Assets", "Backup"]);
  assert.equal(PENPOT_20260721_PRIMARY_ROUTES.includes("Search" as never), false);
  assert.equal(PENPOT_20260721_BOARDS.find((board) => board.surface === "global-search")?.route, "Search");
  assert.equal(PENPOT_20260721_BOARDS.find((board) => board.surface === "global-search")?.primarySidebarRoute, false);
});

test("Penpot 20260721 contract exposes exact design tokens and fidelity rules", () => {
  assert.deepEqual(PENPOT_20260721_PALETTE, {
    cabinetTeal: "#0F8F83",
    knowledgeBlue: "#4E72E6",
    decisionAmber: "#D89A20",
    referenceRose: "#D85C7B",
    ink: "#18212B",
    canvas: "#F4F7F8",
  });
  assert.ok(PENPOT_20260721_ACCEPTANCE_RULES.includes("do_not_transform_penpot_layout"));
  assert.ok(PENPOT_20260721_ACCEPTANCE_RULES.includes("search_only_topbar_or_command_k"));
  assert.ok(PENPOT_20260721_ACCEPTANCE_RULES.includes("left_recent_documents_are_root_owned"));
  assert.ok(PENPOT_20260721_ACCEPTANCE_RULES.includes("document_title_is_first_physical_line"));
});

test("Penpot 20260721 contract blocks internal implementation detail exposure", () => {
  const patterns = PENPOT_20260721_FORBIDDEN_USER_EXPOSURE_PATTERNS.map((pattern) => pattern.source);
  for (const required of ["documentId", "versionId", "assetId", "\\.md\\b", "snapshot", "git|commit|branch|repository"]) {
    assert.ok(patterns.some((pattern) => pattern.includes(required)), required);
  }
});

test("Penpot 20260721 validation rejects incomplete board inventories", () => {
  assert.deepEqual(validatePenpot20260721DesignContract({
    ...PENPOT_20260721_CONTRACT,
    boards: PENPOT_20260721_BOARDS.slice(0, 8),
  }), ["PENPOT_20260721_BOARD_COUNT_MISMATCH"]);
  assert.deepEqual(validatePenpot20260721DesignContract({
    ...PENPOT_20260721_CONTRACT,
    pageId: "wrong",
  }), ["PENPOT_20260721_PAGE_ID_MISMATCH"]);
});

test("production desktop sources do not import Penpot runtime APIs", async () => {
  const source = await readFile(new URL("../src/penpot_20260721_design_contract.ts", import.meta.url), "utf8");
  assert.doesNotMatch(source, /penpotUtils|penpot\\.root|export_shape|execute_code/);
});
