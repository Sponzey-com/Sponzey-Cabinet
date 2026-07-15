import { spawn, execFile } from "node:child_process";
import { createServer } from "node:net";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

import {
  authoringBrowserViewports,
  transitionPhase011AuthoringBrowserState,
  validatePhase011AuthoringBrowserReport,
} from "./phase011_authoring_browser.mjs";

const execFileAsync = promisify(execFile);

export async function runPhase011AuthoringBrowserEvidence({
  root,
  chromePath,
  sourceFingerprint,
  timeoutMs = 20000,
}) {
  let state = "Pending";
  const webPort = await findFreePort();
  const debugPort = await findFreePort();
  const profile = await mkdtemp(join(tmpdir(), "sponzey-phase011-authoring-chrome-"));
  const screenshotsDir = join(root, ".tasks", "release", "screenshots", "authoring");
  const children = [];
  await mkdir(screenshotsDir, { recursive: true });

  try {
    state = transitionPhase011AuthoringBrowserState(state, "Serve").state;
    const server = spawn(process.execPath, ["scripts/run_web_app.mjs", String(webPort)], {
      cwd: root,
      env: {
        ...process.env,
        SPONZEY_CABINET_RUNNER_ANNOUNCED: "1",
        SPONZEY_CABINET_WEB_PUBLIC_DIR: "apps/desktop/dist",
      },
      stdio: "ignore",
    });
    children.push(server);
    await waitForHttp(`http://127.0.0.1:${webPort}/`, timeoutMs);

    state = transitionPhase011AuthoringBrowserState(state, "Launch").state;
    const chrome = spawn(chromePath, [
      "--headless=new",
      "--disable-gpu",
      "--disable-background-networking",
      "--disable-default-apps",
      "--disable-extensions",
      "--no-default-browser-check",
      "--no-first-run",
      `--remote-debugging-port=${debugPort}`,
      `--user-data-dir=${profile}`,
      "about:blank",
    ], { cwd: root, stdio: "ignore" });
    children.push(chrome);
    await waitForHttp(`http://127.0.0.1:${debugPort}/json/version`, timeoutMs);
    const target = await pageTarget(debugPort, timeoutMs);
    const cdp = await CdpClient.connect(target.webSocketDebuggerUrl);

    try {
      await cdp.send("Page.enable");
      await cdp.send("Runtime.enable");
      state = transitionPhase011AuthoringBrowserState(state, "Inject").state;
      await cdp.send("Page.addScriptToEvaluateOnNewDocument", {
        source: injectedAuthoringTauriSource(),
      });

      state = transitionPhase011AuthoringBrowserState(state, "Navigate").state;
      const runs = [];
      for (const viewport of authoringBrowserViewports()) {
        await setViewport(cdp, viewport);
        await navigateToApp(cdp, webPort, timeoutMs);
        await openAuthoringDocument(cdp, timeoutMs);
        await cdp.send("Page.bringToFront");
        await evaluate(cdp, `document.querySelector('.cm-content')?.focus(); true`);
        await dispatchTab(cdp);
        const metrics = await evaluate(cdp, visualMetricsExpression());
        const capture = await cdp.send("Page.captureScreenshot", {
          format: "png",
          captureBeyondViewport: false,
        });
        const fileName = `authoring-${viewport.width}x${viewport.height}.png`;
        const screenshotPath = join(screenshotsDir, fileName);
        await writeFile(screenshotPath, Buffer.from(capture.data, "base64"));
        runs.push({
          ...viewport,
          ...metrics,
          nonBlankPixelCount: await estimateNonBlankPixels(screenshotPath),
          screenshot: fileName,
        });
      }

      await setViewport(cdp, { width: 1280, height: 800 });
      await navigateToApp(cdp, webPort, timeoutMs);
      state = transitionPhase011AuthoringBrowserState(state, "Interact").state;
      const interactions = await runAuthoringInteractionFlow(cdp, timeoutMs);
      state = transitionPhase011AuthoringBrowserState(state, "Capture").state;

      const report = {
        marker: "phase011_authoring_browser=passed",
        sourceFingerprint,
        browserSurface: "local_chrome_cdp",
        diagnostics: "sanitized",
        interactions,
        runs,
      };
      const validation = validatePhase011AuthoringBrowserReport(report, sourceFingerprint);
      if (!validation.passed) {
        throw new Error(`authoring browser validation failed: ${validation.findingIds.join(",")}`);
      }
      state = transitionPhase011AuthoringBrowserState(state, "Pass").state;
      return { ...report, state };
    } finally {
      cdp.close();
    }
  } finally {
    for (const child of children.reverse()) await stopChild(child);
    await rm(profile, { recursive: true, force: true, maxRetries: 3, retryDelay: 100 });
  }
}

