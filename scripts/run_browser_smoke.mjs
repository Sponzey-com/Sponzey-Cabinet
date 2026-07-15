import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createServer } from "node:net";

const STORAGE_KEY = "sponzey-cabinet.local-workspace.v1";
const root = process.cwd();
const timeoutMs = Number.parseInt(process.env.SPONZEY_CABINET_BROWSER_SMOKE_TIMEOUT_MS ?? "15000", 10);

async function main() {
  const webPort = await findFreePort();
  const debugPort = await findFreePort();
  const chromeProfileDir = await mkdtemp(join(tmpdir(), "sponzey-cabinet-chrome-"));
  const children = [];

  try {
    const webServer = spawn(process.execPath, ["scripts/run_web_app.mjs", String(webPort)], {
      cwd: root,
      env: { ...process.env, SPONZEY_CABINET_RUNNER_ANNOUNCED: "1" },
      stdio: ["ignore", "pipe", "pipe"],
    });
    children.push(webServer);
    pipeChildOutput("web", webServer);

    const appUrl = `http://127.0.0.1:${webPort}/`;
    await waitForHttp(appUrl, timeoutMs);

    const chromePath = resolveChromePath();
    const chrome = spawn(chromePath, [
      "--headless=new",
      "--disable-gpu",
      "--disable-background-networking",
      "--disable-default-apps",
      "--disable-extensions",
      "--disable-sync",
      "--no-default-browser-check",
      "--no-first-run",
      "--window-size=1280,900",
      `--remote-debugging-port=${debugPort}`,
      `--user-data-dir=${chromeProfileDir}`,
      "about:blank",
    ], {
      cwd: root,
      stdio: ["ignore", "pipe", "pipe"],
    });
    children.push(chrome);
    pipeChildOutput("chrome", chrome);

    await waitForHttp(`http://127.0.0.1:${debugPort}/json/version`, timeoutMs);
    const target = await getPageTarget(debugPort);
    const cdp = await CdpClient.connect(target.webSocketDebuggerUrl);

    try {
      await cdp.send("Page.enable");
      await cdp.send("Runtime.enable");
      await cdp.send("Page.navigate", { url: appUrl });
      await waitForPageCondition(cdp, "document.readyState === 'complete'", timeoutMs);

      await evaluate(cdp, seedWorkspaceScript());
      await waitForPageCondition(cdp, "Boolean(document.querySelector('.cm-editor .cm-content'))", timeoutMs);
      await waitForPageCondition(cdp, "Boolean(document.querySelector('.markdown-preview table'))", timeoutMs);
      await waitForPageCondition(cdp, "Boolean(document.querySelector('[data-cabinet-app-root=\"mounted\"]'))", timeoutMs);
      await waitForPageCondition(cdp, "Boolean(document.querySelector('[data-cabinet-workspace-shell=\"ready\"]'))", timeoutMs);
      await waitForPageCondition(cdp, "Boolean(document.querySelector('[data-cabinet-editor=\"mounted\"]'))", timeoutMs);
      await waitForPageCondition(cdp, "Boolean(document.querySelector('[data-cabinet-bootstrap-state=\"ready\"]'))", timeoutMs);

      const initial = await evaluate(cdp, initialAssertionsScript());
      assertBrowserResult(initial.appRootMounted, "App root mounted");
      assertBrowserResult(initial.workspaceShellReady, "Workspace shell ready");
      assertBrowserResult(initial.editorMarkerMounted, "Editor marker mounted");
      assertBrowserResult(initial.bootstrapReady, "Bootstrap ready");
      assertBrowserResult(initial.codeMirrorMounted, "CodeMirror editor mounted");
      assertBrowserResult(initial.markdownLanguageVisible, "CodeMirror content is visible");
      assertBrowserResult(initial.previewTableRendered, "Markdown preview table rendered");
      assertBrowserResult(initial.previewTableBodyRendered, "Markdown preview table body rendered");
      assertBrowserResult(initial.leftAlignmentRendered, "Markdown preview left alignment rendered");
      assertBrowserResult(initial.centerAlignmentRendered, "Markdown preview center alignment rendered");
      assertBrowserResult(initial.rightAlignmentRendered, "Markdown preview right alignment rendered");
      assertBrowserResult(initial.currentHistorySplitReady, "Current history split marker ready");
      assertBrowserResult(initial.searchResultFound, "Search result rendered");
      assertBrowserResult(initial.unresolvedLinkRendered, "Unresolved link rendered");
      assertBrowserResult(initial.graphEdgeRendered, "Graph edge rendered");
      assertBrowserResult(initial.graphNodeRendered, "Graph node rendered");
      assertBrowserResult(initial.assetMetadataListed, "Asset metadata listed");
      assertBrowserResult(initial.assetMetadataDetailRendered, "Asset metadata detail rendered");
      assertBrowserResult(initial.assetPathHidden, "Asset path hidden");
      assertBrowserResult(initial.backupPanelReady, "Backup panel ready");
      assertBrowserResult(initial.backupManifestSummaryRendered, "Backup manifest summary rendered");
      assertBrowserResult(initial.importPreviewReady, "Import preview ready");
      assertBrowserResult(initial.restoreConfirmationReady, "Restore confirmation ready");
      assertBrowserResult(initial.recoveryActionReady, "Recovery action ready");
      assertBrowserResult(initial.backupSensitiveDataHidden, "Backup sensitive data hidden");

      const flow = await evaluate(cdp, userFlowScript(), { awaitPromise: true });
      assertBrowserResult(flow.createdDocument, "New document created");
      assertBrowserResult(flow.dirtyMarkerObserved, "Authoring dirty marker observed");
      assertBrowserResult(flow.savedEdit, "Edited document saved");
      assertBrowserResult(flow.savedMarkerObserved, "Authoring saved marker observed");
      assertBrowserResult(flow.savedVersionMarkerObserved, "Authoring saved version marker observed");
      assertBrowserResult(flow.savedBodyIncludesWikilink, "Saved body includes wikilink");
      assertBrowserResult(flow.searchFoundSavedDocument, "Saved document found through search");
      assertBrowserResult(flow.backlinkRendered, "Backlink rendered");
      assertBrowserResult(flow.restoredVersion, "Restore flow completed");

      console.log("browser_smoke=passed");
      console.log(`web_url=${appUrl}`);
      console.log(`chrome_bin=${chromePath}`);
    } finally {
      cdp.close();
    }
  } finally {
    for (const child of children.reverse()) {
      await stopChild(child);
    }
    await removeWithRetry(chromeProfileDir);
  }
}

