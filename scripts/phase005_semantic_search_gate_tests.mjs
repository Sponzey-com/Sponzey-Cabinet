import assert from "node:assert/strict";
import test from "node:test";

import {
  SemanticGateErrorCode,
  SemanticGateEvent,
  SemanticGateState,
  analyzeSemanticGateSources,
  renderSemanticGateMarkdown,
  transitionSemanticGateState,
} from "./phase005_semantic_search_gate.mjs";

const completeSources = {
  "crates/cabinet-domain/src/embedding.rs":
    "EmbeddingInput EmbeddingVectorReference EmbeddingJob EmbeddingJobState",
  "crates/cabinet-domain/tests/embedding_tests.rs":
    "embedding_job_uses_explicit_success_transitions embedding_job_uses_explicit_retry_and_failure_transitions",
  "crates/cabinet-ports/src/embedding.rs":
    "EmbeddingProviderPort VectorIndexPort EmbeddingVector VectorSearchQuery",
  "crates/cabinet-adapters/src/deterministic_embedding_provider.rs":
    "DeterministicEmbeddingProvider EmbeddingProviderPort",
  "crates/cabinet-adapters/src/local_vector_index.rs":
    "LocalVectorIndex VectorIndexPort",
  "crates/cabinet-usecases/src/semantic.rs":
    "MergeHybridSearchUsecase HybridSearchInput HybridSearchResult",
  "crates/cabinet-usecases/tests/semantic_usecase_tests.rs":
    "hybrid_search_merge_dedupes_keyword_and_semantic_matches hybrid_merge_completes_under_300ms_fixture",
};

test("semantic gate marks complete fixture as passed", () => {
  const gate = analyzeSemanticGateSources({ sources: completeSources });

  assert.equal(gate.status, "passed");
  assert.equal(gate.summary.covered, 3);
  assert.equal(gate.summary.targetsNeedingWork, 0);
});

test("semantic gate reports missing merge evidence", () => {
  const {
    "crates/cabinet-usecases/src/semantic.rs": _semanticUsecase,
    ...sources
  } = completeSources;

  const gate = analyzeSemanticGateSources({ sources });

  assert.equal(gate.status, "failed");
  assert.equal(gate.nextImplementationTarget.id, "semantic_merge_usecase");
});

test("semantic gate state machine rejects invalid transitions", () => {
  const running = transitionSemanticGateState(SemanticGateState.Pending, SemanticGateEvent.Start);
  const passed = transitionSemanticGateState(running, SemanticGateEvent.Pass);
  const reported = transitionSemanticGateState(passed, SemanticGateEvent.Report);

  assert.equal(running, SemanticGateState.Running);
  assert.equal(passed, SemanticGateState.Passed);
  assert.equal(reported, SemanticGateState.Reported);
  assert.throws(
    () => transitionSemanticGateState(SemanticGateState.Pending, SemanticGateEvent.Report),
    (error) => error.code === SemanticGateErrorCode.InvalidTransition,
  );
});

test("semantic gate markdown records marker without vector dump", () => {
  const gate = analyzeSemanticGateSources({ sources: completeSources });
  const markdown = renderSemanticGateMarkdown(gate);

  assert.match(markdown, /# Phase 005 Semantic Search Gate Result/);
  assert.match(markdown, /phase005_semantic_search_gate=passed/);
  assert.doesNotMatch(markdown, /\[0\.123, 0\.456\]/);
});
