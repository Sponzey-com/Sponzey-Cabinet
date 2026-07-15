import assert from "node:assert/strict";
import test from "node:test";

import { createFocusRestoringModalAction } from "../src/modal_focus_restoration.ts";

test("modal action runs transition before deferred focus restoration", () => {
  const order: string[] = [];
  const target = { isConnected: true, focus() { order.push("focus"); } };
  const action = createFocusRestoringModalAction(() => order.push("action"), {
    activeElement: () => target,
    defer: (callback) => { order.push("defer"); callback(); },
  });
  action();
  assert.deepEqual(order, ["action", "defer", "focus"]);
});

test("modal action skips disconnected and absent focus targets", () => {
  let focused = 0;
  const disconnected = createFocusRestoringModalAction(() => {}, {
    activeElement: () => ({ isConnected: false, focus() { focused += 1; } }),
    defer: (callback) => callback(),
  });
  const absent = createFocusRestoringModalAction(() => {}, {
    activeElement: () => undefined,
    defer: (callback) => callback(),
  });
  disconnected();
  absent();
  assert.equal(focused, 0);
});
