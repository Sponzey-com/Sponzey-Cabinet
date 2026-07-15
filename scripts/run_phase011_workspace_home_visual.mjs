import { spawn, execFile } from "node:child_process";
import { createServer } from "node:net";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

import {
  transitionWorkspaceHomeVisualState,
  validateWorkspaceHomeVisualReport,
  workspaceHomeVisualViewports,
} from "./phase011_workspace_home_visual.mjs";

const execFileAsync = promisify(execFile);

export async function runWorkspaceHomeVisualEvidence({
  root,
  chromePath,
  sourceFingerprint,
  timeoutMs = 20000,
}) {
  let state = "Pending";
  const webPort = await findFreePort();
  const debugPort = await findFreePort();
  const profile = await mkdtemp(join(tmpdir(), "sponzey-phase011-home-chrome-"));
  const screenshotsDir = join(root, ".tasks", "release", "screenshots", "workspace-home");
  const children = [];
  await mkdir(screenshotsDir, { recursive: true });

  try {
    state = transitionWorkspaceHomeVisualState(state, "Serve").state;
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

    state = transitionWorkspaceHomeVisualState(state, "Launch").state;
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
      state = transitionWorkspaceHomeVisualState(state, "Inject").state;
      await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: injectedTauriSource() });
      state = transitionWorkspaceHomeVisualState(state, "Navigate").state;
      const runs = [];
      for (const viewport of workspaceHomeVisualViewports()) {
        await cdp.send("Emulation.setDeviceMetricsOverride", {
          width: viewport.width,
          height: viewport.height,
          deviceScaleFactor: 1,
          mobile: false,
        });
        await cdp.send("Page.navigate", { url: `http://127.0.0.1:${webPort}/` });
        await waitForExpression(
          cdp,
          `document.querySelector('[data-cabinet-home-state="Ready"]') !== null`,
          timeoutMs,
        );
        state = transitionWorkspaceHomeVisualState(state, "Validate").state;
        await cdp.send("Page.bringToFront");
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
        const metrics = await evaluate(cdp, visualMetricsExpression());
        state = transitionWorkspaceHomeVisualState(state, "Capture").state;
        const capture = await cdp.send("Page.captureScreenshot", {
          format: "png",
          captureBeyondViewport: false,
        });
        const fileName = `workspace-home-${viewport.width}x${viewport.height}.png`;
        const screenshotPath = join(screenshotsDir, fileName);
        await writeFile(screenshotPath, Buffer.from(capture.data, "base64"));
        const nonBlankPixelCount = await estimateNonBlankPixels(screenshotPath);
        runs.push({
          ...viewport,
          ...metrics,
          nonBlankPixelCount,
          screenshot: fileName,
        });
        state = "Navigating";
      }

      await cdp.send("Page.navigate", { url: `http://127.0.0.1:${webPort}/` });
      await waitForExpression(cdp, `document.querySelector('[data-cabinet-home-state="Ready"]') !== null`, timeoutMs);
      await evaluate(cdp, `sessionStorage.setItem("cabinet-phase011-home-mode", "failed"); location.reload(); true`);
      await waitForExpression(cdp, `document.querySelector('[data-cabinet-home-state="Failed"]') !== null`, timeoutMs);
      await evaluate(cdp, `sessionStorage.setItem("cabinet-phase011-home-mode", "ready"); globalThis.__CABINET_PHASE011_HOME_MODE__ = "ready"; document.querySelector('.state-banner.failed button').focus(); true`);
      await cdp.send("Page.bringToFront");
      await cdp.send("Input.dispatchKeyEvent", {
        type: "rawKeyDown",
        key: "Enter",
        code: "Enter",
        text: "\r",
        unmodifiedText: "\r",
        windowsVirtualKeyCode: 13,
        nativeVirtualKeyCode: 13,
      });
      await cdp.send("Input.dispatchKeyEvent", {
        type: "keyUp",
        key: "Enter",
        code: "Enter",
        windowsVirtualKeyCode: 13,
        nativeVirtualKeyCode: 13,
      });
      await waitForExpression(cdp, `document.querySelector('[data-cabinet-home-state="Ready"]') !== null`, timeoutMs);

      await evaluate(cdp, `Array.from(document.querySelectorAll('.nav-item')).find((item) => item.textContent === 'Documents').click(); true`);
      await waitForExpression(cdp, `document.querySelector('[data-cabinet-navigator-state="Ready"]') !== null`, timeoutMs);
      for (const view of ["Tree", "Collection", "Tag", "Recent", "Favorite"]) {
        await evaluate(cdp, `document.querySelector('[data-navigator-view="${view}"]').click(); true`);
        await waitForExpression(
          cdp,
          `document.querySelector('[data-navigator-view="${view}"][aria-selected="true"]') !== null && document.querySelector('[data-cabinet-navigator-state="Ready"]') !== null`,
          timeoutMs,
        );
      }
      await evaluate(cdp, `(() => {
        const input = document.querySelector('input[aria-label="Filter documents"]');
        const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
        setter.call(input, 'missing');
        input.dispatchEvent(new Event('input', { bubbles: true }));
        return true;
      })()`);
      await waitForExpression(cdp, `document.querySelector('[data-cabinet-navigator-state="EmptyResult"]') !== null`, timeoutMs);
      await evaluate(cdp, `globalThis.__CABINET_PHASE011_NAVIGATOR_MODE__ = "failed"; document.querySelector('[data-navigator-view="Recent"]').click(); true`);
      await waitForExpression(cdp, `document.querySelector('[data-cabinet-navigator-state="Failed"]') !== null`, timeoutMs);
      await evaluate(cdp, `globalThis.__CABINET_PHASE011_NAVIGATOR_MODE__ = "ready"; document.querySelector('.state-banner.failed button').focus(); true`);
      await cdp.send("Page.bringToFront");
      await cdp.send("Input.dispatchKeyEvent", {
        type: "rawKeyDown",
        key: "Enter",
        code: "Enter",
        text: "\r",
        unmodifiedText: "\r",
        windowsVirtualKeyCode: 13,
        nativeVirtualKeyCode: 13,
      });
      await cdp.send("Input.dispatchKeyEvent", {
        type: "keyUp",
        key: "Enter",
        code: "Enter",
        windowsVirtualKeyCode: 13,
        nativeVirtualKeyCode: 13,
      });
      await waitForExpression(cdp, `document.querySelector('[data-cabinet-navigator-state="Ready"]') !== null`, timeoutMs);
      await evaluate(cdp, `Array.from(document.querySelectorAll('.nav-item')).find((item) => item.textContent === 'Home').click(); true`);
      await waitForExpression(cdp, `document.querySelector('[data-cabinet-home-state="Ready"]') !== null`, timeoutMs);

      const report = {
        marker: "phase011_workspace_home_visual=passed",
        sourceFingerprint,
        diagnostics: "sanitized",
        retryKeyboardFlow: true,
        navigatorInteractions: {
          fiveViews: true,
          filterEmpty: true,
          retryKeyboardFlow: true,
          homeReturn: true,
        },
        runs,
      };
      const validation = validateWorkspaceHomeVisualReport(report, sourceFingerprint);
      if (!validation.passed) {
        throw new Error(`workspace home visual validation failed: ${validation.findingIds.join(",")}`);
      }
      state = transitionWorkspaceHomeVisualState("Capturing", "Pass").state;
      return { ...report, state };
    } finally {
      cdp.close();
    }
  } finally {
    for (const child of children.reverse()) await stopChild(child);
    await rm(profile, { recursive: true, force: true, maxRetries: 3, retryDelay: 100 });
  }
}