async function runAuthoringInteractionFlow(cdp, timeoutMs) {
  const beforeCreate = await eventCount(cdp, "create");
  await clickSelector(cdp, '[data-action="new-document"]');
  await waitForExpression(
    cdp,
    `document.querySelector('[data-cabinet-authoring-state="Clean"], [data-cabinet-authoring-state="Saved"]') !== null`,
    timeoutMs,
  );
  await waitForExpression(cdp, `document.querySelector('.cm-editor .cm-content') !== null`, timeoutMs);
  await waitForExpression(cdp, `document.body.textContent.includes('Untitled Document')`, timeoutMs);
  const afterCreate = await eventCount(cdp, "create");
  const opened = await evaluate(cdp, `(() => ({
    documentOpened: Boolean(document.querySelector('[data-cabinet-authoring-state]')),
    codeMirrorMounted: Boolean(document.querySelector('.cm-editor .cm-content')),
    previewTableRendered: Boolean(document.querySelector('.markdown-preview'))
  }))()`);
  const beforeManual = await eventCount(cdp, "update");
  await editCodeMirror(cdp, "\n\nManual browser edit.\n\n| Created | State |\n| --- | --- |\n| Local | Ready |\n");
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state="Dirty"]') !== null`, timeoutMs);
  await dispatchModSave(cdp);
  await waitForEventCount(cdp, "update", beforeManual + 1, timeoutMs);
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state="Saved"]') !== null`, timeoutMs);
  const afterManual = await eventCount(cdp, "update");

  await clickSelector(cdp, '[data-editor-mode="source"]');
  await waitForExpression(cdp, `document.querySelector('[data-editor-mode="source"][aria-pressed="true"]') !== null`, timeoutMs);
  const sourceMode = true;
  await clickSelector(cdp, '[data-editor-mode="preview"]');
  await waitForExpression(cdp, `document.querySelector('[data-editor-mode="preview"][aria-pressed="true"]') !== null && document.querySelector('.markdown-preview table tbody td') !== null`, timeoutMs);
  const previewMode = true;
  await clickSelector(cdp, '[data-editor-mode="split"]');
  await waitForExpression(cdp, `document.querySelector('[data-editor-mode="split"][aria-pressed="true"]') !== null && document.querySelector('.cm-editor .cm-content') !== null`, timeoutMs);
  const splitMode = true;

  const beforeAutosave = await eventCount(cdp, "update");
  await editCodeMirror(cdp, "\nAutosave browser edit.");
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state="Dirty"]') !== null`, timeoutMs);
  await waitForEventCount(cdp, "update", beforeAutosave + 1, timeoutMs + 1500);
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state="Saved"]') !== null`, timeoutMs);
  const afterAutosave = await eventCount(cdp, "update");

  const beforeRestore = await eventCount(cdp, "restore");
  await clickSelector(cdp, '[data-action="load-history"]');
  await waitForExpression(cdp, `document.querySelector('[data-history-restore-state="Ready"] [data-action="preview-restore"]') !== null`, timeoutMs);
  await clickSelector(cdp, '[data-action="preview-restore"]');
  await waitForExpression(cdp, `document.querySelector('[data-history-restore-state="PreviewReady"] [data-action="apply-restore"]') !== null`, timeoutMs);
  await clickSelector(cdp, '[data-action="apply-restore"]');
  await waitForEventCount(cdp, "restore", beforeRestore + 1, timeoutMs);
  await waitForExpression(cdp, `document.querySelector('[data-history-restore-state="Applied"]') !== null`, timeoutMs);
  const afterRestore = await eventCount(cdp, "restore");

  await editCodeMirror(cdp, "\nClose cancel browser edit.");
  await clickText(cdp, ".nav-item", "Home");
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state="CloseBlocked"]') !== null`, timeoutMs);
  await clickText(cdp, ".state-banner button", "Cancel");
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state="Dirty"]') !== null`, timeoutMs);

  const beforeRetrySave = await eventCount(cdp, "update");
  await clickText(cdp, ".nav-item", "Home");
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state="CloseBlocked"]') !== null`, timeoutMs);
  await clickText(cdp, ".state-banner button", "Retry save");
  await waitForEventCount(cdp, "update", beforeRetrySave + 1, timeoutMs);
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state="Saved"]') !== null`, timeoutMs);

  await editCodeMirror(cdp, "\nClose discard browser edit.");
  await clickText(cdp, ".nav-item", "Home");
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-authoring-state="CloseBlocked"]') !== null`, timeoutMs);
  await clickText(cdp, ".state-banner button", "Discard");
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-home-state="Ready"]') !== null`, timeoutMs);

  const sanitized = await evaluate(cdp, sanitizedInteractionExpression());
  return {
    ...sanitized,
    ...opened,
    sourceMode,
    splitMode,
    previewMode,
    previewTableRendered: true,
    createDocumentCount: afterCreate - beforeCreate,
    createdDocumentOpened: afterCreate > beforeCreate && opened.documentOpened,
    keyboardSave: afterManual > beforeManual,
    manualSaveCount: afterManual - beforeManual,
    autosaveCount: afterAutosave - beforeAutosave,
    historyLoaded: true,
    restorePreviewReady: true,
    restoreApplyCount: afterRestore - beforeRestore,
  };
}

