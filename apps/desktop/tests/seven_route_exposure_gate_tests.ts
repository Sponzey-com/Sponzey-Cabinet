import assert from "node:assert/strict";
import test from "node:test";
import { renderToStaticMarkup } from "react-dom/server";
import { createPersonalLocalDesktopCapabilityProfile } from "@sponzey-cabinet/client-core";
import { applyDocumentNavigatorResult, createDocumentNavigatorFailedModel, createDocumentNavigatorLoadingModel, createPersonalWorkspaceHomeFailedModel, createPersonalWorkspaceHomeModelFromResult, DocumentSaveCoordinatorState } from "@sponzey-cabinet/ui";
import { createDesktopWorkspaceHomeElement } from "../src/react_workspace_home.ts";
import { createDesktopDocumentNavigatorElement } from "../src/react_document_navigator.ts";
import { createDesktopDocumentAuthoringWorkbenchElement } from "../src/react_document_authoring_workbench.ts";
import { createDesktopAttachmentsElement, createDesktopCanvasElement, createDesktopKnowledgeGraphElement } from "../src/react_exploration_surfaces.ts";
import { createDesktopBackupRecoveryElement } from "../src/react_backup_recovery.ts";
import { createDesktopBackupRecoverySnapshot } from "../src/desktop_backup_recovery_controller.ts";
import { auditUserExposedMarkup } from "../src/ui_exposure_audit.ts";

const home = createPersonalWorkspaceHomeModelFromResult(createPersonalLocalDesktopCapabilityProfile(), { workspaceId: "workspace-secret", state: "Ready", healthStatus: "Healthy", backupStatus: "Fresh", recentDocuments: [{ documentId: "doc-secret", title: "설계 문서", path: "project/design.md" }], favorites: [], tags: [], recentChanges: [], unfinishedItems: [] });
const callbacks: any = new Proxy({}, {
  get: (_target, property) => property === "documentShortcuts" ? [] : () => {},
});

test("all seven ready routes expose no internal identity, error, path, or banned English action", () => {
  const loading = createDocumentNavigatorLoadingModel({ workspaceId: "workspace-secret", view: "Tree", generation: 1 });
  const navigator = applyDocumentNavigatorResult(loading, 1, { workspaceId: "workspace-secret", view: "Tree", state: "Ready", items: [{ documentId: "doc-secret", title: "설계 문서", path: "project/design.md", collections: [], tags: [], favorite: false }] });
  const routes = {
    Home: createDesktopWorkspaceHomeElement(home, callbacks),
    Search: createDesktopDocumentNavigatorElement(navigator, callbacks),
    Document: createDesktopDocumentAuthoringWorkbenchElement({ workspaceId: "workspace-secret", documentId: "doc-secret", title: "설계 문서", path: "project/design.md", body: "# 설계", revision: 1, persistedRevision: 1, expectedVersionId: "version-secret", saveState: DocumentSaveCoordinatorState.Saved }, callbacks),
    Graph: createDesktopKnowledgeGraphElement(home, { state: "Ready", workspaceId: "workspace-secret", generation: 1, query: { depth: 1, direction: "both", includeUnresolved: true, includeAssets: false, nodeLimit: 120, edgeLimit: 240 }, graph: { status: "clean", nodes: [{ id: "doc-secret", kind: "document" }], edges: [], stats: { candidateCount: 1, filteredCount: 0 }, freshnessRevision: "version-secret" } }, callbacks),
    Canvas: createDesktopCanvasElement(home, { state: "Ready", workspaceId: "workspace-secret", canvasId: "canvas-secret", generation: 1, selectedNodeIds: [], canvas: { canvasId: "canvas-secret", title: "설계 캔버스", lifecycle: "active", revision: 1, nodes: [], edges: [], viewport: { centerX: 0, centerY: 0, zoomPercent: 100 } } }, callbacks),
    Assets: createDesktopAttachmentsElement(home, { state: "Empty", workspaceId: "workspace-secret", documentId: "doc-secret", scope: "Document", generation: 1, importState: "Idle" }, callbacks),
    Backup: createDesktopBackupRecoveryElement(createDesktopBackupRecoverySnapshot("workspace-secret"), callbacks),
  };
  for (const [route, element] of Object.entries(routes)) assert.deepEqual(auditUserExposedMarkup(renderToStaticMarkup(element)), [], route);
});

test("representative error and recovery routes expose mapped messages only", () => {
  const profile = createPersonalLocalDesktopCapabilityProfile();
  const loading = createDocumentNavigatorLoadingModel({ workspaceId: "workspace-secret", view: "Tree", generation: 1 });
  const routes = {
    Home: createDesktopWorkspaceHomeElement(createPersonalWorkspaceHomeFailedModel(profile, "WORKSPACE_HOME_PROJECTION_UNAVAILABLE", true), callbacks),
    Search: createDesktopDocumentNavigatorElement(createDocumentNavigatorFailedModel({ workspaceId: "workspace-secret", view: "Tree", generation: 2, errorCode: "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE", retryable: true }), callbacks),
    Document: createDesktopDocumentAuthoringWorkbenchElement({ workspaceId: "workspace-secret", documentId: "doc-secret", title: "설계 문서", path: "project/design.md", body: "# 설계", revision: 2, persistedRevision: 1, expectedVersionId: "version-secret", saveState: DocumentSaveCoordinatorState.SaveFailed, errorCode: "DOCUMENT_AUTHORING_STORAGE_UNAVAILABLE", retryable: true }, callbacks),
    Canvas: createDesktopCanvasElement(home, { state: "RecoveryRequired", workspaceId: "workspace-secret", canvasId: "canvas-secret", generation: 2, selectedNodeIds: [], errorCode: "CANVAS_RECOVERY_REQUIRED", retryable: false }, callbacks),
    Backup: createDesktopBackupRecoveryElement({ ...createDesktopBackupRecoverySnapshot("workspace-secret"), state: "CleanupRequired", errorCode: "RESTORE_CLEANUP_REQUIRED" }, callbacks),
  };
  void loading;
  for (const [route, element] of Object.entries(routes)) assert.deepEqual(auditUserExposedMarkup(renderToStaticMarkup(element)), [], route);
});
