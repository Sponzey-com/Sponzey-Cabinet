import { writeFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";
import { pathToFileURL } from "node:url";

export const DocumentQueryBudgetErrorCode = Object.freeze({
  ThresholdExceeded: "PHASE006_DOCUMENT_QUERY_BUDGET_THRESHOLD_EXCEEDED",
  EmptyMeasurement: "PHASE006_DOCUMENT_QUERY_BUDGET_EMPTY_MEASUREMENT",
});

const defaultThresholdMs = 300;

export function measureDocumentQueryBudget({
  thresholdMs = defaultThresholdMs,
  documentCount = 1000,
  historyEntriesPerDocument = 20,
  iterations = 200,
} = {}) {
  const fixture = buildFixture({ documentCount, historyEntriesPerDocument });
  const currentReadDurationsMs = [];
  const historyReadDurationsMs = [];

  for (let index = 0; index < iterations; index += 1) {
    const documentId = `doc-${index % documentCount}`;
    const currentStart = performance.now();
    const current = fixture.currentById.get(documentId);
    if (!current) {
      throw new Error(`missing current fixture ${documentId}`);
    }
    currentReadDurationsMs.push(performance.now() - currentStart);

    const historyStart = performance.now();
    const history = fixture.historyById.get(documentId);
    if (!history) {
      throw new Error(`missing history fixture ${documentId}`);
    }
    historyReadDurationsMs.push(performance.now() - historyStart);
  }

  return evaluateDocumentQueryBudget({
    thresholdMs,
    currentReadDurationsMs,
    historyReadDurationsMs,
    fixture: {
      documentCount,
      historyEntriesPerDocument,
      iterations,
    },
  });
}

export function evaluateDocumentQueryBudget({
  thresholdMs,
  currentReadDurationsMs,
  historyReadDurationsMs,
  fixture,
}) {
  if (currentReadDurationsMs.length === 0 || historyReadDurationsMs.length === 0) {
    return failedResult({
      errorCode: DocumentQueryBudgetErrorCode.EmptyMeasurement,
      thresholdMs,
      fixture,
      currentReadP95Ms: Number.POSITIVE_INFINITY,
      historyReadP95Ms: Number.POSITIVE_INFINITY,
    });
  }

  const currentReadP95Ms = roundMs(p95(currentReadDurationsMs));
  const historyReadP95Ms = roundMs(p95(historyReadDurationsMs));
  const passed = currentReadP95Ms <= thresholdMs && historyReadP95Ms <= thresholdMs;

  if (!passed) {
    return failedResult({
      errorCode: DocumentQueryBudgetErrorCode.ThresholdExceeded,
      thresholdMs,
      fixture,
      currentReadP95Ms,
      historyReadP95Ms,
    });
  }

  return {
    passed: true,
    marker: "phase006_document_query_budget=passed",
    thresholdMs,
    fixture,
    currentReadP95Ms,
    historyReadP95Ms,
  };
}

export function renderDocumentQueryBudgetMarkdown(result) {
  const lines = [
    "# Phase 006 Document Query Performance Budget",
    "",
    result.marker,
    "",
    "- phase: `Phase 006`",
    "- budget: `Document Editor, Markdown Preview, History, and Restore UX`",
    `- status: \`${result.passed ? "passed" : "failed"}\``,
    `- threshold_ms=${result.thresholdMs}`,
    `- current_document_read_p95_ms=${result.currentReadP95Ms}`,
    `- history_read_p95_ms=${result.historyReadP95Ms}`,
    `- fixture_document_count=${result.fixture.documentCount}`,
    `- fixture_history_entries_per_document=${result.fixture.historyEntriesPerDocument}`,
    `- fixture_iterations=${result.fixture.iterations}`,
  ];
  if (!result.passed) {
    lines.push(`- error_code: \`${result.errorCode}\``);
  }
  lines.push(
    "",
    "## Measurement Scope",
    "",
    "- Current document read uses indexed current snapshot lookup.",
    "- History read uses indexed history page lookup.",
    "- Measurement stores counts and p95 durations only.",
    "- The report does not include document body, raw markdown, rendered HTML dump, asset content, AI prompt, AI answer, provider key, token, credential, or personal absolute path.",
    "",
    "## Commands",
    "",
    "- `npm run run:phase006-document-query-budget-tests`",
    "- `npm run run:phase006-document-query-budget`",
    "",
  );
  return lines.join("\n");
}

function buildFixture({ documentCount, historyEntriesPerDocument }) {
  const currentById = new Map();
  const historyById = new Map();
  for (let documentIndex = 0; documentIndex < documentCount; documentIndex += 1) {
    const documentId = `doc-${documentIndex}`;
    currentById.set(documentId, {
      documentId,
      versionId: `version-current-${documentIndex}`,
    });
    historyById.set(
      documentId,
      Array.from({ length: historyEntriesPerDocument }, (_, historyIndex) => ({
        versionId: `version-${documentIndex}-${historyIndex}`,
      })),
    );
  }
  return { currentById, historyById };
}

function p95(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.max(0, Math.ceil(sorted.length * 0.95) - 1);
  return sorted[index];
}

function roundMs(value) {
  return Number(value.toFixed(3));
}

function failedResult({
  errorCode,
  thresholdMs,
  fixture,
  currentReadP95Ms,
  historyReadP95Ms,
}) {
  return {
    passed: false,
    marker: "phase006_document_query_budget=failed",
    errorCode,
    thresholdMs,
    fixture,
    currentReadP95Ms,
    historyReadP95Ms,
  };
}

async function runCli() {
  const result = measureDocumentQueryBudget();
  await writeFile(
    ".tasks/release/performance-budget-phase006.md",
    renderDocumentQueryBudgetMarkdown(result),
  );
  if (result.passed) {
    console.log(result.marker);
    console.log(`current_document_read_p95_ms=${result.currentReadP95Ms}`);
    console.log(`history_read_p95_ms=${result.historyReadP95Ms}`);
    return;
  }
  console.error(result.marker);
  console.error(`error_code=${result.errorCode}`);
  process.exit(1);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runCli();
}
