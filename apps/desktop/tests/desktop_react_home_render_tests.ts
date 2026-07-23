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
import type { DesktopGraphSurfaceSnapshot } from "../src/desktop_graph_controller.ts";

const readyKnowledgeGraph: DesktopGraphSurfaceSnapshot = {
  state: "Ready",
  workspaceId: "workspace-1",
  generation: 1,
  query: {
    scope: "global",
    depth: 1,
    direction: "both",
    includeUnresolved: true,
    includeAssets: false,
    nodeLimit: 120,
    edgeLimit: 240,
  },
  graph: {
    status: "clean",
    nodes: [
      { id: "doc-internal-alpha", kind: "document", label: "제품 방향", availability: "available", canNavigate: true },
      { id: "doc-internal-beta", kind: "document", label: "로컬 저장소 설계", availability: "available", canNavigate: true },
    ],
    edges: [
      { id: "edge-internal-alpha-beta", sourceId: "doc-internal-alpha", targetId: "doc-internal-beta", kind: "document_link" },
    ],
  },
};

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
        documentCount: 10_000,
        assetCount: 2_500,
        canvasCount: 24,
        summaryUnavailable: ["Assets"],
      }),
    ),
  );

  assert.match(loading, /data-cabinet-home-state="Loading"/);
  assert.match(loading, /aria-live="polite"/);
  assert.match(ready, /data-cabinet-react-root="mounted"/);
  assert.match(ready, /data-action="new-document"/);
  assert.match(ready, /<nav[^>]+aria-label="주요 메뉴"/);
  assert.match(ready, /Source/);
  assert.doesNotMatch(ready, />notes</);
  assert.doesNotMatch(ready, /notes|source\.md|notes\/source\.md/);
  assert.match(ready, /Review draft/);
  assert.match(ready, /백업 상태/);
  assert.match(ready, /<strong>문서<\/strong> 10000/);
  assert.match(ready, /<strong>첨부<\/strong> 확인 필요/);
  assert.match(ready, /<strong>Canvas<\/strong> 24/);
  assert.doesNotMatch(ready, /<strong>즐겨찾기<\/strong>/);
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

test("React desktop home follows the Penpot 20260721 workspace composition", () => {
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
        knowledgeGraph: readyKnowledgeGraph,
      },
    ),
  );

  assert.match(markup, /data-design-reference="penpot-20260721"/);
  assert.match(markup, />Cabinet</);
  assert.match(markup, /내 캐비닛/);
  assert.match(markup, /문서와 첨부 파일 검색/);
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
  assert.match(markup, /data-document-title="/);
  assert.match(markup, /data-document-id="doc-2"/);
  const continueSection = markup.match(/<section class="dashboard-section continue-section"[\s\S]*?<\/section>/)?.[0] ?? "";
  assert.equal((continueSection.match(/data-action="open-recent-document"/g) ?? []).length, 1);
  assert.match(continueSection, /Cabinet 제품 방향/);
  assert.doesNotMatch(continueSection, /로컬 저장소 설계/);
  assert.doesNotMatch(markup, />projects<|>architecture<|projects|architecture|cabinet\.md|storage\.md/);
  assert.match(markup, /data-action="navigate-home"[^>]*disabled/);
  assert.match(markup, /data-action="workspace-search-input"/);
  assert.doesNotMatch(markup, /data-action="navigate-document"[^>]*disabled/);
  assert.doesNotMatch(markup, /data-action="(?:open-settings|toggle-theme|open-ai)"/);
  assert.equal((markup.match(/class="desktop-sidebar"/g) ?? []).length, 1);
  assert.equal((markup.match(/class="desktop-topbar"/g) ?? []).length, 1);
  assert.match(markup, />백업과 복원</);
});

test("home knowledge map renders authoritative graph labels and connection counts without internal identities", () => {
  const profile = createPersonalLocalDesktopCapabilityProfile();
  const model = createPersonalWorkspaceHomeModelFromResult(profile, {
    workspaceId: "workspace-1",
    state: "Ready",
    recentDocuments: [],
    favorites: [],
    tags: [],
    recentChanges: [],
    unfinishedItems: [],
    backupStatus: "Fresh",
    healthStatus: "Healthy",
  });
  const markup = renderToStaticMarkup(createDesktopWorkspaceHomeElement(model, {
    knowledgeGraph: readyKnowledgeGraph,
    onOpenDocument() {},
    onOpenGraph() {},
  }));

  assert.match(markup, /data-knowledge-map-source="authoritative-graph"/);
  assert.match(markup, />제품 방향</);
  assert.match(markup, />로컬 저장소 설계</);
  assert.match(markup, /2개 항목 · 1개 연결/);
  assert.match(markup, /data-action="open-home-graph-document"/);
  assert.doesNotMatch(markup, /doc-internal-alpha|doc-internal-beta|edge-internal-alpha-beta/);
});

test("home knowledge map reports loading, empty, and failed graph states truthfully", () => {
  const profile = createPersonalLocalDesktopCapabilityProfile();
  const model = createPersonalWorkspaceHomeModelFromResult(profile, {
    workspaceId: "workspace-1",
    state: "Ready",
    recentDocuments: [],
    favorites: [],
    tags: [],
    recentChanges: [],
    unfinishedItems: [],
    backupStatus: "Fresh",
    healthStatus: "Healthy",
  });
  const loading = renderToStaticMarkup(createDesktopWorkspaceHomeElement(model, {
    knowledgeGraph: { ...readyKnowledgeGraph, state: "Loading", graph: undefined },
  }));
  const empty = renderToStaticMarkup(createDesktopWorkspaceHomeElement(model, {
    knowledgeGraph: { ...readyKnowledgeGraph, state: "Empty", graph: { status: "clean", nodes: [], edges: [] } },
  }));
  const failed = renderToStaticMarkup(createDesktopWorkspaceHomeElement(model, {
    knowledgeGraph: { ...readyKnowledgeGraph, state: "Failed", graph: undefined, errorCode: "SECRET_GRAPH_ERROR", retryable: true },
    onRetryKnowledgeGraph() {},
  }));

  assert.match(loading, /지식 지도를 불러오는 중입니다/);
  assert.match(empty, /아직 표시할 문서 관계가 없습니다/);
  assert.match(failed, /지식 지도를 불러오지 못했습니다/);
  assert.match(failed, /data-action="retry-home-knowledge-map"/);
  assert.doesNotMatch(failed, /SECRET_GRAPH_ERROR/);
});
