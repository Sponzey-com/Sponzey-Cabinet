import assert from "node:assert/strict";
import test from "node:test";

import {
  ForceAtlas2TopologyLayoutAdapter,
  type ForceAtlas2Supervisor,
  type ForceAtlas2SupervisorFactory,
  type LayoutScheduler,
} from "../src/forceatlas2_topology_layout_adapter.ts";
import type { TopologyLayoutRequest } from "../src/topology_renderer_port.ts";

test("ForceAtlas2 adapter settles one bounded worker result and releases resources", async () => {
  const factory = new FakeSupervisorFactory();
  const scheduler = new FakeScheduler();
  const adapter = new ForceAtlas2TopologyLayoutAdapter(factory, scheduler);

  const resultPromise = adapter.layout(request(1));
  await Promise.resolve();
  assert.deepEqual(factory.latest?.calls, ["start"]);
  assert.equal(scheduler.delayMs, 80);
  scheduler.fire();
  const result = await resultPromise;

  assert.equal(result.generation, 1);
  assert.deepEqual([...result.positions.keys()], ["a", "b"]);
  assert.deepEqual(factory.latest?.calls, ["start", "stop", "read", "kill"]);
  assert.equal(scheduler.activeCount, 0);
});

test("ForceAtlas2 adapter replaces current generation and ignores stale cancel", async () => {
  const factory = new FakeSupervisorFactory();
  const scheduler = new FakeScheduler();
  const adapter = new ForceAtlas2TopologyLayoutAdapter(factory, scheduler);
  const first = adapter.layout(request(1));
  const firstFailure = assert.rejects(first, /TOPOLOGY_LAYOUT_REPLACED/);
  await Promise.resolve();
  const firstSupervisor = factory.latest!;

  const second = adapter.layout(request(2));
  await firstFailure;
  await Promise.resolve();
  adapter.cancel(1);

  assert.deepEqual(firstSupervisor.calls, ["start", "stop", "kill"]);
  assert.deepEqual(factory.latest?.calls, ["start"]);
  scheduler.fire();
  assert.equal((await second).generation, 2);
});

test("ForceAtlas2 adapter handles matching cancel reduced motion and dispose", async () => {
  const factory = new FakeSupervisorFactory();
  const scheduler = new FakeScheduler();
  const adapter = new ForceAtlas2TopologyLayoutAdapter(factory, scheduler);

  const reduced = await adapter.layout(request(1, true));
  assert.equal(factory.created, 0);
  assert.equal(reduced.positions.size, 2);

  const cancelled = adapter.layout(request(2));
  const cancelledFailure = assert.rejects(cancelled, /TOPOLOGY_LAYOUT_CANCELLED/);
  await Promise.resolve();
  adapter.cancel(2);
  await cancelledFailure;
  assert.deepEqual(factory.latest?.calls, ["start", "stop", "kill"]);

  const disposed = adapter.layout(request(3));
  const disposedFailure = assert.rejects(disposed, /TOPOLOGY_LAYOUT_DISPOSED/);
  await Promise.resolve();
  adapter.dispose();
  adapter.dispose();
  await disposedFailure;
  await assert.rejects(adapter.layout(request(4)), /TOPOLOGY_LAYOUT_DISPOSED/);
  assert.equal(scheduler.activeCount, 0);
});

function request(generation: number, reducedMotion = false): TopologyLayoutRequest {
  return Object.freeze({
    generation,
    nodes: Object.freeze([{ key: "a" }, { key: "b" }]),
    edges: Object.freeze([{ sourceKey: "a", targetKey: "b" }]),
    seed: 42,
    reducedMotion,
    iterationLimit: 20,
    timeoutMs: 500,
  });
}

class FakeSupervisorFactory implements ForceAtlas2SupervisorFactory {
  latest?: FakeSupervisor;
  created = 0;

  async create(request: TopologyLayoutRequest, initialPositions: ReadonlyMap<string, { readonly x: number; readonly y: number }>): Promise<ForceAtlas2Supervisor> {
    assert.equal(request.nodes.length, initialPositions.size);
    this.created += 1;
    this.latest = new FakeSupervisor(initialPositions);
    return this.latest;
  }
}

class FakeSupervisor implements ForceAtlas2Supervisor {
  readonly calls: string[] = [];
  private readonly positions: ReadonlyMap<string, { readonly x: number; readonly y: number }>;

  constructor(positions: ReadonlyMap<string, { readonly x: number; readonly y: number }>) {
    this.positions = positions;
  }
  start(): void { this.calls.push("start"); }
  stop(): void { this.calls.push("stop"); }
  readPositions(): ReadonlyMap<string, { readonly x: number; readonly y: number }> { this.calls.push("read"); return this.positions; }
  kill(): void { this.calls.push("kill"); }
}

class FakeScheduler implements LayoutScheduler {
  private callback?: () => void;
  delayMs?: number;
  activeCount = 0;

  schedule(delayMs: number, callback: () => void): object {
    this.delayMs = delayMs;
    this.callback = callback;
    this.activeCount = 1;
    return Object.freeze({ timer: 1 });
  }

  cancel(_handle: object): void {
    this.callback = undefined;
    this.activeCount = 0;
  }

  fire(): void {
    const callback = this.callback;
    this.callback = undefined;
    this.activeCount = 0;
    callback?.();
  }
}