function injectedAuthoringTauriSource() {
  return `(() => {
    const body = [
      '# Architecture Notes',
      '',
      'This local document is opened through the desktop authoring bridge.',
      '',
      '| Item | Owner | State |',
      '| :--- | :---: | ---: |',
      '| Editor | Local | Ready |',
      '| Preview | Browser | Verified |',
      '',
      '- [x] Checklist item',
      '',
      '> Local note',
      ''
    ].join('\\n');
    const documents = new Map();
    documents.set('doc-001', {
      documentId: 'doc-001',
      title: 'Architecture Notes',
      path: 'notes/architecture.md',
      body,
      currentVersionId: 'version-001'
    });
    const versions = new Map();
    versions.set('doc-001', [{
      versionId: 'version-001',
      body,
      summary: 'Created',
      author: 'local-user'
    }]);
    const state = {
      workspaceId: 'workspace-1',
      documents,
      versions,
      saveCount: 0,
      events: []
    };
    globalThis.__CABINET_PHASE011_AUTHORING_STATE__ = state;
    globalThis.__CABINET_PHASE011_AUTHORING_EVENTS__ = state.events;
    globalThis.__TAURI__ = { core: { invoke: async (command, args) => {
      if (command === 'get_desktop_workspace_home') {
        return {
          ok: true,
          retryable: false,
          data: {
            workspaceId: state.workspaceId,
            state: 'Ready',
            recentDocuments: Array.from(state.documents.values()).map((document) => ({
              documentId: document.documentId,
              title: document.title,
              path: document.path
            })),
            favorites: [],
            tags: [{ label: 'architecture', documentCount: 1 }],
            recentChanges: [{ documentId: 'doc-001', summary: 'Opened local document' }],
            unfinishedItems: [],
            backupStatus: 'Fresh',
            healthStatus: 'Healthy'
          }
        };
      }
      if (command === 'get_desktop_document_navigator') {
        const request = args?.request || {};
        const items = Array.from(state.documents.values()).map((document) => ({
          documentId: document.documentId,
          title: document.title,
          path: document.path,
          collections: ['local'],
          tags: ['architecture'],
          favorite: false
        }));
        return {
          ok: true,
          retryable: false,
          data: {
            workspaceId: state.workspaceId,
            view: request.view || 'Tree',
            state: 'Ready',
            items,
            nextCursor: null
          }
        };
      }
      if (command === 'execute_desktop_document_authoring') {
        const request = args?.request || {};
        if (request.kind === 'get_current') {
          const document = state.documents.get(String(request.documentId));
          if (!document) {
            return { ok: false, errorCode: 'DOCUMENT_NOT_FOUND', retryable: false, repairRequired: false };
          }
          return {
            ok: true,
            retryable: false,
            repairRequired: false,
            data: {
              kind: 'current',
              documentId: document.documentId,
              title: document.title,
              path: document.path,
              body: document.body,
              currentVersionId: document.currentVersionId
            }
          };
        }
        if (request.kind === 'create') {
          const documentId = String(request.documentId || '');
          const document = {
            documentId,
            title: String(request.title || 'Untitled Document'),
            path: String(request.path || 'notes/untitled.md'),
            body: String(request.body || ''),
            currentVersionId: String(request.versionId || 'version-created')
          };
          state.documents.set(documentId, document);
          state.versions.set(documentId, [{
            versionId: document.currentVersionId,
            body: document.body,
            summary: 'Created',
            author: 'local-user'
          }]);
          state.events.push({
            kind: 'create',
            bodyLength: document.body.length,
            versionId: document.currentVersionId
          });
          return {
            ok: true,
            retryable: false,
            repairRequired: false,
            data: {
              kind: 'created',
              documentId,
              currentVersionId: document.currentVersionId
            }
          };
        }
        if (request.kind === 'update') {
          const document = state.documents.get(String(request.documentId));
          if (!document) {
            return { ok: false, errorCode: 'DOCUMENT_NOT_FOUND', retryable: false, repairRequired: false };
          }
          if (request.expectedVersionId !== document.currentVersionId) {
            return {
              ok: false,
              errorCode: 'DOCUMENT_AUTHORING_STALE_VERSION',
              retryable: true,
              repairRequired: false
            };
          }
          document.body = String(request.body || '');
          document.currentVersionId = String(request.versionId || ('version-' + (state.saveCount + 2)));
          const history = state.versions.get(document.documentId) || [];
          history.push({
            versionId: document.currentVersionId,
            body: document.body,
            summary: 'Updated',
            author: 'local-user'
          });
          state.versions.set(document.documentId, history);
          state.saveCount += 1;
          state.events.push({
            kind: 'update',
            bodyLength: document.body.length,
            versionId: document.currentVersionId,
            revision: Number(request.revision || state.saveCount)
          });
          return {
            ok: true,
            retryable: false,
            repairRequired: false,
            data: {
              kind: 'updated',
              documentId: document.documentId,
              currentVersionId: document.currentVersionId
            }
          };
        }
        if (request.kind === 'get_history') {
          const history = state.versions.get(String(request.documentId)) || [];
          return {
            ok: true,
            retryable: false,
            repairRequired: false,
            data: {
              kind: 'history',
              documentId: String(request.documentId),
              entries: history.map((entry) => ({
                versionId: entry.versionId,
                summary: entry.summary,
                author: entry.author,
                createdAt: 'local-version'
              }))
            }
          };
        }
        if (request.kind === 'preview_restore') {
          const history = state.versions.get(String(request.documentId)) || [];
          const target = history.find((entry) => entry.versionId === request.targetVersionId);
          if (!target) {
            return { ok: false, errorCode: 'DOCUMENT_RESTORE_NOT_FOUND', retryable: false, repairRequired: false };
          }
          return {
            ok: true,
            retryable: false,
            repairRequired: false,
            data: {
              kind: 'restore_preview',
              documentId: String(request.documentId),
              targetVersionId: target.versionId,
              expectedCurrentVersionId: String(request.expectedCurrentVersionId),
              canRestore: true,
              lines: [
                { kind: 'removed', text: 'current snapshot changed' },
                { kind: 'added', text: 'target snapshot selected' }
              ]
            }
          };
        }
        if (request.kind === 'restore') {
          const document = state.documents.get(String(request.documentId));
          const history = state.versions.get(String(request.documentId)) || [];
          const target = history.find((entry) => entry.versionId === request.targetVersionId);
          if (!document || !target) {
            return { ok: false, errorCode: 'DOCUMENT_RESTORE_NOT_FOUND', retryable: false, repairRequired: false };
          }
          if (document.currentVersionId !== request.expectedCurrentVersionId) {
            return { ok: false, errorCode: 'DOCUMENT_RESTORE_VERSION_CONFLICT', retryable: false, repairRequired: false };
          }
          document.body = target.body;
          document.currentVersionId = String(request.restoredVersionId || 'version-restored');
          history.push({
            versionId: document.currentVersionId,
            body: document.body,
            summary: 'Restored',
            author: 'local-user'
          });
          state.events.push({ kind: 'restore', versionId: document.currentVersionId });
          return {
            ok: true,
            retryable: false,
            repairRequired: false,
            data: {
              kind: 'restored',
              documentId: document.documentId,
              restoredVersionId: document.currentVersionId,
              currentVersionId: document.currentVersionId
            }
          };
        }
      }
      return { ok: false, errorCode: 'COMMAND_BRIDGE_FAILED', retryable: false, repairRequired: false };
    }}};
  })();`;
}

