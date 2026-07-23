import assert from "node:assert/strict";
import test from "node:test";

import {
  changeCanvasTextEditDraft,
  closeCanvasTextEditDialog,
  createCanvasTextEditIntent,
  createClosedCanvasTextEditDialog,
  openCanvasTextEditDialog,
} from "../src/canvas_text_edit_dialog.ts";

test("text edit dialog opens only an editable text card and creates a normalized intent", () => {
  const closed = createClosedCanvasTextEditDialog();
  const open = openCanvasTextEditDialog(closed, "note-1", "Original", true);
  assert.equal(open.kind, "Editing");

  const changed = changeCanvasTextEditDraft(open, "  Edited note  ");
  assert.deepEqual(createCanvasTextEditIntent(changed), { nodeId: "note-1", text: "Edited note" });
  assert.deepEqual(closeCanvasTextEditDialog(changed), { kind: "Closed" });
});

test("text edit dialog rejects archived, empty, unchanged and oversized intents", () => {
  const closed = createClosedCanvasTextEditDialog();
  assert.equal(openCanvasTextEditDialog(closed, "note-1", "Original", false).kind, "Closed");
  const open = openCanvasTextEditDialog(closed, "note-1", "Original", true);

  assert.equal(createCanvasTextEditIntent(open), undefined);
  assert.equal(createCanvasTextEditIntent(changeCanvasTextEditDraft(open, "  ")), undefined);
  assert.equal(createCanvasTextEditIntent(changeCanvasTextEditDraft(open, "x".repeat(20_001))), undefined);
});
