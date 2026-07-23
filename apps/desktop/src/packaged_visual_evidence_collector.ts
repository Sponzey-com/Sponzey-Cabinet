import {
  createViewportVisualEvidence,
  validateRendererVisualMeasurement,
  validateViewportVisualMeasurement,
  type RendererVisualMeasurement,
  type RendererVisualReceipt,
  type ViewportVisualMeasurement,
  type ViewportVisualReceipt,
  type VisualEvidencePolicy,
  type VisualRoute,
  type VisualViewport,
  VisualEvidenceError,
} from "./viewport_visual_evidence_contract.ts"

export interface PackagedVisualCollectionPort {
  configureViewport(viewport: VisualViewport): Promise<void>
  openRoute(route: VisualRoute): Promise<void>
  measureViewport(route: VisualRoute, viewport: VisualViewport): Promise<ViewportVisualMeasurement>
  measureRenderer(route: "Graph" | "Canvas", viewport: VisualViewport): Promise<RendererVisualMeasurement>
}

export interface PackagedVisualCollectionTiming {
  readonly operationTimeoutMs: number
}

export class PackagedVisualCollectionError extends Error {
  readonly code: string
  readonly stage: string
  readonly detailCode?: string

  constructor(code: string, stage: string, detailCode?: string) {
    super(code)
    this.name = "PackagedVisualCollectionError"
    this.code = code
    this.stage = stage
    this.detailCode = detailCode
  }
}

const key = (viewport: VisualViewport): string =>
  `${viewport.width}x${viewport.height}@${viewport.zoomPercent}`

export const collectPackagedVisualEvidence = async (input: Readonly<{
  sourceFingerprint: string
  appFingerprint: string
  policy: VisualEvidencePolicy
  port: PackagedVisualCollectionPort
}>): Promise<Readonly<{
  evidence: ReturnType<typeof createViewportVisualEvidence>
  viewportReceipts: readonly ViewportVisualReceipt[]
  rendererReceipts: readonly RendererVisualReceipt[]
}>> => {
  const receipts = await collectPackagedVisualReceipts(input.policy, input.port)
  return Object.freeze({
    evidence: createViewportVisualEvidence({
      sourceFingerprint: input.sourceFingerprint,
      appFingerprint: input.appFingerprint,
      policy: input.policy,
      viewportReceipts: receipts.viewportReceipts,
      rendererReceipts: receipts.rendererReceipts,
    }),
    ...receipts,
  })
}

export const collectPackagedVisualReceipts = async (
  policy: VisualEvidencePolicy,
  port: PackagedVisualCollectionPort,
  timing: PackagedVisualCollectionTiming = { operationTimeoutMs: 10_000 },
): Promise<Readonly<{
  viewportReceipts: readonly ViewportVisualReceipt[]
  rendererReceipts: readonly RendererVisualReceipt[]
}>> => {
  const viewportReceipts: ViewportVisualReceipt[] = []
  const rendererReceipts: RendererVisualReceipt[] = []

  for (const viewport of policy.viewports) {
    const viewportStage = key(viewport)
    try {
      await boundedVisualOperation(
        port.configureViewport(viewport),
        timing.operationTimeoutMs,
        `${viewportStage}:viewport`,
      )
    } catch (error) {
      if (error instanceof PackagedVisualCollectionError) throw error
      throw new PackagedVisualCollectionError("PACKAGED_VISUAL_VIEWPORT_FAILED", viewportStage)
    }

    for (const route of policy.routes) {
      const stage = `${route}:${viewportStage}`
      try {
        await boundedVisualOperation(
          port.openRoute(route),
          timing.operationTimeoutMs,
          `${stage}:route`,
        )
      } catch (error) {
        if (error instanceof PackagedVisualCollectionError) throw error
        const detailCode = error instanceof Error && /^[A-Z0-9_]+$/.test(error.message)
          ? error.message
          : undefined
        throw new PackagedVisualCollectionError("PACKAGED_VISUAL_ROUTE_FAILED", stage, detailCode)
      }
      try {
        viewportReceipts.push(validateViewportVisualMeasurement(
          await boundedVisualOperation(
            port.measureViewport(route, viewport),
            timing.operationTimeoutMs,
            `${stage}:viewport-measurement`,
          ),
          policy,
        ))
        if (policy.rendererRoutes.includes(route as "Graph" | "Canvas")) {
          rendererReceipts.push(validateRendererVisualMeasurement(
            await boundedVisualOperation(
              port.measureRenderer(route as "Graph" | "Canvas", viewport),
              timing.operationTimeoutMs,
              `${stage}:renderer-measurement`,
            ),
            policy,
          ))
        }
      } catch (error) {
        if (error instanceof PackagedVisualCollectionError) throw error
        throw new PackagedVisualCollectionError(
          "PACKAGED_VISUAL_MEASUREMENT_FAILED",
          stage,
          error instanceof VisualEvidenceError ? error.code : undefined,
        )
      }
    }
  }

  return Object.freeze({
    viewportReceipts: Object.freeze([...viewportReceipts]),
    rendererReceipts: Object.freeze([...rendererReceipts]),
  })
}

async function boundedVisualOperation<T>(
  operation: Promise<T>,
  timeoutMs: number,
  stage: string,
): Promise<T> {
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) {
    throw new PackagedVisualCollectionError("PACKAGED_VISUAL_TIMEOUT_INVALID", stage)
  }
  let timer: ReturnType<typeof setTimeout> | undefined
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new PackagedVisualCollectionError(
      "PACKAGED_VISUAL_OPERATION_TIMEOUT",
      stage,
    )), timeoutMs)
  })
  try {
    return await Promise.race([operation, timeout])
  } finally {
    if (timer !== undefined) clearTimeout(timer)
  }
}
