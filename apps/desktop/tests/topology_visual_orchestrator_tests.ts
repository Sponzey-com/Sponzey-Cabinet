import assert from "node:assert/strict";
import test from "node:test";

import {
  TopologyVisualOrchestrator,
  createTopologyRendererModel,
} from "../src/topology_visual_orchestrator.ts";
import type {
  TopologyLayoutAdapter,
  TopologyLayoutRequest,
  TopologyLayoutResult,
  TopologyRendererAdapter,
  TopologyRendererEvent,
  TopologyRendererModel,
} from "../src/topology_renderer_port.ts";

const MODEL = createTopologyRendererModel(
  [
    { identity: "doc-a", kind: "document", label: "첫 문서", canNavigate: true },
    { identity: "asset-b", kind: "attachment", label: "설계 자료", canNavigate: false },
  ],
  [{ id: "edge-a-b", sourceId: "doc-a", targetId: "asset-b", kind: "attachment_reference" }],
  "doc-a",
  "doc-a",
);

test("topology renderer model derives selected neighbors without hiding unrelated nodes", () => {
  const model = createTopologyRendererModel(
    [
      { identity: "selected", kind: "document", label: "선택", canNavigate: true },
      { identity: "neighbor", kind: "document", label: "이웃", canNavigate: true },
      { identity: "unrelated", kind: "document", label: "무관", canNavigate: true },
    ],
    [{ id: "edge", sourceId: "selected", targetId: "neighbor", kind: "document_link" }],
    "selected",
  );
  assert.deepEqual(model.nodes.map((node) => node.emphasis), ["primary", "neighbor", "muted"]);
  assert.equal(model.edges[0]?.emphasis, "primary");
  const normal = createTopologyRendererModel(
    model.nodes.map((node) => ({ identity: node.key, kind: node.kind, label: node.title, canNavigate: node.canNavigate })),
    [{ id: "edge", sourceId: "selected", targetId: "neighbor", kind: "document_link" }],
  );
  assert.ok(normal.nodes.every((node) => node.emphasis === "normal"));
  assert.equal(normal.edges[0]?.emphasis, "normal");
});

test("topology orchestrator maps display data and applies only current layout generation", async () => {
  assert.equal(MODEL.nodes[0]?.center, true);
  assert.equal(MODEL.nodes[0]?.selected, true);
  assert.equal(MODEL.nodes[0]?.title, "첫 문서");
  const renderer = new FakeRenderer();
  const layout = new FakeLayout();
  const events: string[] = [];
  const orchestrator = new TopologyVisualOrchestrator(renderer, layout, {
    onNodeSelected: (key) => events.push(`select:${key}`),
    onNodeActivated: (key) => events.push(`activate:${key}`),
    onFailure: (code) => events.push(`failure:${code}`),
  });

  await orchestrator.mount({}, MODEL, false);
  const first = layout.requests[0]!;
  orchestrator.update({ ...MODEL, nodes: MODEL.nodes.slice(0, 1), edges: [] }, false);
  const second = layout.requests[1]!;
  layout.resolve(first.generation, new Map([["doc-a", { x: 1, y: 1 }], ["asset-b", { x: 2, y: 2 }]]));
  await Promise.resolve();
  assert.equal(renderer.models.at(-1)?.nodes[0]?.position, undefined);
  layout.resolve(second.generation, new Map([["doc-a", { x: 7, y: 9 }]]));
  await Promise.resolve();

  assert.deepEqual(renderer.models.at(-1)?.nodes[0]?.position, { x: 7, y: 9 });
  renderer.emit({ type: "NodeSelected", key: "doc-a" });
  renderer.emit({ type: "NodeActivated", key: "doc-a" });
  assert.deepEqual(events, ["select:doc-a", "activate:doc-a"]);
});

test("topology orchestrator owns camera resize and deterministic disposal", async () => {
  const renderer = new FakeRenderer();
  const layout = new FakeLayout();
  const cameras: { x: number; y: number; ratio: number }[] = [];
  const orchestrator = new TopologyVisualOrchestrator(renderer, layout, {
    onNodeSelected() {}, onNodeActivated() {}, onFailure() {},
    onCameraChanged: (camera) => cameras.push(camera),
  });
  await orchestrator.mount({}, MODEL, true, { x: 0.25, y: 0.75, ratio: 0.8 });
  renderer.emit({ type: "CameraChanged", camera: { x: 0.3, y: 0.7, ratio: 0.5 } });
  renderer.emit({ type: "CameraChanged", camera: { x: Number.NaN, y: 0.7, ratio: 0.5 } });
  orchestrator.resize({ width: 800, height: 500, pixelRatio: 2 });
  orchestrator.setZoomPercent(150);
  orchestrator.fit();
  orchestrator.dispose();
  orchestrator.dispose();

  assert.deepEqual(renderer.calls, ["mount", "camera:0.8", "resize:800:500:2", "camera:0.6666666666666666", "fit", "dispose"]);
  assert.deepEqual(cameras, [{ x: 0.3, y: 0.7, ratio: 0.5 }]);
  assert.equal(layout.disposeCount, 1);
  assert.equal(renderer.listenerCount, 0);
});

