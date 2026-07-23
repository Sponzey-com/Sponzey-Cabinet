import assert from "node:assert/strict";
import test from "node:test";

import {
  ASSET_DRAG_STATE_EVENT,
  ASSET_DROP_SELECTION_EVENT,
  subscribeTauriAssetDrop,
} from "../src/tauri_asset_drop_transport.ts";

test("asset drop transport emits immutable path-free drag and selection DTOs", async () => {
  const handlers = new Map<string, (event: { readonly payload: unknown }) => void>();
  const removed: string[] = [];
  const states: unknown[] = [];
  const selections: unknown[] = [];
  const errors: string[] = [];
  const listen = async (event: string, handler: (event: { readonly payload: unknown }) => void) => {
    handlers.set(event, handler);
    return () => { removed.push(event); };
  };

  const unsubscribe = await subscribeTauriAssetDrop(listen, {
    onState: (state) => states.push(state),
    onSelection: (selection) => selections.push(selection),
    onError: (code) => errors.push(code),
  });
  handlers.get(ASSET_DRAG_STATE_EVENT)?.({ payload: { state: "entered", fileCount: 2 } });
  handlers.get(ASSET_DROP_SELECTION_EVENT)?.({ payload: {
    ok: true,
    data: {
      cancelled: false,
      files: [{ handle: "drop:1", fileName: "design.pdf", mediaType: "application/pdf", byteSize: 42 }],
    },
  } });
  handlers.get(ASSET_DRAG_STATE_EVENT)?.({ payload: { state: "left", fileCount: 0 } });
  unsubscribe();

  assert.deepEqual(states, [{ state: "entered", fileCount: 2 }, { state: "left", fileCount: 0 }]);
  assert.deepEqual(selections, [{ cancelled: false, files: [{ handle: "drop:1", fileName: "design.pdf", mediaType: "application/pdf", byteSize: 42 }] }]);
  assert.deepEqual(errors, []);
  assert.deepEqual(removed.sort(), [ASSET_DRAG_STATE_EVENT, ASSET_DROP_SELECTION_EVENT].sort());
  assert.equal(Object.isFrozen(states[0]), true);
  assert.equal(Object.isFrozen(selections[0]), true);
  assert.equal(JSON.stringify({ states, selections }).includes("path"), false);
});

test("asset drop transport rejects malformed and path-bearing payloads", async () => {
  const handlers = new Map<string, (event: { readonly payload: unknown }) => void>();
  const errors: string[] = [];
  const selections: unknown[] = [];
  const unsubscribe = await subscribeTauriAssetDrop(async (event, handler) => {
    handlers.set(event, handler);
    return () => {};
  }, {
    onState() {},
    onSelection: (selection) => selections.push(selection),
    onError: (code) => errors.push(code),
  });

  handlers.get(ASSET_DROP_SELECTION_EVENT)?.({ payload: {
    ok: true,
    data: { cancelled: false, files: [{ handle: "drop:2", fileName: "bad.pdf", mediaType: "application/pdf", byteSize: 1, path: "/private/bad.pdf" }] },
  } });
  handlers.get(ASSET_DRAG_STATE_EVENT)?.({ payload: { state: "entered", fileCount: -1, path: "/private" } });
  unsubscribe();

  assert.deepEqual(selections, []);
  assert.deepEqual(errors, ["ASSET_DROP_EVENT_INVALID", "ASSET_DRAG_STATE_INVALID"]);
});
