import assert from "node:assert/strict";
import test from "node:test";

import {
  createUnmountedTopologyRendererState,
  transitionTopologyRenderer,
  type TopologyRendererAdapter,
} from "../src/topology_renderer_port.ts";

test("topology renderer lifecycle reaches stable and ignores stale layout generations", () => {
  const unmounted = createUnmountedTopologyRendererState();
  const initializing = transitionTopologyRenderer(unmounted, { type: "MountRequested" });
  const ready = transitionTopologyRenderer(initializing, { type: "Initialized", liveResourceCount: 3 });
  const layingOut = transitionTopologyRenderer(ready, { type: "LayoutRequested", generation: 2 });
  const stale = transitionTopologyRenderer(layingOut, { type: "LayoutSettled", generation: 1 });
  const stable = transitionTopologyRenderer(stale, { type: "LayoutSettled", generation: 2 });

  assert.equal(initializing.phase, "Initializing");
  assert.equal(ready.phase, "Ready");
  assert.strictEqual(stale, layingOut);
  assert.equal(stable.phase, "Stable");
});

test("topology renderer pause resume and dispose require zero live resources", () => {
  const ready = transitionTopologyRenderer(
    transitionTopologyRenderer(createUnmountedTopologyRendererState(), { type: "MountRequested" }),
    { type: "Initialized", liveResourceCount: 4 },
  );
  const layingOut = transitionTopologyRenderer(ready, { type: "LayoutRequested", generation: 1 });
  const paused = transitionTopologyRenderer(layingOut, { type: "PauseRequested" });
  const resumed = transitionTopologyRenderer(paused, { type: "ResumeRequested" });
  const disposing = transitionTopologyRenderer(resumed, { type: "DisposeRequested" });

  assert.equal(paused.phase, "Paused");
  assert.equal(resumed.phase, "LayingOut");
  assert.throws(
    () => transitionTopologyRenderer(disposing, { type: "Disposed", liveResourceCount: 1 }),
    /TOPOLOGY_RENDERER_RESOURCES_ACTIVE/,
  );
  const unmounted = transitionTopologyRenderer(disposing, { type: "Disposed", liveResourceCount: 0 });
  assert.equal(unmounted.phase, "Unmounted");
  assert.strictEqual(
    transitionTopologyRenderer(unmounted, { type: "DisposeRequested" }),
    unmounted,
  );
});

test("topology renderer adapter contract can be replaced by a fake", async () => {
  const calls: string[] = [];
  const adapter: TopologyRendererAdapter<object> = {
    async mount() { calls.push("mount"); },
    update() { calls.push("update"); },
    resize() { calls.push("resize"); },
    focusNode() { calls.push("focus"); },
    fit() { calls.push("fit"); },
    setCamera() { calls.push("camera"); },
    subscribe() { calls.push("subscribe"); return () => calls.push("unsubscribe"); },
    dispose() { calls.push("dispose"); },
  };

  await adapter.mount({}, { nodes: [], edges: [] });
  adapter.update({ nodes: [], edges: [] });
  adapter.resize({ width: 800, height: 600, pixelRatio: 2 });
  adapter.focusNode("opaque-node-key");
  adapter.fit();
  adapter.setCamera({ x: 0, y: 0, ratio: 1 });
  const unsubscribe = adapter.subscribe(() => undefined);
  unsubscribe();
  adapter.dispose();

  assert.deepEqual(calls, ["mount", "update", "resize", "focus", "fit", "camera", "subscribe", "unsubscribe", "dispose"]);
});
