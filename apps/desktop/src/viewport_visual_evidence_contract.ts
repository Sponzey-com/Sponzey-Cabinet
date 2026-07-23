const SHA256_PATTERN = /^[a-f0-9]{64}$/

export type VisualRoute = "Home" | "Document" | "Graph" | "Canvas" | "Assets" | "Backup"

export interface VisualViewport {
  readonly width: number
  readonly height: number
  readonly zoomPercent: number
}

export interface VisualRect {
  readonly x: number
  readonly y: number
  readonly width: number
  readonly height: number
}

export interface VisualEvidencePolicy {
  readonly routes: readonly VisualRoute[]
  readonly viewports: readonly VisualViewport[]
  readonly rendererRoutes: readonly Extract<VisualRoute, "Graph" | "Canvas">[]
  readonly minimumNonBackgroundPixels: number
  readonly requiredActionsByRoute: Readonly<Record<VisualRoute, readonly string[]>>
}

export interface ViewportVisualMeasurement {
  readonly route: VisualRoute
  readonly viewport: VisualViewport
  readonly bodyScrollWidth: number
  readonly bodyClientWidth: number
  readonly shellBounds: VisualRect
  readonly visibleActions: readonly Readonly<{ actionId: string; bounds: VisualRect }>[]
  readonly focusTargetCount: number
  readonly artifactDigest: string
}

export interface RendererVisualMeasurement {
  readonly route: "Graph" | "Canvas"
  readonly viewport: VisualViewport
  readonly canvasBounds: VisualRect
  readonly sampledPixelCount: number
  readonly nonBackgroundPixelCount: number
  readonly semanticFallbackCount: number
  readonly safeLabelsVerified: boolean
  readonly artifactDigest: string
}

export type ViewportVisualReceipt = Readonly<{
  kind: "ViewportPassed"
  route: VisualRoute
  viewportKey: string
  visibleActionCount: number
  focusTargetCount: number
  artifactDigest: string
}>

export type RendererVisualReceipt = Readonly<{
  kind: "RendererPassed"
  route: "Graph" | "Canvas"
  viewportKey: string
  sampledPixelCount: number
  nonBackgroundPixelCount: number
  semanticFallbackCount: number
  artifactDigest: string
}>

export class VisualEvidenceError extends Error {
  readonly code: string

  constructor(code: string) {
    super(code)
    this.name = "VisualEvidenceError"
    this.code = code
  }
}

const fail = (code: string): never => {
  throw new VisualEvidenceError(code)
}

const viewportKey = (viewport: VisualViewport): string =>
  `${viewport.width}x${viewport.height}@${viewport.zoomPercent}`

const validateDigest = (digest: string): string => {
  if (!SHA256_PATTERN.test(digest)) fail("VISUAL_ARTIFACT_DIGEST_INVALID")
  return digest
}

const validateFingerprint = (digest: string): string => {
  if (!SHA256_PATTERN.test(digest)) fail("VISUAL_FINGERPRINT_INVALID")
  return digest
}

const isFiniteNonNegativeInteger = (value: number): boolean =>
  Number.isSafeInteger(value) && value >= 0

const validateRect = (rect: VisualRect): void => {
  if (![rect.x, rect.y, rect.width, rect.height].every(Number.isFinite) || rect.width <= 0 || rect.height <= 0) {
    fail("VISUAL_GEOMETRY_INVALID")
  }
}

const rectInside = (inner: VisualRect, outer: VisualRect): boolean =>
  inner.x >= outer.x && inner.y >= outer.y
  && inner.x + inner.width <= outer.x + outer.width
  && inner.y + inner.height <= outer.y + outer.height

const rectanglesOverlap = (left: VisualRect, right: VisualRect): boolean =>
  left.x < right.x + right.width
  && left.x + left.width > right.x
  && left.y < right.y + right.height
  && left.y + left.height > right.y

const requireSupportedViewport = (viewport: VisualViewport, policy: VisualEvidencePolicy): string => {
  const key = viewportKey(viewport)
  if (!policy.viewports.some((candidate) => viewportKey(candidate) === key)) {
    fail("VISUAL_VIEWPORT_UNSUPPORTED")
  }
  return key
}

export const validateViewportVisualMeasurement = (
  measurement: ViewportVisualMeasurement,
  policy: VisualEvidencePolicy,
): ViewportVisualReceipt => {
  if (!policy.routes.includes(measurement.route)) fail("VISUAL_ROUTE_UNSUPPORTED")
  const key = requireSupportedViewport(measurement.viewport, policy)
  validateDigest(measurement.artifactDigest)
  validateRect(measurement.shellBounds)
  const viewportRect = { x: 0, y: 0, width: measurement.viewport.width, height: measurement.viewport.height }
  if (!rectInside(measurement.shellBounds, viewportRect)) fail("VISUAL_SHELL_CLIPPED")
  if (!isFiniteNonNegativeInteger(measurement.bodyClientWidth)
    || !isFiniteNonNegativeInteger(measurement.bodyScrollWidth)) {
    fail("VISUAL_GEOMETRY_INVALID")
  }
  if (measurement.bodyClientWidth !== measurement.viewport.width
    || measurement.bodyScrollWidth > measurement.bodyClientWidth) {
    fail("VISUAL_BODY_OVERFLOW")
  }
  if (!isFiniteNonNegativeInteger(measurement.focusTargetCount) || measurement.focusTargetCount === 0) {
    fail("VISUAL_FOCUS_TARGET_MISSING")
  }

  const actionIds = new Set<string>()
  for (const action of measurement.visibleActions) {
    if (!action.actionId.trim() || actionIds.has(action.actionId)) fail("VISUAL_ACTION_ID_INVALID")
    actionIds.add(action.actionId)
    validateRect(action.bounds)
    if (!rectInside(action.bounds, viewportRect) || !rectInside(action.bounds, measurement.shellBounds)) {
      fail("VISUAL_ACTION_CLIPPED")
    }
  }
  for (const requiredAction of policy.requiredActionsByRoute[measurement.route]) {
    if (![...actionIds].some((actionId) => actionId === requiredAction || actionId.startsWith(`${requiredAction}-`))) {
      fail("VISUAL_REQUIRED_ACTION_MISSING")
    }
  }
  for (let left = 0; left < measurement.visibleActions.length; left += 1) {
    for (let right = left + 1; right < measurement.visibleActions.length; right += 1) {
      if (rectanglesOverlap(measurement.visibleActions[left].bounds, measurement.visibleActions[right].bounds)) {
        fail("VISUAL_ACTION_OVERLAP")
      }
    }
  }

  return Object.freeze({
    kind: "ViewportPassed",
    route: measurement.route,
    viewportKey: key,
    visibleActionCount: measurement.visibleActions.length,
    focusTargetCount: measurement.focusTargetCount,
    artifactDigest: measurement.artifactDigest,
  })
}

