import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("successful attachment mutations synchronize the current document graph projection", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");

  assert.match(source, /from "\.\/desktop_projection_synchronizer\.ts"/);
  assert.match(source, /const synchronizeAssetProjection = useCallback/);
  assert.match(source, /result\.importState === "Completed"[\s\S]*await synchronizeAssetProjection\(result\)/);
  assert.match(source, /result\.mutationState === "Idle"[\s\S]*await synchronizeAssetProjection\(result\)/);
  assert.match(source, /completion\.documentAssets[\s\S]*await synchronizeAssetProjection\(completion\.documentAssets\)/);
  assert.match(source, /const synchronizeAssetProjection = useCallback[\s\S]*?synchronizeDocumentKnowledgeSurfaces\(/);
});

test("attachment projection synchronization is skipped without an exact document identity", async () => {
  const source = await readFile(new URL("../src/desktop_entry.ts", import.meta.url), "utf8");

  assert.match(source, /snapshot\.scope !== "Document" \|\| !snapshot\.documentId \|\| !projectionClient/);
  assert.match(source, /snapshot\.workspaceId,\s*snapshot\.documentId/);
});