async function navigateToApp(cdp, webPort, timeoutMs) {
  await cdp.send("Page.navigate", { url: `http://127.0.0.1:${webPort}/` });
  await waitForExpression(cdp, `document.querySelector('[data-cabinet-home-state="Ready"]') !== null`, timeoutMs);
}

async function openAuthoringDocument(cdp, timeoutMs) {
  await evaluate(cdp, `document.querySelector('[data-document-id="doc-001"]')?.click(); true`);
  await waitForExpression(
    cdp,
    `document.querySelector('[data-cabinet-authoring-state="Clean"], [data-cabinet-authoring-state="Saved"]') !== null`,
    timeoutMs,
  );
  await waitForExpression(cdp, `document.querySelector('.cm-editor .cm-content') !== null`, timeoutMs);
  await waitForExpression(cdp, `document.querySelector('.markdown-preview table tbody td') !== null`, timeoutMs);
}

async function editCodeMirror(cdp, text) {
  await evaluate(cdp, `document.querySelector('.cm-content')?.focus(); true`);
  await cdp.send("Input.insertText", { text });
}

async function dispatchModSave(cdp) {
  await cdp.send("Page.bringToFront");
  for (const modifiers of [4, 2]) {
    await cdp.send("Input.dispatchKeyEvent", {
      type: "rawKeyDown",
      key: "s",
      code: "KeyS",
      windowsVirtualKeyCode: 83,
      nativeVirtualKeyCode: 83,
      modifiers,
    });
    await cdp.send("Input.dispatchKeyEvent", {
      type: "keyUp",
      key: "s",
      code: "KeyS",
      windowsVirtualKeyCode: 83,
      nativeVirtualKeyCode: 83,
      modifiers,
    });
    await delay(100);
  }
}

