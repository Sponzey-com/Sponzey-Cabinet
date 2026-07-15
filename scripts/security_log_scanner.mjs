import { readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const ScanState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingManifest: "ReadingManifest",
  Scanning: "Scanning",
  Passed: "Passed",
  Failed: "Failed",
});

export const ScanEvent = Object.freeze({
  Start: "Start",
  ManifestLoaded: "ManifestLoaded",
  MatchFound: "MatchFound",
  Complete: "Complete",
  Fail: "Fail",
});

export const SecurityScanErrorCode = Object.freeze({
  MalformedManifest: "SECURITY_SCAN_MALFORMED_MANIFEST",
  MissingTarget: "SECURITY_SCAN_MISSING_TARGET",
  SensitiveFixtureFound: "SECURITY_SCAN_SENSITIVE_FIXTURE_FOUND",
  IoFailed: "SECURITY_SCAN_IO_FAILED",
  InvalidTransition: "SECURITY_SCAN_INVALID_TRANSITION",
});

export function transitionScanState(currentState, event, detail = {}) {
  if (currentState === ScanState.NotStarted && event === ScanEvent.Start) {
    return { state: ScanState.ReadingManifest };
  }
  if (currentState === ScanState.ReadingManifest && event === ScanEvent.ManifestLoaded) {
    return { state: ScanState.Scanning };
  }
  if (currentState === ScanState.Scanning && event === ScanEvent.Complete) {
    return { state: ScanState.Passed };
  }
  if (
    [ScanState.ReadingManifest, ScanState.Scanning].includes(currentState) &&
    [ScanEvent.MatchFound, ScanEvent.Fail].includes(event)
  ) {
    return {
      state: ScanState.Failed,
      errorCode: detail.errorCode ?? SecurityScanErrorCode.IoFailed,
      filePath: detail.filePath,
      tokenId: detail.tokenId,
    };
  }
  return {
    state: ScanState.Failed,
    errorCode: SecurityScanErrorCode.InvalidTransition,
  };
}

export function validateLogPolicyManifest(manifest) {
  if (!isRecord(manifest) || manifest.schemaVersion !== 1) {
    throw new SecurityScanError(SecurityScanErrorCode.MalformedManifest, "schema_version");
  }
  if (!Array.isArray(manifest.logClasses) || manifest.logClasses.length !== 3) {
    throw new SecurityScanError(SecurityScanErrorCode.MalformedManifest, "log_classes");
  }
  const requiredLogClasses = ["Product Log", "Field Debug Log", "Development Log"];
  for (const className of requiredLogClasses) {
    const logClass = manifest.logClasses.find((candidate) => candidate.name === className);
    if (!isRecord(logClass)) {
      throw new SecurityScanError(SecurityScanErrorCode.MalformedManifest, "missing_log_class");
    }
    validateFieldSet(logClass, className);
  }
  if (!Array.isArray(manifest.deniedFixtures) || manifest.deniedFixtures.length === 0) {
    throw new SecurityScanError(SecurityScanErrorCode.MalformedManifest, "denied_fixtures");
  }
  for (const fixture of manifest.deniedFixtures) {
    if (
      !isRecord(fixture) ||
      !isNonEmptyString(fixture.id) ||
      !isNonEmptyString(fixture.kind) ||
      !isNonEmptyString(fixture.value)
    ) {
      throw new SecurityScanError(SecurityScanErrorCode.MalformedManifest, "fixture_shape");
    }
  }
  if (!Array.isArray(manifest.scanTargets) || manifest.scanTargets.length === 0) {
    throw new SecurityScanError(SecurityScanErrorCode.MalformedManifest, "scan_targets");
  }
  for (const target of manifest.scanTargets) {
    if (
      !isRecord(target) ||
      !isNonEmptyString(target.id) ||
      !isNonEmptyString(target.path) ||
      typeof target.required !== "boolean"
    ) {
      throw new SecurityScanError(SecurityScanErrorCode.MalformedManifest, "target_shape");
    }
  }
}

