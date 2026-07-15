import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import {
  compareActionInventory,
  summarizeGeometryDeltas,
  transitionActionGeometryCapture,
  validateActionGeometryReport,
} from "./phase013_action_geometry_baseline.mjs";

const routes = Object.freeze(["Home", "Search", "Document", "Graph", "Canvas", "Assets", "Backup"]);
const viewports = Object.freeze([
  { width: 1024, height: 768 },
  { width: 1280, height: 800 },
  { width: 1440, height: 900 },
  { width: 1728, height: 1117 },
  { width: 1920, height: 1080 },
]);

export async function runPhase013ActionGeometryBaseline({
  root,
  chromePath,
  timeoutMs = 20_000,
  captureProfile = Object.freeze({
    marker: "phase013_action_geometry_baseline=recorded",
    fixtureVersion: "phase013-baseline-v1",
    textZoomPercent: 100,
    fixtureItemCount: 1,
    longContentFixture: false,
  }),
  captureViewports = viewports,
}) {
  let state = "Pending";
  const sourceFingerprint = await fingerprintSources(root);
  const manifestEntries = await readManifestEntries(root);
  const webPort = await freePort();
  const debugPort = await freePort();
  const profile = await mkdtemp(join(tmpdir(), "sponzey-phase013-action-geometry-"));
  const children = [];
  try {
    state = transitionActionGeometryCapture(state, "Serve");
    const server = spawn(process.execPath, ["scripts/run_web_app.mjs", String(webPort)], {
      cwd: root,
      env: { ...process.env, SPONZEY_CABINET_RUNNER_ANNOUNCED: "1", SPONZEY_CABINET_WEB_PUBLIC_DIR: "apps/desktop/dist" },
      stdio: "ignore",
    });
    children.push(server);
    await waitForHttp(`http://127.0.0.1:${webPort}/`, timeoutMs);
    const chrome = spawn(chromePath, [
      "--headless=new", "--disable-gpu", "--disable-background-networking", "--disable-default-apps",
      "--disable-extensions", "--no-default-browser-check", "--no-first-run",
      `--remote-debugging-port=${debugPort}`, `--user-data-dir=${profile}`, "about:blank",
    ], { cwd: root, stdio: "ignore" });
    children.push(chrome);
    await waitForHttp(`http://127.0.0.1:${debugPort}/json/version`, timeoutMs);
    const target = await pageTarget(debugPort, timeoutMs);
    const cdp = await CdpClient.connect(target.webSocketDebuggerUrl);
    try {
      await cdp.send("Page.enable");
      await cdp.send("Runtime.enable");
      await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: injectedTauriSource(captureProfile) });
      state = transitionActionGeometryCapture(state, "Browse");
      const runs = [];
      const actions = [];
      for (const viewport of captureViewports) {
        const scale = captureProfile.textZoomPercent / 100;
        await cdp.send("Emulation.setDeviceMetricsOverride", {
          width: Math.round(viewport.width / scale),
          height: Math.round(viewport.height / scale),
          screenWidth: viewport.width,
          screenHeight: viewport.height,
          deviceScaleFactor: scale,
          mobile: false,
        });
        for (const route of routes) {
          await navigateToRoute(cdp, webPort, route, timeoutMs);
          state = transitionActionGeometryCapture(state, "Capture");
          const captured = await evaluate(cdp, captureExpression(route));
          runs.push({ ...viewport, route, state: captured.state, ...captured.geometry });
          if (viewport === captureViewports[0]) actions.push(...captured.actions);
          state = transitionActionGeometryCapture(state, "Continue");
        }
      }
      const gaps = compareActionInventory(actions, manifestEntries);
      const report = {
        marker: captureProfile.marker,
        sourceFingerprint,
        fixtureVersion: captureProfile.fixtureVersion,
        textZoomPercent: captureProfile.textZoomPercent,
        fixtureItemCount: captureProfile.fixtureItemCount,
        longContentFixture: captureProfile.longContentFixture,
        diagnostics: "sanitized",
        browserSurface: "local_headless_chrome_cdp",
        actions,
        gaps,
        geometryDeltas: summarizeGeometryDeltas(runs, routes, captureViewports),
        runs,
      };
      if (captureProfile.marker === "phase013_action_geometry_baseline=recorded") {
        const validation = validateActionGeometryReport(report, { fingerprint: sourceFingerprint, routes, viewports: captureViewports });
        if (!validation.passed) {
          const missing = actions.filter((action) => action.identityMissing || !action.hasAccessibleName);
          throw new Error(`action geometry validation failed: ${validation.findingIds.join(",")}; controls=${JSON.stringify(missing)}`);
        }
      }
      state = transitionActionGeometryCapture("Capturing", "Pass");
      return { ...report, state };
    } finally {
      cdp.close();
    }
  } finally {
    for (const child of children.reverse()) await stopChild(child);
    await rm(profile, { recursive: true, force: true, maxRetries: 3, retryDelay: 100 });
  }
}