async function dispatchTab(cdp) {
  await cdp.send("Input.dispatchKeyEvent", {
    type: "rawKeyDown",
    key: "Tab",
    code: "Tab",
    windowsVirtualKeyCode: 9,
    nativeVirtualKeyCode: 9,
  });
  await cdp.send("Input.dispatchKeyEvent", {
    type: "keyUp",
    key: "Tab",
    code: "Tab",
    windowsVirtualKeyCode: 9,
    nativeVirtualKeyCode: 9,
  });
}

async function clickText(cdp, selector, text) {
  const clicked = await evaluate(cdp, `(() => {
    const needle = ${JSON.stringify(text)};
    const target = Array.from(document.querySelectorAll(${JSON.stringify(selector)}))
      .find((node) => node.textContent.trim() === needle);
    target?.click();
    return Boolean(target);
  })()`);
  if (!clicked) throw new Error(`Unable to click ${selector} with text ${text}`);
}

async function clickSelector(cdp, selector) {
  const clicked = await evaluate(cdp, `(() => {
    const target = document.querySelector(${JSON.stringify(selector)});
    target?.click();
    return Boolean(target);
  })()`);
  if (!clicked) throw new Error(`Unable to click ${selector}`);
}

async function eventCount(cdp, kind) {
  return Number(await evaluate(
    cdp,
    `globalThis.__CABINET_PHASE011_AUTHORING_EVENTS__?.filter((event) => event.kind === ${JSON.stringify(kind)}).length ?? 0`,
  ));
}

async function waitForEventCount(cdp, kind, expectedCount, timeoutMs) {
  await waitForExpression(
    cdp,
    `(globalThis.__CABINET_PHASE011_AUTHORING_EVENTS__?.filter((event) => event.kind === ${JSON.stringify(kind)}).length ?? 0) >= ${expectedCount}`,
    timeoutMs,
  );
}

function sanitizedInteractionExpression() {
  return `(() => {
    const text = document.body.textContent || '';
    return {
      closeBlocked: true,
      closeCancel: true,
      closeRetrySave: true,
      closeDiscard: true,
      rawBodyExcluded: !text.includes('raw markdown body') && !JSON.stringify(globalThis.__CABINET_PHASE011_AUTHORING_EVENTS__ || []).includes('Manual browser edit'),
      rawPathExcluded: !text.includes('/Users/') && !text.includes('C:\\\\Users\\\\')
    };
  })()`;
}

