import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  ObservabilityMatrixGateEvent,
  ObservabilityMatrixGateState,
  analyzeObservabilityMatrixGateSources,
  renderObservabilityMatrixGateMarkdown,
  transitionObservabilityMatrixGateState,
} from "./phase005_observability_matrix_gate.mjs";

const completeMatrix = [
  "# Phase 005 Product Log Event Matrix",
  "",
  "## Product Log Events",
  "",
  "`ai.retrieval.degraded` `ai.answer.requested` `ai.answer.completed` `ai.answer.failed`",
  "`mcp.tool.invocation.failed` `webhook.delivery.dead_lettered` `connector.authorization.failed` `connector.sync.failed`",
  "`AI_PROVIDER_UNAVAILABLE` `WEBHOOK_DEAD_LETTERED` `CONNECTOR_AUTHORIZATION_FAILED`",
  "",
  "## Field Debug Log Events",
  "",
  "`field.ai.retrieval` `field.ai.provider` `field.mcp.tool` `field.webhook.delivery` `field.connector.sync`",
  "scope TTL query hash provider name connector id retry count retrieval count citation count",
  "",
  "## Development Log Events",
  "",
  "`dev.ai.fake_provider.called` `dev.webhook.fake_transport.called` `dev.connector.fake_gateway.called`",
  "fixture id fake port call count local/test only production default behavior is forbidden",
  "",
  "## Sensitive Data Denied Rules",
  "",
  "Do not record raw prompt, raw answer, retrieval source text, embedding input, provider API key, connector access token, connector refresh token, connector client secret, webhook signing secret, raw payload body, request body, response body, token, credential, or secret.",
].join("\n");

test("observability matrix gate marks complete matrix as passed", () => {
  const result = analyzeObservabilityMatrixGateSources({
    sources: {
      ".tasks/release/product-log-event-matrix.md": completeMatrix,
    },
  });

  assert.equal(result.status, "passed");
  assert.equal(result.summary.covered, 4);
  assert.equal(result.nextImplementationTarget, null);
});

test("active product log event matrix covers Phase 005 observability targets", async () => {
  const matrix = await readFile(".tasks/release/product-log-event-matrix.md", "utf8");
  const result = analyzeObservabilityMatrixGateSources({
    sources: {
      ".tasks/release/product-log-event-matrix.md": matrix,
    },
  });

  assert.equal(result.status, "passed");
  assert.equal(result.summary.covered, 4);
});

test("observability matrix gate fails when Phase 005 product events are missing", () => {
  const result = analyzeObservabilityMatrixGateSources({
    sources: {
      ".tasks/release/product-log-event-matrix.md": completeMatrix.replace(
        "`connector.sync.failed`",
        "",
      ),
    },
  });

  assert.equal(result.status, "failed");
  assert.equal(result.nextImplementationTarget.id, "phase005_product_log_events");
  assert.ok(result.nextImplementationTarget.missingEvidence.includes("connector.sync.failed"));
});

test("observability matrix gate state machine rejects invalid transition", () => {
  assert.throws(
    () =>
      transitionObservabilityMatrixGateState(
        ObservabilityMatrixGateState.Pending,
        ObservabilityMatrixGateEvent.Report,
      ),
    /PHASE005_OBSERVABILITY_MATRIX_GATE_INVALID_TRANSITION/,
  );
});

test("observability matrix gate markdown excludes sensitive fixture values", async () => {
  const manifest = JSON.parse(
    await readFile(".tasks/release/security-log-policy-manifest.json", "utf8"),
  );
  const sensitiveValues = manifest.deniedFixtures
    .filter((fixture) =>
      [
        "ai_prompt_fixture",
        "ai_answer_fixture",
        "retrieval_source_text_fixture",
        "embedding_input_fixture",
        "provider_api_key_fixture",
        "connector_access_token_fixture",
        "connector_refresh_token_fixture",
        "connector_client_secret_fixture",
        "webhook_secret_fixture",
        "webhook_payload_body_fixture",
      ].includes(fixture.id),
    )
    .map((fixture) => fixture.value);
  const result = analyzeObservabilityMatrixGateSources({
    sources: {
      ".tasks/release/product-log-event-matrix.md": completeMatrix,
    },
  });
  const markdown = renderObservabilityMatrixGateMarkdown(result);

  for (const value of sensitiveValues) {
    assert.doesNotMatch(markdown, new RegExp(value));
  }
});
