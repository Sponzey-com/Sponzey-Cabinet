import assert from "node:assert/strict";
import test from "node:test";

import {
  SigmaTopologyRendererAdapter,
  mapTopologyRendererModel,
  type SigmaTopologyRuntime,
  type SigmaTopologyRuntimeEvents,
  type SigmaTopologyRuntimeFactory,
  type SigmaTopologyRuntimeGraph,
} from "../src/sigma_topology_renderer_adapter.ts";
import type { TopologyRendererEvent, TopologyRendererModel } from "../src/topology_renderer_port.ts";

const MODEL: TopologyRendererModel = Object.freeze({
  nodes: Object.freeze([
    Object.freeze({ key: "node-a", title: "첫 문서", kind: "document", selected: true, center: true, canNavigate: true, emphasis: "primary" }),
    Object.freeze({ key: "asset-b", title: "설계 자료", kind: "attachment", selected: false, center: false, canNavigate: false, emphasis: "neighbor" }),
    Object.freeze({ key: "node-c", title: "무관 문서", kind: "document", selected: false, center: false, canNavigate: true, emphasis: "muted" }),
  ]),
  edges: Object.freeze([
    Object.freeze({ key: "edge-a-b", sourceKey: "node-a", targetKey: "asset-b", kind: "attachment_reference", emphasis: "primary" }),
  ]),
});

test("Sigma topology mapper creates safe bounded attributes and rejects invalid graph identity", () => {
  const mapped = mapTopologyRendererModel(MODEL);

  assert.equal(mapped.nodes.length, 3);
  assert.deepEqual(mapped.nodes.map((node) => node.key), ["node-a", "asset-b", "node-c"]);
  assert.deepEqual(mapped.nodes.map((node) => node.label), ["첫 문서", "설계 자료", "무관 문서"]);
  assert.ok(mapped.nodes.every((node) => Number.isFinite(node.x) && Number.isFinite(node.y)));
  assert.equal(mapped.nodes[0]?.highlighted, true);
  assert.equal(mapped.nodes[1]?.forceLabel, true);
  assert.equal(mapped.nodes[2]?.forceLabel, false);
  assert.equal(mapped.nodes[2]?.color, "#c8ced3");
  assert.equal(mapped.edges[0]?.zIndex, 3);
  assert.equal(mapped.edges[0]?.sourceKey, "node-a");
  assert.equal(mapped.edges[0]?.targetKey, "asset-b");
  assert.throws(
    () => mapTopologyRendererModel({ nodes: MODEL.nodes, edges: [{ ...MODEL.edges[0]!, targetKey: "missing" }] }),
    /TOPOLOGY_RENDERER_DANGLING_EDGE/,
  );
  assert.throws(
    () => mapTopologyRendererModel({ nodes: [...MODEL.nodes, MODEL.nodes[0]!], edges: [] }),
    /TOPOLOGY_RENDERER_DUPLICATE_NODE/,
  );
});

test("Sigma topology adapter bridges mount update camera focus fit resize and events", async () => {
  const factory = new FakeRuntimeFactory();
  const adapter = new SigmaTopologyRendererAdapter<object>(factory);
  const events: TopologyRendererEvent[] = [];
  const unsubscribe = adapter.subscribe((event) => events.push(event));

  await adapter.mount({}, MODEL);
  adapter.update({ ...MODEL, nodes: MODEL.nodes.slice(0, 1), edges: [] });
  adapter.resize({ width: 900, height: 600, pixelRatio: 2 });
  adapter.focusNode("node-a");
  adapter.fit();
  adapter.setCamera({ x: 0.25, y: 0.75, ratio: 1.5 });
  factory.events?.nodeSelected("node-a");
  factory.events?.nodeActivated("node-a");
  factory.events?.cameraChanged({ x: 0.5, y: 0.5, ratio: 2 });
  factory.events?.nodePositionChanged("node-a", { x: 4, y: 7 });
  unsubscribe();
  factory.events?.nodeSelected("asset-b");

  assert.equal(factory.created, 1);
  assert.deepEqual(factory.runtime.calls, ["update:1:0", "resize:900:600:2", "focus:node-a", "fit", "camera:0.25:0.75:1.5"]);
  assert.deepEqual(events, [
    { type: "NodeSelected", key: "node-a" },
    { type: "NodeActivated", key: "node-a" },
    { type: "CameraChanged", camera: { x: 0.5, y: 0.5, ratio: 2 } },
    { type: "NodePositionChanged", key: "node-a", position: { x: 4, y: 7 } },
  ]);
  await assert.rejects(() => adapter.mount({}, MODEL), /TOPOLOGY_RENDERER_ALREADY_MOUNTED/);
});

test("Sigma topology adapter disposes vendor runtime and blocks post-dispose operations", async () => {
  const factory = new FakeRuntimeFactory();
  const adapter = new SigmaTopologyRendererAdapter<object>(factory);
  let eventCount = 0;
  adapter.subscribe(() => { eventCount += 1; });
  await adapter.mount({}, MODEL);

  adapter.dispose();
  adapter.dispose();
  factory.events?.nodeSelected("node-a");

  assert.equal(factory.runtime.disposeCount, 1);
  assert.equal(eventCount, 0);
  assert.throws(() => adapter.update(MODEL), /TOPOLOGY_RENDERER_NOT_MOUNTED/);
});

test("Sigma topology adapter releases a runtime that resolves after route disposal", async () => {
  let resolveRuntime: ((runtime: SigmaTopologyRuntime) => void) | undefined;
  const runtime = new FakeRuntime();
  const factory: SigmaTopologyRuntimeFactory<object> = {
    create: async () => new Promise((resolve) => { resolveRuntime = resolve; }),
  };
  const adapter = new SigmaTopologyRendererAdapter<object>(factory);
  const mounting = adapter.mount({}, MODEL);

  adapter.dispose();
  resolveRuntime?.(runtime);

  await assert.rejects(mounting, /TOPOLOGY_RENDERER_MOUNT_CANCELLED/);
  assert.equal(runtime.disposeCount, 1);
});

class FakeRuntimeFactory implements SigmaTopologyRuntimeFactory<object> {
  readonly runtime = new FakeRuntime();
  created = 0;
  events?: SigmaTopologyRuntimeEvents;

  async create(_host: object, graph: SigmaTopologyRuntimeGraph, events: SigmaTopologyRuntimeEvents): Promise<SigmaTopologyRuntime> {
    assert.equal(graph.nodes.length, 3);
    this.created += 1;
    this.events = events;
    return this.runtime;
  }
}

class FakeRuntime implements SigmaTopologyRuntime {
  readonly calls: string[] = [];
  disposeCount = 0;

  update(graph: SigmaTopologyRuntimeGraph): void { this.calls.push(`update:${graph.nodes.length}:${graph.edges.length}`); }
  resize(viewport: { readonly width: number; readonly height: number; readonly pixelRatio: number }): void { this.calls.push(`resize:${viewport.width}:${viewport.height}:${viewport.pixelRatio}`); }
  focusNode(key: string): void { this.calls.push(`focus:${key}`); }
  fit(): void { this.calls.push("fit"); }
  setCamera(camera: { readonly x: number; readonly y: number; readonly ratio: number }): void { this.calls.push(`camera:${camera.x}:${camera.y}:${camera.ratio}`); }
  dispose(): void { this.disposeCount += 1; }
}
