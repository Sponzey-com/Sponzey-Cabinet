import { execFile, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

import {
  explorationVisualViewports,
  transitionExplorationVisualState,
  validateExplorationVisualReport,
} from "./phase012_exploration_visual.mjs";

const execFileAsync = promisify(execFile);
const surfaces = Object.freeze(["graph", "canvas", "assets"]);

export async function runPhase012ExplorationVisual({ root, chromePath, timeoutMs = 20_000 }) {
  let state = "Pending";
  const sourceFingerprint = await fingerprintSources(root);
  const webPort = await freePort();
  const debugPort = await freePort();
  const profile = await mkdtemp(join(tmpdir(), "sponzey-phase012-exploration-"));
  const screenshotsDir = join(root, ".tasks", "release", "screenshots", "exploration-phase012");
  const children = [];
  await mkdir(screenshotsDir, { recursive: true });
  try {
    state = transitionExplorationVisualState(state, "Serve").state;
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
      await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: injectedTauriSource() });
      state = transitionExplorationVisualState(state, "Browse").state;
      const runs = [];
      for (const viewport of explorationVisualViewports()) {
        await cdp.send("Emulation.setDeviceMetricsOverride", { ...viewport, deviceScaleFactor: 1, mobile: false });
        await navigateHome(cdp, webPort, timeoutMs);
        for (const surface of surfaces) {
          await openSurface(cdp, surface, timeoutMs);
          await focusSurfaceControl(cdp, surface);
          const metrics = await evaluate(cdp, visualMetricsExpression(surface));
          state = transitionExplorationVisualState(state, "Capture").state;
          const capture = await cdp.send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
          const fileName = `${surface}-${viewport.width}x${viewport.height}.png`;
          const screenshotPath = join(screenshotsDir, fileName);
          await writeFile(screenshotPath, Buffer.from(capture.data, "base64"));
          runs.push({ ...viewport, surface, ...metrics, nonBlankPixelCount: await nonBlankPixels(screenshotPath), screenshot: fileName });
          state = "Browsing";
        }
      }
      await cdp.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
      await navigateHome(cdp, webPort, timeoutMs);
      const interactions = await interactionEvidence(cdp, timeoutMs);
      const report = {
        marker: "phase012_exploration_visual=passed",
        sourceFingerprint,
        diagnostics: "sanitized",
        browserSurface: "local_headless_chrome_cdp",
        interactions,
        runs,
      };
      const validation = validateExplorationVisualReport(report, sourceFingerprint);
      if (!validation.passed) throw new Error(`exploration visual validation failed: ${validation.findingIds.join(",")}`);
      state = transitionExplorationVisualState("Capturing", "Pass").state;
      return { ...report, state };
    } finally {
      cdp.close();
    }
  } finally {
    for (const child of children.reverse()) await stopChild(child);
    await rm(profile, { recursive: true, force: true, maxRetries: 3, retryDelay: 100 });
  }
}

async function interactionEvidence(cdp, timeoutMs) {
  await openSurface(cdp, "graph", timeoutMs);
  await click(cdp, '[data-action="select-graph-node"][data-graph-node-id="doc-001"]');
  await click(cdp, '[data-action="open-graph-document"]');
  const graphOpenedDocumentId = await waitForOpenedDocument(cdp, timeoutMs);

  await click(cdp, '[data-action="navigate-canvas"]');
  await waitSurface(cdp, "canvas", timeoutMs);
  await evaluate(cdp, `document.querySelector('[data-action="select-canvas-node"][aria-label="Canvas card Architecture Notes"]')?.focus(); true`);
  await dispatchKey(cdp, "Enter", 13);
  await waitForExpression(cdp, `document.querySelector('[data-action="select-canvas-node"][aria-pressed="true"]') !== null`, timeoutMs);
  const canvasKeyboardSelection = true;
  await click(cdp, '[data-action="open-canvas-document"]');
  const canvasOpenedDocumentId = await waitForOpenedDocument(cdp, timeoutMs);

  await click(cdp, '[data-action="navigate-assets"]');
  await waitSurface(cdp, "assets", timeoutMs);
  await click(cdp, '[data-action="select-asset"]');
  await waitForExpression(cdp, `document.querySelector('[data-action="open-linked-document"]') !== null`, timeoutMs);
  await click(cdp, '[data-action="open-linked-document"]');
  const assetOpenedDocumentId = await waitForOpenedDocument(cdp, timeoutMs);
  return { graphOpenedDocumentId, canvasOpenedDocumentId, assetOpenedDocumentId, canvasKeyboardSelection };
}