function injectedTauriSource() {
  return `(() => {
    globalThis.__CABINET_PHASE011_HOME_MODE__ = sessionStorage.getItem("cabinet-phase011-home-mode") || "ready";
    globalThis.__CABINET_PHASE011_NAVIGATOR_MODE__ = "ready";
    const ready = {
      ok: true,
      retryable: false,
      data: {
        workspaceId: "workspace-1",
        state: "Ready",
        recentDocuments: [
          { documentId: "doc-001", title: "Architecture Notes", path: "notes/architecture.md" },
          { documentId: "doc-002", title: "Weekly Review", path: "reviews/weekly.md" }
        ],
        favorites: [{ documentId: "doc-003", title: "Project Index", path: "index/project.md" }],
        tags: [{ label: "design", documentCount: 4 }, { label: "research", documentCount: 3 }],
        recentChanges: [{ documentId: "doc-001", summary: "Updated document" }],
        unfinishedItems: [{ documentId: "doc-002", label: "Review draft" }],
        backupStatus: "Fresh",
        healthStatus: "Healthy"
      }
    };
    globalThis.__TAURI__ = { core: { invoke: async (command, args) => {
      if (command === "get_desktop_workspace_home") {
        return globalThis.__CABINET_PHASE011_HOME_MODE__ === "failed"
          ? { ok: false, errorCode: "WORKSPACE_HOME_PROJECTION_UNAVAILABLE", retryable: true }
          : ready;
      }
      if (command === "get_desktop_document_navigator") {
        if (globalThis.__CABINET_PHASE011_NAVIGATOR_MODE__ === "failed") {
          return { ok: false, errorCode: "DOCUMENT_NAVIGATOR_PROJECTION_UNAVAILABLE", retryable: true };
        }
        const request = args?.request || {};
        const empty = request.filter === "missing";
        return {
          ok: true,
          retryable: false,
          data: {
            workspaceId: "workspace-1",
            view: request.view,
            state: empty ? "EmptyResult" : "Ready",
            items: empty ? [] : [{
              documentId: "doc-001",
              title: "Architecture Notes",
              path: "notes/architecture.md",
              collections: ["work"],
              tags: ["rust"],
              favorite: true
            }],
            nextCursor: null
          }
        };
      }
      return { ok: false, errorCode: "COMMAND_BRIDGE_FAILED", retryable: false };
    }}};
  })();`;
}

