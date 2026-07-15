import assert from "node:assert/strict";
import test from "node:test";

import { createDesktopCanvasViewportDebouncer } from "../src/desktop_canvas_viewport_debouncer.ts";

test("Canvas viewport debouncer dispatches only the latest scheduled draft", () => {
  const pending = new Map<number, () => void>();
  const cancelled: number[] = [];
  let next = 0;
  const debouncer = createDesktopCanvasViewportDebouncer({
    schedule(_delayMs, callback) { const id = ++next; pending.set(id, callback); return id; },
    cancel(handle) { cancelled.push(handle as number); pending.delete(handle as number); },
  }, 250);
  const dispatched: number[] = [];
  debouncer.queue({ kind: "update_viewport", centerX: 100, centerY: 0, zoomPercent: 100 }, (draft) => dispatched.push(draft.centerX));
  debouncer.queue({ kind: "update_viewport", centerX: 200, centerY: 0, zoomPercent: 100 }, (draft) => dispatched.push(draft.centerX));
  assert.deepEqual(cancelled, [1]);
  assert.equal(debouncer.state(), "Scheduled");
  pending.get(2)?.();
  assert.deepEqual(dispatched, [200]);
  assert.equal(debouncer.state(), "Idle");
});

test("Canvas viewport debouncer cancels pending work on dispose", () => {
  let callback: (() => void) | undefined;
  const dispatched: number[] = [];
  const debouncer = createDesktopCanvasViewportDebouncer({
    schedule(_delayMs, value) { callback = value; return 1; },
    cancel() {},
  }, 250);
  debouncer.queue({ kind: "update_viewport", centerX: 100, centerY: 0, zoomPercent: 100 }, (draft) => dispatched.push(draft.centerX));
  debouncer.dispose();
  callback?.();
  assert.equal(debouncer.state(), "Disposed");
  assert.deepEqual(dispatched, []);
});
