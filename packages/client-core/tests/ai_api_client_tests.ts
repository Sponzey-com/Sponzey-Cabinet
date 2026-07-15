import assert from "node:assert/strict";
import test from "node:test";

import {
  CabinetApiClientError,
  createSelfHostApiClient,
  createSelfHostApiClientConfig,
  type AiAnswerResultView,
  type CabinetHttpRequest,
  type CabinetHttpResponse,
  type CabinetHttpTransport,
} from "../src/index.ts";

test("self-host AI client sends retrieval, answer, status, and result requests through explicit config", async () => {
  const transport = new CapturingTransport([
    jsonResponse(200, {
      queryName: "ai-retrieval",
      workspaceId: "workspace-1",
      textHash: "query-hash-1",
      candidates: [
        {
          sourceId: "document-1",
          sourceKind: "document",
          citationReference: "citation:document-1:1",
          freshness: "fresh",
          permissionDecision: "allowed",
        },
      ],
      durationMs: 42,
    }),
    jsonResponse(200, {
      jobId: "answer-job-1",
      state: "Queued",
      citationCount: 0,
      freshnessStatus: "unknown",
    }),
    jsonResponse(200, {
      jobId: "answer-job-1",
      state: "Completed",
      citationCount: 1,
      freshnessStatus: "fresh",
    }),
    jsonResponse(200, answerResult()),
  ]);
  const client = createSelfHostApiClient(
    createSelfHostApiClientConfig({
      baseUrl: "https://cabinet.local",
      sessionToken: "session-token",
    }),
    transport.handle,
  );

  const retrieval = await client.searchAiRetrieval({
    workspaceId: "workspace-1",
    text: "find source",
    limit: 5,
  });
  const answer = await client.askKnowledgeBase({
    workspaceId: "workspace-1",
    question: "What changed?",
    retrievalLimit: 5,
  });
  const status = await client.getAiAnswerStatus({
    workspaceId: "workspace-1",
    jobId: "answer-job-1",
  });
  const result = await client.getAiAnswerResult({
    workspaceId: "workspace-1",
    jobId: "answer-job-1",
  });

  assert.equal(retrieval.candidates.length, 1);
  assert.equal(answer.state, "Queued");
  assert.equal(status.state, "Completed");
  assert.equal(result.citations[0].sourceId, "document-1");
  assert.deepEqual(
    transport.requests.map((request) => [request.method, request.url]),
    [
      [
        "GET",
        "https://cabinet.local/api/workspaces/workspace-1/ai/retrieval?text=find+source&limit=5",
      ],
      ["POST", "https://cabinet.local/api/workspaces/workspace-1/ai/answers"],
      [
        "GET",
        "https://cabinet.local/api/ai/answers/answer-job-1/status?workspaceId=workspace-1",
      ],
      [
        "GET",
        "https://cabinet.local/api/ai/answers/answer-job-1/result?workspaceId=workspace-1",
      ],
    ],
  );
  assert.deepEqual(JSON.parse(transport.requests[1].body ?? "{}"), {
    question: "What changed?",
    retrievalLimit: 5,
  });
  assert.equal(transport.requests[1].headers.authorization, "Bearer session-token");
});

test("AI answer result DTO carries citation, refusal, and freshness without provider secrets", () => {
  const result: AiAnswerResultView = answerResult();
  const serialized = JSON.stringify(result);

  assert.equal(result.state, "Refused");
  assert.equal(result.refusalCode, "insufficient_context");
  assert.equal(result.freshnessStatus, "stale");
  assert.equal(result.citations[0].freshness, "stale");
  assert.equal(serialized.includes("provider_api_key_fixture"), false);
  assert.equal(serialized.includes("connector_access_token_fixture"), false);
  assert.equal(serialized.includes("ai_prompt_fixture"), false);
});

test("AI API client config does not require provider endpoint, model, or key", () => {
  const config = createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" });

  assert.equal(config.baseUrl, "https://cabinet.local");
  assert.equal("providerEndpoint" in config, false);
  assert.equal("providerModel" in config, false);
  assert.equal("providerApiKey" in config, false);
});

test("AI API client maps server and network errors through CabinetApiClientError", async () => {
  const serverClient = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    new CapturingTransport([
      jsonResponse(503, {
        errorCode: "AI_PROVIDER_UNAVAILABLE",
        message: "provider unavailable",
      }),
    ]).handle,
  );

  await assert.rejects(
    () => serverClient.getAiAnswerStatus({ workspaceId: "workspace-1", jobId: "job-1" }),
    (error) =>
      error instanceof CabinetApiClientError &&
      error.code === "AI_PROVIDER_UNAVAILABLE" &&
      error.status === 503,
  );

  const networkClient = createSelfHostApiClient(
    createSelfHostApiClientConfig({ baseUrl: "https://cabinet.local" }),
    async () => {
      throw new Error("connection refused");
    },
  );

  await assert.rejects(
    () => networkClient.searchAiRetrieval({ workspaceId: "workspace-1", text: "x", limit: 1 }),
    (error) => error instanceof CabinetApiClientError && error.code === "NETWORK_FAILURE",
  );
});

class CapturingTransport {
  readonly requests: CabinetHttpRequest[] = [];
  private responses: CabinetHttpResponse[];

  constructor(responses: CabinetHttpResponse[]) {
    this.responses = [...responses];
  }

  readonly handle: CabinetHttpTransport = async (request) => {
    this.requests.push(request);
    const response = this.responses.shift();
    if (!response) {
      throw new Error(`Unexpected request ${request.method} ${request.url}`);
    }
    return response;
  };
}

function jsonResponse(status: number, body: unknown): CabinetHttpResponse {
  return {
    status,
    body: JSON.stringify(body),
    headers: { "content-type": "application/json" },
  };
}

function answerResult(): AiAnswerResultView {
  return {
    jobId: "answer-job-1",
    state: "Refused",
    answerReference: "answer:answer-job-1",
    refusalCode: "insufficient_context",
    freshnessStatus: "stale",
    citations: [
      {
        sourceId: "document-1",
        sourceKind: "document",
        citationReference: "citation:document-1:1",
        freshness: "stale",
      },
    ],
  };
}
