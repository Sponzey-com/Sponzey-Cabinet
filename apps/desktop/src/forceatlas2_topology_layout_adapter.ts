import { MultiDirectedGraph } from "graphology";
import type FA2LayoutSupervisor from "graphology-layout-forceatlas2/worker";

import type {
  TopologyLayoutAdapter,
  TopologyLayoutRequest,
  TopologyLayoutResult,
} from "./topology_renderer_port.ts";

type Position = Readonly<{ x: number; y: number }>;

export interface ForceAtlas2Supervisor {
  start(): void;
  stop(): void;
  readPositions(): ReadonlyMap<string, Position>;
  kill(): void;
}

export interface ForceAtlas2SupervisorFactory {
  create(request: TopologyLayoutRequest, initialPositions: ReadonlyMap<string, Position>): Promise<ForceAtlas2Supervisor>;
}

export interface LayoutScheduler {
  schedule(delayMs: number, callback: () => void): unknown;
  cancel(handle: unknown): void;
}

interface ActiveLayout {
  readonly generation: number;
  readonly request: TopologyLayoutRequest;
  readonly resolve: (result: TopologyLayoutResult) => void;
  readonly reject: (error: Error) => void;
  supervisor?: ForceAtlas2Supervisor;
  timer?: unknown;
}

export class ForceAtlas2TopologyLayoutAdapter implements TopologyLayoutAdapter {
  private active?: ActiveLayout;
  private disposed = false;
  private readonly factory: ForceAtlas2SupervisorFactory;
  private readonly scheduler: LayoutScheduler;

  constructor(
    factory: ForceAtlas2SupervisorFactory = new BrowserForceAtlas2SupervisorFactory(),
    scheduler: LayoutScheduler = new BrowserLayoutScheduler(),
  ) {
    this.factory = factory;
    this.scheduler = scheduler;
  }

  layout(request: TopologyLayoutRequest): Promise<TopologyLayoutResult> {
    if (this.disposed) return Promise.reject(new Error("TOPOLOGY_LAYOUT_DISPOSED"));
    validateRequest(request);
    this.cancelActive("TOPOLOGY_LAYOUT_REPLACED");
    const initialPositions = createInitialPositions(request);
    if (request.reducedMotion || request.nodes.length <= 1) {
      return Promise.resolve(Object.freeze({ generation: request.generation, positions: initialPositions }));
    }
    return new Promise<TopologyLayoutResult>((resolve, reject) => {
      const run: ActiveLayout = { generation: request.generation, request, resolve, reject };
      this.active = run;
      void this.start(run, initialPositions);
    });
  }

  cancel(generation: number): void {
    if (this.active?.generation !== generation) return;
    this.cancelActive("TOPOLOGY_LAYOUT_CANCELLED");
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.cancelActive("TOPOLOGY_LAYOUT_DISPOSED");
  }

  private async start(run: ActiveLayout, initialPositions: ReadonlyMap<string, Position>): Promise<void> {
    let supervisor: ForceAtlas2Supervisor;
    try {
      supervisor = await this.factory.create(run.request, initialPositions);
    } catch {
      if (this.active === run) {
        this.active = undefined;
        run.reject(new Error("TOPOLOGY_LAYOUT_WORKER_INITIALIZATION_FAILED"));
      }
      return;
    }
    if (this.active !== run || this.disposed) {
      supervisor.kill();
      return;
    }
    run.supervisor = supervisor;
    supervisor.start();
    const delayMs = Math.min(run.request.timeoutMs, Math.max(16, run.request.iterationLimit * 4));
    run.timer = this.scheduler.schedule(delayMs, () => this.settle(run));
  }

  private settle(run: ActiveLayout): void {
    if (this.active !== run || !run.supervisor) return;
    this.active = undefined;
    run.timer = undefined;
    try {
      run.supervisor.stop();
      const positions = validateResult(run.request, run.supervisor.readPositions());
      run.supervisor.kill();
      run.resolve(Object.freeze({ generation: run.generation, positions }));
    } catch {
      run.supervisor.kill();
      run.reject(new Error("TOPOLOGY_LAYOUT_RESULT_INVALID"));
    }
  }

  private cancelActive(errorCode: string): void {
    const run = this.active;
    if (!run) return;
    this.active = undefined;
    if (run.timer !== undefined) this.scheduler.cancel(run.timer);
    if (run.supervisor) {
      run.supervisor.stop();
      run.supervisor.kill();
    }
    run.reject(new Error(errorCode));
  }
}

