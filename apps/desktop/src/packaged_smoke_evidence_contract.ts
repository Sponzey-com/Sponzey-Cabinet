const SHA256_PATTERN = /^[a-f0-9]{64}$/

const INITIAL_STATUS_KEY = "phase015_packaged_ui_smoke_initial"
const RESTART_STATUS_KEY = "phase015_packaged_ui_smoke_restart"

const INITIAL_WORKFLOW_KEYS = [
  "document_version_workflow_verified",
  "document_attachment_workflow_verified",
  "attachment_import_completed",
  "attachment_current_readback_verified",
  "attachment_document_readback_verified",
  "keyboard_document_workflow_verified",
  "graph_link_fixture_saved",
  "graph_local_edge_verified",
  "graph_global_edge_verified",
  "graph_safe_labels_verified",
] as const

export class PackagedSmokeEvidenceError extends Error {
  readonly code: string

  constructor(code: string) {
    super(code)
    this.name = "PackagedSmokeEvidenceError"
    this.code = code
  }
}

export type InitialPackagedSmokeResult = Readonly<{
  stage: "InitialPassed"
  profileFingerprint: string
  sampleCount: number
  p95Ms: number
  errorCount: number
  actionCount: number
  durableReadbackCount: number
  accessibilityRouteFocusCount: number
  accessibilityKeyboardJourneyCount: number
  accessibilityFocusRestorationCount: number
  accessibilityVisibleControlCount: number
  accessibilityNamedControlCount: number
  accessibilityTextZoomPercent: number
}>

export type RestartPackagedSmokeResult = Readonly<{
  stage: "RestartPassed"
  profileFingerprint: string
  errorCount: number
  attachmentRestartReadbackVerified: true
  canvasTextRestartReadbackVerified: true
}>

export type PackagedSmokeEvidence = Readonly<{
  status: "Passed"
  sourceFingerprint: string
  appFingerprint: string
  profileFingerprint: string
  sampleCount: number
  p95Ms: number
  actionCount: number
  durableReadbackCount: number
  accessibilityRouteFocusCount: number
  accessibilityKeyboardJourneyCount: number
  accessibilityFocusRestorationCount: number
  accessibilityVisibleControlCount: number
  accessibilityNamedControlCount: number
  accessibilityTextZoomPercent: number
  attachmentRestartReadbackVerified: true
  canvasTextRestartReadbackVerified: true
}>

const fail = (code: string): never => {
  throw new PackagedSmokeEvidenceError(code)
}

const validateFingerprint = (value: string): string => {
  if (!SHA256_PATTERN.test(value)) {
    fail("PACKAGED_SMOKE_FINGERPRINT_INVALID")
  }
  return value
}

const parseFields = (stdout: string, recognizedKeys: ReadonlySet<string>): ReadonlyMap<string, string> => {
  const fields = new Map<string, string>()
  for (const line of stdout.split(/\r?\n/u)) {
    const separator = line.indexOf("=")
    if (separator <= 0) continue
    const key = line.slice(0, separator).trim()
    if (!recognizedKeys.has(key)) continue
    if (fields.has(key)) fail("PACKAGED_SMOKE_FIELD_DUPLICATE")
    fields.set(key, line.slice(separator + 1).trim())
  }
  return fields
}

const required = (fields: ReadonlyMap<string, string>, key: string): string => {
  const value = fields.get(key)
  if (value === undefined || value.length === 0) {
    fail("PACKAGED_SMOKE_FIELD_MISSING")
  }
  return value
}

const parseInteger = (fields: ReadonlyMap<string, string>, key: string): number => {
  const value = required(fields, key)
  if (!/^(0|[1-9]\d*)$/u.test(value)) {
    fail("PACKAGED_SMOKE_FIELD_MALFORMED")
  }
  const parsed = Number(value)
  if (!Number.isSafeInteger(parsed)) {
    fail("PACKAGED_SMOKE_FIELD_MALFORMED")
  }
  return parsed
}

