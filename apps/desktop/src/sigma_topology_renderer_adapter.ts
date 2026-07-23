import { MultiDirectedGraph } from "graphology";
import type Sigma from "sigma";

import type {
  TopologyRendererAdapter,
  TopologyRendererCamera,
  TopologyRendererEvent,
  TopologyRendererModel,
  TopologyRendererViewport,
} from "./topology_renderer_port.ts";

export interface SigmaTopologyRuntimeNode {
  readonly key: string;
  readonly label: string;
  readonly kind: string;
  readonly x: number;
  readonly y: number;
  readonly size: number;
  readonly color: string;
  readonly highlighted: boolean;
  readonly forceLabel: boolean;
  readonly zIndex: number;
}

export interface SigmaTopologyRuntimeEdge {
  readonly key: string;
  readonly sourceKey: string;
  readonly targetKey: string;
  readonly kind: string;
  readonly size: number;
  readonly color: string;
  readonly zIndex: number;
}

export interface SigmaTopologyRuntimeGraph {
  readonly nodes: readonly SigmaTopologyRuntimeNode[];
  readonly edges: readonly SigmaTopologyRuntimeEdge[];
}

export interface SigmaTopologyRuntimeEvents {
  nodeSelected(key: string): void;
  nodeActivated(key: string): void;
  cameraChanged(camera: TopologyRendererCamera): void;
  nodePositionChanged(key: string, position: Readonly<{ readonly x: number; readonly y: number }>): void;
}

export interface SigmaTopologyRuntime {
  update(graph: SigmaTopologyRuntimeGraph): void;
  resize(viewport: TopologyRendererViewport): void;
  focusNode(key: string): void;
  fit(): void;
  setCamera(camera: TopologyRendererCamera): void;
  dispose(): void;
}

export interface SigmaTopologyRuntimeFactory<Host> {
  create(host: Host, graph: SigmaTopologyRuntimeGraph, events: SigmaTopologyRuntimeEvents): Promise<SigmaTopologyRuntime>;
}

export class SigmaTopologyRendererAdapter<Host = HTMLElement> implements TopologyRendererAdapter<Host> {
  private runtime?: SigmaTopologyRuntime;
  private readonly listeners = new Set<(event: TopologyRendererEvent) => void>();
  private readonly factory: SigmaTopologyRuntimeFactory<Host>;
  private mountGeneration = 0;
  private mounting = false;

  constructor(
    factory: SigmaTopologyRuntimeFactory<Host> = new BrowserSigmaTopologyRuntimeFactory() as SigmaTopologyRuntimeFactory<Host>,
  ) {
    this.factory = factory;
  }

  async mount(host: Host, model: TopologyRendererModel): Promise<void> {
    if (this.runtime || this.mounting) throw new Error("TOPOLOGY_RENDERER_ALREADY_MOUNTED");
    const generation = ++this.mountGeneration;
    this.mounting = true;
    const graph = mapTopologyRendererModel(model);
    let runtime: SigmaTopologyRuntime;
    try {
      runtime = await this.factory.create(host, graph, Object.freeze({
        nodeSelected: (key: string) => this.emit(Object.freeze({ type: "NodeSelected", key })),
        nodeActivated: (key: string) => this.emit(Object.freeze({ type: "NodeActivated", key })),
        cameraChanged: (camera: TopologyRendererCamera) => this.emit(Object.freeze({ type: "CameraChanged", camera: Object.freeze({ ...camera }) })),
        nodePositionChanged: (key, position) => this.emit(Object.freeze({ type: "NodePositionChanged", key, position: Object.freeze({ ...position }) })),
      }));
    } catch {
      if (generation === this.mountGeneration) this.mounting = false;
      throw new Error("TOPOLOGY_RENDERER_INITIALIZATION_FAILED");
    }
    if (generation !== this.mountGeneration) {
      runtime.dispose();
      throw new Error("TOPOLOGY_RENDERER_MOUNT_CANCELLED");
    }
    this.mounting = false;
    this.runtime = runtime;
  }

  update(model: TopologyRendererModel): void {
    this.requireRuntime().update(mapTopologyRendererModel(model));
  }

  resize(viewport: TopologyRendererViewport): void {
    assertViewport(viewport);
    this.requireRuntime().resize(viewport);
  }

  focusNode(key: string): void {
    assertKey(key);
    this.requireRuntime().focusNode(key);
  }

  fit(): void {
    this.requireRuntime().fit();
  }

  setCamera(camera: TopologyRendererCamera): void {
    assertCamera(camera);
    this.requireRuntime().setCamera(camera);
  }

