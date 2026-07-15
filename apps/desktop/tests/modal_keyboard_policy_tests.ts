import assert from "node:assert/strict";
import test from "node:test";

import { handleModalKeyboard } from "../src/modal_keyboard_policy.ts";

test("modal keyboard dismisses once on Escape", () => {
  let dismissed = 0;
  let prevented = 0;
  handleModalKeyboard(event("Escape"), () => { dismissed += 1; });
  assert.equal(dismissed, 1);
  assert.equal(prevented, 1);

  function event(key: string) {
    return { key, shiftKey: false, currentTarget: container([]), target: undefined, preventDefault() { prevented += 1; } };
  }
});

test("modal keyboard wraps Tab and Shift+Tab at focus boundaries", () => {
  const first = focusable();
  const last = focusable();
  const root = container([first, last]);
  let prevented = 0;
  handleModalKeyboard({ key: "Tab", shiftKey: false, currentTarget: root, target: last, preventDefault() { prevented += 1; } }, () => {});
  handleModalKeyboard({ key: "Tab", shiftKey: true, currentTarget: root, target: first, preventDefault() { prevented += 1; } }, () => {});
  assert.equal(first.focusCount, 1);
  assert.equal(last.focusCount, 1);
  assert.equal(prevented, 2);
});

test("modal keyboard leaves middle Tab and empty dialog to browser defaults", () => {
  const first = focusable();
  const middle = focusable();
  const last = focusable();
  let prevented = 0;
  handleModalKeyboard({ key: "Tab", shiftKey: false, currentTarget: container([first, middle, last]), target: middle, preventDefault() { prevented += 1; } }, () => {});
  handleModalKeyboard({ key: "Tab", shiftKey: false, currentTarget: container([]), target: undefined, preventDefault() { prevented += 1; } }, () => {});
  assert.equal(prevented, 0);
});

function focusable() {
  return { disabled: false, focusCount: 0, focus() { this.focusCount += 1; } };
}
function container(values: ReturnType<typeof focusable>[]) {
  return { querySelectorAll() { return values; } };
}