function pipeChildOutput(label, child) {
  if (process.env.SPONZEY_CABINET_BROWSER_SMOKE_VERBOSE !== "1") {
    return;
  }
  child.stdout?.on("data", (chunk) => process.stdout.write(`[${label}] ${chunk}`));
  child.stderr?.on("data", (chunk) => process.stderr.write(`[${label}] ${chunk}`));
}

function assertBrowserResult(condition, message) {
  if (!condition) {
    throw new Error(`Browser smoke failed: ${message}`);
  }
  console.log(`${toSnakeCase(message)}=true`);
}

function toSnakeCase(value) {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_+|_+$/g, "");
}

async function findFreePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (typeof address === "object" && address?.port) {
          resolve(address.port);
          return;
        }
        reject(new Error("Unable to allocate a local port"));
      });
    });
  });
}

function resolveChromePath() {
  const candidates = [
    process.env.SPONZEY_CABINET_CHROME_BIN,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
  ].filter(Boolean);

  const found = candidates.find((candidate) => existsSync(candidate));
  if (found) {
    return found;
  }

  throw new Error("Chrome was not found. Set SPONZEY_CABINET_CHROME_BIN to a Chrome or Chromium executable.");
}

async function waitForHttp(url, deadlineMs) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < deadlineMs) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return response;
      }
      lastError = new Error(`HTTP ${response.status} for ${url}`);
    } catch (error) {
      lastError = error;
    }
    await delay(100);
  }
  throw lastError ?? new Error(`Timed out waiting for ${url}`);
}

