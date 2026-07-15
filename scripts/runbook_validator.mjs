import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const RunbookValidationState = Object.freeze({
  NotStarted: "NotStarted",
  ReadingManifest: "ReadingManifest",
  Validating: "Validating",
  Passed: "Passed",
  Failed: "Failed",
});

export const RunbookValidationEvent = Object.freeze({
  Start: "Start",
  ManifestLoaded: "ManifestLoaded",
  Complete: "Complete",
  Fail: "Fail",
});

export const RunbookValidationErrorCode = Object.freeze({
  MalformedManifest: "RUNBOOK_MALFORMED_MANIFEST",
  MissingRunbook: "RUNBOOK_MISSING_FILE",
  RequiredSectionMissing: "RUNBOOK_REQUIRED_SECTION_MISSING",
  RequiredPhraseMissing: "RUNBOOK_REQUIRED_PHRASE_MISSING",
  ForbiddenTextFound: "RUNBOOK_FORBIDDEN_TEXT_FOUND",
  IoFailed: "RUNBOOK_IO_FAILED",
  InvalidTransition: "RUNBOOK_INVALID_TRANSITION",
});

export function transitionRunbookValidationState(currentState, event, detail = {}) {
  if (currentState === RunbookValidationState.NotStarted && event === RunbookValidationEvent.Start) {
    return { state: RunbookValidationState.ReadingManifest };
  }
  if (
    currentState === RunbookValidationState.ReadingManifest &&
    event === RunbookValidationEvent.ManifestLoaded
  ) {
    return { state: RunbookValidationState.Validating };
  }
  if (currentState === RunbookValidationState.Validating && event === RunbookValidationEvent.Complete) {
    return { state: RunbookValidationState.Passed };
  }
  if (
    [RunbookValidationState.ReadingManifest, RunbookValidationState.Validating].includes(
      currentState,
    ) &&
    event === RunbookValidationEvent.Fail
  ) {
    return {
      state: RunbookValidationState.Failed,
      errorCode: detail.errorCode ?? RunbookValidationErrorCode.IoFailed,
      runbookId: detail.runbookId,
      findingId: detail.findingId,
    };
  }
  return {
    state: RunbookValidationState.Failed,
    errorCode: RunbookValidationErrorCode.InvalidTransition,
  };
}

export function validateRunbookManifest(manifest) {
  if (!isRecord(manifest) || manifest.schemaVersion !== 1) {
    throw new RunbookValidationError(RunbookValidationErrorCode.MalformedManifest, "schema_version");
  }
  if (!isNonEmptyArray(manifest.requiredSections) || !allStrings(manifest.requiredSections)) {
    throw new RunbookValidationError(RunbookValidationErrorCode.MalformedManifest, "required_sections");
  }
  if (!isNonEmptyArray(manifest.requiredPhrases) || !allStrings(manifest.requiredPhrases)) {
    throw new RunbookValidationError(RunbookValidationErrorCode.MalformedManifest, "required_phrases");
  }
  if (!isNonEmptyArray(manifest.forbiddenText)) {
    throw new RunbookValidationError(RunbookValidationErrorCode.MalformedManifest, "forbidden_text");
  }
  for (const item of manifest.forbiddenText) {
    if (!isRecord(item) || !isNonEmptyString(item.id) || !isNonEmptyString(item.value)) {
      throw new RunbookValidationError(RunbookValidationErrorCode.MalformedManifest, "forbidden_shape");
    }
  }
  if (!isNonEmptyArray(manifest.runbooks)) {
    throw new RunbookValidationError(RunbookValidationErrorCode.MalformedManifest, "runbooks");
  }
  for (const runbook of manifest.runbooks) {
    if (!isRecord(runbook) || !isNonEmptyString(runbook.id) || !isNonEmptyString(runbook.path)) {
      throw new RunbookValidationError(RunbookValidationErrorCode.MalformedManifest, "runbook_shape");
    }
    if (runbook.requiredPhrases !== undefined && !allStrings(runbook.requiredPhrases)) {
      throw new RunbookValidationError(
        RunbookValidationErrorCode.MalformedManifest,
        "runbook_required_phrases",
      );
    }
  }
}

