import assert from "node:assert/strict";
import test from "node:test";

import { createPersonalLocalDesktopCapabilityProfile } from "../../client-core/src/index.ts";
import type {
  AiCitationCardViewModel,
  LocalAiToolDescriptorView,
} from "../src/index.ts";
import {
  createAiCitationSourceOpenAction,
  createAiProviderSettingsViewModel,
  createLocalAiToolScopeViewModel,
  transitionAiCitationSourceState,
} from "../src/index.ts";

test("AI citation source open action separates current document and version reads", () => {
  const citation = citationCard("fresh");
  const current = createAiCitationSourceOpenAction({
    workspaceId: "workspace-1",
    citation,
    target: { kind: "current" },
    sourceState: "CitationReady",
  });
  const version = createAiCitationSourceOpenAction({
    workspaceId: "workspace-1",
    citation,
    target: { kind: "version", versionId: "version-3" },
    sourceState: "SourceStale",
  });

  assert.equal(current.state, "CitationReady");
  assert.equal(current.canOpen, true);
  assert.deepEqual(current.command, {
    type: "open-current-document",
    workspaceId: "workspace-1",
    documentId: "document-1",
    citationReference: "citation:document-1:1",
    anchor: "source-heading",
  });

  assert.equal(version.state, "SourceStale");
  assert.equal(version.warningCode, "AI_CITATION_SOURCE_STALE");
  assert.deepEqual(version.command, {
    type: "open-document-version",
    workspaceId: "workspace-1",
    documentId: "document-1",
    versionId: "version-3",
    citationReference: "citation:document-1:1",
    anchor: "source-heading",
  });
});

test("AI citation source open action reports unavailable and access denied without source content", () => {
  const unavailable = createAiCitationSourceOpenAction({
    workspaceId: "workspace-1",
    citation: citationCard("unknown"),
    target: { kind: "current" },
    sourceState: "SourceUnavailable",
  });
  const denied = createAiCitationSourceOpenAction({
    workspaceId: "workspace-1",
    citation: citationCard("fresh"),
    target: { kind: "current" },
    sourceState: "SourceAccessDenied",
  });

  assert.equal(unavailable.canOpen, false);
  assert.equal(unavailable.errorCode, "AI_CITATION_SOURCE_UNAVAILABLE");
  assert.equal(denied.canOpen, false);
  assert.equal(denied.errorCode, "AI_CITATION_SOURCE_ACCESS_DENIED");

  const serialized = JSON.stringify([unavailable, denied]);
  assert.equal(serialized.includes("retrieval_source_text_fixture"), false);
  assert.equal(serialized.includes("raw markdown"), false);
});

test("AI citation source state machine exposes explicit transitions", () => {
  const ready = transitionAiCitationSourceState("NoCitation", "OpenCurrentRequested");
  const stale = transitionAiCitationSourceState("CitationReady", "SourceStaleDetected");
  const denied = transitionAiCitationSourceState("CitationReady", "AccessDenied");
  const invalid = transitionAiCitationSourceState("SourceAccessDenied", "OpenVersionRequested");

  assert.deepEqual(ready, { state: "CitationReady" });
  assert.deepEqual(stale, { state: "SourceStale", warningCode: "AI_CITATION_SOURCE_STALE" });
  assert.deepEqual(denied, {
    state: "SourceAccessDenied",
    errorCode: "AI_CITATION_SOURCE_ACCESS_DENIED",
  });
  assert.equal(invalid.state, "SourceAccessDenied");
  assert.equal(invalid.errorCode, "AI_CITATION_SOURCE_INVALID_TRANSITION");
});

test("local AI tool scope view hides server admin and destructive tools", () => {
  const model = createLocalAiToolScopeViewModel({
    profile: createPersonalLocalDesktopCapabilityProfile(),
    tools: [
      tool("tool.read", "Read current document", "read-document"),
      tool("tool.search", "Search local index", "search-documents"),
      tool("tool.citation", "Open citation source", "open-citation"),
      tool("tool.ask", "Ask AI", "ask-ai"),
      tool("tool.write", "Write document", "write-document"),
      tool("tool.delete", "Delete document", "delete-document"),
      tool("tool.admin", "Admin console", "admin"),
      tool("tool.server", "Server setup", "server"),
      tool("tool.team", "Team invite", "team"),
      tool("tool.billing", "Billing", "billing"),
      tool("tool.sso", "SSO", "sso"),
    ],
  });

  assert.equal(model.mode, "local-ai-tool-scope");
  assert.equal(model.state, "VisibleReadOnly");
  assert.deepEqual(
    model.tools.map((item) => item.id),
    ["tool.read", "tool.search", "tool.citation", "tool.ask"],
  );
  assert.deepEqual(model.hiddenToolIds, [
    "tool.write",
    "tool.delete",
    "tool.admin",
    "tool.server",
    "tool.team",
    "tool.billing",
    "tool.sso",
  ]);
});

test("AI provider settings model is optional and excludes credentials", () => {
  const notConfigured = createAiProviderSettingsViewModel({
    state: "NotConfigured",
    credentialHandlePresent: false,
  });
  const configured = createAiProviderSettingsViewModel({
    state: "Configured",
    providerName: "Local Fake Provider",
    modelName: "fake-model",
    credentialHandlePresent: true,
    validationCode: "provider_api_key_fixture",
  });

  assert.equal(notConfigured.mode, "ai-provider-settings");
  assert.equal(notConfigured.blocksLocalWorkspace, false);
  assert.deepEqual(
    notConfigured.actions.map((action) => action.id),
    ["open-optional-provider-settings"],
  );
  assert.equal(configured.credentialState, "handle-present");

  const serialized = JSON.stringify([notConfigured, configured]);
  assert.equal(serialized.includes("provider_api_key_fixture"), false);
  assert.equal(serialized.includes("connector_access_token_fixture"), false);
  assert.equal(serialized.includes("AI_PROVIDER_KEY"), false);
  assert.equal(serialized.includes("endpoint"), false);
});

function citationCard(freshness: AiCitationCardViewModel["freshness"]): AiCitationCardViewModel {
  return {
    sourceId: "document-1",
    sourceKind: "document",
    sourceTitle: "Source Document",
    citationReference: "citation:document-1:1",
    headingAnchor: "source-heading",
    blockReference: "block-1",
    freshness,
    permissionDecision: "allowed",
  };
}

function tool(
  id: string,
  label: string,
  operation: LocalAiToolDescriptorView["operation"],
): LocalAiToolDescriptorView {
  return {
    id,
    label,
    operation,
  };
}