async function navigateToRoute(cdp, port, route, timeoutMs) {
  await cdp.send("Page.navigate", { url: `http://127.0.0.1:${port}/` });
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-home-state]') !== null`, timeoutMs);
  const action = {
    Search: "navigate-search",
    Document: "open-recent-document",
    Graph: "navigate-graph",
    Canvas: "navigate-canvas",
    Assets: "navigate-assets",
    Backup: "navigate-backup",
  }[route];
  if (action) {
    const clicked = await evaluate(cdp, `(() => { const control = document.querySelector('[data-action="${action}"]'); if (!control) return 'missing'; if (control.disabled) return 'disabled'; control.click(); return 'clicked'; })()`);
    if (clicked !== "clicked") throw new Error(`route action unavailable: ${route}:${clicked}`);
  }
  const ready = {
    Home: "[data-cabinet-home-state]",
    Search: "[data-cabinet-navigator-state]",
    Document: "[data-cabinet-authoring-state]",
    Graph: '[data-exploration-surface="graph"]',
    Canvas: '[data-exploration-surface="canvas"]',
    Assets: '[data-exploration-surface="assets"]',
    Backup: "[data-backup-state]",
  }[route];
  try {
    await waitForExpression(cdp, `document.querySelector('${ready}') !== null`, timeoutMs);
  } catch (error) {
    const diagnostic = await evaluate(cdp, `({ shellRoute: document.querySelector('[data-shell-route]')?.getAttribute('data-shell-route') ?? 'missing', bodyText: (document.body?.innerText ?? '').slice(0, 240), runtimeError: globalThis.__PHASE013_RUNTIME_ERROR__ ?? 'none' })`);
    throw new Error(`route readiness failed: ${route}:${JSON.stringify(diagnostic)}`, { cause: error });
  }
}

