import assert from "node:assert/strict";
import test from "node:test";
import { createDesktopGraphCameraSaveScheduler } from "../src/desktop_graph_camera_save_scheduler.ts";

test("camera save scheduler coalesces the latest value and blocks dispatch after dispose", () => {
  let scheduled: (() => void) | undefined;
  let cancelCount = 0;
  const saved: number[] = [];
  const scheduler = createDesktopGraphCameraSaveScheduler({
    delayMs: 120,
    schedule: (run) => { scheduled = run; return 1; },
    cancel: () => { cancelCount += 1; },
  });
  scheduler.queue({ centerX: 0, centerY: 0, zoomPercent: 100 }, (camera) => saved.push(camera.zoomPercent));
  scheduler.queue({ centerX: 1, centerY: 2, zoomPercent: 150 }, (camera) => saved.push(camera.zoomPercent));
  assert.equal(cancelCount, 1);
  scheduled?.();
  assert.deepEqual(saved, [150]);
  scheduler.queue({ centerX: 3, centerY: 4, zoomPercent: 175 }, (camera) => saved.push(camera.zoomPercent));
  scheduler.dispose();
  scheduled?.();
  assert.deepEqual(saved, [150]);
  assert.equal(scheduler.state(), "Disposed");
});
