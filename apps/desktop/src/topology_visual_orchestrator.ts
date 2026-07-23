import type {
  TopologyLayoutAdapter,
  TopologyRendererAdapter,
  TopologyRendererEvent,
  TopologyRendererModel,
  TopologyRendererCamera,
  TopologyRendererViewport,
} from "./topology_renderer_port.ts";

export interface TopologyDisplayNodeInput {
  readonly identity: string;
  readonly kind: "document" | "unresolved_link" | "attachment" | "external_link";
  readonly label: string;
  readonly canNavigate: boolean;
}

export interface TopologyDisplayEdgeInput {
  readonly id: string;
  readonly sourceId: string;
  readonly targetId: string;
  readonly kind: "document_link" | "attachment_reference" | "external_reference" | "canvas_relation";
}

export interface TopologyVisualCallbacks {
  onNodeSelected(key: string): void;
  onNodeActivated(key: string): void;
  onFailure(errorCode: string): void;
  onLayoutPaused?(paused: boolean): void;
  onCameraChanged?(camera: TopologyRendererCamera): void;
}

const LAYOUT_POLICY = Object.freeze({
  seed: 2_026_071_8,
  iterationLimit: 80,
  timeoutMs: 800,
});

export class TopologyVisualOrchestrator<Host = unknown> {
  private readonly renderer: TopologyRendererAdapter<Host>;
  private readonly layout: TopologyLayoutAdapter;
  private readonly callbacks: TopologyVisualCallbacks;
  private unsubscribe?: () => void;
  private model?: TopologyRendererModel;
  private generation = 0;
  private mounted = false;
  private disposed = false;
  private layoutMode: "Running" | "Paused" | "Disposed" = "Running";

  constructor(
    renderer: TopologyRendererAdapter<Host>,
    layout: TopologyLayoutAdapter,
    callbacks: TopologyVisualCallbacks,
  ) {
    this.renderer = renderer;
    this.layout = layout;
    this.callbacks = callbacks;
  }

  async mount(host: Host, model: TopologyRendererModel, reducedMotion: boolean, initialCamera?: TopologyRendererCamera): Promise<void> {
    if (this.disposed) throw new Error("TOPOLOGY_VISUAL_DISPOSED");
    if (this.mounted) throw new Error("TOPOLOGY_VISUAL_ALREADY_MOUNTED");
    this.unsubscribe = this.renderer.subscribe((event) => this.handleEvent(event));
    try {
      await this.renderer.mount(host, model);
    } catch {
      this.unsubscribe?.();
      this.unsubscribe = undefined;
      this.callbacks.onFailure("GRAPH_RENDERER_INITIALIZATION_FAILED");
      throw new Error("GRAPH_RENDERER_INITIALIZATION_FAILED");
    }
    this.mounted = true;
    this.model = model;
    if (initialCamera && validCamera(initialCamera)) this.renderer.setCamera(initialCamera);
    this.startLayout(model, reducedMotion);
  }

  update(model: TopologyRendererModel, reducedMotion: boolean): void {
    this.assertMounted();
    this.model = model;
    this.renderer.update(model);
    this.startLayout(model, reducedMotion);
  }

  resize(viewport: TopologyRendererViewport): void {
    this.assertMounted();
    this.renderer.resize(viewport);
  }

  setZoomPercent(zoomPercent: number): void {
    this.assertMounted();
    if (!Number.isFinite(zoomPercent) || zoomPercent < 50 || zoomPercent > 200) {
      throw new Error("TOPOLOGY_VISUAL_ZOOM_INVALID");
    }
    this.renderer.setCamera({ x: 0.5, y: 0.5, ratio: 100 / zoomPercent });
  }

  fit(): void {
    this.assertMounted();
    this.renderer.fit();
  }

  pauseLayout(): void {
    this.assertMounted();
    if (this.layoutMode === "Paused") return;
    const activeGeneration = this.generation;
    this.layoutMode = "Paused";
    this.callbacks.onLayoutPaused?.(true);
    this.generation += 1;
    this.layout.cancel(activeGeneration);
  }

  resumeLayout(reducedMotion: boolean): void {
    this.assertMounted();
    if (this.layoutMode === "Running") return;
    this.layoutMode = "Running";
    this.callbacks.onLayoutPaused?.(false);
    if (this.model) this.startLayout(this.model, reducedMotion);
  }

  resetLayout(reducedMotion: boolean): void {
    this.assertMounted();
    const activeGeneration = this.generation;
    this.generation += 1;
    this.layout.cancel(activeGeneration);
    this.layoutMode = "Running";
    this.callbacks.onLayoutPaused?.(false);
    const resetModel = Object.freeze({
      ...this.model!,
      nodes: Object.freeze(this.model!.nodes.map((node) => {
        const { position: _position, pinned: _pinned, ...resetNode } = node;
        return Object.freeze(resetNode);
      })),
    });
    this.model = resetModel;
    this.renderer.update(resetModel);
    this.startLayout(resetModel, reducedMotion);
    this.renderer.fit();
  }

