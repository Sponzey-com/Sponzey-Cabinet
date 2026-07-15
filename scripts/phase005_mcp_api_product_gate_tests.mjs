import assert from "node:assert/strict";
import test from "node:test";

import {
  McpApiGateErrorCode,
  McpApiGateEvent,
  McpApiGateState,
  analyzeMcpApiGateSources,
  renderMcpApiGateMarkdown,
  transitionMcpApiGateState,
} from "./phase005_mcp_api_product_gate.mjs";

const completeSources = {
  "crates/cabinet-domain/src/tool.rs":
    "ToolId ToolScope ToolOperation ToolExecutionRequest ToolExecutionResult ToolExecutionState transition_tool_execution",
  "crates/cabinet-domain/tests/tool_tests.rs":
    "tool_execution_request_requires_explicit_scope tool_operation_maps_to_required_scope_without_direct_write_scope tool_execution_state_machine_rejects_invalid_transitions",
  "crates/cabinet-usecases/src/tool.rs":
    "AuthorizeToolExecutionUsecase ToolAuthorizationOutput ToolAuthorizationError",
  "crates/cabinet-usecases/tests/tool_usecase_tests.rs":
    "authorize_tool_execution_allows_request_with_required_scope authorize_tool_execution_denies_request_missing_required_scope authorize_tool_execution_limits_write_operation_to_draft_suggestion_scope",
  "crates/cabinet-adapters/src/tool_mapper.rs":
    "ExternalToolRequest ExternalToolKind ToolRequestMapper ToolMapperError",
  "crates/cabinet-adapters/tests/tool_mapper_tests.rs":
    "tool_mapper_maps_mcp_like_search_request_to_internal_request tool_mapper_maps_api_like_write_suggestion_request_without_direct_write tool_mapper_output_does_not_expose_token_or_credential_fixture",
};

test("MCP/API gate marks complete fixture as passed", () => {
  const gate = analyzeMcpApiGateSources({ sources: completeSources });

  assert.equal(gate.status, "passed");
  assert.equal(gate.summary.covered, 2);
  assert.equal(gate.summary.targetsNeedingWork, 0);
});

test("MCP/API gate reports missing mapper evidence", () => {
  const {
    "crates/cabinet-adapters/src/tool_mapper.rs": _mapper,
    ...sources
  } = completeSources;

  const gate = analyzeMcpApiGateSources({ sources });

  assert.equal(gate.status, "failed");
  assert.equal(gate.nextImplementationTarget.id, "tool_mapper_boundary");
});

test("MCP/API gate state machine rejects invalid transitions", () => {
  const running = transitionMcpApiGateState(McpApiGateState.Pending, McpApiGateEvent.Start);
  const passed = transitionMcpApiGateState(running, McpApiGateEvent.Pass);
  const reported = transitionMcpApiGateState(passed, McpApiGateEvent.Report);

  assert.equal(running, McpApiGateState.Running);
  assert.equal(passed, McpApiGateState.Passed);
  assert.equal(reported, McpApiGateState.Reported);
  assert.throws(
    () => transitionMcpApiGateState(McpApiGateState.Pending, McpApiGateEvent.Report),
    (error) => error.code === McpApiGateErrorCode.InvalidTransition,
  );
});

test("MCP/API gate markdown records marker without sensitive raw fixtures", () => {
  const gate = analyzeMcpApiGateSources({ sources: completeSources });
  const markdown = renderMcpApiGateMarkdown(gate);

  assert.match(markdown, /# Phase 005 MCP API Product Gate Result/);
  assert.match(markdown, /phase005_mcp_api_product_gate=passed/);
  assert.doesNotMatch(markdown, /provider_api_key_fixture/);
  assert.doesNotMatch(markdown, /connector_access_token_fixture/);
  assert.doesNotMatch(markdown, /raw transport/);
  assert.doesNotMatch(markdown, /token value/);
});
