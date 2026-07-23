import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("desktop entry links an isolated document asset library through durable readback", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");
  assert.match(source, /documentAssetLibraryState/);
  assert.match(source, /requestDocumentAssetLibraryOpen/);
  assert.match(source, /loadDesktopWorkspaceAssets/);
  assert.match(source, /linkDesktopSelectedAsset/);
  assert.match(source, /completeDocumentAssetLibraryLink/);
  assert.match(source, /requestDocumentAssetLibraryMore/);
  assert.match(source, /onAssetLibraryLoadMore/);
  assert.doesNotMatch(source, /onOpenLibrary:\s*\(\)\s*=>\s*requestDesktopRoute/);
});
