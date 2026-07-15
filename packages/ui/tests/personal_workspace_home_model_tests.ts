import assert from "node:assert/strict";
import test from "node:test";

import { createPersonalLocalDesktopCapabilityProfile } from "../../client-core/src/index.ts";
import {
  createPersonalWorkspaceCommandPaletteActions,
  createPersonalWorkspaceHomeModel,
  createPersonalWorkspaceHomeModelFromResult,
  createPersonalWorkspaceHomeFailedModel,
  transitionPersonalWorkspaceAppFrameState,
  PersonalWorkspaceAppFrameEvent,
  PersonalWorkspaceAppFrameState,
  PersonalWorkspaceAppFrameErrorCode,
} from "../src/index.ts";

test("personal workspace home model exposes required daily workspace sections", () => {
  const model = createPersonalWorkspaceHomeModel({
    profile: createPersonalLocalDesktopCapabilityProfile(),
    healthState: "Ready",
    summary: {
      recentDocumentCount: 2,
      favoriteCount: 1,
      tagCount: 3,
      recentChangeCount: 4,
      backupState: "Fresh",
    },
  });

  assert.equal(model.mode, "personal-workspace-home");
  assert.equal(model.productScope, "personal_local_desktop");
  assert.equal(model.firstRoute, "home");
  assert.deepEqual(
    model.sections.map((section) => section.id),
    [
      "recent-documents",
      "favorites",
      "tags",
      "recent-changes",
      "unfinished-items",
      "quick-search",
      "ai-entry",
      "backup-status",
      "workspace-health",
    ],
  );
  assert.equal(model.sections.find((section) => section.id === "recent-documents")?.itemCount, 2);
  assert.equal(model.sections.find((section) => section.id === "quick-search")?.primaryActionId, "quick-search");
  assert.equal(model.sections.find((section) => section.id === "ai-entry")?.primaryActionId, "ask-ai");
  assert.equal(model.sections.find((section) => section.id === "backup-status")?.status, "Fresh");
});

test("personal workspace home maps command-backed ready items and actions", () => {
  const model = createPersonalWorkspaceHomeModelFromResult(
    createPersonalLocalDesktopCapabilityProfile(),
    {
      workspaceId: "workspace-1",
      state: "Ready",
      recentDocuments: [{ documentId: "doc-1", title: "Source", path: "notes/source.md" }],
      favorites: [{ documentId: "doc-2", title: "Favorite", path: "notes/favorite.md" }],
      tags: [{ label: "rust", documentCount: 2 }],
      recentChanges: [{ documentId: "doc-1", summary: "Updated document" }],
      unfinishedItems: [{ documentId: "doc-3", label: "Review draft" }],
      backupStatus: "Fresh",
      healthStatus: "Healthy",
    },
  );

  assert.equal(model.displayState, "Ready");
  assert.equal(model.recentDocuments[0]?.documentId, "doc-1");
  assert.equal(model.recentDocuments[0]?.actionId, "open-document");
  assert.equal(model.favorites[0]?.documentId, "doc-2");
  assert.equal(model.tags[0]?.documentCount, 2);
  assert.equal(model.recentChanges[0]?.summary, "Updated document");
  assert.equal(model.unfinishedItems[0]?.label, "Review draft");
  assert.equal(model.sections.find((section) => section.id === "unfinished-items")?.itemCount, 1);
  assert.equal(model.sections.find((section) => section.id === "backup-status")?.status, "Fresh");
});

test("personal workspace home distinguishes empty degraded and safe failed models", () => {
  const profile = createPersonalLocalDesktopCapabilityProfile();
  const empty = createPersonalWorkspaceHomeModelFromResult(profile, {
    workspaceId: "workspace-1",
    state: "Empty",
    recentDocuments: [],
    favorites: [],
    tags: [],
    recentChanges: [],
    unfinishedItems: [],
    backupStatus: "NeverCreated",
    healthStatus: "Healthy",
  });
  const degraded = createPersonalWorkspaceHomeModelFromResult(profile, {
    workspaceId: "workspace-1",
    state: "Degraded",
    recentDocuments: [],
    favorites: [],
    tags: [],
    recentChanges: [],
    unfinishedItems: [],
    backupStatus: "Failed",
    healthStatus: "ReadOnlyRecovery",
  });
  const failed = createPersonalWorkspaceHomeFailedModel(profile, "STORE_UNAVAILABLE", true);

  assert.equal(empty.displayState, "Empty");
  assert.equal(degraded.displayState, "Degraded");
  assert.equal(degraded.health.displayState, "read-only-recovery");
  assert.equal(failed.displayState, "Failed");
  assert.deepEqual(failed.error, {
    code: "STORE_UNAVAILABLE",
    retryable: true,
    actionId: "retry-workspace-home",
  });
  assert.equal(JSON.stringify(failed).includes("raw transport message"), false);
});

test("personal workspace command palette exposes only local user actions", () => {
  const actions = createPersonalWorkspaceCommandPaletteActions(
    createPersonalLocalDesktopCapabilityProfile().actions,
  );

  assert.deepEqual(
    actions.map((action) => action.id),
    [
      "new-document",
      "quick-search",
      "open-graph",
      "ask-ai",
      "create-backup",
      "import-markdown",
      "export-package",
      "open-settings",
    ],
  );

  const serialized = JSON.stringify(actions);
  for (const forbidden of [
    "team-invite",
    "admin-console",
    "billing",
    "tenant-settings",
    "sso-settings",
    "server-workspace-connect",
  ]) {
    assert.equal(serialized.includes(forbidden), false);
  }
});

test("personal workspace app frame state machine blocks failed to home without recovery", () => {
  const loading = transitionPersonalWorkspaceAppFrameState(
    PersonalWorkspaceAppFrameState.Booting,
    PersonalWorkspaceAppFrameEvent.StartLoading,
  );
  const ready = transitionPersonalWorkspaceAppFrameState(
    loading.state,
    PersonalWorkspaceAppFrameEvent.WorkspaceLoaded,
  );
  const failed = transitionPersonalWorkspaceAppFrameState(
    ready.state,
    PersonalWorkspaceAppFrameEvent.Fail,
  );
  const invalid = transitionPersonalWorkspaceAppFrameState(
    failed.state,
    PersonalWorkspaceAppFrameEvent.WorkspaceLoaded,
  );
  const recovered = transitionPersonalWorkspaceAppFrameState(
    failed.state,
    PersonalWorkspaceAppFrameEvent.RecoveryCompleted,
  );

  assert.equal(loading.state, PersonalWorkspaceAppFrameState.LoadingWorkspace);
  assert.equal(ready.state, PersonalWorkspaceAppFrameState.HomeReady);
  assert.equal(failed.state, PersonalWorkspaceAppFrameState.Failed);
  assert.equal(invalid.state, PersonalWorkspaceAppFrameState.Failed);
  assert.equal(invalid.errorCode, PersonalWorkspaceAppFrameErrorCode.InvalidTransition);
  assert.equal(recovered.state, PersonalWorkspaceAppFrameState.HomeReady);
});