async function waitForOpenedDocument(cdp, timeoutMs) {
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state]') !== null`, timeoutMs);
  return evaluate(cdp, `globalThis.__CABINET_PHASE012_EVENTS__.filter((event) => event.command === 'execute_desktop_document_authoring' && event.args?.request?.kind === 'get_current').at(-1)?.args?.request?.documentId`);
}

async function navigateHome(cdp, port, timeoutMs) {
  await cdp.send("Page.navigate", { url: `http://127.0.0.1:${port}/` });
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-home-state="Ready"]') !== null`, timeoutMs);
}

async function openSurface(cdp, surface, timeoutMs) {
  const current = await evaluate(cdp, `document.querySelector('[data-exploration-surface]')?.getAttribute('data-exploration-surface')`);
  if (current !== surface) await click(cdp, `[data-action="navigate-${surface}"]`);
  await waitSurface(cdp, surface, timeoutMs);
  if (surface === "assets") {
    await click(cdp, '[data-action="select-asset"]');
    await waitForExpression(cdp, `document.querySelector('[data-action="open-linked-document"]') !== null`, timeoutMs);
  }
}

async function waitSurface(cdp, surface, timeoutMs) {
  const ready = surface === "graph" ? ".graph-node" : surface === "canvas" ? "[data-canvas-revision]" : '[data-action="select-asset"]';
  await waitForExpression(cdp, `document.querySelector('[data-exploration-surface="${surface}"]') !== null && document.querySelector('${ready}') !== null`, timeoutMs);
}

async function focusSurfaceControl(cdp, surface) {
  const selector = surface === "graph" ? '[data-action="select-graph-node"]' : surface === "canvas" ? '[data-action="select-canvas-node"]' : '[data-action="select-asset"]';
  await evaluate(cdp, `document.querySelector('${selector}')?.focus(); true`);
  await dispatchKey(cdp, "Tab", 9);
}

function visualMetricsExpression(surface) {
  return `(() => {
    const rect = (selector) => document.querySelector(selector)?.getBoundingClientRect();
    const intersects = (a, b) => Boolean(a && b && a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top);
    const top = rect('.desktop-topbar'); const side = rect('.desktop-sidebar'); const main = rect('.desktop-main');
    const focused = document.activeElement; const style = focused instanceof Element ? getComputedStyle(focused) : null;
    const clipped = Array.from(document.querySelectorAll('button:not(:disabled)')).filter((button) => button.clientWidth > 0 && button.scrollWidth > button.clientWidth + 1).length;
    const forbidden = ['/Users/', 'C:\\\\Users\\\\', 'raw document body', 'provider_api_key', 'sessionToken'];
    return {
      readyState: document.querySelector('[data-exploration-surface="${surface}"]') !== null,
      overlapCount: Number(intersects(top, main)) + Number(intersects(side, main)),
      horizontalOverflow: document.documentElement.scrollWidth > innerWidth + 1,
      clippedControlCount: clipped,
      focusVisible: Boolean(style && style.outlineStyle !== 'none' && parseFloat(style.outlineWidth || '0') > 0),
      navLandmark: Boolean(document.querySelector('nav[aria-label="Workspace"]')),
      mainLandmark: Boolean(document.querySelector('main h1')),
      sensitiveDataExcluded: forbidden.every((token) => !document.body.textContent.includes(token)),
    };
  })()`;
}

