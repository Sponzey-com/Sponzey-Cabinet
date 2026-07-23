export const TOPOLOGY_SCALE_NODE_COUNTS = Object.freeze([1_000, 5_000, 10_000] as const);

export interface TopologyScaleProfileResult {
  readonly nodeCount: number;
  readonly edgeCount: number;
  readonly sampleCount: number;
  readonly mappingP50Ms: number;
  readonly mappingP95Ms: number;
  readonly mappingMaxMs: number;
  readonly browserReadyMs: number;
  readonly editorInputDelayMs: number;
  readonly rendererCanvasCount: number;
  readonly browserErrorCount: number;
  readonly unhandledRejectionCount: number;
}

export interface TopologyScaleReport {
  readonly marker: "topology_scale_budget=passed";
  readonly sourceFingerprint: string;
  readonly diagnostics: "sanitized";
  readonly profiles: readonly TopologyScaleProfileResult[];
}

export interface TopologyScaleValidation {
  readonly passed: boolean;
  readonly findingIds: readonly string[];
}

const MAPPING_P95_BUDGET_MS = 300;
const BROWSER_READY_BUDGET_MS = 3_000;
const EDITOR_INPUT_DELAY_BUDGET_MS = 100;
const REQUIRED_SAMPLE_COUNT = 30;

export function validateTopologyScaleReport(
  report: TopologyScaleReport,
  expectedSourceFingerprint?: string,
): TopologyScaleValidation {
  const findings: string[] = [];
  if (report?.marker !== "topology_scale_budget=passed") findings.push("marker");
  if (!/^[a-f0-9]{64}$/.test(report?.sourceFingerprint ?? "")) findings.push("source_fingerprint");
  if (expectedSourceFingerprint && report.sourceFingerprint !== expectedSourceFingerprint) findings.push("stale_source_fingerprint");
  if (containsSensitiveData(JSON.stringify(report ?? {}))) findings.push("sensitive_data");
  const profiles = Array.isArray(report?.profiles) ? report.profiles : [];
  for (const nodeCount of TOPOLOGY_SCALE_NODE_COUNTS) {
    const profile = profiles.find((candidate) => candidate.nodeCount === nodeCount);
    const suffix = `${nodeCount}`;
    if (!profile) {
      findings.push(`profile_${suffix}`);
      continue;
    }
    if (profile.edgeCount !== nodeCount * 2) findings.push(`edge_count_${suffix}`);
    if (profile.sampleCount < REQUIRED_SAMPLE_COUNT) findings.push(`sample_count_${suffix}`);
    if (!isBounded(profile.mappingP50Ms, 0, profile.mappingP95Ms)) findings.push(`mapping_p50_${suffix}`);
    if (!isBounded(profile.mappingP95Ms, profile.mappingP50Ms, MAPPING_P95_BUDGET_MS)) findings.push(`mapping_p95_${suffix}`);
    if (!isBounded(profile.mappingMaxMs, profile.mappingP95Ms, Number.MAX_SAFE_INTEGER)) findings.push(`mapping_max_${suffix}`);
    if (!isBounded(profile.browserReadyMs, 0, BROWSER_READY_BUDGET_MS)) findings.push(`browser_ready_${suffix}`);
    if (!isBounded(profile.editorInputDelayMs, 0, EDITOR_INPUT_DELAY_BUDGET_MS)) findings.push(`editor_input_delay_${suffix}`);
    if (!(profile.rendererCanvasCount >= 1)) findings.push(`renderer_canvas_${suffix}`);
    if (profile.browserErrorCount !== 0) findings.push(`browser_error_${suffix}`);
    if (profile.unhandledRejectionCount !== 0) findings.push(`unhandled_rejection_${suffix}`);
  }
  if (profiles.some((profile) => !TOPOLOGY_SCALE_NODE_COUNTS.includes(profile.nodeCount as 1000 | 5000 | 10000))) {
    findings.push("unknown_profile");
  }
  return Object.freeze({ passed: findings.length === 0, findingIds: Object.freeze(findings) });
}

export function nearestRankPercentile(samples: readonly number[], percentile: number): number {
  if (samples.length === 0) return 0;
  if (!Number.isFinite(percentile) || percentile <= 0 || percentile > 1) throw new Error("TOPOLOGY_PERCENTILE_INVALID");
  const sorted = [...samples].sort((left, right) => left - right);
  return sorted[Math.ceil(sorted.length * percentile) - 1] ?? 0;
}

function isBounded(value: number, minimum: number, maximum: number): boolean {
  return Number.isFinite(value) && value >= minimum && value <= maximum;
}

function containsSensitiveData(value: string): boolean {
  return ["/Users/", "C:\\Users\\", "notes/", ".md", "document body", "provider_api_key", "sessionToken"].some((token) => value.includes(token));
}