async function getPageTarget(port) {
  const response = await waitForHttp(`http://127.0.0.1:${port}/json/list`, timeoutMs);
  const targets = await response.json();
  const page = targets.find((target) => target.type === "page" && target.webSocketDebuggerUrl);
  if (!page) {
    throw new Error("Chrome DevTools page target was not found");
  }
  return page;
}

async function waitForPageCondition(cdp, expression, deadlineMs) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < deadlineMs) {
    try {
      const value = await evaluate(cdp, `Boolean(${expression})`);
      if (value) {
        return;
      }
    } catch (error) {
      lastError = error;
    }
    await delay(100);
  }
  throw lastError ?? new Error(`Timed out waiting for page condition: ${expression}`);
}

async function evaluate(cdp, expression, options = {}) {
  const response = await cdp.send("Runtime.evaluate", {
    expression,
    awaitPromise: options.awaitPromise ?? false,
    returnByValue: true,
  });
  if (response.exceptionDetails) {
    throw new Error(response.exceptionDetails.text ?? "Runtime.evaluate failed");
  }
  return response.result?.value;
}

function seedWorkspaceScript() {
  const workspace = {
    workspaceId: "browser-smoke",
    selectedDocumentId: "doc-source",
    documents: [
      {
        id: "doc-source",
        title: "Source Document",
        path: "docs/source.md",
        body: [
          "# Source Document",
          "",
          "This document links to [[Target Document]] and keeps attachment metadata outside the body.",
          "This document also references [[Missing Page]] for unresolved link checks.",
          "",
          "Search term: searchneedle",
          "",
          "| Item | Value | State |",
          "| :--- | :---: | ---: |",
          "| Row 1 | Centered | Done |",
          "| Row 2 | Aligned | Ready |",
          "",
          "![[asset:asset-mvp|MVP Asset]]",
          "",
        ].join("\n"),
        assets: [
          {
            id: "asset-mvp",
            label: "MVP Asset",
            fileName: "mvp-e2e.txt",
            mediaType: "text/plain",
            byteSize: 23,
            status: "available",
          },
        ],
        versions: [
          {
            id: "source-v-0002",
            summary: "Table source",
            author: "system",
            createdAt: "2026-06-23T13:00:00.000Z",
            body: [
              "# Source Document",
              "",
              "Search term: searchneedle",
              "",
              "| Item | Value | State |",
              "| :--- | :---: | ---: |",
              "| Row 1 | Centered | Done |",
              "",
              "![[asset:asset-mvp|MVP Asset]]",
              "",
            ].join("\n"),
          },
          {
            id: "source-v-0001",
            summary: "Create source",
            author: "system",
            createdAt: "2026-06-23T12:55:00.000Z",
            body: "# Source Document\n\nInitial local document.\n",
          },
        ],
      },
      {
        id: "doc-target",
        title: "Target Document",
        path: "docs/target.md",
        body: "# Target Document\n\nBacklinks from the source document resolve here.\n",
        assets: [],
        versions: [
          {
            id: "target-v-0001",
            summary: "Create target",
            author: "system",
            createdAt: "2026-06-23T12:54:00.000Z",
            body: "# Target Document\n\nBacklinks from the source document resolve here.\n",
          },
        ],
      },
    ],
  };

  return `(() => {
    localStorage.setItem(${JSON.stringify(STORAGE_KEY)}, ${JSON.stringify(JSON.stringify(workspace))});
    location.reload();
    return true;
  })()`;
}