function injectedTauriSource() {
  return `(() => {
    const assetId = '${"a".repeat(64)}';
    const events = []; globalThis.__CABINET_PHASE012_EVENTS__ = events;
    const home = { ok: true, retryable: false, data: { workspaceId: 'workspace-1', state: 'Ready', healthStatus: 'Healthy', backupStatus: 'Fresh', recentDocuments: [{ documentId: 'doc-001', title: 'Architecture Notes', path: 'notes/architecture.md' }], favorites: [], tags: [{ label: 'local', documentCount: 1 }], recentChanges: [], unfinishedItems: [] } };
    const canvas = { canvasId: 'default-canvas', title: 'Cabinet Product Map', revision: 7, lifecycle: 'updated', viewport: { centerX: 600, centerY: 360, zoomPercent: 100 }, nodes: [{ nodeId: 'document-node', targetKind: 'document', targetId: 'doc-001', displayLabel: 'Architecture Notes', targetStatus: 'available', x: 120, y: 120, width: 320, height: 180 }, { nodeId: 'memo-node', targetKind: 'text', targetId: 'Local memo', displayLabel: 'Local memo', targetStatus: 'available', x: 520, y: 360, width: 320, height: 180 }], edges: [{ edgeId: 'edge-1', sourceNodeId: 'document-node', targetNodeId: 'memo-node' }] };
    const asset = { assetId, label: 'Architecture PDF', fileName: 'architecture.pdf', mediaType: 'application/pdf', byteSize: 4096, status: 'available' };
    globalThis.__TAURI__ = { core: { invoke: async (command, args) => {
      events.push({ command, args });
      if (command === 'get_desktop_workspace_home') return home;
      if (command === 'get_desktop_global_knowledge_graph') return { ok: true, data: { status: 'clean', nodes: [{ id: 'doc-001', kind: 'document' }, { id: 'doc-002', kind: 'document' }], edges: [{ id: 'edge-1', sourceId: 'doc-001', targetId: 'doc-002', kind: 'document_link' }], candidateCount: 2, nextCursor: null } };
      if (command === 'execute_desktop_canvas') return { ok: true, retryable: false, recoveryRequired: false, data: canvas };
      if (command === 'get_desktop_workspace_assets') return { ok: true, data: { workspaceId: 'workspace-1', assets: [asset] } };
      if (command === 'get_desktop_document_assets') return { ok: true, data: { queryName: 'list-document-assets', workspaceId: 'workspace-1', documentId: args?.request?.payload?.document_id ?? 'doc-001', assets: [asset] } };
      if (command === 'get_desktop_asset_detail') return { ok: true, data: { ...asset, version: 1, previewCapability: 'pdf', extractionStatus: 'ready', referenceCount: 1, linkedDocumentIds: ['doc-001'] } };
      if (command === 'execute_desktop_document_authoring' && args?.request?.kind === 'get_current') return { ok: true, data: { kind: 'current', documentId: args.request.documentId, title: 'Architecture Notes', path: 'notes/architecture.md', body: '# Architecture Notes', currentVersionId: 'version-001' } };
      return { ok: false, errorCode: 'COMMAND_BRIDGE_FAILED', retryable: false, repairRequired: false, recoveryRequired: false };
    } } };
  })();`;
}

async function fingerprintSources(root) {
  const files = ["apps/desktop/src/desktop_entry.ts", "apps/desktop/src/react_exploration_surfaces.ts", "apps/desktop/public/styles.css"];
  const hash = createHash("sha256");
  for (const file of files) hash.update(file).update("\0").update(await readFile(join(root, file))).update("\0");
  return hash.digest("hex");
}

async function nonBlankPixels(path) {
  const { stdout } = await execFileAsync("magick", [path, "-colorspace", "gray", "-threshold", "98%", "-format", "%[fx:(1-mean)*w*h]", "info:"]);
  return Math.round(Number.parseFloat(stdout.trim()));
}

async function click(cdp, selector) { await evaluate(cdp, `document.querySelector('${selector}')?.click(); true`); }
async function dispatchKey(cdp, key, code) {
  await cdp.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key, code: key, windowsVirtualKeyCode: code, nativeVirtualKeyCode: code });
  await cdp.send("Input.dispatchKeyEvent", { type: "keyUp", key, code: key, windowsVirtualKeyCode: code, nativeVirtualKeyCode: code });
}
async function evaluate(cdp, expression) {
  const response = await cdp.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw new Error("browser evaluation failed");
  return response.result?.value;
}
async function waitForExpression(cdp, expression, timeoutMs) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) { if (await evaluate(cdp, `Boolean(${expression})`)) return; await delay(100); }
  throw new Error(`browser condition timeout: ${expression}`);
}
async function freePort() {
  return new Promise((resolve, reject) => { const server = createServer(); server.once("error", reject); server.listen(0, "127.0.0.1", () => { const address = server.address(); server.close(() => typeof address === "object" && address?.port ? resolve(address.port) : reject(new Error("port unavailable"))); }); });
}
async function waitForHttp(url, timeoutMs) {
  const started = Date.now(); while (Date.now() - started < timeoutMs) { try { const response = await fetch(url); if (response.ok) return response; } catch {} await delay(100); } throw new Error(`http timeout: ${url}`);
}
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

function resolveChromePath() {
  const candidates = [process.env.SPONZEY_CABINET_CHROME_BIN, "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/Applications/Chromium.app/Contents/MacOS/Chromium", "/usr/bin/google-chrome", "/usr/bin/chromium"].filter(Boolean);
  const found = candidates.find((candidate) => existsSync(candidate)); if (!found) throw new Error("Chrome not found"); return found;
}

async function main() {
  const root = process.cwd();
  const report = await runPhase012ExplorationVisual({ root, chromePath: resolveChromePath() });
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(join(root, ".tasks", "release", "exploration-visual-phase012.json"), `${JSON.stringify(report, null, 2)}\n`);
  console.log("phase012_exploration_visual=passed");
}

if (process.argv[1]?.endsWith("run_phase012_exploration_visual.mjs")) await main();