class BrowserForceAtlas2SupervisorFactory implements ForceAtlas2SupervisorFactory {
  async create(request: TopologyLayoutRequest, initialPositions: ReadonlyMap<string, Position>): Promise<ForceAtlas2Supervisor> {
    const { default: Supervisor } = await import("graphology-layout-forceatlas2/worker");
    const graph = new MultiDirectedGraph();
    for (const node of request.nodes) {
      const position = initialPositions.get(node.key);
      if (!position) throw new Error("TOPOLOGY_LAYOUT_POSITION_MISSING");
      graph.addNode(node.key, { x: position.x, y: position.y, size: 1 });
    }
    request.edges.forEach((edge, index) => {
      graph.addDirectedEdgeWithKey(`layout-edge-${index}`, edge.sourceKey, edge.targetKey, { weight: 1 });
    });
    const supervisor = new Supervisor(graph, {
      settings: {
        barnesHutOptimize: request.nodes.length >= 500,
        gravity: 1,
        slowDown: 5,
      },
    });
    return new BrowserForceAtlas2Supervisor(supervisor, graph);
  }
}

class BrowserForceAtlas2Supervisor implements ForceAtlas2Supervisor {
  private readonly supervisor: FA2LayoutSupervisor;
  private readonly graph: MultiDirectedGraph;

  constructor(supervisor: FA2LayoutSupervisor, graph: MultiDirectedGraph) {
    this.supervisor = supervisor;
    this.graph = graph;
  }

  start(): void { this.supervisor.start(); }
  stop(): void { this.supervisor.stop(); }
  kill(): void { this.supervisor.kill(); }

  readPositions(): ReadonlyMap<string, Position> {
    const positions = new Map<string, Position>();
    this.graph.forEachNode((key, attributes) => {
      positions.set(key, Object.freeze({ x: Number(attributes.x), y: Number(attributes.y) }));
    });
    return positions;
  }
}

class BrowserLayoutScheduler implements LayoutScheduler {
  schedule(delayMs: number, callback: () => void): unknown {
    return setTimeout(callback, delayMs);
  }

  cancel(handle: unknown): void {
    clearTimeout(handle as ReturnType<typeof setTimeout>);
  }
}

function validateRequest(request: TopologyLayoutRequest): void {
  if (!Number.isInteger(request.generation) || request.generation <= 0) throw new Error("TOPOLOGY_LAYOUT_GENERATION_INVALID");
  if (!Number.isInteger(request.seed)) throw new Error("TOPOLOGY_LAYOUT_SEED_INVALID");
  if (!Number.isInteger(request.iterationLimit) || request.iterationLimit < 1 || request.iterationLimit > 10_000) throw new Error("TOPOLOGY_LAYOUT_ITERATION_LIMIT_INVALID");
  if (!Number.isFinite(request.timeoutMs) || request.timeoutMs < 16 || request.timeoutMs > 30_000) throw new Error("TOPOLOGY_LAYOUT_TIMEOUT_INVALID");
  if (request.nodes.length > 10_000 || request.edges.length > 50_000) throw new Error("TOPOLOGY_LAYOUT_CAPACITY_EXCEEDED");
  const keys = new Set<string>();
  for (const node of request.nodes) {
    if (!node.key.trim()) throw new Error("TOPOLOGY_LAYOUT_KEY_INVALID");
    if (keys.has(node.key)) throw new Error("TOPOLOGY_LAYOUT_DUPLICATE_NODE");
    keys.add(node.key);
  }
  for (const edge of request.edges) {
    if (!keys.has(edge.sourceKey) || !keys.has(edge.targetKey)) throw new Error("TOPOLOGY_LAYOUT_DANGLING_EDGE");
  }
}

function createInitialPositions(request: TopologyLayoutRequest): ReadonlyMap<string, Position> {
  const positions = new Map<string, Position>();
  request.nodes.forEach((node, index) => {
    const xHash = hash(`${request.seed}:x:${node.key}`);
    const yHash = hash(`${request.seed}:y:${node.key}`);
    positions.set(node.key, Object.freeze({
      x: (xHash + index + 1) / (0xffff_ffff + request.nodes.length + 1),
      y: (yHash + request.nodes.length - index) / (0xffff_ffff + request.nodes.length + 1),
    }));
  });
  return positions;
}

function validateResult(request: TopologyLayoutRequest, positions: ReadonlyMap<string, Position>): ReadonlyMap<string, Position> {
  if (positions.size !== request.nodes.length) throw new Error("TOPOLOGY_LAYOUT_RESULT_SIZE_INVALID");
  const result = new Map<string, Position>();
  for (const node of request.nodes) {
    const position = positions.get(node.key);
    if (!position || !Number.isFinite(position.x) || !Number.isFinite(position.y)) {
      throw new Error("TOPOLOGY_LAYOUT_RESULT_POSITION_INVALID");
    }
    result.set(node.key, Object.freeze({ x: position.x, y: position.y }));
  }
  return result;
}

function hash(value: string): number {
  let result = 2_166_136_261;
  for (let index = 0; index < value.length; index += 1) {
    result ^= value.charCodeAt(index);
    result = Math.imul(result, 16_777_619);
  }
  return result >>> 0;
}
