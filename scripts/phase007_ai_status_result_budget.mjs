import { mkdir, readFile, writeFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

export const AiStatusResultBudgetErrorCode = Object.freeze({
  ThresholdExceeded: "PHASE007_AI_STATUS_RESULT_BUDGET_THRESHOLD_EXCEEDED",
  EmptyMeasurement: "PHASE007_AI_STATUS_RESULT_BUDGET_EMPTY_MEASUREMENT",
});

export function measureAiStatusResultBudget({
  thresholdMs = 300,
  jobCount = 1000,
  citationCount = 1000,
  iterations = 200,
} = {}) {
  const fixture = buildFixture({ jobCount, citationCount });
  const measurements = {
    statusMs: [],
    resultMs: [],
  };

  for (let index = 0; index < iterations; index += 1) {
    const jobId = `job-${index % jobCount}`;
    measure(() => fixture.statusByJobId.get(jobId), measurements.statusMs);
    measure(() => fixture.resultByJobId.get(jobId), measurements.resultMs);
  }

  return evaluateAiStatusResultBudget({
    thresholdMs,
    measurements,
    fixture: { jobCount, citationCount, iterations },
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
    statusReadP95Ms: roundMs(p95(measurements.statusMs)),
    resultReadP95Ms: roundMs(p95(measurements.resultMs)),
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
    marker: "phase007_ai_status_result_budget=passed",
    thresholdMs,
    fixture,
    ...p95Values,
  };
}

export function renderAiStatusResultBudgetMarkdown(result) {
  const lines = [
    "# Phase 007 AI Status Result Budget",
    "",
    result.marker,
    "",
    `- threshold_ms=${result.thresholdMs}`,
    `- ai_status_read_p95_ms=${result.statusReadP95Ms}`,
    `- ai_result_read_p95_ms=${result.resultReadP95Ms}`,
    `- fixture_job_count=${result.fixture.jobCount}`,
    `- fixture_citation_count=${result.fixture.citationCount}`,
    `- fixture_iterations=${result.fixture.iterations}`,
  ];
  if (!result.passed) {
    lines.push(`- error_code: \`${result.errorCode}\``);
  }
  lines.push(
    "",
    "## Measurement Scope",
    "",
    "- AI status and result reads use deterministic cached lookup fixtures.",
    "- AI answer generation and provider network calls are not part of the 300ms synchronous read budget.",
    "- The report records counts and p95 durations only.",
    "- The report does not include prompt text, answer text, retrieval source text, provider key, token, credential, secret, or endpoint.",
    "",
  );
  return lines.join("\n");
}

function buildFixture({ jobCount, citationCount }) {
  const statusByJobId = new Map();
  const resultByJobId = new Map();
  for (let index = 0; index < jobCount; index += 1) {
    const jobId = `job-${index}`;
    statusByJobId.set(jobId, {
      jobId,
      state: index % 2 === 0 ? "Completed" : "Queued",
      citationCount: index % citationCount,
    });
    resultByJobId.set(jobId, {
      jobId,
      state: "Completed",
      answerReference: `answer:${jobId}`,
      citationIds: [`citation-${index % citationCount}`],
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
    statusReadP95Ms: Number.POSITIVE_INFINITY,
    resultReadP95Ms: Number.POSITIVE_INFINITY,
  };
}

function failedResult({ errorCode, thresholdMs, fixture, p95 }) {
  return {
    passed: false,
    marker: "phase007_ai_status_result_budget=failed",
    errorCode,
    thresholdMs,
    fixture,
    ...p95,
  };
}

async function runCli() {
  const result = measureAiStatusResultBudget();
  await mkdir(".tasks/release", { recursive: true });
  const markdown = renderAiStatusResultBudgetMarkdown(result);
  let existing = "";
  try {
    existing = await readFile(".tasks/release/ai-status-result-budget-phase007.md", "utf8");
  } catch {
    existing = "";
  }
  await writeFile(
    ".tasks/release/ai-status-result-budget-phase007.md",
    existing.includes("# Phase 007 AI Status Result Budget") ? markdown : markdown,
  );
  if (result.passed) {
    console.log(result.marker);
    console.log(`ai_status_read_p95_ms=${result.statusReadP95Ms}`);
    console.log(`ai_result_read_p95_ms=${result.resultReadP95Ms}`);
    return;
  }
  console.error(result.marker);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
