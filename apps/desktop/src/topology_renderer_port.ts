export interface TopologyRendererModel {
  readonly nodes: readonly TopologyRendererNode[];
  readonly edges: readonly TopologyRendererEdge[];
}

export interface TopologyRendererNode {
  readonly key: string;
  readonly title: string;
  readonly kind: "document" | "unresolved_link" | "attachment" | "external_link";
  readonly selected: boolean;
  readonly center: boolean;
  readonly canNavigate: boolean;
  readonly emphasis: "primary" | "neighbor" | "muted" | "normal";
  readonly pinned?: boolean;
  readonly position?: Readonly<{ readonly x: number; readonly y: number }>;
}

export interface TopologyRendererEdge {
  readonly key: string;
  readonly sourceKey: string;
  readonly targetKey: string;
  readonly kind: "document_link" | "attachment_reference" | "external_reference" | "canvas_relation";
  readonly emphasis: "primary" | "muted" | "normal";
}

export interface TopologyRendererViewport {
  readonly width: number;
  readonly height: number;
  readonly pixelRatio: number;
}

export interface TopologyRendererCamera {
  readonly x: number;
  readonly y: number;
  readonly ratio: number;
}

export type TopologyRendererEvent =
  | { readonly type: "NodeSelected"; readonly key: string }
  | { readonly type: "NodeActivated"; readonly key: string }
  | { readonly type: "NodePositionChanged"; readonly key: string; readonly position: Readonly<{ readonly x: number; readonly y: number }> }
  | { readonly type: "CameraChanged"; readonly camera: TopologyRendererCamera };

export interface TopologyRendererAdapter<Host = unknown> {
  mount(host: Host, model: TopologyRendererModel): Promise<void>;
  update(model: TopologyRendererModel): void;
  resize(viewport: TopologyRendererViewport): void;
  focusNode(key: string): void;
  fit(): void;
  setCamera(camera: TopologyRendererCamera): void;
  subscribe(listener: (event: TopologyRendererEvent) => void): () => void;
  dispose(): void;
}

export interface TopologyLayoutRequest {
  readonly generation: number;
  readonly nodes: readonly { readonly key: string }[];
  readonly edges: readonly { readonly sourceKey: string; readonly targetKey: string }[];
  readonly seed: number;
  readonly reducedMotion: boolean;
  readonly iterationLimit: number;
  readonly timeoutMs: number;
}

export interface TopologyLayoutResult {
  readonly generation: number;
  readonly positions: ReadonlyMap<string, { readonly x: number; readonly y: number }>;
}

export interface TopologyLayoutAdapter {
  layout(request: TopologyLayoutRequest): Promise<TopologyLayoutResult>;
  cancel(generation: number): void;
  dispose(): void;
}

export type TopologyRendererState =
  | { readonly phase: "Unmounted"; readonly liveResourceCount: 0 }
  | { readonly phase: "Initializing"; readonly liveResourceCount: 0 }
  | { readonly phase: "Ready"; readonly liveResourceCount: number }
  | { readonly phase: "LayingOut"; readonly liveResourceCount: number; readonly generation: number }
  | { readonly phase: "Paused"; readonly liveResourceCount: number; readonly generation: number }
  | { readonly phase: "Stable"; readonly liveResourceCount: number; readonly generation: number }
  | { readonly phase: "Failed"; readonly liveResourceCount: number; readonly errorCode: string }
  | { readonly phase: "Disposing"; readonly liveResourceCount: number };

export type TopologyRendererTransitionEvent =
  | { readonly type: "MountRequested" }
  | { readonly type: "Initialized"; readonly liveResourceCount: number }
  | { readonly type: "InitializationFailed"; readonly errorCode: string }
  | { readonly type: "LayoutRequested"; readonly generation: number }
  | { readonly type: "LayoutSettled"; readonly generation: number }
  | { readonly type: "PauseRequested" }
  | { readonly type: "ResumeRequested" }
  | { readonly type: "DisposeRequested" }
  | { readonly type: "Disposed"; readonly liveResourceCount: number };

export function createUnmountedTopologyRendererState(): TopologyRendererState {
  return Object.freeze({ phase: "Unmounted", liveResourceCount: 0 });
}

export function transitionTopologyRenderer(
  state: TopologyRendererState,
  event: TopologyRendererTransitionEvent,
): TopologyRendererState {
  if (event.type === "DisposeRequested") {
    if (state.phase === "Unmounted" || state.phase === "Disposing") return state;
    return Object.freeze({ phase: "Disposing", liveResourceCount: state.liveResourceCount });
  }
  if (state.phase === "Unmounted" && event.type === "MountRequested") {
    return Object.freeze({ phase: "Initializing", liveResourceCount: 0 });
  }
  if (state.phase === "Initializing" && event.type === "Initialized") {
    if (event.liveResourceCount <= 0) throw new Error("TOPOLOGY_RENDERER_RESOURCE_COUNT_INVALID");
    return Object.freeze({ phase: "Ready", liveResourceCount: event.liveResourceCount });
  }
  if (state.phase === "Initializing" && event.type === "InitializationFailed") {
    return Object.freeze({
      phase: "Failed",
      liveResourceCount: 0,
      errorCode: event.errorCode,
    });
  }
  if (
    (state.phase === "Ready" || state.phase === "Stable")
    && event.type === "LayoutRequested"
  ) {
    if (event.generation <= 0) throw new Error("TOPOLOGY_LAYOUT_GENERATION_INVALID");
    return Object.freeze({
      phase: "LayingOut",
      liveResourceCount: state.liveResourceCount,
      generation: event.generation,
    });
  }
  if (state.phase === "LayingOut" && event.type === "LayoutSettled") {
    if (event.generation !== state.generation) return state;
    return Object.freeze({ ...state, phase: "Stable" });
  }
  if (state.phase === "LayingOut" && event.type === "PauseRequested") {
    return Object.freeze({ ...state, phase: "Paused" });
  }
  if (state.phase === "Paused" && event.type === "ResumeRequested") {
    return Object.freeze({ ...state, phase: "LayingOut" });
  }
  if (state.phase === "Disposing" && event.type === "Disposed") {
    if (event.liveResourceCount !== 0) throw new Error("TOPOLOGY_RENDERER_RESOURCES_ACTIVE");
    return createUnmountedTopologyRendererState();
  }
  throw new Error("TOPOLOGY_RENDERER_TRANSITION_INVALID");
}
