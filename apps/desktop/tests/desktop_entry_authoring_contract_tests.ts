import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("desktop entry composes explicit authoring surface controller timer and generation guard", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");

  assert.match(source, /"Home"\s*\|\s*"Navigator"\s*\|\s*"Authoring"/);
  assert.match(source, /createDesktopDocumentAuthoringController/);
  assert.match(source, /createDesktopRevisionMetadataGenerator/);
  assert.match(source, /createNewDocument/);
  assert.match(source, /desktopClient\s*\.\s*createDocument/);
  assert.match(source, /제목 없는 문서/);
  assert.doesNotMatch(source, /Untitled Document/);
  assert.match(source, /notes\/\$\{documentId\}\.md/);
  assert.match(source, /createDocument\([\s\S]*?authoringController\.open\([\s\S]*?setAuthoringSnapshot\(snapshot\)/);
  assert.match(source, /setTimeout\([^,]+,\s*800\)/s);
  assert.match(source, /authoringGeneration/);
  assert.match(source, /createDesktopDocumentAuthoringWorkbenchElement/);
  assert.doesNotMatch(source, /desktopClient\s*\.\s*renameDocument/);
  assert.doesNotMatch(source, /titleFocusDocumentId|setTitleEditState/);
  assert.match(source, /restoreDocumentVersion[\s\S]*?restoredVersionId[\s\S]*?expectedVersionId/);
  assert.match(source, /restoreDocumentVersion[\s\S]*?listDocumentHistory/);
  assert.doesNotMatch(source, /summary:\s*`Restore \$\{preview\.targetVersionId\}`/);
  assert.doesNotMatch(source, /Date\.now|Math\.random|localStorage|sessionStorage|process\.env/);
});