test("topology orchestrator pauses updates and resumes only the latest model generation", async () => {
  const renderer = new FakeRenderer();
  const layout = new FakeLayout();
  const orchestrator = new TopologyVisualOrchestrator(renderer, layout, {
    onNodeSelected() {}, onNodeActivated() {}, onFailure() {},
  });
  await orchestrator.mount({}, MODEL, false);
  const firstGeneration = layout.requests[0]!.generation;
  orchestrator.pauseLayout();
  const latest = { ...MODEL, nodes: MODEL.nodes.slice(0, 1), edges: [] };
  orchestrator.update(latest, false);

  assert.equal(orchestrator.isLayoutPaused(), true);
  assert.deepEqual(layout.cancelled, [firstGeneration]);
  assert.equal(layout.requests.length, 1);
  layout.resolve(firstGeneration, new Map([["doc-a", { x: 2, y: 3 }], ["asset-b", { x: 4, y: 5 }]]));
  await Promise.resolve();
  assert.equal(renderer.models.at(-1)?.nodes[0]?.position, undefined);

  orchestrator.resumeLayout(false);
  assert.equal(orchestrator.isLayoutPaused(), false);
  assert.equal(layout.requests.length, 2);
  assert.equal(layout.requests[1]?.nodes.length, 1);
});

test("topology orchestrator reset removes temporary positions starts a new layout and fits", async () => {
  const renderer = new FakeRenderer();
  const layout = new FakeLayout();
  const positioned = {
    ...MODEL,
    nodes: MODEL.nodes.map((node, index) => ({ ...node, position: { x: index + 1, y: index + 2 } })),
  };
  const orchestrator = new TopologyVisualOrchestrator(renderer, layout, {
    onNodeSelected() {}, onNodeActivated() {}, onFailure() {},
  });
  await orchestrator.mount({}, positioned, false);
  orchestrator.resetLayout(false);

  assert.equal(layout.requests.length, 2);
  assert.ok(renderer.models.at(-1)?.nodes.every((node) => node.position === undefined));
  assert.equal(renderer.calls.at(-1), "fit");
  orchestrator.dispose();
  assert.throws(() => orchestrator.pauseLayout(), /TOPOLOGY_VISUAL_DISPOSED/);
  assert.throws(() => orchestrator.resumeLayout(false), /TOPOLOGY_VISUAL_DISPOSED/);
  assert.throws(() => orchestrator.resetLayout(false), /TOPOLOGY_VISUAL_DISPOSED/);
});

test("topology orchestrator pins dragged position and preserves it across later layout results", async () => {
  const renderer = new FakeRenderer();
  const layout = new FakeLayout();
  const orchestrator = new TopologyVisualOrchestrator(renderer, layout, {
    onNodeSelected() {}, onNodeActivated() {}, onFailure() {},
  });
  await orchestrator.mount({}, MODEL, false);
  renderer.emit({ type: "NodePositionChanged", key: "doc-a", position: { x: 12, y: 34 } });

  const pinned = renderer.models.at(-1)?.nodes.find((node) => node.key === "doc-a");
  assert.deepEqual(pinned?.position, { x: 12, y: 34 });
  assert.equal(pinned?.pinned, true);
  assert.equal(orchestrator.isLayoutPaused(), true);

  orchestrator.resumeLayout(false);
  const generation = layout.requests.at(-1)!.generation;
  layout.resolve(generation, new Map([["doc-a", { x: 99, y: 99 }], ["asset-b", { x: 5, y: 6 }]]));
  await Promise.resolve();
  const settled = renderer.models.at(-1)!;
  assert.deepEqual(settled.nodes.find((node) => node.key === "doc-a")?.position, { x: 12, y: 34 });
  assert.deepEqual(settled.nodes.find((node) => node.key === "asset-b")?.position, { x: 5, y: 6 });

  orchestrator.resetLayout(false);
  assert.ok(renderer.models.at(-1)?.nodes.every((node) => node.position === undefined && node.pinned !== true));
});

class FakeRenderer implements TopologyRendererAdapter<object> {
  readonly models: TopologyRendererModel[] = [];
  readonly calls: string[] = [];
  private listener?: (event: TopologyRendererEvent) => void;
  listenerCount = 0;

  async mount(_host: object, model: TopologyRendererModel): Promise<void> { this.calls.push("mount"); this.models.push(model); }
  update(model: TopologyRendererModel): void { this.models.push(model); }
  resize(viewport: { width: number; height: number; pixelRatio: number }): void { this.calls.push(`resize:${viewport.width}:${viewport.height}:${viewport.pixelRatio}`); }
  focusNode(): void {}
  fit(): void { this.calls.push("fit"); }
  setCamera(camera: { x: number; y: number; ratio: number }): void { this.calls.push(`camera:${camera.ratio}`); }
  subscribe(listener: (event: TopologyRendererEvent) => void): () => void { this.listener = listener; this.listenerCount = 1; return () => { this.listener = undefined; this.listenerCount = 0; }; }
  dispose(): void { this.calls.push("dispose"); }
  emit(event: TopologyRendererEvent): void { this.listener?.(event); }
}

class FakeLayout implements TopologyLayoutAdapter {
  readonly requests: TopologyLayoutRequest[] = [];
  readonly cancelled: number[] = [];
  private readonly pending = new Map<number, (result: TopologyLayoutResult) => void>();
  disposeCount = 0;

  layout(request: TopologyLayoutRequest): Promise<TopologyLayoutResult> {
    this.requests.push(request);
    return new Promise((resolve) => this.pending.set(request.generation, resolve));
  }
  cancel(generation: number): void { this.cancelled.push(generation); }
  dispose(): void { this.disposeCount += 1; }
  resolve(generation: number, positions: ReadonlyMap<string, { readonly x: number; readonly y: number }>): void {
    this.pending.get(generation)?.({ generation, positions });
  }
}