function visualMetricsExpression() {
  return `(() => {
    const rect = (selector) => document.querySelector(selector)?.getBoundingClientRect();
    const intersects = (a, b) => Boolean(a && b && a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top);
    const top = rect('.desktop-topbar');
    const side = rect('.desktop-sidebar');
    const main = rect('.desktop-main');
    const sections = Array.from(document.querySelectorAll('.home-section')).map((node) => node.getBoundingClientRect());
    const incoherent = Number(intersects(top, main)) + Number(intersects(side, main)) + sections.filter((item, index) => sections.slice(index + 1).some((other) => intersects(item, other))).length;
    const focused = document.activeElement;
    const style = focused instanceof HTMLElement ? getComputedStyle(focused) : null;
    const forbidden = ['/Users/', 'C:\\\\Users\\\\', 'raw document body', 'serverBaseUrl', 'tenant admin'];
    return {
      readyState: Boolean(document.querySelector('[data-cabinet-react-root="mounted"]')),
      overlapCount: incoherent,
      horizontalOverflow: document.documentElement.scrollWidth > innerWidth + 1,
      focusVisible: focused instanceof HTMLButtonElement && style?.outlineStyle !== 'none' && parseFloat(style?.outlineWidth || '0') > 0,
      navLandmark: Boolean(document.querySelector('nav[aria-label="Workspace"]')),
      mainLandmark: Boolean(document.querySelector('main h1#home-title')),
      liveRegion: Boolean(document.querySelector('[aria-live="polite"]')),
      sensitiveDataExcluded: forbidden.every((token) => !document.body.textContent.includes(token)),
    };
  })()`;
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
  const response = await cdp.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
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

function delay(ms) { return new Promise((resolve) => setTimeout(resolve, ms)); }

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
  close() { this.socket.close(); }
}

async function main() {
  const root = process.cwd();
  const inventory = await readFile(join(root, ".tasks", "phase011-current-implementation-inventory.md"), "utf8");
  const sourceFingerprint = inventory.match(/source_fingerprint=([a-f0-9]{64})/)?.[1];
  if (!sourceFingerprint) throw new Error("Phase011 source fingerprint missing");
  const chromePath = resolveChromePath();
  const report = await runWorkspaceHomeVisualEvidence({ root, chromePath, sourceFingerprint });
  await writeFile(join(root, ".tasks", "release", "workspace-home-visual-phase011.json"), `${JSON.stringify(report, null, 2)}\n`);
  console.log("phase011_workspace_home_visual=passed");
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

if (process.argv[1]?.endsWith("run_phase011_workspace_home_visual.mjs")) {
  await main();
}