function captureExpression(route) {
  return `(() => {
    const safeRect = (selector) => {
      const element = document.querySelector(selector);
      if (!element) return { x: 0, y: 0, width: 0, height: 0 };
      const value = element.getBoundingClientRect();
      return { x: Math.round(value.x), y: Math.round(value.y), width: Math.round(value.width), height: Math.round(value.height) };
    };
    const controls = Array.from(document.querySelectorAll('button, a[href], input, textarea, select, [contenteditable="true"], [role="menuitem"]'))
      .filter((element) => {
        const style = getComputedStyle(element); const rect = element.getBoundingClientRect();
        if (style.display === 'none' || style.visibility === 'hidden' || rect.width <= 0 || rect.height <= 0) return false;
        if (element.closest('.canvas-world')) {
          const stage = element.closest('.canvas-stage')?.getBoundingClientRect();
          if (stage && (rect.right <= stage.left || rect.left >= stage.right || rect.bottom <= stage.top || rect.top >= stage.bottom)) return false;
        }
        return true;
      });
    const seen = new Map();
    const actions = controls.map((element, index) => {
      const explicit = element.getAttribute('data-action') ?? '';
      const key = explicit || 'missing'; const sequence = (seen.get(key) ?? 0) + 1; seen.set(key, sequence);
      const actionId = explicit || 'missing-action-' + String(index + 1);
      const accessible = element.getAttribute('aria-label') || element.getAttribute('title') || element.textContent || element.getAttribute('placeholder') || '';
      const rect = element.getBoundingClientRect();
      return { route: '${route}', actionId, kind: element.tagName.toLowerCase(), controlHint: String(element.className || element.getAttribute('role') || '').slice(0, 80), disabled: Boolean(element.disabled || element.getAttribute('aria-disabled') === 'true'), hasAccessibleName: accessible.trim().length > 0, identityMissing: !explicit, horizontallyClipped: rect.left < -1 || rect.right > window.innerWidth + 1, rectLeft: Math.round(rect.left), rectRight: Math.round(rect.right), viewportWidth: window.innerWidth };
    });
    const clippedActionCount = controls.filter((element) => {
      const rect = element.getBoundingClientRect();
      return rect.left < -1 || rect.right > window.innerWidth + 1;
    }).length;
    const horizontalOverflow = document.documentElement.scrollWidth > window.innerWidth + 1;
    const canvasToolbar = document.querySelector('.canvas-toolbar');
    const responsiveDebug = canvasToolbar ? { media600: matchMedia('(max-width: 600px)').matches, toolbarWidth: Math.round(canvasToolbar.getBoundingClientRect().width), toolbarScrollWidth: canvasToolbar.scrollWidth, toolbarFlexWrap: getComputedStyle(canvasToolbar).flexWrap } : undefined;
    const stateHost = document.querySelector('[data-cabinet-home-state], [data-cabinet-navigator-state], [data-cabinet-authoring-state], [data-graph-state], [data-canvas-state], [data-asset-state], [data-backup-state]');
    return { state: stateHost ? Array.from(stateHost.attributes).find((item) => item.name.endsWith('-state'))?.value ?? 'Ready' : 'Ready', actions, geometry: { sidebar: safeRect('.desktop-sidebar'), topbar: safeRect('.desktop-topbar'), main: safeRect('main, .backup-recovery-surface'), horizontalOverflow, clippedActionCount, responsiveDebug } };
  })()`;
}