function visualMetricsExpression() {
  return `(() => {
    const rect = (selector) => document.querySelector(selector)?.getBoundingClientRect();
    const intersects = (a, b) => Boolean(a && b && a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top);
    const top = rect('.desktop-topbar');
    const side = rect('.desktop-sidebar');
    const main = rect('.authoring-main');
    const workspace = rect('.authoring-workspace');
    const focused = document.activeElement;
    const style = focused instanceof HTMLElement ? getComputedStyle(focused) : null;
    return {
      readyState: Boolean(document.querySelector('[data-cabinet-authoring-state="Clean"], [data-cabinet-authoring-state="Saved"]')),
      codeMirrorMounted: Boolean(document.querySelector('.cm-editor .cm-content')),
      previewTableRendered: Boolean(document.querySelector('.markdown-preview table tbody td')),
      overlapCount: Number(intersects(top, main)) + Number(intersects(side, main)) + Number(!workspace),
      horizontalOverflow: document.documentElement.scrollWidth > innerWidth + 1,
      focusVisible: focused instanceof HTMLElement && style?.outlineStyle !== 'none' && parseFloat(style?.outlineWidth || '0') > 0,
    };
  })()`;
}

async function setViewport(cdp, viewport) {
  await cdp.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
}

async function estimateNonBlankPixels(path) {
  const { stdout } = await execFileAsync("magick", [
    path,
    "-colorspace", "gray",
    "-threshold", "98%",
    "-format", "%[fx:(1-mean)*w*h]",
    "info:",
  ]);
  return Math.round(Number.parseFloat(stdout.trim()));
}

async function evaluate(cdp, expression) {
  const response = await cdp.send("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true,
  });
  if (response.exceptionDetails) throw new Error("browser evaluation failed");
  return response.result?.value;
}

async function waitForExpression(cdp, expression, timeoutMs) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (await evaluate(cdp, `Boolean(${expression})`)) return;
    await delay(100);
  }
  throw new Error(`browser condition timeout: ${expression}`);
}

async function findFreePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => typeof address === "object" && address?.port ? resolve(address.port) : reject(new Error("port unavailable")));
    });
  });
}

async function waitForHttp(url, timeoutMs) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) return response;
    } catch {}
    await delay(100);
  }
  throw new Error(`http timeout: ${url}`);
}

async function pageTarget(port, timeoutMs) {
  const response = await waitForHttp(`http://127.0.0.1:${port}/json/list`, timeoutMs);
  const targets = await response.json();
  const target = targets.find((item) => item.type === "page" && item.webSocketDebuggerUrl);
  if (!target) throw new Error("CDP page target unavailable");
  return target;
}

async function stopChild(child) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  child.kill("SIGTERM");
  await Promise.race([new Promise((resolve) => child.once("exit", resolve)), delay(2000)]);
  if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

class CdpClient {
  constructor(socket) {
    this.socket = socket;
    this.nextId = 1;
    this.pending = new Map();
    socket.addEventListener("message", (event) => this.onMessage(event.data));
  }

  static async connect(url) {
    const socket = new WebSocket(url);
    await new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve, { once: true });
      socket.addEventListener("error", () => reject(new Error("CDP socket unavailable")), { once: true });
    });
    return new CdpClient(socket);
  }

  send(method, params = {}) {
    const id = this.nextId++;
    const promise = new Promise((resolve, reject) => this.pending.set(id, { resolve, reject }));
    this.socket.send(JSON.stringify({ id, method, params }));
    return promise;
  }

  onMessage(raw) {
    const message = JSON.parse(raw);
    const pending = this.pending.get(message.id);
    if (!pending) return;
    this.pending.delete(message.id);
    message.error ? pending.reject(new Error(message.error.message)) : pending.resolve(message.result ?? {});
  }

  close() {
    this.socket.close();
  }
}

async function main() {
  const root = process.cwd();
  const inventory = await readFile(join(root, ".tasks", "phase011-current-implementation-inventory.md"), "utf8");
  const sourceFingerprint = inventory.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
  if (!sourceFingerprint) throw new Error("Phase011 source fingerprint missing");
  const chromePath = resolveChromePath();
  const report = await runPhase011AuthoringBrowserEvidence({ root, chromePath, sourceFingerprint });
  await mkdir(join(root, ".tasks", "release"), { recursive: true });
  await writeFile(
    join(root, ".tasks", "release", "authoring-browser-phase011.json"),
    `${JSON.stringify(report, null, 2)}\n`,
  );
  console.log("phase011_authoring_browser=passed");
}

function resolveChromePath() {
  const candidates = [
    process.env.SPONZEY_CABINET_CHROME_BIN,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
  ].filter(Boolean);
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) throw new Error("Chrome not found");
  return found;
}

if (process.argv[1]?.endsWith("run_phase011_authoring_browser.mjs")) {
  await main();
}
