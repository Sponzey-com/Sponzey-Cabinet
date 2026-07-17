import assert from "node:assert/strict";
import test from "node:test";

import {
  PackagedUiSmokeState,
  activateButtonByKeyboard,
  dispatchMacSaveShortcut,
  nearestRankP95,
  runPackagedUiSmoke,
  transitionPackagedUiSmoke,
  waitForSelector,
} from "../src/packaged_ui_smoke.ts";
import {
  CODEMIRROR_REPLACE_DOCUMENT_EVENT,
  requestCodeMirrorDocumentReplacement,
} from "../src/codemirror_document_editor.ts";

test("packaged document input crosses the CodeMirror transaction event boundary", () => {
  const target = new EventTarget();
  let received: unknown;
  target.addEventListener(CODEMIRROR_REPLACE_DOCUMENT_EVENT, (event) => {
    received = (event as Event & { detail?: unknown }).detail;
  });

  const dispatched = requestCodeMirrorDocumentReplacement(
    target,
    "# Durable packaged content",
    (detail) => Object.assign(new Event(CODEMIRROR_REPLACE_DOCUMENT_EVENT), { detail }),
  );

  assert.equal(dispatched, true);
  assert.deepEqual(received, { body: "# Durable packaged content" });
});

test("packaged keyboard helpers require focus and emit the macOS save shortcut", () => {
  const events: Array<{ readonly key: string; readonly metaKey: boolean }> = [];
  let focused = false;
  let clicked = 0;
  const createEvent = (key: string, options: KeyboardEventInit) => Object.assign(
    new Event("keydown", { bubbles: options.bubbles, cancelable: options.cancelable }),
    { key, metaKey: options.metaKey ?? false },
  );
  const button = {
    disabled: false,
    focus() { focused = true; },
    click() { clicked += 1; },
    dispatchEvent(event: Event) {
      events.push({ key: (event as KeyboardEvent).key, metaKey: (event as KeyboardEvent).metaKey });
      return true;
    },
  };
  activateButtonByKeyboard(button, () => focused, createEvent);
  const shortcutTarget = new EventTarget();
  shortcutTarget.addEventListener("keydown", (event) => {
    events.push({ key: (event as KeyboardEvent).key, metaKey: (event as KeyboardEvent).metaKey });
  });
  dispatchMacSaveShortcut(shortcutTarget, createEvent);
  assert.equal(clicked, 1);
  assert.deepEqual(events, [
    { key: "Enter", metaKey: false },
    { key: "s", metaKey: true },
  ]);

  assert.throws(
    () => activateButtonByKeyboard(button, () => false, createEvent),
    /PACKAGED_UI_FOCUS_FAILED/,
  );
});

test("document reopen waits for the asynchronously refreshed recent-document action", async () => {
  let attempts = 0;
  await waitForSelector(
    {
      querySelector(selector) {
        assert.equal(selector, '[data-action="open-recent-document"][data-document-id="doc-1"]');
        attempts += 1;
        return attempts >= 3 ? ({} as Element) : null;
      },
    },
    '[data-action="open-recent-document"][data-document-id="doc-1"]',
    async () => undefined,
    100,
  );
  assert.equal(attempts, 3);
});

test("packaged smoke follows the explicit terminal state sequence", () => {
  let state = PackagedUiSmokeState.Booting;
  state = transitionPackagedUiSmoke(state, "home_ready");
  assert.equal(state, PackagedUiSmokeState.HomeReady);
  state = transitionPackagedUiSmoke(state, "document_saved");
  assert.equal(state, PackagedUiSmokeState.DocumentSaved);
  state = transitionPackagedUiSmoke(state, "document_reopened");
  assert.equal(state, PackagedUiSmokeState.DocumentReopened);
  state = transitionPackagedUiSmoke(state, "document_version_verified");
  assert.equal(state, PackagedUiSmokeState.DocumentVersionWorkflowVerified);
  state = transitionPackagedUiSmoke(state, "graph_actions_verified");
  assert.equal(state, PackagedUiSmokeState.GraphActionsVerified);
  state = transitionPackagedUiSmoke(state, "canvas_mutations_verified");
  assert.equal(state, PackagedUiSmokeState.CanvasMutationsVerified);
  state = transitionPackagedUiSmoke(state, "asset_actions_verified");
  assert.equal(state, PackagedUiSmokeState.AssetActionsVerified);
  state = transitionPackagedUiSmoke(state, "document_attachment_verified");
  assert.equal(state, PackagedUiSmokeState.DocumentAttachmentWorkflowVerified);
  state = transitionPackagedUiSmoke(state, "cross_surface_verified");
  assert.equal(state, PackagedUiSmokeState.CrossSurfaceVerified);
  state = transitionPackagedUiSmoke(state, "backup_restore_verified");
  assert.equal(state, PackagedUiSmokeState.BackupRestoreVerified);
  state = transitionPackagedUiSmoke(state, "canvas_lifecycle_verified");
  assert.equal(state, PackagedUiSmokeState.CanvasLifecycleVerified);
  state = transitionPackagedUiSmoke(state, "canvas_recovery_verified");
  assert.equal(state, PackagedUiSmokeState.CanvasRecoveryVerified);
  state = transitionPackagedUiSmoke(state, "route_ready");
  assert.equal(state, PackagedUiSmokeState.RoutesMeasured);
  state = transitionPackagedUiSmoke(state, "samples_ready");
  assert.equal(state, PackagedUiSmokeState.NativeReadsMeasured);
  state = transitionPackagedUiSmoke(state, "report_ready");
  assert.equal(state, PackagedUiSmokeState.Reporting);
  state = transitionPackagedUiSmoke(state, "reported");
  assert.equal(state, PackagedUiSmokeState.Passed);
});

test("invalid transitions and failures terminate without hidden flags", () => {
  assert.equal(
    transitionPackagedUiSmoke(PackagedUiSmokeState.Booting, "samples_ready"),
    PackagedUiSmokeState.Failed,
  );
  assert.equal(
    transitionPackagedUiSmoke(PackagedUiSmokeState.HomeReady, "failed"),
    PackagedUiSmokeState.Failed,
  );
});

test("nearest-rank p95 is deterministic for the bounded sample set", () => {
  const samples = Array.from({ length: 200 }, (_, index) => index + 1);
  assert.equal(nearestRankP95(samples), 190);
  assert.equal(nearestRankP95([]), 0);
});

test("normal desktop mode performs no UI automation", async () => {
  let queried = false;
  const state = await runPackagedUiSmoke({
    async invoke(command) {
      assert.equal(command, "get_packaged_ui_smoke_mode");
      return { enabled: false };
    },
    document: {
      querySelector() {
        queried = true;
        return null;
      },
    },
  });
  assert.equal(state, PackagedUiSmokeState.Disabled);
  assert.equal(queried, false);
});
