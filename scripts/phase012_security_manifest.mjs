import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";

import { collectPhase012SourceFingerprint } from "./phase012_release_evidence.mjs";

const outputPath = ".tasks/release/security-log-policy-manifest-phase012.json";

export function buildPhase012SecurityManifest(sourceFingerprint) {
  return {
    schemaVersion: 1,
    marker: "phase012_security_log_manifest=passed",
    sourceFingerprint,
    productScope: "personal_local_macos_desktop",
    logClasses: [
      {
        name: "Product Log",
        allowedFields: ["event_name", "masked_resource_id", "stable_error_code", "duration_bucket", "count"],
        deniedFields: ["document_body", "asset_bytes", "filename", "raw_path", "secret", "search_text"],
      },
      {
        name: "Field Debug Log",
        allowedFields: ["scope_hash", "expiry_bucket", "state", "count", "cache_status"],
        deniedFields: ["raw_body", "token", "credential", "absolute_path", "unscoped_global_activation"],
      },
      {
        name: "Development Log",
        allowedFields: ["test_marker", "fixture_hash", "duration_bucket", "command_id"],
        deniedFields: ["production_default", "customer_data", "secret", "raw_document"],
      },
    ],
    deniedFixtures: [
      { id: "phase012_auth_material", kind: "auth_material", value: "PHASE012_AUTH_MATERIAL_FIXTURE" },
      { id: "phase012_document_body", kind: "document_body", value: "PHASE012_RAW_DOCUMENT_BODY_FIXTURE" },
      { id: "phase012_personal_path", kind: "absolute_path", value: "PHASE012_PERSONAL_PATH_FIXTURE" },
      { id: "phase012_asset_bytes", kind: "asset_bytes", value: "PHASE012_ASSET_BYTES_FIXTURE" },
      { id: "phase012_search_text", kind: "search_text", value: "PHASE012_RAW_SEARCH_TEXT_FIXTURE" },
    ],
    scanTargets: [
      target("archive_validation", ".tasks/phase012-archive-validation-result.md"),
      target("plan_validation", ".tasks/phase012-plan-validation-result.md"),
      target("asset_gate", ".tasks/phase012-asset-gate-evidence.md"),
      target("canvas_gate", ".tasks/phase012-canvas-gate-evidence.md"),
      target("query_performance", ".tasks/release/query-performance-phase012.md"),
      target("query_render_performance", ".tasks/release/query-render-performance-phase012.md"),
      target("exploration_visual", ".tasks/release/exploration-visual-phase012.json"),
      target("packaged_ui_smoke", ".tasks/release/packaged-ui-smoke-phase012.md"),
    ],
  };
}

export async function writePhase012SecurityManifest({ root = process.cwd(), sourceFingerprint }) {
  const manifest = buildPhase012SecurityManifest(sourceFingerprint);
  const destination = join(root, outputPath);
  await mkdir(dirname(destination), { recursive: true });
  await writeFile(destination, `${JSON.stringify(manifest, null, 2)}\n`);
  return outputPath;
}

function target(id, path) {
  return { id, path, required: true };
}

async function main() {
  const root = process.argv[2] ?? process.cwd();
  const fingerprint = await collectPhase012SourceFingerprint(root);
  const path = await writePhase012SecurityManifest({
    root,
    sourceFingerprint: fingerprint.sourceFingerprint,
  });
  process.stdout.write(`phase012_security_log_manifest=passed target_count=8 output=${path}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