export function findDeniedFixturesInText(text, deniedFixtures) {
  const findings = [];
  for (const fixture of deniedFixtures) {
    if (text.includes(fixture.value)) {
      findings.push({
        tokenId: fixture.id,
        kind: fixture.kind,
        errorCode: SecurityScanErrorCode.SensitiveFixtureFound,
      });
    }
  }
  return findings;
}

export async function runSecurityLogScan({ manifestPath, root = process.cwd() }) {
  let state = transitionScanState(ScanState.NotStarted, ScanEvent.Start);
  try {
    const manifest = JSON.parse(await readFile(resolve(root, manifestPath), "utf8"));
    validateLogPolicyManifest(manifest);
    state = transitionScanState(state.state, ScanEvent.ManifestLoaded);

    const findings = [];
    for (const target of manifest.scanTargets) {
      const absoluteTargetPath = resolve(root, target.path);
      if (!existsSync(absoluteTargetPath)) {
        if (target.required) {
          state = transitionScanState(state.state, ScanEvent.Fail, {
            errorCode: SecurityScanErrorCode.MissingTarget,
            filePath: target.path,
          });
          return failedResult(state, findings);
        }
        continue;
      }
      const text = await readFile(absoluteTargetPath, "utf8");
      for (const finding of findDeniedFixturesInText(text, manifest.deniedFixtures)) {
        findings.push({
          ...finding,
          targetId: target.id,
          filePath: target.path,
        });
      }
    }

    if (findings.length > 0) {
      state = transitionScanState(state.state, ScanEvent.MatchFound, {
        errorCode: SecurityScanErrorCode.SensitiveFixtureFound,
        filePath: findings[0].filePath,
        tokenId: findings[0].tokenId,
      });
      return failedResult(state, findings);
    }

    state = transitionScanState(state.state, ScanEvent.Complete);
    return {
      passed: true,
      state: state.state,
      scannedTargetCount: manifest.scanTargets.length,
      findings: [],
    };
  } catch (error) {
    if (error instanceof SecurityScanError) {
      state = transitionScanState(state.state, ScanEvent.Fail, {
        errorCode: error.code,
      });
      return failedResult(state, []);
    }
    state = transitionScanState(state.state, ScanEvent.Fail, {
      errorCode: SecurityScanErrorCode.IoFailed,
    });
    return failedResult(state, []);
  }
}

export function renderScanResult(result) {
  if (result.passed) {
    return [
      "security_log_scan=passed",
      `scan_state=${result.state}`,
      `scanned_target_count=${result.scannedTargetCount}`,
    ].join("\n");
  }

  const lines = [
    "security_log_scan=failed",
    `scan_state=${result.state}`,
    `error_code=${result.errorCode}`,
  ];
  if (result.filePath) {
    lines.push(`file_path=${result.filePath}`);
  }
  if (result.tokenId) {
    lines.push(`token_id=${result.tokenId}`);
  }
  if (result.findings?.length) {
    lines.push(`finding_count=${result.findings.length}`);
  }
  return lines.join("\n");
}

class SecurityScanError extends Error {
  constructor(code, detail) {
    super(`${code}:${detail}`);
    this.code = code;
  }
}

function validateFieldSet(logClass, className) {
  if (!Array.isArray(logClass.allowedFields) || !Array.isArray(logClass.deniedFields)) {
    throw new SecurityScanError(SecurityScanErrorCode.MalformedManifest, "field_set");
  }
  const allowed = new Set(logClass.allowedFields);
  for (const deniedField of logClass.deniedFields) {
    if (!isNonEmptyString(deniedField)) {
      throw new SecurityScanError(SecurityScanErrorCode.MalformedManifest, "denied_field");
    }
    if (allowed.has(deniedField)) {
      throw new SecurityScanError(
        SecurityScanErrorCode.MalformedManifest,
        `${className}:field_conflict`,
      );
    }
  }
}

function failedResult(state, findings) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    filePath: state.filePath,
    tokenId: state.tokenId,
    findings,
  };
}

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

async function runCli() {
  const manifestPath = process.argv[2] ?? ".tasks/release/security-log-policy-manifest.json";
  const result = await runSecurityLogScan({ manifestPath });
  const rendered = renderScanResult(result);
  if (result.passed) {
    console.log(rendered);
    return;
  }
  console.error(rendered);
  process.exit(1);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  runCli();
}
