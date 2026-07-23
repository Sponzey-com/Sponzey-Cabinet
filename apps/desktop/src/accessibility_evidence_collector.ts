import {
  createAccessibilityEvidence,
  type AccessibilityEvidence,
} from "./accessibility_evidence_contract.ts";

const PRIMARY_ROUTES = Object.freeze([
  "Home", "Document", "Graph", "Canvas", "Assets", "Backup",
] as const);

export type AccessibilityRoute = typeof PRIMARY_ROUTES[number];

export type AccessibilityRouteMeasurement = Readonly<{
  route: AccessibilityRoute;
  visibleControlCount: number;
  namedControlCount: number;
  mainFocusReached: boolean;
  keyboardJourneyPassed: boolean;
  focusRestorationCount: number;
  keyboardErrorCount: number;
  focusErrorCount: number;
  internalExposureCount: number;
}>;

export type AccessibilityAggregateMeasurement = Readonly<{
  routeFocusCount: number;
  keyboardJourneyCount: number;
  focusRestorationCount: number;
  visibleControlCount: number;
  namedControlCount: number;
  textZoomPercent: number;
  keyboardErrorCount: number;
  focusErrorCount: number;
  internalExposureCount: number;
}>;

export interface AccessibilityMeasurementPort {
  measureRoute(route: AccessibilityRoute, textZoomPercent: number): Promise<AccessibilityRouteMeasurement>;
}

export interface AccessibilityCollectionTiming {
  run<T>(operation: () => Promise<T>, timeoutMs: number): Promise<T>;
}

export class AccessibilityEvidenceCollectorError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "AccessibilityEvidenceCollectorError";
    this.code = code;
  }
}

export function createAccessibilityCollectionTiming(): AccessibilityCollectionTiming {
  return Object.freeze({
    async run<T>(operation: () => Promise<T>, timeoutMs: number): Promise<T> {
      let timer: ReturnType<typeof setTimeout> | undefined;
      try {
        return await Promise.race([
          operation(),
          new Promise<T>((_resolve, reject) => {
            timer = setTimeout(
              () => reject(new AccessibilityEvidenceCollectorError("ACCESSIBILITY_COLLECTION_TIMEOUT")),
              timeoutMs,
            );
          }),
        ]);
      } finally {
        if (timer !== undefined) clearTimeout(timer);
      }
    },
  });
}

export async function collectAccessibilityEvidence(input: Readonly<{
  sourceFingerprint: string;
  appFingerprint: string;
  policy: Readonly<{
    routes: readonly AccessibilityRoute[];
    requiredTextZoomPercent: number;
    minimumKeyboardJourneyCount: number;
    minimumFocusRestorationCount: number;
    routeTimeoutMs: number;
  }>;
  timing: AccessibilityCollectionTiming;
  port: AccessibilityMeasurementPort;
}>): Promise<AccessibilityEvidence> {
  if (!sameRoutes(input.policy.routes, PRIMARY_ROUTES)
    || !Number.isSafeInteger(input.policy.routeTimeoutMs)
    || input.policy.routeTimeoutMs <= 0) {
    fail("ACCESSIBILITY_COLLECTION_POLICY_INVALID");
  }

  const measurements: AccessibilityRouteMeasurement[] = [];
  for (const route of PRIMARY_ROUTES) {
    let measurement: AccessibilityRouteMeasurement;
    try {
      measurement = await input.timing.run(
        () => input.port.measureRoute(route, input.policy.requiredTextZoomPercent),
        input.policy.routeTimeoutMs,
      );
    } catch (error) {
      if (error instanceof AccessibilityEvidenceCollectorError) throw error;
      fail("ACCESSIBILITY_COLLECTION_ROUTE_FAILED");
    }
    if (measurement.route !== route) fail("ACCESSIBILITY_COLLECTION_ROUTE_MISMATCH");
    measurements.push(measurement);
  }

  const aggregate = aggregateAccessibilityRouteMeasurements(
    measurements,
    input.policy.requiredTextZoomPercent,
  );

  return createAccessibilityEvidence({
    sourceFingerprint: input.sourceFingerprint,
    appFingerprint: input.appFingerprint,
    policy: {
      requiredRouteFocusCount: PRIMARY_ROUTES.length,
      requiredTextZoomPercent: input.policy.requiredTextZoomPercent,
      minimumKeyboardJourneyCount: input.policy.minimumKeyboardJourneyCount,
      minimumFocusRestorationCount: input.policy.minimumFocusRestorationCount,
    },
    measurement: aggregate,
  });
}

export function aggregateAccessibilityRouteMeasurements(
  measurements: readonly AccessibilityRouteMeasurement[],
  textZoomPercent: number,
): AccessibilityAggregateMeasurement {
  if (measurements.length !== PRIMARY_ROUTES.length
    || measurements.some((measurement, index) => measurement.route !== PRIMARY_ROUTES[index])) {
    fail("ACCESSIBILITY_COLLECTION_ROUTE_SET_INVALID");
  }
  const sum = (select: (measurement: AccessibilityRouteMeasurement) => number): number =>
    measurements.reduce((total, measurement) => total + select(measurement), 0);
  return Object.freeze({
    routeFocusCount: measurements.filter((measurement) => measurement.mainFocusReached).length,
    keyboardJourneyCount: measurements.filter((measurement) => measurement.keyboardJourneyPassed).length,
    focusRestorationCount: sum((measurement) => measurement.focusRestorationCount),
    visibleControlCount: sum((measurement) => measurement.visibleControlCount),
    namedControlCount: sum((measurement) => measurement.namedControlCount),
    textZoomPercent,
    keyboardErrorCount: sum((measurement) => measurement.keyboardErrorCount),
    focusErrorCount: sum((measurement) => measurement.focusErrorCount),
    internalExposureCount: sum((measurement) => measurement.internalExposureCount),
  });
}

function sameRoutes(
  actual: readonly AccessibilityRoute[],
  expected: readonly AccessibilityRoute[],
): boolean {
  return actual.length === expected.length && actual.every((route, index) => route === expected[index]);
}

function fail(code: string): never {
  throw new AccessibilityEvidenceCollectorError(code);
}