function initialAssertionsScript() {
  return `(() => {
    const text = (selector) => Array.from(document.querySelectorAll(selector)).map((node) => node.textContent.trim());
    return {
      codeMirrorMounted: Boolean(document.querySelector(".cm-editor .cm-content")),
      appRootMounted: Boolean(document.querySelector('[data-cabinet-app-root="mounted"]')),
      workspaceShellReady: Boolean(document.querySelector('[data-cabinet-workspace-shell="ready"]')),
      editorMarkerMounted: Boolean(document.querySelector('[data-cabinet-editor="mounted"]')),
      bootstrapReady: Boolean(document.querySelector('[data-cabinet-bootstrap-state="ready"]')),
      markdownLanguageVisible: document.querySelector(".cm-content")?.textContent.includes("Source Document") ?? false,
      previewTableRendered: Boolean(document.querySelector(".markdown-preview table thead th")),
      previewTableBodyRendered: Boolean(document.querySelector(".markdown-preview table tbody td")),
      leftAlignmentRendered: Boolean(document.querySelector(".markdown-preview th.align-left")),
      centerAlignmentRendered: Boolean(document.querySelector(".markdown-preview th.align-center")),
      rightAlignmentRendered: Boolean(document.querySelector(".markdown-preview th.align-right")),
      currentHistorySplitReady: document.querySelector(".statusbar")?.dataset.cabinetCurrentHistorySplit === "ready",
      searchResultFound: text(".search-results .document-title").some((value) => value.includes("Source Document")),
      unresolvedLinkRendered: text('[data-cabinet-link-group="unresolved"] .link-item').some((value) => value.includes("Missing Page")),
      graphEdgeRendered: Boolean(document.querySelector('[data-cabinet-graph-panel="ready"] line')),
      graphNodeRendered: Boolean(document.querySelector('[data-cabinet-graph-panel="ready"] circle')),
      assetMetadataListed: text(".asset-item strong").some((value) => value.includes("MVP Asset")),
      assetMetadataDetailRendered: text('[data-cabinet-asset-metadata="ready"]').some((value) => value.includes("mvp-e2e.txt") && value.includes("text/plain") && value.includes("23 bytes")),
      assetPathHidden: !document.body.textContent.includes("/Users/") && !document.body.textContent.includes("C:\\\\Users\\\\"),
      backupPanelReady: Boolean(document.querySelector('[data-cabinet-backup-panel="ready"]')),
      backupManifestSummaryRendered: text('[data-cabinet-backup-manifest="ready"]').some((value) => value.includes("documents") && value.includes("versions") && value.includes("assets")),
      importPreviewReady: Boolean(document.querySelector('[data-cabinet-import-panel="preview-ready"]')),
      restoreConfirmationReady: Boolean(document.querySelector('[data-cabinet-restore-panel="confirmation-required"]')),
      recoveryActionReady: Boolean(document.querySelector('[data-cabinet-recovery-panel="action-ready"]')),
      backupSensitiveDataHidden: !document.body.textContent.includes("raw markdown body should not leak") &&
        !document.body.textContent.includes("asset binary content should not leak") &&
        !document.body.textContent.includes("/Users/") &&
        !document.body.textContent.includes("C:\\\\Users\\\\"),
    };
  })()`;
}

