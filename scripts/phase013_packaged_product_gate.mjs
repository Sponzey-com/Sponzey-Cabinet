export const PHASE013_PACKAGED_JOURNEYS = Object.freeze([
  "home", "document", "graph", "canvas", "assets", "backup_restore", "recovery",
]);

export function validatePhase013PackagedProductReport(report, expectedSourceFingerprint) {
  const findingIds = [];
  if (report?.marker !== "phase013_packaged_product_gate=passed") findingIds.push("marker");
  if (!/^[a-f0-9]{64}$/.test(report?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (!/^[a-f0-9]{64}$/.test(report?.appFingerprint ?? "")) findingIds.push("app_fingerprint");
  if (expectedSourceFingerprint && report?.sourceFingerprint !== expectedSourceFingerprint) findingIds.push("stale_source_fingerprint");
  if (report?.platform !== "macos" || report?.cleanProfile !== true || report?.externalRuntimeRequired !== false) findingIds.push("runtime_contract");
  const journeys = Array.isArray(report?.journeys) ? report.journeys : [];
  for (const journey of PHASE013_PACKAGED_JOURNEYS) if (!journeys.includes(journey)) findingIds.push(`journey_${journey}`);
  if (report?.sampleCount !== 200) findingIds.push("sample_count");
  if (!Number.isFinite(report?.p95Ms) || report.p95Ms > 300) findingIds.push("p95_ms");
  if (report?.errorCount !== 0) findingIds.push("error_count");
  if (!Number.isInteger(report?.actionCount) || report.actionCount < 15) findingIds.push("action_count");
  if (!Number.isInteger(report?.durableReadbackCount) || report.durableReadbackCount < 4) findingIds.push("durable_readback_count");
  const serialized = JSON.stringify(report ?? {});
  if (["/Users/", "C:\\Users\\", "documentBody", "assetBytes", "sessionToken", "provider_api_key"].some((token) => serialized.includes(token))) findingIds.push("sensitive_data");
  return Object.freeze({ passed: findingIds.length === 0, findingIds: Object.freeze(findingIds) });
}