const parseDuration = (fields: ReadonlyMap<string, string>, key: string): number => {
  const value = required(fields, key)
  const parsed = Number(value)
  if (!Number.isFinite(parsed) || parsed < 0) {
    fail("PACKAGED_SMOKE_FIELD_MALFORMED")
  }
  return parsed
}

const parseBoolean = (fields: ReadonlyMap<string, string>, key: string): boolean => {
  const value = required(fields, key)
  if (value === "true") return true
  if (value === "false") return false
  return fail("PACKAGED_SMOKE_FIELD_MALFORMED")
}

export const parseInitialPackagedSmokeOutput = (
  stdout: string,
  p95BudgetMs: number,
  profileFingerprint: string,
): InitialPackagedSmokeResult => {
  if (!Number.isFinite(p95BudgetMs) || p95BudgetMs <= 0) {
    fail("PACKAGED_SMOKE_BUDGET_INVALID")
  }
  validateFingerprint(profileFingerprint)
  const recognized = new Set<string>([
    INITIAL_STATUS_KEY,
    "sample_count",
    "p95_ms",
    "error_count",
    "action_count",
    "durable_readback_count",
    "accessibility_route_focus_count",
    "accessibility_keyboard_journey_count",
    "accessibility_focus_restoration_count",
    "accessibility_visible_control_count",
    "accessibility_named_control_count",
    "accessibility_text_zoom_percent",
    "accessibility_keyboard_error_count",
    "accessibility_focus_error_count",
    "accessibility_internal_exposure_count",
    ...INITIAL_WORKFLOW_KEYS,
  ])
  const fields = parseFields(stdout, recognized)
  if (required(fields, INITIAL_STATUS_KEY) !== "passed") {
    fail("PACKAGED_SMOKE_INITIAL_FAILED")
  }

  const sampleCount = parseInteger(fields, "sample_count")
  const p95Ms = parseDuration(fields, "p95_ms")
  const errorCount = parseInteger(fields, "error_count")
  const actionCount = parseInteger(fields, "action_count")
  const durableReadbackCount = parseInteger(fields, "durable_readback_count")
  const accessibilityRouteFocusCount = parseInteger(fields, "accessibility_route_focus_count")
  const accessibilityKeyboardJourneyCount = parseInteger(fields, "accessibility_keyboard_journey_count")
  const accessibilityFocusRestorationCount = parseInteger(fields, "accessibility_focus_restoration_count")
  const accessibilityVisibleControlCount = parseInteger(fields, "accessibility_visible_control_count")
  const accessibilityNamedControlCount = parseInteger(fields, "accessibility_named_control_count")
  const accessibilityTextZoomPercent = parseInteger(fields, "accessibility_text_zoom_percent")
  const accessibilityKeyboardErrorCount = parseInteger(fields, "accessibility_keyboard_error_count")
  const accessibilityFocusErrorCount = parseInteger(fields, "accessibility_focus_error_count")
  const accessibilityInternalExposureCount = parseInteger(fields, "accessibility_internal_exposure_count")

  if (sampleCount !== 200) fail("PACKAGED_SMOKE_SAMPLE_COUNT_INVALID")
  if (errorCount !== 0) fail("PACKAGED_SMOKE_UI_ERROR_REPORTED")
  if (actionCount < 90 || durableReadbackCount < 33) {
    fail("PACKAGED_SMOKE_ACTION_COVERAGE_INCOMPLETE")
  }
  if (p95Ms > p95BudgetMs) {
    fail("PACKAGED_SMOKE_PERFORMANCE_BUDGET_EXCEEDED")
  }
  if (INITIAL_WORKFLOW_KEYS.some((key) => !parseBoolean(fields, key))) {
    fail("PACKAGED_SMOKE_WORKFLOW_EVIDENCE_MISSING")
  }
  if (accessibilityRouteFocusCount !== 6
    || accessibilityKeyboardJourneyCount !== 6
    || accessibilityFocusRestorationCount < 6
    || accessibilityVisibleControlCount === 0
    || accessibilityNamedControlCount !== accessibilityVisibleControlCount
    || accessibilityTextZoomPercent !== 200
    || accessibilityKeyboardErrorCount !== 0
    || accessibilityFocusErrorCount !== 0
    || accessibilityInternalExposureCount !== 0) {
    fail("PACKAGED_SMOKE_ACCESSIBILITY_INCOMPLETE")
  }

  return Object.freeze({
    stage: "InitialPassed",
    profileFingerprint,
    sampleCount,
    p95Ms,
    errorCount,
    actionCount,
    durableReadbackCount,
    accessibilityRouteFocusCount,
    accessibilityKeyboardJourneyCount,
    accessibilityFocusRestorationCount,
    accessibilityVisibleControlCount,
    accessibilityNamedControlCount,
    accessibilityTextZoomPercent,
  })
}