export function validateRunbookText({ runbook, text, manifest = {} }) {
  const findings = [];
  const forbiddenText = manifest.forbiddenText ?? [
    { id: "manual_env_edit", value: "edit .env" },
    { id: "raw_token_example", value: "raw-token-example" },
  ];

  for (const item of forbiddenText) {
    if (text.includes(item.value)) {
      findings.push({
        errorCode: RunbookValidationErrorCode.ForbiddenTextFound,
        runbookId: runbook.id,
        findingId: item.id,
      });
    }
  }
  if (findings.length > 0) {
    return findings;
  }

  for (const section of manifest.requiredSections ?? []) {
    if (!text.includes(section)) {
      findings.push({
        errorCode: RunbookValidationErrorCode.RequiredSectionMissing,
        runbookId: runbook.id,
        findingId: section,
      });
    }
  }

  const requiredPhrases = [...(manifest.requiredPhrases ?? []), ...(runbook.requiredPhrases ?? [])];
  for (const phrase of requiredPhrases) {
    if (!containsCaseInsensitive(text, phrase)) {
      findings.push({
        errorCode: RunbookValidationErrorCode.RequiredPhraseMissing,
        runbookId: runbook.id,
        findingId: phrase,
      });
    }
  }

  return findings;
}

export async function runRunbookValidation({
  manifestPath = ".tasks/release/runbook-validation-manifest.json",
  root = process.cwd(),
} = {}) {
  let state = transitionRunbookValidationState(
    RunbookValidationState.NotStarted,
    RunbookValidationEvent.Start,
  );
  const findings = [];

  try {
    const manifest = JSON.parse(await readFile(resolve(root, manifestPath), "utf8"));
    validateRunbookManifest(manifest);
    state = transitionRunbookValidationState(state.state, RunbookValidationEvent.ManifestLoaded);

    for (const runbook of manifest.runbooks) {
      const absolutePath = resolve(root, runbook.path);
      if (!existsSync(absolutePath)) {
        findings.push({
          errorCode: RunbookValidationErrorCode.MissingRunbook,
          runbookId: runbook.id,
          findingId: "missing_file",
        });
        break;
      }
      const text = await readFile(absolutePath, "utf8");
      const runbookFindings = validateRunbookText({ runbook, text, manifest });
      findings.push(...runbookFindings);
      if (runbookFindings.length > 0) {
        break;
      }
    }

    if (findings.length > 0) {
      state = transitionRunbookValidationState(state.state, RunbookValidationEvent.Fail, findings[0]);
      return failedResult(state, findings);
    }

    state = transitionRunbookValidationState(state.state, RunbookValidationEvent.Complete);
    return {
      passed: true,
      state: state.state,
      checkedRunbookCount: manifest.runbooks.length,
      findings: [],
    };
  } catch (error) {
    if (error instanceof RunbookValidationError) {
      state = transitionRunbookValidationState(state.state, RunbookValidationEvent.Fail, {
        errorCode: error.code,
      });
      return failedResult(state, findings);
    }
    state = transitionRunbookValidationState(state.state, RunbookValidationEvent.Fail, {
      errorCode: RunbookValidationErrorCode.IoFailed,
    });
    return failedResult(state, findings);
  }
}

export function renderRunbookValidationResult(result) {
  if (result.passed) {
    return [
      "runbook_validation=passed",
      `validation_state=${result.state}`,
      `checked_runbook_count=${result.checkedRunbookCount}`,
    ].join("\n");
  }

  const lines = [
    "runbook_validation=failed",
    `validation_state=${result.state}`,
    `error_code=${result.errorCode}`,
  ];
  if (result.runbookId) {
    lines.push(`runbook_id=${result.runbookId}`);
  }
  if (result.findingId) {
    lines.push(`finding_id=${result.findingId}`);
  }
  if (result.findings?.length) {
    lines.push(`finding_count=${result.findings.length}`);
  }
  return lines.join("\n");
}

class RunbookValidationError extends Error {
  constructor(code, detail) {
    super(`${code}:${detail}`);
    this.code = code;
  }
}

function failedResult(state, findings) {
  return {
    passed: false,
    state: state.state,
    errorCode: state.errorCode,
    runbookId: state.runbookId,
    findingId: state.findingId,
    findings,
  };
}

function containsCaseInsensitive(text, phrase) {
  return text.toLocaleLowerCase("en-US").includes(phrase.toLocaleLowerCase("en-US"));
}

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function isNonEmptyArray(value) {
  return Array.isArray(value) && value.length > 0;
}

function allStrings(value) {
  return Array.isArray(value) && value.every(isNonEmptyString);
}

async function runCli() {
  const manifestPath = process.argv[2] ?? ".tasks/release/runbook-validation-manifest.json";
  const result = await runRunbookValidation({ manifestPath });
  const rendered = renderRunbookValidationResult(result);
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
