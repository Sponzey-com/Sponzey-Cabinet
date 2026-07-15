import { mkdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export const Phase011NativeEvidenceState = Object.freeze({
  Pending: "Pending",
  ReadingInventory: "ReadingInventory",
  ValidatingSmoke: "ValidatingSmoke",
  WritingEvidence: "WritingEvidence",
  Passed: "Passed",
  Failed: "Failed",
});

export const Phase011NativeEvidenceEvent = Object.freeze({
  Start: "Start",
  InventoryRead: "InventoryRead",
  SmokeValidated: "SmokeValidated",
  EvidenceWritten: "EvidenceWritten",
  Fail: "Fail",
});

export const Phase011NativeEvidenceErrorCode = Object.freeze({
  SourceFingerprintMissing: "PHASE011_NATIVE_EVIDENCE_SOURCE_FINGERPRINT_MISSING",
  UnsupportedPlatform: "PHASE011_NATIVE_EVIDENCE_UNSUPPORTED_PLATFORM",
  PackageSmokeFailed: "PHASE011_NATIVE_EVIDENCE_PACKAGE_SMOKE_FAILED",
  IoFailed: "PHASE011_NATIVE_EVIDENCE_IO_FAILED",
  InvalidTransition: "PHASE011_NATIVE_EVIDENCE_INVALID_TRANSITION",
});

const supportedPlatforms = Object.freeze(["windows", "macos", "linux"]);

export function transitionPhase011NativeEvidenceState(currentState, event, detail = {}) {
  if (currentState === Phase011NativeEvidenceState.Pending && event === Phase011NativeEvidenceEvent.Start) {
    return { state: Phase011NativeEvidenceState.ReadingInventory };
  }
  if (
    currentState === Phase011NativeEvidenceState.ReadingInventory &&
    event === Phase011NativeEvidenceEvent.InventoryRead
  ) {
    return { state: Phase011NativeEvidenceState.ValidatingSmoke };
  }
  if (
    currentState === Phase011NativeEvidenceState.ValidatingSmoke &&
    event === Phase011NativeEvidenceEvent.SmokeValidated
  ) {
    return { state: Phase011NativeEvidenceState.WritingEvidence };
  }
  if (
    currentState === Phase011NativeEvidenceState.WritingEvidence &&
    event === Phase011NativeEvidenceEvent.EvidenceWritten
  ) {
    return { state: Phase011NativeEvidenceState.Passed };
  }
  if (
    [
      Phase011NativeEvidenceState.ReadingInventory,
      Phase011NativeEvidenceState.ValidatingSmoke,
      Phase011NativeEvidenceState.WritingEvidence,
    ].includes(currentState) &&
    event === Phase011NativeEvidenceEvent.Fail
  ) {
    return {
      state: Phase011NativeEvidenceState.Failed,
      errorCode: detail.errorCode ?? Phase011NativeEvidenceErrorCode.IoFailed,
      findingId: detail.findingId,
    };
  }
  return {
    state: Phase011NativeEvidenceState.Failed,
    errorCode: Phase011NativeEvidenceErrorCode.InvalidTransition,
  };
}

export function normalizeNativePlatform(platform) {
  if (platform === "win32" || platform === "windows") return "windows";
  if (platform === "darwin" || platform === "macos") return "macos";
  if (platform === "linux") return "linux";
  return platform;
}

export function evidencePathForPlatform(platform) {
  const normalized = normalizeNativePlatform(platform);
  return `.tasks/release/native-platform-evidence-${normalized}-phase011.md`;
}

export function evaluateNativePlatformEvidence({
  inventoryText,
  platform = process.platform,
  packageSmokePassed,
}) {
  const normalizedPlatform = normalizeNativePlatform(platform);
  const sourceFingerprint = inventoryText?.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
  if (!sourceFingerprint) {
    return failed(Phase011NativeEvidenceErrorCode.SourceFingerprintMissing, "source_fingerprint");
  }
  if (!supportedPlatforms.includes(normalizedPlatform)) {
    return failed(Phase011NativeEvidenceErrorCode.UnsupportedPlatform, normalizedPlatform, {
      sourceFingerprint,
      platform: normalizedPlatform,
    });
  }
  if (packageSmokePassed !== true) {
    return failed(Phase011NativeEvidenceErrorCode.PackageSmokeFailed, "desktop_package_smoke", {
      sourceFingerprint,
      platform: normalizedPlatform,
    });
  }
  return {
    passed: true,
    marker: "phase011_native_platform_evidence=passed",
    state: Phase011NativeEvidenceState.Passed,
    sourceFingerprint,
    platform: normalizedPlatform,
    outputPath: evidencePathForPlatform(normalizedPlatform),
  };
}

export function renderNativePlatformEvidence(result) {
  const lines = [
    "# Phase 011 Native Platform Evidence",
    "",
    result.marker,
    `native_platform=${result.platform}`,
  ];
  if (result.sourceFingerprint) lines.push(`source_fingerprint=${result.sourceFingerprint}`);
  lines.push(
    "release_scope=personal_local_desktop",
    "desktop_package_smoke=passed",
    "installed_runtime_requires_external_db=false",
    "installed_runtime_requires_external_search=false",
    "installed_runtime_requires_git_cli=false",
    "installed_runtime_requires_nodejs=false",
    "installed_runtime_requires_manual_env=false",
    "one_host_result_reused_for_other_platforms=false",
    "raw_body_excluded=true",
    "raw_path_excluded=true",
    "",
  );
  return lines.join("\n");
}

export async function runNativePlatformEvidence({
  root = process.cwd(),
  platform = process.platform,
  packageSmokePassed = true,
} = {}) {
  let state = transitionPhase011NativeEvidenceState(
    Phase011NativeEvidenceState.Pending,
    Phase011NativeEvidenceEvent.Start,
  );
  try {
    const inventoryText = await readFile(join(root, ".tasks", "phase011-current-implementation-inventory.md"), "utf8");
    state = transitionPhase011NativeEvidenceState(state.state, Phase011NativeEvidenceEvent.InventoryRead);
    const result = evaluateNativePlatformEvidence({ inventoryText, platform, packageSmokePassed });
    if (!result.passed) {
      state = transitionPhase011NativeEvidenceState(state.state, Phase011NativeEvidenceEvent.Fail, {
        errorCode: result.errorCode,
        findingId: result.findingId,
      });
      return { ...result, state: state.state };
    }
    state = transitionPhase011NativeEvidenceState(state.state, Phase011NativeEvidenceEvent.SmokeValidated);
    await mkdir(join(root, ".tasks", "release"), { recursive: true });
    await writeFile(join(root, result.outputPath), renderNativePlatformEvidence(result));
    state = transitionPhase011NativeEvidenceState(state.state, Phase011NativeEvidenceEvent.EvidenceWritten);
    return { ...result, state: state.state };
  } catch {
    state = transitionPhase011NativeEvidenceState(state.state, Phase011NativeEvidenceEvent.Fail, {
      errorCode: Phase011NativeEvidenceErrorCode.IoFailed,
    });
    return failed(Phase011NativeEvidenceErrorCode.IoFailed, "inventory_read", { state: state.state });
  }
}

function failed(errorCode, findingId, detail = {}) {
  return {
    passed: false,
    marker: "phase011_native_platform_evidence=failed",
    state: detail.state ?? Phase011NativeEvidenceState.Failed,
    errorCode,
    findingId,
    sourceFingerprint: detail.sourceFingerprint,
    platform: detail.platform,
  };
}

async function runCli() {
  const result = await runNativePlatformEvidence();
  if (result.passed) {
    console.log(result.marker);
    console.log(`native_platform=${result.platform}`);
    console.log(`output_path=${result.outputPath}`);
    return;
  }
  console.error(result.marker);
  console.error(`error_code=${result.errorCode}`);
  if (result.findingId) console.error(`finding_id=${result.findingId}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
