const SHA256_PATTERN = /^[a-f0-9]{64}$/;

export type AccessibilityEvidence = Readonly<{
  status: "Passed";
  sourceFingerprint: string;
  appFingerprint: string;
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

export class AccessibilityEvidenceError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.name = "AccessibilityEvidenceError";
    this.code = code;
  }
}

export function createAccessibilityEvidence(input: Readonly<{
  sourceFingerprint: string;
  appFingerprint: string;
  policy: Readonly<{
    requiredRouteFocusCount: number;
    requiredTextZoomPercent: number;
    minimumKeyboardJourneyCount: number;
    minimumFocusRestorationCount: number;
  }>;
  measurement: Readonly<{
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
}>): AccessibilityEvidence {
  if (!SHA256_PATTERN.test(input.sourceFingerprint) || !SHA256_PATTERN.test(input.appFingerprint)) {
    fail("ACCESSIBILITY_FINGERPRINT_INVALID");
  }
  validateAccessibilityMeasurement(input.policy, input.measurement);
  return Object.freeze({
    status: "Passed",
    sourceFingerprint: input.sourceFingerprint,
    appFingerprint: input.appFingerprint,
    ...input.measurement,
  });
}

export function validateAccessibilityMeasurement(
  policy: Readonly<{
    requiredRouteFocusCount: number;
    requiredTextZoomPercent: number;
    minimumKeyboardJourneyCount: number;
    minimumFocusRestorationCount: number;
  }>,
  measurement: Readonly<{
    routeFocusCount: number;
    keyboardJourneyCount: number;
    focusRestorationCount: number;
    visibleControlCount: number;
    namedControlCount: number;
    textZoomPercent: number;
    keyboardErrorCount: number;
    focusErrorCount: number;
    internalExposureCount: number;
  }>,
): void {
  const policyValues = Object.values(policy);
  const measurementValues = Object.values(measurement);
  if (!policyValues.every((value) => Number.isSafeInteger(value) && value > 0)
    || !measurementValues.every((value) => Number.isSafeInteger(value) && value >= 0)
    || measurement.visibleControlCount === 0) {
    fail("ACCESSIBILITY_MEASUREMENT_INVALID");
  }
  if (measurement.routeFocusCount !== policy.requiredRouteFocusCount) {
    fail("ACCESSIBILITY_ROUTE_FOCUS_INCOMPLETE");
  }
  if (measurement.textZoomPercent !== policy.requiredTextZoomPercent) {
    fail("ACCESSIBILITY_TEXT_ZOOM_INCOMPLETE");
  }
  if (measurement.visibleControlCount !== measurement.namedControlCount) {
    fail("ACCESSIBILITY_CONTROL_NAME_INCOMPLETE");
  }
  if (measurement.keyboardJourneyCount < policy.minimumKeyboardJourneyCount) {
    fail("ACCESSIBILITY_KEYBOARD_COVERAGE_INCOMPLETE");
  }
  if (measurement.focusRestorationCount < policy.minimumFocusRestorationCount) {
    fail("ACCESSIBILITY_FOCUS_RESTORATION_INCOMPLETE");
  }
  if (measurement.keyboardErrorCount !== 0
    || measurement.focusErrorCount !== 0
    || measurement.internalExposureCount !== 0) {
    fail("ACCESSIBILITY_ERROR_REPORTED");
  }
}

function fail(code: string): never {
  throw new AccessibilityEvidenceError(code);
}