  subscribe(listener: (event: TopologyRendererEvent) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  dispose(): void {
    this.mountGeneration += 1;
    this.mounting = false;
    const runtime = this.runtime;
    this.listeners.clear();
    if (!runtime) return;
    this.runtime = undefined;
    runtime.dispose();
  }

  private requireRuntime(): SigmaTopologyRuntime {
    if (!this.runtime) throw new Error("TOPOLOGY_RENDERER_NOT_MOUNTED");
    return this.runtime;
  }

  private emit(event: TopologyRendererEvent): void {
    for (const listener of this.listeners) listener(event);
  }
}

export function mapTopologyRendererModel(model: TopologyRendererModel): SigmaTopologyRuntimeGraph {
  const keys = new Set<string>();
  const dimension = Math.max(1, Math.ceil(Math.sqrt(model.nodes.length)));
  const nodes = model.nodes.map((node, index) => {
    assertKey(node.key);
    if (keys.has(node.key)) throw new Error("TOPOLOGY_RENDERER_DUPLICATE_NODE");
    if (!node.title.trim()) throw new Error("TOPOLOGY_RENDERER_LABEL_INVALID");
    keys.add(node.key);
    const column = index % dimension;
    const row = Math.floor(index / dimension);
    const position = node.position ?? Object.freeze({
      x: (column + 0.5) / dimension,
      y: (row + 0.5) / dimension,
    });
    return Object.freeze({
      key: node.key,
      label: node.title,
      kind: node.kind,
      x: position.x,
      y: position.y,
      size: node.center ? 11 : node.selected ? 9 : node.emphasis === "neighbor" ? 8 : node.kind === "document" ? 7 : 6,
      color: node.emphasis === "muted" ? "#c8ced3" : nodeColor(node.kind, node.selected, node.center),
      highlighted: node.emphasis === "primary" || node.emphasis === "neighbor",
      forceLabel: node.emphasis === "primary" || node.emphasis === "neighbor" || node.pinned === true,
      zIndex: node.emphasis === "primary" ? 3 : node.emphasis === "neighbor" ? 2 : 1,
    });
  });
  const edgeKeys = new Set<string>();
  const edges = model.edges.map((edge) => {
    assertKey(edge.key);
    if (edgeKeys.has(edge.key)) throw new Error("TOPOLOGY_RENDERER_DUPLICATE_EDGE");
    if (!keys.has(edge.sourceKey) || !keys.has(edge.targetKey)) {
      throw new Error("TOPOLOGY_RENDERER_DANGLING_EDGE");
    }
    edgeKeys.add(edge.key);
    return Object.freeze({
      key: edge.key,
      sourceKey: edge.sourceKey,
      targetKey: edge.targetKey,
      kind: edge.kind,
      size: edge.emphasis === "primary" ? 2.25 : edge.emphasis === "muted" ? 0.75 : edge.kind === "canvas_relation" ? 2 : 1.25,
      color: edge.emphasis === "muted" ? "#d6dadd" : edgeColor(edge.kind),
      zIndex: edge.emphasis === "primary" ? 3 : edge.emphasis === "muted" ? 0 : edge.kind === "document_link" ? 2 : 1,
    });
  });
  return Object.freeze({ nodes: Object.freeze(nodes), edges: Object.freeze(edges) });
}

class BrowserSigmaTopologyRuntimeFactory implements SigmaTopologyRuntimeFactory<HTMLElement> {
  async create(host: HTMLElement, graph: SigmaTopologyRuntimeGraph, events: SigmaTopologyRuntimeEvents): Promise<SigmaTopologyRuntime> {
    const { default: SigmaConstructor } = await import("sigma");
    return new BrowserSigmaTopologyRuntime(SigmaConstructor, host, graph, events);
  }
}

class BrowserSigmaTopologyRuntime implements SigmaTopologyRuntime {
  private graph = createGraphologyGraph({ nodes: [], edges: [] });
  private readonly renderer: Sigma;
  private disposed = false;
  private readonly onClickNode: (payload: { node: string }) => void;
  private readonly onDoubleClickNode: (payload: { node: string }) => void;
  private readonly onCameraUpdated: (state: { x: number; y: number; ratio: number }) => void;
  private readonly onDownNode: (payload: { node: string; event: { x: number; y: number; preventSigmaDefault(): void } }) => void;
  private readonly onMoveBody: (payload: { event: { x: number; y: number; preventSigmaDefault(): void } }) => void;
  private readonly onUpNode: () => void;
  private readonly onUpStage: () => void;
  private draggedNode?: string;
  private draggedPosition?: Readonly<{ x: number; y: number }>;

