import assert from "node:assert/strict";
import test from "node:test";
import { renderToStaticMarkup } from "react-dom/server";

import { createPersonalLocalDesktopCapabilityProfile } from "../../../packages/client-core/src/index.ts";
import {
  createPersonalWorkspaceHomeFailedModel,
  createPersonalWorkspaceHomeModel,
  createPersonalWorkspaceHomeModelFromResult,
} from "../../../packages/ui/src/index.ts";
import { createDesktopWorkspaceHomeElement } from "../src/react_workspace_home.ts";

test("React desktop home renders semantic loading and command-backed ready content", () => {
  const profile = createPersonalLocalDesktopCapabilityProfile();
  const loading = renderToStaticMarkup(
    createDesktopWorkspaceHomeElement(
      createPersonalWorkspaceHomeModel({ profile, healthState: "Loading" }),
    ),
  );
  const ready = renderToStaticMarkup(
    createDesktopWorkspaceHomeElement(
      createPersonalWorkspaceHomeModelFromResult(profile, {
        workspaceId: "workspace-1",
        state: "Ready",
        recentDocuments: [{ documentId: "doc-1", title: "Source", path: "notes/source.md" }],
        favorites: [],
        tags: [{ label: "rust", documentCount: 1 }],
        recentChanges: [{ documentId: "doc-1", summary: "Updated document" }],
        unfinishedItems: [{ documentId: "doc-1", label: "Review draft" }],
        backupStatus: "Fresh",
        healthStatus: "Healthy",
      }),
    ),
  );

  assert.match(loading, /data-cabinet-home-state="Loading"/);
  assert.match(loading, /aria-live="polite"/);
  assert.match(ready, /data-cabinet-react-root="mounted"/);
  assert.match(ready, /data-action="new-document"/);
  assert.match(ready, /<nav[^>]+aria-label="주요 메뉴"/);
  assert.match(ready, /Source/);
  assert.match(ready, />notes</);
  assert.doesNotMatch(ready, /notes\/source\.md/);
  assert.match(ready, /Review draft/);
  assert.match(ready, /백업 상태/);
  assertNoUnidentifiedInteractiveControls(ready);
  assert.doesNotMatch(ready, /Workspace status|Favorites|Tags|Recent changes|Backup status|Fresh/);
  assert.doesNotMatch(ready, /server|tenant|billing|admin-console/i);
});

function assertNoUnidentifiedInteractiveControls(markup: string): void {
  const controls = markup.match(/<(?:button|input|select|textarea|a)\b[^>]*>/g) ?? [];
  assert.ok(controls.length > 0);
  assert.deepEqual(controls.filter((control) => !control.includes("data-action=")), []);
}

test("React desktop home renders empty, degraded, failed, and retry states safely", () => {
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
  const failed = createPersonalWorkspaceHomeFailedModel(
    profile,
    "WORKSPACE_HOME_PROJECTION_UNAVAILABLE",
    true,
  );

  const emptyMarkup = renderToStaticMarkup(createDesktopWorkspaceHomeElement(empty));
  const degradedMarkup = renderToStaticMarkup(createDesktopWorkspaceHomeElement(degraded));
  const failedMarkup = renderToStaticMarkup(
    createDesktopWorkspaceHomeElement(failed, { onRetry() {} }),
  );

  assert.match(emptyMarkup, /data-cabinet-home-state="Empty"/);
  assert.match(emptyMarkup, /data-action="new-document"/);
  assert.doesNotMatch(emptyMarkup, /Create document/);
  assert.match(degradedMarkup, /data-cabinet-home-state="Degraded"/);
  assert.match(degradedMarkup, /읽기 전용/);
  assert.match(failedMarkup, /data-cabinet-home-state="Failed"/);
  assert.match(failedMarkup, /다시 시도/);
  assert.doesNotMatch(failedMarkup, /WORKSPACE_HOME_PROJECTION_UNAVAILABLE|COMMAND_BRIDGE_FAILED/);
  assert.doesNotMatch(failedMarkup, /private|raw native error|app-data/i);
});

test("React desktop home follows the Penpot 20260713 workspace composition", () => {
  const profile = createPersonalLocalDesktopCapabilityProfile();
  const markup = renderToStaticMarkup(
    createDesktopWorkspaceHomeElement(
      createPersonalWorkspaceHomeModelFromResult(profile, {
        workspaceId: "workspace-1",
        state: "Ready",
        recentDocuments: [
          { documentId: "doc-1", title: "Cabinet 제품 방향", path: "projects/cabinet.md" },
          { documentId: "doc-2", title: "로컬 저장소 설계", path: "architecture/storage.md" },
        ],
        favorites: [],
        tags: [{ label: "Rust", documentCount: 3 }],
        recentChanges: [{ documentId: "doc-1", summary: "12분 전 수정" }],
        unfinishedItems: [{ documentId: "doc-2", label: "검색 메모 정리" }],
        backupStatus: "Fresh",
        healthStatus: "Healthy",
      }),
      {
        onCreateDocument() {},
        onOpenNavigator() {},
        onResumeDocument() {},
        onOpenDocument() {},
      },
    ),
  );

  assert.match(markup, /data-design-reference="penpot-20260713"/);
  assert.match(markup, />Cabinet</);
  assert.match(markup, /내 캐비닛/);
  assert.match(markup, /검색어를 입력하세요/);
  assert.match(markup, /좋은 오후예요/);
  assert.match(markup, /이어서 작업하기/);
  assert.match(markup, /최근 문서/);
  assert.match(markup, /내 지식 지도/);
  assert.match(markup, /오늘/);
  assert.doesNotMatch(markup, /내 문서에 질문하기|ask-documents|질문 시작|>AI</);
  assert.doesNotMatch(markup, /Workspace overview|Workspace status|Favorites|Tags|Recent changes|Backup status|Fresh/);
  assert.doesNotMatch(markup, /projects\/cabinet\.md|architecture\/storage\.md/);
  assert.match(markup, /data-document-id="doc-1"/);
  assert.match(markup, /data-action="open-recent-document"/);
  assert.match(markup, /data-document-id="doc-2"/);
  assert.match(markup, /data-action="navigate-home"[^>]*disabled/);
  assert.match(markup, /data-action="navigate-search"/);
  assert.doesNotMatch(markup, /data-action="navigate-document"[^>]*disabled/);
  assert.doesNotMatch(markup, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.equal((markup.match(/class="desktop-sidebar"/g) ?? []).length, 1);
  assert.equal((markup.match(/class="desktop-topbar"/g) ?? []).length, 1);
  assert.match(markup, />백업 및 복원</);
});
