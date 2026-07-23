import assert from "node:assert/strict"
import test from "node:test"

import {
  VisualEvidenceError,
  createViewportVisualEvidence,
  validateRendererVisualMeasurement,
  validateViewportVisualMeasurement,
  type VisualEvidencePolicy,
  type ViewportVisualMeasurement,
} from "../src/viewport_visual_evidence_contract.ts"

const hash = (digit: string): string => digit.repeat(64)
const routes = ["Home", "Document", "Graph", "Canvas", "Assets", "Backup"] as const
const viewports = [
  { width: 1440, height: 900, zoomPercent: 100 },
  { width: 1180, height: 800, zoomPercent: 100 },
  { width: 960, height: 720, zoomPercent: 100 },
  { width: 760, height: 640, zoomPercent: 100 },
  { width: 760, height: 640, zoomPercent: 200 },
] as const

const policy: VisualEvidencePolicy = Object.freeze({
  routes,
  viewports,
  rendererRoutes: ["Graph", "Canvas"],
  minimumNonBackgroundPixels: 8,
  requiredActionsByRoute: {
    Home: ["first"], Document: [], Graph: [], Canvas: [], Assets: [], Backup: [],
  },
})

const measurement = (
  route: (typeof routes)[number] = "Home",
  viewport = viewports[0],
  overrides: Partial<ViewportVisualMeasurement> = {},
): ViewportVisualMeasurement => ({
  route,
  viewport,
  bodyScrollWidth: viewport.width,
  bodyClientWidth: viewport.width,
  shellBounds: { x: 0, y: 0, width: viewport.width, height: viewport.height },
  visibleActions: [
    { actionId: "first", bounds: { x: 20, y: 20, width: 80, height: 32 } },
    { actionId: "second", bounds: { x: 110, y: 20, width: 80, height: 32 } },
  ],
  focusTargetCount: 2,
  artifactDigest: hash("a"),
  ...overrides,
})

const expectCode = (operation: () => unknown, code: string): void => {
  assert.throws(operation, (error: unknown) => {
    assert.ok(error instanceof VisualEvidenceError)
    assert.equal(error.code, code)
    return true
  })
}

test("viewport measurement accepts bounded actions and returns path-free receipt", () => {
  const result = validateViewportVisualMeasurement(measurement(), policy)
  assert.deepEqual(result, {
    kind: "ViewportPassed",
    route: "Home",
    viewportKey: "1440x900@100",
    visibleActionCount: 2,
    focusTargetCount: 2,
    artifactDigest: hash("a"),
  })
  assert.equal(Object.isFrozen(result), true)
})

test("viewport measurement rejects unsupported geometry overflow clipping overlap and missing focus", () => {
  expectCode(() => validateViewportVisualMeasurement(measurement("Home", { width: 800, height: 600, zoomPercent: 100 }), policy), "VISUAL_VIEWPORT_UNSUPPORTED")
  expectCode(() => validateViewportVisualMeasurement(measurement("Home", viewports[0], { bodyScrollWidth: 1441 }), policy), "VISUAL_BODY_OVERFLOW")
  expectCode(() => validateViewportVisualMeasurement(measurement("Home", viewports[0], {
    visibleActions: [{ actionId: "clipped", bounds: { x: 1400, y: 20, width: 80, height: 32 } }],
  }), policy), "VISUAL_ACTION_CLIPPED")
  expectCode(() => validateViewportVisualMeasurement(measurement("Home", viewports[0], {
    visibleActions: [
      { actionId: "first", bounds: { x: 20, y: 20, width: 80, height: 32 } },
      { actionId: "second", bounds: { x: 50, y: 20, width: 80, height: 32 } },
    ],
  }), policy), "VISUAL_ACTION_OVERLAP")
  expectCode(() => validateViewportVisualMeasurement(measurement("Home", viewports[0], { focusTargetCount: 0 }), policy), "VISUAL_FOCUS_TARGET_MISSING")
  expectCode(() => validateViewportVisualMeasurement(measurement("Home", viewports[0], {
    visibleActions: [{ actionId: "second", bounds: { x: 110, y: 20, width: 80, height: 32 } }],
  }), policy), "VISUAL_REQUIRED_ACTION_MISSING")
})

test("renderer measurement rejects blank out-of-bounds and unsafe evidence", () => {
  const validMeasurement = {
    route: "Graph",
    viewport: viewports[0],
    canvasBounds: { x: 244, y: 50, width: 1196, height: 850 },
    sampledPixelCount: 100,
    nonBackgroundPixelCount: 20,
    semanticFallbackCount: 2,
    safeLabelsVerified: true,
    artifactDigest: hash("b"),
  } as const
  const passed = validateRendererVisualMeasurement(validMeasurement, policy)
  assert.equal(passed.kind, "RendererPassed")

  expectCode(() => validateRendererVisualMeasurement({ ...validMeasurement, sampledPixelCount: 0 }, policy), "VISUAL_PIXEL_SAMPLE_INVALID")
  expectCode(() => validateRendererVisualMeasurement({ ...validMeasurement, nonBackgroundPixelCount: 0 }, policy), "VISUAL_RENDERER_BLANK")
  expectCode(() => validateRendererVisualMeasurement({ ...validMeasurement, canvasBounds: { x: 1400, y: 50, width: 100, height: 100 } }, policy), "VISUAL_RENDERER_CLIPPED")
  expectCode(() => validateRendererVisualMeasurement({ ...validMeasurement, safeLabelsVerified: false }, policy), "VISUAL_RENDERER_UNSAFE_LABEL")
})

test("aggregate requires every route viewport and renderer combination exactly once", () => {
  const viewportReceipts = routes.flatMap((route) => viewports.map((viewport) =>
    validateViewportVisualMeasurement(measurement(route, viewport), policy)
  ))
  const rendererReceipts = (["Graph", "Canvas"] as const).flatMap((route) => viewports.map((viewport) =>
    validateRendererVisualMeasurement({
      route,
      viewport,
      canvasBounds: { x: 244, y: 50, width: viewport.width - 244, height: viewport.height - 50 },
      sampledPixelCount: 100,
      nonBackgroundPixelCount: 20,
      semanticFallbackCount: 2,
      safeLabelsVerified: true,
      artifactDigest: hash(route === "Graph" ? "b" : "c"),
    }, policy)
  ))

  const evidence = createViewportVisualEvidence({
    sourceFingerprint: hash("d"),
    appFingerprint: hash("e"),
    policy,
    viewportReceipts,
    rendererReceipts,
  })
  assert.deepEqual(evidence, {
    status: "Passed",
    sourceFingerprint: hash("d"),
    appFingerprint: hash("e"),
    routeViewportCount: 30,
    rendererViewportCount: 10,
    artifactCount: 40,
  })
  assert.equal(Object.isFrozen(evidence), true)

  expectCode(() => createViewportVisualEvidence({
    sourceFingerprint: hash("d"), appFingerprint: hash("e"), policy,
    viewportReceipts: viewportReceipts.slice(1), rendererReceipts,
  }), "VISUAL_EVIDENCE_INCOMPLETE")
  expectCode(() => createViewportVisualEvidence({
    sourceFingerprint: hash("d"), appFingerprint: hash("e"), policy,
    viewportReceipts: [...viewportReceipts, viewportReceipts[0]], rendererReceipts,
  }), "VISUAL_EVIDENCE_DUPLICATE")
  expectCode(() => createViewportVisualEvidence({
    sourceFingerprint: "stale", appFingerprint: hash("e"), policy,
    viewportReceipts, rendererReceipts,
  }), "VISUAL_FINGERPRINT_INVALID")
})