  isLayoutPaused(): boolean {
    return this.layoutMode === "Paused";
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.layoutMode = "Disposed";
    this.generation += 1;
    this.unsubscribe?.();
    this.unsubscribe = undefined;
    this.layout.dispose();
    this.renderer.dispose();
    this.model = undefined;
    this.mounted = false;
  }

  private startLayout(model: TopologyRendererModel, reducedMotion: boolean): void {
    if (this.layoutMode !== "Running") return;
    const generation = ++this.generation;
    void this.layout.layout(Object.freeze({
      generation,
      nodes: Object.freeze(model.nodes.map((node) => Object.freeze({ key: node.key }))),
      edges: Object.freeze(model.edges.map((edge) => Object.freeze({ sourceKey: edge.sourceKey, targetKey: edge.targetKey }))),
      seed: LAYOUT_POLICY.seed,
      reducedMotion,
      iterationLimit: LAYOUT_POLICY.iterationLimit,
      timeoutMs: LAYOUT_POLICY.timeoutMs,
    })).then((result) => {
      if (this.disposed || !this.mounted || result.generation !== this.generation || this.model !== model) return;
      const positioned = Object.freeze({
        ...model,
        nodes: Object.freeze(model.nodes.map((node) => Object.freeze({
          ...node,
          position: node.pinned && node.position ? node.position : result.positions.get(node.key),
        }))),
      });
      this.model = positioned;
      this.renderer.update(positioned);
    }).catch((error: unknown) => {
      if (this.disposed || generation !== this.generation) return;
      const code = error instanceof Error && /^TOPOLOGY_LAYOUT_[A-Z_]+$/.test(error.message)
        ? error.message
        : "GRAPH_LAYOUT_FAILED";
      if (!["TOPOLOGY_LAYOUT_REPLACED", "TOPOLOGY_LAYOUT_CANCELLED", "TOPOLOGY_LAYOUT_DISPOSED"].includes(code)) {
        this.callbacks.onFailure(code);
      }
    });
  }

  private handleEvent(event: TopologyRendererEvent): void {
    if (this.disposed) return;
    if (event.type === "NodeSelected") this.callbacks.onNodeSelected(event.key);
    if (event.type === "NodeActivated") this.callbacks.onNodeActivated(event.key);
    if (event.type === "CameraChanged" && validCamera(event.camera)) this.callbacks.onCameraChanged?.(event.camera);
    if (event.type === "NodePositionChanged") {
      if (!Number.isFinite(event.position.x) || !Number.isFinite(event.position.y)) return;
      const model = this.model;
      if (!model || !model.nodes.some((node) => node.key === event.key)) return;
      this.pauseLayout();
      const pinned = Object.freeze({
        ...model,
        nodes: Object.freeze(model.nodes.map((node) => node.key === event.key
          ? Object.freeze({ ...node, position: event.position, pinned: true })
          : node)),
      });
      this.model = pinned;
      this.renderer.update(pinned);
    }
  }

  private assertMounted(): void {
    if (this.disposed) throw new Error("TOPOLOGY_VISUAL_DISPOSED");
    if (!this.mounted) throw new Error("TOPOLOGY_VISUAL_NOT_MOUNTED");
  }
}

function validCamera(camera: TopologyRendererCamera): boolean {
  return [camera.x, camera.y, camera.ratio].every(Number.isFinite) && camera.ratio > 0;
}

export function createTopologyRendererModel(
  nodes: readonly TopologyDisplayNodeInput[],
  edges: readonly TopologyDisplayEdgeInput[],
  selectedNodeId?: string,
  centerNodeId?: string,
): TopologyRendererModel {
  const focusNodeId = selectedNodeId ?? centerNodeId;
  const neighbors = new Set<string>();
  if (focusNodeId) {
    for (const edge of edges) {
      if (edge.sourceId === focusNodeId) neighbors.add(edge.targetId);
      if (edge.targetId === focusNodeId) neighbors.add(edge.sourceId);
    }
  }
  return Object.freeze({
    nodes: Object.freeze(nodes.map((node) => Object.freeze({
      key: node.identity,
      title: node.label,
      kind: node.kind,
      selected: node.identity === selectedNodeId,
      center: node.identity === centerNodeId,
      canNavigate: node.canNavigate,
      emphasis: !focusNodeId
        ? "normal"
        : node.identity === focusNodeId
          ? "primary"
          : neighbors.has(node.identity) ? "neighbor" : "muted",
    }))),
    edges: Object.freeze(edges.map((edge) => Object.freeze({
      key: edge.id,
      sourceKey: edge.sourceId,
      targetKey: edge.targetId,
      kind: edge.kind,
      emphasis: !focusNodeId
        ? "normal"
        : edge.sourceId === focusNodeId || edge.targetId === focusNodeId ? "primary" : "muted",
    }))),
  });
}