function userFlowScript() {
  return `(async () => {
    const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    const dispatchInput = (element) => element.dispatchEvent(new Event("input", { bubbles: true }));
    const text = (selector) => Array.from(document.querySelectorAll(selector)).map((node) => node.textContent.trim());

    document.querySelector("#new-document").click();
    await delay(100);

    const titleInput = document.querySelector("#title-input");
    const pathInput = document.querySelector("#path-input");
    titleInput.value = "Browser Smoke Note";
    pathInput.value = "docs/browser-smoke.md";
    dispatchInput(titleInput);
    dispatchInput(pathInput);
    await delay(50);
    document.querySelector("#insert-wikilink").click();
    await delay(100);

    const dirtyMarkerObserved =
      document.querySelector("#body-editor")?.dataset.cabinetEditorState === "ReadyDirty" &&
      document.querySelector("#save-document")?.dataset.cabinetSaveState === "dirty";

    document.querySelector("#save-document").click();
    await delay(100);

    const createdDocument = text(".document-list .document-title").some((value) => value.includes("Browser Smoke Note"));
    const savedEdit = document.querySelector(".statusbar span")?.textContent.includes("Document saved") ?? false;
    const savedMarkerObserved =
      document.querySelector("#body-editor")?.dataset.cabinetEditorState === "Saved" &&
      document.querySelector("#save-document")?.dataset.cabinetSaveState === "saved";
    const savedVersionMarkerObserved =
      (document.querySelector("#save-document")?.dataset.cabinetSavedVersion ?? "").startsWith("v-");
    const savedWorkspace = JSON.parse(localStorage.getItem("sponzey-cabinet.local-workspace.v1"));
    const savedDocument = savedWorkspace.documents.find((document) => document.title === "Browser Smoke Note");
    const savedBodyIncludesWikilink = savedDocument?.body.includes("[[Source Document]]") ?? false;

    const searchInput = document.querySelector("#search-input");
    searchInput.value = "Browser Smoke";
    dispatchInput(searchInput);
    await delay(100);

    const searchFoundSavedDocument = text(".search-results .document-title").some((value) => value.includes("Browser Smoke Note"));

    const sourceButton = Array.from(document.querySelectorAll(".document-list [data-select-document]"))
      .find((button) => button.textContent.includes("Source Document"));
    sourceButton.click();
    await delay(100);
    const backlinkRendered = text('[data-cabinet-link-group="backlinks"] .link-item')
      .some((value) => value.includes("Browser Smoke Note"));

    document.querySelector("#restore-latest").click();
    await delay(100);

    const restoredVersion = document.querySelector(".statusbar span")?.textContent.includes("Restored") ?? false;

    return {
      createdDocument,
      dirtyMarkerObserved,
      savedEdit,
      savedMarkerObserved,
      savedVersionMarkerObserved,
      savedBodyIncludesWikilink,
      searchFoundSavedDocument,
      backlinkRendered,
      restoredVersion,
    };
  })()`;
}

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function stopChild(child) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return;
  }

  child.kill("SIGTERM");
  const exited = new Promise((resolve) => child.once("exit", resolve));
  const timedOut = delay(2000).then(() => "timeout");
  const result = await Promise.race([exited, timedOut]);
  if (result === "timeout" && child.exitCode === null && child.signalCode === null) {
    child.kill("SIGKILL");
    await new Promise((resolve) => child.once("exit", resolve));
  }
}

async function removeWithRetry(path) {
  let lastError;
  for (let attempt = 0; attempt < 10; attempt += 1) {
    try {
      await rm(path, { recursive: true, force: true, maxRetries: 3, retryDelay: 100 });
      return;
    } catch (error) {
      lastError = error;
      await delay(100);
    }
  }
  throw lastError;
}

class CdpClient {
  constructor(socket) {
    this.socket = socket;
    this.nextId = 1;
    this.pending = new Map();
    this.socket.addEventListener("message", (event) => this.handleMessage(event.data));
    this.socket.addEventListener("error", (event) => {
      for (const { reject } of this.pending.values()) {
        reject(new Error(`Chrome DevTools socket error: ${event.message ?? "unknown"}`));
      }
      this.pending.clear();
    });
  }

  static async connect(url) {
    const socket = new WebSocket(url);
    await new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve, { once: true });
      socket.addEventListener("error", () => reject(new Error("Unable to open Chrome DevTools socket")), { once: true });
    });
    return new CdpClient(socket);
  }

  send(method, params = {}) {
    const id = this.nextId;
    this.nextId += 1;
    const payload = JSON.stringify({ id, method, params });
    const promise = new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
    this.socket.send(payload);
    return promise;
  }

  handleMessage(raw) {
    const message = JSON.parse(raw);
    if (!message.id) {
      return;
    }
    const pending = this.pending.get(message.id);
    if (!pending) {
      return;
    }
    this.pending.delete(message.id);
    if (message.error) {
      pending.reject(new Error(message.error.message ?? "Chrome DevTools command failed"));
      return;
    }
    pending.resolve(message.result ?? {});
  }

  close() {
    this.socket.close();
  }
}

await main();
