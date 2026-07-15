import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  Phase011NativeEvidenceErrorCode,
  Phase011NativeEvidenceEvent,
  Phase011NativeEvidenceState,
  evaluateNativePlatformEvidence,
  evidencePathForPlatform,
  normalizeNativePlatform,
  renderNativePlatformEvidence,
  runNativePlatformEvidence,
  transitionPhase011NativeEvidenceState,
} from "./phase011_native_platform_evidence.mjs";

test("native platform evidence normalizes supported platforms", () => {
  assert.equal(normalizeNativePlatform("win32"), "windows");
  assert.equal(normalizeNativePlatform("darwin"), "macos");
  assert.equal(normalizeNativePlatform("linux"), "linux");
  assert.equal(evidencePathForPlatform("darwin"), ".tasks/release/native-platform-evidence-macos-phase011.md");
});

test("native platform evidence rejects missing fingerprint unsupported platform and failed smoke", () => {
  const missingFingerprint = evaluateNativePlatformEvidence({
    inventoryText: "phase011_current_inventory=passed",
    platform: "darwin",
    packageSmokePassed: true,
  });
  const unsupported = evaluateNativePlatformEvidence({
    inventoryText: inventory(),
    platform: "freebsd",
    packageSmokePassed: true,
  });
  const failedSmoke = evaluateNativePlatformEvidence({
    inventoryText: inventory(),
    platform: "linux",
    packageSmokePassed: false,
  });

  assert.equal(missingFingerprint.errorCode, Phase011NativeEvidenceErrorCode.SourceFingerprintMissing);
  assert.equal(unsupported.errorCode, Phase011NativeEvidenceErrorCode.UnsupportedPlatform);
  assert.equal(failedSmoke.errorCode, Phase011NativeEvidenceErrorCode.PackageSmokeFailed);
});

test("native platform evidence renders sanitized marker", () => {
  const result = evaluateNativePlatformEvidence({
    inventoryText: inventory(),
    platform: "win32",
    packageSmokePassed: true,
  });
  const artifact = renderNativePlatformEvidence(result);

  assert.equal(result.passed, true);
  assert.equal(result.outputPath, ".tasks/release/native-platform-evidence-windows-phase011.md");
  assert.match(artifact, /phase011_native_platform_evidence=passed/);
  assert.match(artifact, /native_platform=windows/);
  assert.match(artifact, /one_host_result_reused_for_other_platforms=false/);
  assert.doesNotMatch(artifact, /\/Users\//);
  assert.doesNotMatch(artifact, /RAW_DOC_BODY_SAMPLE/);
});

test("native platform evidence writes current platform file under explicit root", async () => {
  const root = await mkdtemp(join(tmpdir(), "sponzey-native-platform-evidence-"));
  await mkdir(join(root, ".tasks"), { recursive: true });
  await writeFile(join(root, ".tasks", "phase011-current-implementation-inventory.md"), inventory());

  const result = await runNativePlatformEvidence({
    root,
    platform: "linux",
    packageSmokePassed: true,
  });
  const artifact = await readFile(join(root, ".tasks", "release", "native-platform-evidence-linux-phase011.md"), "utf8");

  assert.equal(result.passed, true);
  assert.match(artifact, /native_platform=linux/);
  assert.match(artifact, /source_fingerprint=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/);
});

test("native platform evidence state machine reaches terminal states", () => {
  const reading = transitionPhase011NativeEvidenceState(
    Phase011NativeEvidenceState.Pending,
    Phase011NativeEvidenceEvent.Start,
  );
  const validating = transitionPhase011NativeEvidenceState(
    reading.state,
    Phase011NativeEvidenceEvent.InventoryRead,
  );
  const writing = transitionPhase011NativeEvidenceState(
    validating.state,
    Phase011NativeEvidenceEvent.SmokeValidated,
  );
  const passed = transitionPhase011NativeEvidenceState(
    writing.state,
    Phase011NativeEvidenceEvent.EvidenceWritten,
  );
  const invalid = transitionPhase011NativeEvidenceState(
    Phase011NativeEvidenceState.Pending,
    Phase011NativeEvidenceEvent.EvidenceWritten,
  );

  assert.equal(reading.state, Phase011NativeEvidenceState.ReadingInventory);
  assert.equal(validating.state, Phase011NativeEvidenceState.ValidatingSmoke);
  assert.equal(writing.state, Phase011NativeEvidenceState.WritingEvidence);
  assert.equal(passed.state, Phase011NativeEvidenceState.Passed);
  assert.equal(invalid.errorCode, Phase011NativeEvidenceErrorCode.InvalidTransition);
});

function inventory() {
  return [
    "phase011_current_inventory=passed",
    "source_fingerprint=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  ].join("\n");
}
