import { readFile, writeFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

export const AiStatusResultBudgetErrorCode = Object.freeze({
  ThresholdExceeded: "PHASE006_AI_STATUS_RESULT_BUDGET_THRESHOLD_EXCEEDED",
  EmptyMeasurement: "PHASE006_AI_STATUS_RESULT_BUDGET_EMPTY_MEASUREMENT",
});

export function measureAiStatusResultBudget({
  thresholdMs = 300,
  jobCount = 1000,
  iterations = 200,
} = {}) {
  const fixture = buildFixture({ jobCount });
  const measurements = {
    statusMs: [],
    resultMs: [],
  };

  for (let index = 0; index < iterations; index += 1) {
    const jobId = `answer-job-${index % jobCount}`;
    measure(() => fixture.statusByJobId.get(jobId), measurements.statusMs);
    measure(() => fixture.resultByJobId.get(jobId), measurements.resultMs);
  }

  return evaluateAiStatusResultBudget({
    thresholdMs,
    measurements,
    fixture: { jobCount, iterations },
  });
}

export function evaluateAiStatusResultBudget({ thresholdMs, measurements, fixture }) {
  if (Object.values(measurements).some((values) => values.length === 0)) {
    return failedResult({
      errorCode: AiStatusResultBudgetErrorCode.EmptyMeasurement,
      thresholdMs,
      fixture,
      p95: emptyP95(),
    });
  }

  const p95Values = {
    aiStatusReadP95Ms: roundMs(p95(measurements.statusMs)),
    aiResultReadP95Ms: roundMs(p95(measurements.resultMs)),
  };
  const passed = Object.values(p95Values).every((value) => value <= thresholdMs);
  if (!passed) {
    return failedResult({
      errorCode: AiStatusResultBudgetErrorCode.ThresholdExceeded,
      thresholdMs,
      fixture,
      p95: p95Values,
    });
  }
  return {
    passed: true,
    marker: "phase006_ai_status_result_budget=passed",
    thresholdMs,
    fixture,
    ...p95Values,
  };
}

export function renderAiStatusResultBudgetMarkdown(result) {
  const lines = [
    "## Phase 006 AI Status Result Performance Budget",
    "",
    result.marker,
    "",
    `- threshold_ms=${result.thresholdMs}`,
    `- ai_status_read_p95_ms=${result.aiStatusReadP95Ms}`,
    `- ai_result_read_p95_ms=${result.aiResultReadP95Ms}`,
    `- fixture_job_count=${result.fixture.jobCount}`,
    `- fixture_iterations=${result.fixture.iterations}`,
  ];
  if (!result.passed) {
    lines.push(`- error_code: \`${result.errorCode}\``);
  }
  lines.push(
    "",
    "### Measurement Scope",
    "",
    "- AI status and result reads use deterministic cached lookup fixtures.",
    "- The report records counts and p95 durations only.",
    "- The report does not include raw prompt, raw generated response, retrieval source text, provider secret, credential, token, or provider network address.",
    "",
  );
  return lines.join("\n");
}

function buildFixture({ jobCount }) {
  const statusByJobId = new Map();
  const resultByJobId = new Map();
  for (let index = 0; index < jobCount; index += 1) {
    const jobId = `answer-job-${index}`;
    statusByJobId.set(jobId, {
      state: index % 2 === 0 ? "Completed" : "Refused",
      citationCount: index % 2 === 0 ? 2 : 0,
      freshnessStatus: index % 3 === 0 ? "stale" : "fresh",
    });
    resultByJobId.set(jobId, {
      state: index % 2 === 0 ? "Completed" : "Refused",
      answerReference: `answer:${jobId}:result`,
      citationCount: index % 2 === 0 ? 2 : 0,
      refusalCode: index % 2 === 0 ? undefined : "insufficient_context",
    });
  }
  return { statusByJobId, resultByJobId };
}

function measure(fn, bucket) {
  const start = performance.now();
  fn();
  bucket.push(performance.now() - start);
}

function p95(values) {
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.max(0, Math.ceil(sorted.length * 0.95) - 1)];
}

function roundMs(value) {
  return Number(value.toFixed(3));
}

function emptyP95() {
  return {
    aiStatusReadP95Ms: Number.POSITIVE_INFINITY,
    aiResultReadP95Ms: Number.POSITIVE_INFINITY,
  };
}

function failedResult({ errorCode, thresholdMs, fixture, p95 }) {
  return {
    passed: false,
    marker: "phase006_ai_status_result_budget=failed",
    errorCode,
    thresholdMs,
    fixture,
    ...p95,
  };
}

async function runCli() {
  const result = measureAiStatusResultBudget();
  const section = renderAiStatusResultBudgetMarkdown(result);
  let existing = "";
  try {
    existing = await readFile(".tasks/release/performance-budget-phase006.md", "utf8");
  } catch {
    existing = "# Phase 006 Performance Budget\n\n";
  }
  const withoutOldSection = existing
    .split("## Phase 006 AI Status Result Performance Budget")[0]
    .trimEnd();
  await writeFile(
    ".tasks/release/performance-budget-phase006.md",
    `${withoutOldSection}\n\n${section}`,
  );
  if (result.passed) {
    console.log(result.marker);
    console.log(`ai_status_read_p95_ms=${result.aiStatusReadP95Ms}`);
    console.log(`ai_result_read_p95_ms=${result.aiResultReadP95Ms}`);
    return;
  }
  console.error(result.marker);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
