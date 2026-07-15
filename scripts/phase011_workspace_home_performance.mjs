import { containsSensitiveData } from "./phase011_workspace_home_visual.mjs";

export function validateWorkspaceHomePerformanceReport(report, expectedSourceFingerprint) {
  const findingIds = [];
  if (report?.marker !== "phase011_workspace_home_performance=passed") findingIds.push("marker");
  if (!/^[a-f0-9]{64}$/.test(report?.sourceFingerprint ?? "")) findingIds.push("source_fingerprint");
  if (expectedSourceFingerprint && report?.sourceFingerprint !== expectedSourceFingerprint) {
    findingIds.push("stale_source_fingerprint");
  }
  if (!/^[a-f0-9]{64}$/.test(report?.fixtureHash ?? "")) findingIds.push("fixture_hash");
  if (report?.currentDocumentCount !== 10000) findingIds.push("current_document_count");
  if (report?.totalVersionCount !== 100000) findingIds.push("total_version_count");
  if (report?.warmupCount < 20) findingIds.push("warmup_count");
  if (report?.sampleCount < 200) findingIds.push("sample_count");
  if (typeof report?.p95Ms !== "number" || report.p95Ms > 300) findingIds.push("p95_ms");
  if (report?.buildProfile !== "release") findingIds.push("build_profile");
  if (report?.queryPath !== "bounded_workspace_home_projection") findingIds.push("query_path");
  if (containsSensitiveData(JSON.stringify(report ?? {}))) findingIds.push("sensitive_data");
  return { passed: findingIds.length === 0, findingIds };
}
