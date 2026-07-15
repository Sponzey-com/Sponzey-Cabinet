import assert from "node:assert/strict";
import test from "node:test";

import { createPersonalLocalDesktopCapabilityProfile } from "../../../packages/client-core/src/index.ts";
import type {
  AiCitationCardViewModel,
  AiProviderSettingsSummaryView,
  LocalAiToolDescriptorView,
} from "../../../packages/ui/src/index.ts";
import {
  createDesktopAiCitationSourceOpenAction,
  createDesktopAiProviderSettings,
  createDesktopLocalAiToolScope,
} from "../src/index.ts";

test("desktop AI local UX smoke separates citation source current and history opens", () => {
  const current = createDesktopAiCitationSourceOpenAction({
    workspaceId: "workspace-1",
    citation: citationCard(),
    target: { kind: "current" },
    sourceState: "CitationReady",
  });
  const version = createDesktopAiCitationSourceOpenAction({
    workspaceId: "workspace-1",
    citation: citationCard(),
    target: { kind: "version", versionId: "version-9" },
    sourceState: "SourceStale",
  });

  assert.equal(current.command?.type, "open-current-document");
  assert.equal(current.command?.anchor, "source-heading");
  assert.equal(version.command?.type, "open-document-version");
  assert.equal(version.warningCode, "AI_CITATION_SOURCE_STALE");
});

test("desktop AI local UX smoke exposes read-only tool scope", () => {
  const model = createDesktopLocalAiToolScope({
    profile: createPersonalLocalDesktopCapabilityProfile(),
    tools: [
      tool("tool.read", "Read", "read-document"),
      tool("tool.search", "Search", "search-documents"),
      tool("tool.citation", "Citation", "open-citation"),
      tool("tool.write", "Write", "write-document"),
      tool("tool.admin", "Admin", "admin"),
      tool("tool.sso", "SSO", "sso"),
    ],
  });

  assert.equal(model.state, "VisibleReadOnly");
  assert.deepEqual(
    model.tools.map((item) => item.id),
    ["tool.read", "tool.search", "tool.citation"],
  );
  assert.deepEqual(model.hiddenToolIds, ["tool.write", "tool.admin", "tool.sso"]);
});

test("desktop AI local UX smoke keeps provider setup optional and secret-free", () => {
  const settings: AiProviderSettingsSummaryView = {
    state: "NotConfigured",
    credentialHandlePresent: false,
    validationCode: "provider_api_key_fixture",
  };
  const model = createDesktopAiProviderSettings(settings);
  const serialized = JSON.stringify(model);

  assert.equal(model.blocksLocalWorkspace, false);
  assert.equal(model.credentialState, "not-configured");
  assert.equal(serialized.includes("provider_api_key_fixture"), false);
  assert.equal(serialized.includes("AI_PROVIDER_KEY"), false);
  assert.equal(serialized.includes("endpoint"), false);
});

function citationCard(): AiCitationCardViewModel {
  return {
    sourceId: "document-1",
    sourceKind: "document",
    sourceTitle: "Source Document",
    citationReference: "citation:document-1:1",
    headingAnchor: "source-heading",
    blockReference: "block-1",
    freshness: "fresh",
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