function injectedTauriSource(captureProfile) {
  return `(() => {
    globalThis.addEventListener('error', (event) => { globalThis.__PHASE013_RUNTIME_ERROR__ = String(event.error?.stack ?? event.message ?? 'error'); });
    globalThis.addEventListener('unhandledrejection', (event) => { globalThis.__PHASE013_RUNTIME_ERROR__ = String(event.reason?.stack ?? event.reason ?? 'rejection'); });
    const assetId = '${"a".repeat(64)}';
    const itemCount = ${captureProfile.fixtureItemCount};
    const longContent = ${captureProfile.longContentFixture};
    const title = longContent ? '아키텍처와 제품 운영 정책을 함께 설명하는 매우 긴 한국어 문서 제목 '.repeat(4).trim() : 'Architecture Notes';
    const fileName = longContent ? '개인 지식 관리 제품의 장기 보존용 아키텍처 설계 검토 자료 '.repeat(4).trim() + '.pdf' : 'architecture.pdf';
    const documents = Array.from({ length: itemCount }, (_, index) => ({ documentId: 'doc-' + String(index + 1).padStart(3, '0'), title: title + (itemCount > 1 ? ' ' + String(index + 1) : ''), path: 'notes/document-' + String(index + 1) + '.md' }));
    const document = documents[0];
    const home = { ok: true, retryable: false, data: { workspaceId: 'workspace-1', state: 'Ready', healthStatus: 'Healthy', backupStatus: 'Fresh', recentDocuments: documents, favorites: [], tags: [{ label: 'local', documentCount: itemCount }], recentChanges: [], unfinishedItems: [] } };
    const canvas = { canvasId: 'default-canvas', title, revision: 7, lifecycle: 'updated', viewport: { centerX: 600, centerY: 360, zoomPercent: 100 }, nodes: [{ nodeId: 'document-node', targetKind: 'document', targetId: document.documentId, displayLabel: title, targetStatus: 'available', x: 120, y: 120, width: 320, height: 180 }], edges: [] };
    const asset = { assetId, label: fileName, fileName, mediaType: 'application/pdf', byteSize: 4096, status: 'available' };
    globalThis.__TAURI__ = { core: { invoke: async (command, args) => {
      if (command === 'get_desktop_workspace_home') return home;
      if (command === 'get_desktop_document_navigator') return { ok: true, retryable: false, data: { workspaceId: 'workspace-1', view: args?.request?.view ?? 'Tree', state: 'Ready', items: documents } };
      if (command === 'get_desktop_global_knowledge_graph') return { ok: true, data: { status: 'clean', nodes: [{ id: 'doc-001', kind: 'document' }], edges: [], candidateCount: 1, nextCursor: null } };
      if (command === 'execute_desktop_canvas') return { ok: true, retryable: false, recoveryRequired: false, data: canvas };
      if (command === 'get_desktop_workspace_assets') return { ok: true, data: { workspaceId: 'workspace-1', assets: [asset] } };
      if (command === 'get_desktop_document_assets') return { ok: true, data: { queryName: 'list-document-assets', workspaceId: 'workspace-1', documentId: 'doc-001', assets: [asset] } };
      if (command === 'get_desktop_asset_detail') return { ok: true, data: { ...asset, version: 1, previewCapability: 'pdf', extractionStatus: 'ready', referenceCount: 1, linkedDocumentIds: ['doc-001'] } };
      if (command === 'execute_desktop_document_authoring' && args?.request?.kind === 'get_current') return { ok: true, data: { kind: 'current', documentId: document.documentId, title, path: document.path, body: '# ' + title, currentVersionId: 'version-001' } };
      if (command === 'recover_desktop_backup_startup') return { ok: true, retryable: false, state: 'Completed', recovery: { cleanedStagingCount: 0, rolledBackOperationIds: [], cleanupRequiredOperationIds: [] } };
      return { ok: false, errorCode: 'COMMAND_BRIDGE_FAILED', retryable: false, repairRequired: false, recoveryRequired: false };
    } } };
  })();`;
}

async function readManifestEntries(root) {
  const coreModule = await import(pathToFileURL(join(root, "apps/desktop/src/core_ui_action_manifest.ts")).href);
  const explorationModule = await import(pathToFileURL(join(root, "apps/desktop/src/exploration_ui_action_manifest.ts")).href);
  return [...coreModule.CORE_UI_ACTION_MANIFEST, ...explorationModule.EXPLORATION_UI_ACTION_CONTRACTS];
}

async function fingerprintSources(root) {
  const files = [
    "apps/desktop/src/desktop_entry.ts",
    "apps/desktop/src/react_workspace_shell.ts",
    "apps/desktop/src/react_workspace_home.ts",
    "apps/desktop/src/react_document_navigator.ts",
    "apps/desktop/src/react_document_authoring_workbench.ts",
    "apps/desktop/src/react_exploration_surfaces.ts",
    "apps/desktop/src/react_backup_recovery.ts",
    "apps/desktop/src/core_ui_action_manifest.ts",
    "apps/desktop/src/exploration_ui_action_manifest.ts",
    "apps/desktop/src/ui_action_contract.ts",
    "apps/desktop/src/modal_keyboard_policy.ts",
    "apps/desktop/src/modal_focus_restoration.ts",
    "apps/desktop/src/route_main_focus.ts",
    "apps/desktop/public/styles.css",
  ];
  const hash = createHash("sha256");
  for (const file of files) hash.update(file).update("\0").update(await readFile(join(root, file))).update("\0");
  return hash.digest("hex");
}