export const validateRendererVisualMeasurement = (
  measurement: RendererVisualMeasurement,
  policy: VisualEvidencePolicy,
): RendererVisualReceipt => {
  if (!policy.rendererRoutes.includes(measurement.route)) fail("VISUAL_RENDERER_ROUTE_UNSUPPORTED")
  const key = requireSupportedViewport(measurement.viewport, policy)
  validateDigest(measurement.artifactDigest)
  validateRect(measurement.canvasBounds)
  const viewportRect = { x: 0, y: 0, width: measurement.viewport.width, height: measurement.viewport.height }
  if (!rectInside(measurement.canvasBounds, viewportRect)) fail("VISUAL_RENDERER_CLIPPED")
  if (!isFiniteNonNegativeInteger(measurement.sampledPixelCount)
    || measurement.sampledPixelCount === 0
    || measurement.nonBackgroundPixelCount > measurement.sampledPixelCount) {
    fail("VISUAL_PIXEL_SAMPLE_INVALID")
  }
  if (!isFiniteNonNegativeInteger(measurement.nonBackgroundPixelCount)
    || measurement.nonBackgroundPixelCount < policy.minimumNonBackgroundPixels) {
    fail("VISUAL_RENDERER_BLANK")
  }
  if (!isFiniteNonNegativeInteger(measurement.semanticFallbackCount) || measurement.semanticFallbackCount === 0) {
    fail("VISUAL_RENDERER_SEMANTIC_FALLBACK_MISSING")
  }
  if (!measurement.safeLabelsVerified) fail("VISUAL_RENDERER_UNSAFE_LABEL")

  return Object.freeze({
    kind: "RendererPassed",
    route: measurement.route,
    viewportKey: key,
    sampledPixelCount: measurement.sampledPixelCount,
    nonBackgroundPixelCount: measurement.nonBackgroundPixelCount,
    semanticFallbackCount: measurement.semanticFallbackCount,
    artifactDigest: measurement.artifactDigest,
  })
}

export const createViewportVisualEvidence = (input: Readonly<{
  sourceFingerprint: string
  appFingerprint: string
  policy: VisualEvidencePolicy
  viewportReceipts: readonly ViewportVisualReceipt[]
  rendererReceipts: readonly RendererVisualReceipt[]
}>): Readonly<{
  status: "Passed"
  sourceFingerprint: string
  appFingerprint: string
  routeViewportCount: number
  rendererViewportCount: number
  artifactCount: number
}> => {
  const sourceFingerprint = validateFingerprint(input.sourceFingerprint)
  const appFingerprint = validateFingerprint(input.appFingerprint)
  const expectedViewportKeys = input.policy.routes.flatMap((route) =>
    input.policy.viewports.map((viewport) => `${route}:${viewportKey(viewport)}`))
  const expectedRendererKeys = input.policy.rendererRoutes.flatMap((route) =>
    input.policy.viewports.map((viewport) => `${route}:${viewportKey(viewport)}`))
  const viewportKeys = input.viewportReceipts.map((receipt) => `${receipt.route}:${receipt.viewportKey}`)
  const rendererKeys = input.rendererReceipts.map((receipt) => `${receipt.route}:${receipt.viewportKey}`)

  if (new Set(viewportKeys).size !== viewportKeys.length || new Set(rendererKeys).size !== rendererKeys.length) {
    fail("VISUAL_EVIDENCE_DUPLICATE")
  }
  const sameSet = (actual: readonly string[], expected: readonly string[]): boolean =>
    actual.length === expected.length && expected.every((key) => actual.includes(key))
  if (!sameSet(viewportKeys, expectedViewportKeys) || !sameSet(rendererKeys, expectedRendererKeys)) {
    fail("VISUAL_EVIDENCE_INCOMPLETE")
  }
  for (const receipt of [...input.viewportReceipts, ...input.rendererReceipts]) {
    validateDigest(receipt.artifactDigest)
  }

  return Object.freeze({
    status: "Passed",
    sourceFingerprint,
    appFingerprint,
    routeViewportCount: input.viewportReceipts.length,
    rendererViewportCount: input.rendererReceipts.length,
    artifactCount: input.viewportReceipts.length + input.rendererReceipts.length,
  })
}