export const parseRestartPackagedSmokeOutput = (
  stdout: string,
  profileFingerprint: string,
): RestartPackagedSmokeResult => {
  validateFingerprint(profileFingerprint)
  const fields = parseFields(stdout, new Set([
    RESTART_STATUS_KEY,
    "attachment_restart_readback_verified",
    "canvas_text_restart_readback_verified",
    "error_count",
  ]))
  if (required(fields, RESTART_STATUS_KEY) !== "passed") {
    fail("PACKAGED_SMOKE_RESTART_FAILED")
  }
  const errorCount = parseInteger(fields, "error_count")
  if (errorCount !== 0) fail("PACKAGED_SMOKE_UI_ERROR_REPORTED")
  if (!parseBoolean(fields, "attachment_restart_readback_verified")) {
    fail("PACKAGED_SMOKE_RESTART_READBACK_MISSING")
  }
  if (!parseBoolean(fields, "canvas_text_restart_readback_verified")) {
    fail("PACKAGED_SMOKE_RESTART_READBACK_MISSING")
  }

  return Object.freeze({
    stage: "RestartPassed",
    profileFingerprint,
    errorCount,
    attachmentRestartReadbackVerified: true,
    canvasTextRestartReadbackVerified: true,
  })
}

export const parseUpgradedProfileSmokeOutput = (
  stdout: string,
): Readonly<{ upgradeExistingDocumentReadbackVerified: true }> => {
  const fields = parseFields(stdout, new Set([
    "upgrade_existing_document_readback_verified",
  ]))
  if (!parseBoolean(fields, "upgrade_existing_document_readback_verified")) {
    fail("PACKAGED_SMOKE_UPGRADE_READBACK_MISSING")
  }
  return Object.freeze({ upgradeExistingDocumentReadbackVerified: true })
}

export const createPackagedSmokeEvidence = (input: Readonly<{
  sourceFingerprint: string
  appFingerprint: string
  initial: InitialPackagedSmokeResult
  restart: RestartPackagedSmokeResult
}>): PackagedSmokeEvidence => {
  const sourceFingerprint = validateFingerprint(input.sourceFingerprint)
  const appFingerprint = validateFingerprint(input.appFingerprint)
  if (input.initial.profileFingerprint !== input.restart.profileFingerprint) {
    fail("PACKAGED_SMOKE_PROFILE_MISMATCH")
  }
  const profileFingerprint = validateFingerprint(input.initial.profileFingerprint)

  return Object.freeze({
    status: "Passed",
    sourceFingerprint,
    appFingerprint,
    profileFingerprint,
    sampleCount: input.initial.sampleCount,
    p95Ms: input.initial.p95Ms,
    actionCount: input.initial.actionCount,
    durableReadbackCount: input.initial.durableReadbackCount,
    accessibilityRouteFocusCount: input.initial.accessibilityRouteFocusCount,
    accessibilityKeyboardJourneyCount: input.initial.accessibilityKeyboardJourneyCount,
    accessibilityFocusRestorationCount: input.initial.accessibilityFocusRestorationCount,
    accessibilityVisibleControlCount: input.initial.accessibilityVisibleControlCount,
    accessibilityNamedControlCount: input.initial.accessibilityNamedControlCount,
    accessibilityTextZoomPercent: input.initial.accessibilityTextZoomPercent,
    attachmentRestartReadbackVerified: input.restart.attachmentRestartReadbackVerified,
    canvasTextRestartReadbackVerified: input.restart.canvasTextRestartReadbackVerified,
  })
}
