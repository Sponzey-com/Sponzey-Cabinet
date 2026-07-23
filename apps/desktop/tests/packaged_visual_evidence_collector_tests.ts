import assert from "node:assert/strict"
import test from "node:test"

import {
  collectPackagedVisualEvidence,
  collectPackagedVisualReceipts,
  PackagedVisualCollectionError,
  type PackagedVisualCollectionPort,
} from "../src/packaged_visual_evidence_collector.ts"
import type {
  RendererVisualMeasurement,
  ViewportVisualMeasurement,
  VisualEvidencePolicy,
  VisualRoute,
  VisualViewport,
} from "../src/viewport_visual_evidence_contract.ts"

const digest = (digit: string): string => digit.repeat(64)
const policy: VisualEvidencePolicy = {
  routes: ["Home", "Graph", "Canvas"],
  viewports: [
    { width: 760, height: 640, zoomPercent: 100 },
    { width: 760, height: 640, zoomPercent: 200 },
  ],
  rendererRoutes: ["Graph", "Canvas"],
  minimumNonBackgroundPixels: 4,
  requiredActionsByRoute: {
    Home: [], Document: [], Graph: [], Canvas: [], Assets: [], Backup: [],
  },
}

class FakePort implements PackagedVisualCollectionPort {
  readonly calls: string[] = []
  failRoute?: VisualRoute

  async configureViewport(viewport: VisualViewport): Promise<void> {
    this.calls.push(`viewport:${viewport.width}x${viewport.height}@${viewport.zoomPercent}`)
  }

  async openRoute(route: VisualRoute): Promise<void> {
    this.calls.push(`route:${route}`)
    if (route === this.failRoute) throw new Error("raw failure")
  }

  async measureViewport(route: VisualRoute, viewport: VisualViewport): Promise<ViewportVisualMeasurement> {
    this.calls.push(`measure:${route}`)
    return {
      route,
      viewport,
      bodyScrollWidth: viewport.width,
      bodyClientWidth: viewport.width,
      shellBounds: { x: 0, y: 0, width: viewport.width, height: viewport.height },
      visibleActions: [{ actionId: `${route}-action`, bounds: { x: 10, y: 10, width: 100, height: 30 } }],
      focusTargetCount: 1,
      artifactDigest: digest("a"),
    }
  }

  async measureRenderer(route: "Graph" | "Canvas", viewport: VisualViewport): Promise<RendererVisualMeasurement> {
    this.calls.push(`renderer:${route}`)
    return {
      route,
      viewport,
      canvasBounds: { x: 0, y: 50, width: viewport.width, height: viewport.height - 50 },
      sampledPixelCount: 100,
      nonBackgroundPixelCount: 20,
      semanticFallbackCount: 2,
      safeLabelsVerified: true,
      artifactDigest: digest("a"),
    }
  }
}

test("collector follows viewport then route order and creates complete evidence", async () => {
  const port = new FakePort()
  const result = await collectPackagedVisualEvidence({
    sourceFingerprint: digest("b"),
    appFingerprint: digest("c"),
    policy,
    port,
  })

  assert.deepEqual(result.evidence, {
    status: "Passed",
    sourceFingerprint: digest("b"),
    appFingerprint: digest("c"),
    routeViewportCount: 6,
    rendererViewportCount: 4,
    artifactCount: 10,
  })
  assert.equal(result.viewportReceipts.length, 6)
  assert.equal(result.rendererReceipts.length, 4)
  assert.deepEqual(port.calls.slice(0, 8), [
    "viewport:760x640@100",
    "route:Home", "measure:Home",
    "route:Graph", "measure:Graph", "renderer:Graph",
    "route:Canvas", "measure:Canvas",
  ])
})

test("collector stops at the first route failure with a stable code", async () => {
  const port = new FakePort()
  port.failRoute = "Graph"
  await assert.rejects(
    collectPackagedVisualEvidence({
      sourceFingerprint: digest("b"), appFingerprint: digest("c"), policy, port,
    }),
    (error: unknown) => error instanceof PackagedVisualCollectionError
      && error.code === "PACKAGED_VISUAL_ROUTE_FAILED"
      && error.stage === "Graph:760x640@100",
  )
  assert.equal(port.calls.includes("route:Canvas"), false)
})

test("collector maps invalid measurements without exposing raw errors", async () => {
  const port = new FakePort()
  port.measureViewport = async (route, viewport) => ({
    route, viewport,
    bodyScrollWidth: viewport.width + 1,
    bodyClientWidth: viewport.width,
    shellBounds: { x: 0, y: 0, width: viewport.width, height: viewport.height },
    visibleActions: [],
    focusTargetCount: 1,
    artifactDigest: digest("a"),
  })
  await assert.rejects(
    collectPackagedVisualEvidence({
      sourceFingerprint: digest("b"), appFingerprint: digest("c"), policy, port,
    }),
    (error: unknown) => error instanceof PackagedVisualCollectionError
      && error.code === "PACKAGED_VISUAL_MEASUREMENT_FAILED"
      && error.message === "PACKAGED_VISUAL_MEASUREMENT_FAILED",
  )
})

test("collector fails closed when a visual boundary never settles", async () => {
  const port = new FakePort()
  port.openRoute = async () => new Promise<void>(() => undefined)

  await assert.rejects(
    collectPackagedVisualReceipts(policy, port, { operationTimeoutMs: 10 }),
    (error: unknown) => error instanceof PackagedVisualCollectionError
      && error.code === "PACKAGED_VISUAL_OPERATION_TIMEOUT"
      && error.stage === "Home:760x640@100:route",
  )
})
