import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("desktop entry subscribes and cleans native asset drop events behind document attachment guards", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");

  assert.match(source, /subscribeTauriAssetDrop/);
  assert.match(source, /getGlobalTauriEventListen/);
  assert.match(source, /documentInspectorStateRef\.current\.tab === "attachments"/);
  assert.match(source, /visibleRoute\(routeStateRef\.current\)\.kind === "Document"/);
  assert.match(source, /return \(\) => \{[\s\S]*unsubscribe/s);
  assert.doesNotMatch(source, /FileReader|readAsArrayBuffer/);
});
