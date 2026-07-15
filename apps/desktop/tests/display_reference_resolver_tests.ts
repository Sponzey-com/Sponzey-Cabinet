import assert from "node:assert/strict";
import test from "node:test";

import {
  createKoKrDisplayFallbackPolicy,
  resolveDisplayReferences,
  type DisplayProjectionEntry,
  type DisplayProjectionPort,
  type DisplayReferenceRequest,
} from "../src/display_reference_resolver.ts";

class FakeDisplayProjection implements DisplayProjectionPort {
  calls: DisplayReferenceRequest[][] = [];
  private readonly entries: readonly DisplayProjectionEntry[];
  constructor(entries: readonly DisplayProjectionEntry[]) { this.entries = entries; }
  async resolveBatch(requests: readonly DisplayReferenceRequest[]): Promise<readonly DisplayProjectionEntry[]> {
    this.calls.push([...requests]);
    return this.entries;
  }
}

test("resolver performs one deduplicated query and preserves request order", async () => {
  const port = new FakeDisplayProjection([
    { category: "document", identity: "doc-secret-2", title: "같은 제목", breadcrumb: ["프로젝트", "둘째"], freshness: "ready" },
    { category: "document", identity: "doc-secret-1", title: "같은 제목", breadcrumb: ["프로젝트", "첫째"], freshness: "ready" },
  ]);
  const requests = [
    { category: "document", identity: "doc-secret-1" },
    { category: "document", identity: "doc-secret-2" },
    { category: "document", identity: "doc-secret-1" },
  ] as const;
  const result = await resolveDisplayReferences(port, requests, createKoKrDisplayFallbackPolicy());

  assert.equal(port.calls.length, 1);
  assert.deepEqual(port.calls[0], requests.slice(0, 2));
  assert.deepEqual(result.map((item) => item.identity), requests.map((item) => item.identity));
  assert.deepEqual(result.map((item) => item.label), ["같은 제목", "같은 제목", "같은 제목"]);
  assert.deepEqual(result.map((item) => item.breadcrumbLabel), ["프로젝트 / 첫째", "프로젝트 / 둘째", "프로젝트 / 첫째"]);
});

test("empty missing and stale projection entries never expose internal identity", async () => {
  const identities = ["doc-secret-empty", "doc-secret-missing", "asset-secret-stale"] as const;
  const port = new FakeDisplayProjection([
    { category: "document", identity: identities[0], title: "   ", freshness: "ready" },
    { category: "asset", identity: identities[2], title: "설계 자료.pdf", breadcrumb: ["첨부 파일"], freshness: "stale" },
    { category: "document", identity: "not-requested", title: "무시할 결과", freshness: "ready" },
  ]);
  const result = await resolveDisplayReferences(port, [
    { category: "document", identity: identities[0] },
    { category: "document", identity: identities[1] },
    { category: "asset", identity: identities[2] },
  ], createKoKrDisplayFallbackPolicy());

  assert.deepEqual(result.map((item) => [item.label, item.state]), [
    ["제목 없는 문서", "resolved"],
    ["찾을 수 없는 문서", "missing"],
    ["설계 자료.pdf", "stale"],
  ]);
  const visible = result.map((item) => `${item.label} ${item.breadcrumbLabel} ${item.statusLabel}`).join(" ");
  for (const identity of identities) assert.equal(visible.includes(identity), false);
  assert.match(result[2].statusLabel, /최신 정보 확인 필요/);
});

test("resolver rejects blank identity before calling the projection port", async () => {
  const port = new FakeDisplayProjection([]);
  await assert.rejects(
    resolveDisplayReferences(port, [{ category: "canvas", identity: " " }], createKoKrDisplayFallbackPolicy()),
    /DISPLAY_IDENTITY_INVALID/,
  );
  assert.equal(port.calls.length, 0);
});

test("empty request list returns an immutable empty result without I/O", async () => {
  const port = new FakeDisplayProjection([]);
  const result = await resolveDisplayReferences(port, [], createKoKrDisplayFallbackPolicy());
  assert.deepEqual(result, []);
  assert.equal(Object.isFrozen(result), true);
  assert.equal(port.calls.length, 0);
});