  constructor(SigmaConstructor: typeof Sigma, host: HTMLElement, data: SigmaTopologyRuntimeGraph, events: SigmaTopologyRuntimeEvents) {
    this.graph = createGraphologyGraph(data);
    this.renderer = new SigmaConstructor(this.graph, host, {
      allowInvalidContainer: false,
      renderEdgeLabels: false,
      labelRenderedSizeThreshold: 6,
      zIndex: true,
    });
    this.onClickNode = ({ node }) => events.nodeSelected(node);
    this.onDoubleClickNode = ({ node }) => events.nodeActivated(node);
    this.onCameraUpdated = ({ x, y, ratio }) => events.cameraChanged(Object.freeze({ x, y, ratio }));
    this.onDownNode = ({ node, event }) => {
      event.preventSigmaDefault();
      this.draggedNode = node;
      this.draggedPosition = undefined;
      this.renderer.getCamera().disable();
    };
    this.onMoveBody = ({ event }) => {
      if (!this.draggedNode) return;
      event.preventSigmaDefault();
      const position = this.renderer.viewportToGraph({ x: event.x, y: event.y });
      if (!Number.isFinite(position.x) || !Number.isFinite(position.y)) return;
      this.graph.setNodeAttribute(this.draggedNode, "x", position.x);
      this.graph.setNodeAttribute(this.draggedNode, "y", position.y);
      this.draggedPosition = Object.freeze({ x: position.x, y: position.y });
      this.renderer.refresh();
    };
    const finishDrag = () => {
      const key = this.draggedNode;
      const position = this.draggedPosition;
      this.draggedNode = undefined;
      this.draggedPosition = undefined;
      this.renderer.getCamera().enable();
      if (key && position) events.nodePositionChanged(key, position);
    };
    this.onUpNode = finishDrag;
    this.onUpStage = finishDrag;
    this.renderer.on("clickNode", this.onClickNode);
    this.renderer.on("doubleClickNode", this.onDoubleClickNode);
    this.renderer.on("downNode", this.onDownNode);
    this.renderer.on("moveBody", this.onMoveBody);
    this.renderer.on("upNode", this.onUpNode);
    this.renderer.on("upStage", this.onUpStage);
    this.renderer.getCamera().on("updated", this.onCameraUpdated);
  }

  update(data: SigmaTopologyRuntimeGraph): void {
    this.assertLive();
    this.graph = createGraphologyGraph(data);
    this.renderer.setGraph(this.graph);
    this.renderer.refresh();
  }

  resize(_viewport: TopologyRendererViewport): void {
    this.assertLive();
    this.renderer.resize(true);
  }

  focusNode(key: string): void {
    this.assertLive();
    if (!this.graph.hasNode(key)) throw new Error("TOPOLOGY_RENDERER_NODE_NOT_FOUND");
    const position = this.renderer.getNodeDisplayData(key);
    if (!position) throw new Error("TOPOLOGY_RENDERER_NODE_NOT_RENDERED");
    void this.renderer.getCamera().animate({ x: position.x, y: position.y, ratio: Math.min(this.renderer.getCamera().ratio, 0.7) }, { duration: 220 });
  }

  fit(): void {
    this.assertLive();
    void this.renderer.getCamera().animatedReset({ duration: 220 });
  }

  setCamera(camera: TopologyRendererCamera): void {
    this.assertLive();
    this.renderer.getCamera().setState(camera);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.renderer.off("clickNode", this.onClickNode);
    this.renderer.off("doubleClickNode", this.onDoubleClickNode);
    this.renderer.off("downNode", this.onDownNode);
    this.renderer.off("moveBody", this.onMoveBody);
    this.renderer.off("upNode", this.onUpNode);
    this.renderer.off("upStage", this.onUpStage);
    this.draggedNode = undefined;
    this.draggedPosition = undefined;
    this.renderer.getCamera().enable();
    this.renderer.getCamera().off("updated", this.onCameraUpdated);
    this.renderer.kill();
  }

  private assertLive(): void {
    if (this.disposed) throw new Error("TOPOLOGY_RENDERER_DISPOSED");
  }
}

function createGraphologyGraph(data: SigmaTopologyRuntimeGraph): MultiDirectedGraph {
  const graph = new MultiDirectedGraph();
  for (const node of data.nodes) {
    graph.addNode(node.key, {
      label: node.label,
      kind: node.kind,
      x: node.x,
      y: node.y,
      size: node.size,
      color: node.color,
      highlighted: node.highlighted,
      forceLabel: node.forceLabel,
      zIndex: node.zIndex,
    });
  }
  for (const edge of data.edges) {
    graph.addDirectedEdgeWithKey(edge.key, edge.sourceKey, edge.targetKey, {
      kind: edge.kind,
      size: edge.size,
      color: edge.color,
      zIndex: edge.zIndex,
    });
  }
  return graph;
}

function nodeColor(kind: string, selected: boolean, center: boolean): string {
  if (center) return "#146c5d";
  if (selected) return "#2b8172";
  if (kind === "attachment") return "#b07635";
  if (kind === "unresolved_link") return "#a85a62";
  if (kind === "external_link") return "#637083";
  return "#5b6877";
}

function edgeColor(kind: string): string {
  if (kind === "attachment_reference") return "#b99467";
  if (kind === "external_reference") return "#9aa4b1";
  if (kind === "canvas_relation") return "#7297a0";
  return "#81908d";
}

function assertKey(key: string): void {
  if (!key.trim()) throw new Error("TOPOLOGY_RENDERER_KEY_INVALID");
}

function assertViewport(viewport: TopologyRendererViewport): void {
  if (viewport.width <= 0 || viewport.height <= 0 || viewport.pixelRatio <= 0) {
    throw new Error("TOPOLOGY_RENDERER_VIEWPORT_INVALID");
  }
}

function assertCamera(camera: TopologyRendererCamera): void {
  if (![camera.x, camera.y, camera.ratio].every(Number.isFinite) || camera.ratio <= 0) {
    throw new Error("TOPOLOGY_RENDERER_CAMERA_INVALID");
  }
}
