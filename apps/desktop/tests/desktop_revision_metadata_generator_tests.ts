import assert from "node:assert/strict";
import test from "node:test";

import { createDesktopRevisionMetadataGenerator } from "../src/desktop_revision_metadata_generator.ts";

test("desktop revision metadata generator uses only the injected id source", () => {
  const ids = ["id-one", "id-two"];
  const generator = createDesktopRevisionMetadataGenerator(() => ids.shift() ?? "exhausted");

  assert.deepEqual(generator.next("doc-1", 3), {
    versionId: "version-id-one",
    snapshotRef: "snapshot-id-one",
  });
  assert.deepEqual(generator.next("doc-1", 4), {
    versionId: "version-id-two",
    snapshotRef: "snapshot-id-two",
  });
});
