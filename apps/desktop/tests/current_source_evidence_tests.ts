import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { access, readdir, readFile } from "node:fs/promises";
import test from "node:test";

import { CORE_UI_ACTION_MANIFEST } from "../src/core_ui_action_manifest.ts";
import { CURRENT_CONDITIONAL_UI_ACTION_FAMILIES } from "../src/current_conditional_ui_action_families.ts";
import { EXPLORATION_UI_ACTION_CONTRACTS } from "../src/exploration_ui_action_manifest.ts";
import {
  auditConditionalUiActionFamilies,
  createUnifiedUiActionCatalog,
  extractConditionalUiActionExpressions,
} from "../src/ui_action_inventory.ts";
import {
  createSourceFingerprint,
  SourceFingerprintError,
} from "../src/source_fingerprint_contract.ts";

test("every current conditional or variable data action has classified bounded branches", async () => {
  const sourceDirectory = new URL("../src/", import.meta.url);
  const sourceNames = (await readdir(sourceDirectory))
    .filter((name) => name.startsWith("react_") && name.endsWith(".ts"))
    .sort();
  const occurrences = (await Promise.all(sourceNames.map(async (name) =>
    extractConditionalUiActionExpressions(name, await readFile(new URL(name, sourceDirectory), "utf8"))
  ))).flat();
  const catalog = createUnifiedUiActionCatalog([
    { source: "core", contracts: CORE_UI_ACTION_MANIFEST },
    { source: "exploration", contracts: EXPLORATION_UI_ACTION_CONTRACTS },
  ]);

  assert.deepEqual(auditConditionalUiActionFamilies(
    catalog.contracts,
    occurrences,
    CURRENT_CONDITIONAL_UI_ACTION_FAMILIES,
  ), []);
  assert.equal(occurrences.length, 11);
});

test("conditional action audit fails for missing and unclassified branches", () => {
  const occurrences = extractConditionalUiActionExpressions(
    "fixture.ts",
    'e("button", { "data-action": ready ? "retry-a" : "retry-b", onClick })',
  );
  assert.equal(auditConditionalUiActionFamilies([], occurrences, [])[0]?.code, "CONDITIONAL_ACTION_FAMILY_MISSING");
  assert.equal(auditConditionalUiActionFamilies(
    [],
    occurrences,
    [{ source: "fixture.ts", expression: 'ready ? "retry-a" : "retry-b"', actionIds: ["retry-a", "retry-b"] }],
  )[0]?.code, "CONDITIONAL_ACTION_UNCLASSIFIED");
});

test("every connected action contract references an existing repository-relative interaction test", async () => {
  const catalog = createUnifiedUiActionCatalog([
    { source: "core", contracts: CORE_UI_ACTION_MANIFEST },
    { source: "exploration", contracts: EXPLORATION_UI_ACTION_CONTRACTS },
  ]);
  const paths = [...new Set(catalog.contracts
    .filter((contract) => contract.availability === "connected")
    .map((contract) => contract.interactionTest))];
  for (const path of paths) {
    assert.match(path, /^apps\/desktop\/tests\/[a-z0-9_]+_tests\.ts$/);
    assert.doesNotMatch(path, /\.\.|^\//);
    await access(new URL(`../../../${path}`, import.meta.url));
  }
});

test("source fingerprint is order-independent and changes with path or content", () => {
  const hash = (value: string) => createHash("sha256").update(value).digest("hex");
  const first = createSourceFingerprint([
    { path: "b.ts", content: "second" },
    { path: "a.ts", content: "first" },
  ], hash);
  const reordered = createSourceFingerprint([
    { path: "a.ts", content: "first" },
    { path: "b.ts", content: "second" },
  ], hash);
  const changed = createSourceFingerprint([
    { path: "a.ts", content: "changed" },
    { path: "b.ts", content: "second" },
  ], hash);

  assert.deepEqual(first, reordered);
  assert.match(first.digest, /^[a-f0-9]{64}$/);
  assert.equal(first.fileCount, 2);
  assert.notEqual(first.digest, changed.digest);
});

test("source fingerprint rejects empty duplicate and invalid hash results", () => {
  assert.throws(() => createSourceFingerprint([], () => "hash"), fingerprintError("SOURCE_SET_EMPTY"));
  assert.throws(() => createSourceFingerprint([
    { path: "same.ts", content: "a" },
    { path: "same.ts", content: "b" },
  ], () => "hash"), fingerprintError("SOURCE_PATH_DUPLICATE"));
  assert.throws(() => createSourceFingerprint([{ path: "a.ts", content: "a" }], () => " "), fingerprintError("SOURCE_HASH_INVALID"));
});

test("current presentation source produces a non-empty reproducible fingerprint", async () => {
  const sourceDirectory = new URL("../src/", import.meta.url);
  const names = (await readdir(sourceDirectory))
    .filter((name) => name.endsWith(".ts"))
    .sort();
  const entries = await Promise.all(names.map(async (name) => ({
    path: `apps/desktop/src/${name}`,
    content: await readFile(new URL(name, sourceDirectory), "utf8"),
  })));
  const fingerprint = createSourceFingerprint(
    entries,
    (value) => createHash("sha256").update(value).digest("hex"),
  );
  assert.equal(fingerprint.fileCount, names.length);
  assert.match(fingerprint.digest, /^[a-f0-9]{64}$/);
});

function fingerprintError(code: string): (error: unknown) => boolean {
  return (error) => error instanceof SourceFingerprintError && error.code === code;
}