async function evaluate(cdp, expression) { const response = await cdp.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true }); if (response.exceptionDetails) throw new Error("browser evaluation failed"); return response.result?.value; }
async function waitForExpression(cdp, expression, timeoutMs) { const started = Date.now(); while (Date.now() - started < timeoutMs) { if (await evaluate(cdp, `Boolean(${expression})`)) return; await delay(100); } throw new Error(`browser condition timeout: ${expression}`); }
async function freePort() { return new Promise((resolve, reject) => { const server = createServer(); server.once("error", reject); server.listen(0, "127.0.0.1", () => { const address = server.address(); server.close(() => typeof address === "object" && address?.port ? resolve(address.port) : reject(new Error("port unavailable"))); }); }); }
async function waitForHttp(url, timeoutMs) { const started = Date.now(); while (Date.now() - started < timeoutMs) { try { const response = await fetch(url); if (response.ok) return response; } catch {} await delay(100); } throw new Error(`http timeout: ${url}`); }
async function pageTarget(port, timeoutMs) { const response = await waitForHttp(`http://127.0.0.1:${port}/json/list`, timeoutMs); const targets = await response.json(); const target = targets.find((item) => item.type === "page" && item.webSocketDebuggerUrl); if (!target) throw new Error("CDP page target unavailable"); return target; }
async function stopChild(child) { if (child.exitCode !== null || child.signalCode !== null) return; child.kill("SIGTERM"); await Promise.race([new Promise((resolve) => child.once("exit", resolve)), delay(2000)]); if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL"); }
function delay(ms) { return new Promise((resolve) => setTimeout(resolve, ms)); }

class CdpClient {
  constructor(socket) { this.socket = socket; this.nextId = 1; this.pending = new Map(); socket.addEventListener("message", (event) => this.onMessage(event.data)); }
  static async connect(url) { const socket = new WebSocket(url); await new Promise((resolve, reject) => { socket.addEventListener("open", resolve, { once: true }); socket.addEventListener("error", () => reject(new Error("CDP socket unavailable")), { once: true }); }); return new CdpClient(socket); }
  send(method, params = {}) { const id = this.nextId++; const promise = new Promise((resolve, reject) => this.pending.set(id, { resolve, reject })); this.socket.send(JSON.stringify({ id, method, params })); return promise; }
  onMessage(raw) { const message = JSON.parse(raw); const pending = this.pending.get(message.id); if (!pending) return; this.pending.delete(message.id); message.error ? pending.reject(new Error(message.error.message)) : pending.resolve(message.result ?? {}); }
  close() { this.socket.close(); }
}

export function resolveChromePath() {
  const candidates = [process.env.SPONZEY_CABINET_CHROME_BIN, "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/Applications/Chromium.app/Contents/MacOS/Chromium", "/usr/bin/google-chrome", "/usr/bin/chromium"].filter(Boolean);
  const found = candidates.find((candidate) => existsSync(candidate)); if (!found) throw new Error("Chrome not found"); return found;
}

async function main() {
  const root = process.cwd();
  const report = await runPhase013ActionGeometryBaseline({ root, chromePath: resolveChromePath() });
  const releaseDir = join(root, ".tasks", "release");
  await mkdir(releaseDir, { recursive: true });
  await writeFile(join(releaseDir, "ui-action-geometry-baseline-phase013.json"), `${JSON.stringify(report, null, 2)}\n`);
  console.log("phase013_action_geometry_baseline=recorded");
  console.log(`source_fingerprint=${report.sourceFingerprint}`);
  console.log(`route_run_count=${report.runs.length}`);
  console.log(`action_count=${report.actions.length}`);
  console.log(`action_gap_count=${report.gaps.length}`);
}

if (process.argv[1]?.endsWith("run_phase013_action_geometry_baseline.mjs")) await main();
