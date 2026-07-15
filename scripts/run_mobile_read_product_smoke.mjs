import { spawn } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import net from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";

let build;

try {
  ({ build } = await import("esbuild"));
} catch {
  console.error("Mobile read product smoke requires npm dependencies. Run `npm install` once, then retry.");
  process.exit(1);
}

const root = process.cwd();
const outputArtifactPath = join(root, ".tmp", "mobile-read-product-smoke-output.txt");
const productLogSourceTargets = [
  "apps/mobile/src/index.ts",
  "apps/mobile/tests/mobile_read_product_smoke.ts",
];
const forbiddenProductLogTokens = [
  "product_log_event",
  "ProductLogEvent",
  "ProductLogger",
  "write_product",
  "writeProduct",
];
const sensitiveFixtures = [
  "e2e-password-should-not-log",
  "e2e-session-token-should-not-log",
  "mobile-product-invalid-token-should-not-log",
  "E2E document body should not be logged",
  "comment body should not leak",
  "asset-content-should-not-log",
  "phase002-secret-fixture-should-not-log",
  "mobile product raw document body",
  "mobile product raw comment body",
  "mobile-product-push-token",
  "mobile-product-push-session",
  "mobile-product-canvas",
];
const requiredSmokeMarkers = [
  "mobile_read_product_smoke=passed",
  "mobile_review_decision_product_smoke=passed",
  "mobile_canvas_unsupported_product_smoke=passed",
  "mobile_push_payload_product_smoke=passed",
];

async function main() {
  await assertMobileClientDoesNotWriteProductLog();

  const port = await reservePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const tempRoot = await mkdtemp(join(tmpdir(), "sponzey-cabinet-mobile-read-smoke-"));
  const server = startServer(port, tempRoot);
  let mobileOutput = "";

  try {
    await waitForServer(baseUrl, server);
    const token = await loginForToken(baseUrl);
    mobileOutput = await runMobileSmoke(baseUrl, token);
    assertRequiredMarkersPresent(mobileOutput);
    assertSensitiveOutputClean(mobileOutput);
    assertSensitiveOutputClean(server.output());
    await writeOutputArtifact(`${mobileOutput}${server.output()}`);
    const renderedOutput = mobileOutput.trimEnd();
    console.log(renderedOutput);
    if (!renderedOutput.includes("mobile_read_product_smoke=passed")) {
      console.log("mobile_read_product_smoke=passed");
    }
  } catch (error) {
    console.error("mobile_read_product_smoke=failed");
    console.error(`failure_category=${error instanceof SmokeAssertionError ? error.category : "unexpected_failure"}`);
    process.exitCode = 1;
  } finally {
    await stopServer(baseUrl, server);
    await rm(tempRoot, { recursive: true, force: true });
    console.log("mobile_read_product_child_cleanup=completed");
  }
}

async function assertMobileClientDoesNotWriteProductLog() {
  for (const relativePath of productLogSourceTargets) {
    const absolutePath = join(root, relativePath);
    if (!existsSync(absolutePath)) {
      throw new SmokeAssertionError(`missing_source_${sanitizePath(relativePath)}`);
    }
    const source = await readFile(absolutePath, "utf8");
    for (const token of forbiddenProductLogTokens) {
      if (source.includes(token)) {
        throw new SmokeAssertionError("mobile_client_product_log_direct_write");
      }
    }
  }
}

async function runMobileSmoke(baseUrl, token) {
  const outdir = join(root, ".tmp", "mobile-read-product-smoke");
  await rm(outdir, { recursive: true, force: true });
  await mkdir(outdir, { recursive: true });
  const outfile = join(outdir, "mobile-read-product-smoke.mjs");

  await build({
    entryPoints: [join(root, "apps/mobile/tests/mobile_read_product_smoke.ts")],
    outfile,
    bundle: true,
    platform: "node",
    format: "esm",
    target: "node20",
    logLevel: "silent",
  });

  return spawnAndCapture(process.execPath, [
    outfile,
    "--server-base-url",
    baseUrl,
    "--session-token",
    token,
  ]);
}

function startServer(port, tempRoot) {
  const stdout = [];
  const stderr = [];
  const child = spawn("sh", ["scripts/run_self_host_server.sh", "--e2e-http-server"], {
    cwd: root,
    env: {
      ...process.env,
      SPONZEY_CABINET_SERVER_BIND_ADDRESS: `127.0.0.1:${port}`,
      SPONZEY_CABINET_SERVER_PUBLIC_URL: `http://127.0.0.1:${port}`,
      SPONZEY_CABINET_SERVER_METADATA_STORE_LOCATION: join(tempRoot, "metadata.sqlite3"),
      SPONZEY_CABINET_SERVER_OBJECT_STORAGE_BACKEND: "local-disk",
      SPONZEY_CABINET_SERVER_OBJECT_STORAGE_LOCATION: join(tempRoot, "object-store"),
      SPONZEY_CABINET_SERVER_BACKUP_STORE_LOCATION: join(tempRoot, "backups"),
      SPONZEY_CABINET_AUTH_TOKEN_SECRET: "phase003-mobile-read-token-secret",
      SPONZEY_CABINET_AUTH_TOKEN_BYTE_LENGTH: "32",
      SPONZEY_CABINET_SERVER_PRODUCT_LOG_SINK: "stdout",
      SPONZEY_CABINET_SERVER_DEVELOPMENT_LOG_MODE: "disabled",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  child.stdout.on("data", (chunk) => stdout.push(chunk.toString("utf8")));
  child.stderr.on("data", (chunk) => stderr.push(chunk.toString("utf8")));

  return {
    child,
    output() {
      return `${stdout.join("")}${stderr.join("")}`;
    },
  };
}

async function waitForServer(baseUrl, server) {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    if (server.child.exitCode !== null) {
      throw new SmokeAssertionError("server_start_failed");
    }
    try {
      const response = await fetch(`${baseUrl}/api/health`);
      if (response.status === 200) {
        await response.arrayBuffer();
        return;
      }
    } catch {
      await sleep(200);
    }
  }
  throw new SmokeAssertionError("server_start_timeout");
}

async function loginForToken(baseUrl) {
  const response = await fetch(`${baseUrl}/api/auth/login`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      login: "actor-a",
      credential: "e2e-password-should-not-log",
    }),
  });
  if (response.status !== 200) {
    throw new SmokeAssertionError("login_status_failed");
  }
  const body = await response.json();
  if (typeof body.token !== "string" || body.token.length === 0) {
    throw new SmokeAssertionError("login_token_missing");
  }
  return body.token;
}

async function stopServer(baseUrl, server) {
  if (server.child.exitCode !== null) {
    return;
  }
  try {
    await fetch(`${baseUrl}/__shutdown`, { method: "POST" });
  } catch {
    // The smoke may fail before the e2e server has accepted shutdown requests.
  }
  const stopped = await waitForChildExit(server.child, 5_000);
  if (!stopped) {
    server.child.kill("SIGTERM");
    await waitForChildExit(server.child, 5_000);
  }
  assertSensitiveOutputClean(server.output());
}

async function writeOutputArtifact(output) {
  assertSensitiveOutputClean(output);
  await mkdir(join(root, ".tmp"), { recursive: true });
  await writeFile(outputArtifactPath, output);
}

async function spawnAndCapture(command, args) {
  return new Promise((resolve, reject) => {
    const stdout = [];
    const stderr = [];
    const child = spawn(command, args, {
      cwd: root,
      stdio: ["ignore", "pipe", "pipe"],
    });
    child.stdout.on("data", (chunk) => stdout.push(chunk.toString("utf8")));
    child.stderr.on("data", (chunk) => stderr.push(chunk.toString("utf8")));
    child.on("error", reject);
    child.on("exit", (code, signal) => {
      const output = `${stdout.join("")}${stderr.join("")}`;
      if (signal) {
        emitSafeFailureOutput(output);
        reject(new SmokeAssertionError(`mobile_smoke_signal_${signal}`));
        return;
      }
      if (code !== 0) {
        emitSafeFailureOutput(output);
        reject(new SmokeAssertionError("mobile_smoke_failed"));
        return;
      }
      resolve(output);
    });
  });
}

function emitSafeFailureOutput(output) {
  assertSensitiveOutputClean(output);
  if (output.trim()) {
    console.error(output.trimEnd());
  }
}

function assertSensitiveOutputClean(output) {
  for (const fixture of sensitiveFixtures) {
    if (output.includes(fixture)) {
      throw new SmokeAssertionError("sensitive_output_detected");
    }
  }
}

function assertRequiredMarkersPresent(output) {
  for (const marker of requiredSmokeMarkers) {
    if (!output.includes(marker)) {
      throw new SmokeAssertionError(`missing_${marker.replace(/[^a-zA-Z0-9]+/g, "_")}`);
    }
  }
}

function sanitizePath(path) {
  return path.replace(/[^a-zA-Z0-9]+/g, "_").replace(/^_+|_+$/g, "");
}

async function reservePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (typeof address === "object" && address?.port) {
          resolve(address.port);
          return;
        }
        reject(new SmokeAssertionError("port_reservation_failed"));
      });
    });
  });
}

async function waitForChildExit(child, timeoutMs) {
  if (child.exitCode !== null) {
    return true;
  }
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      cleanup();
      resolve(false);
    }, timeoutMs);
    const onExit = () => {
      cleanup();
      resolve(true);
    };
    const cleanup = () => {
      clearTimeout(timer);
      child.off("exit", onExit);
    };
    child.once("exit", onExit);
  });
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

class SmokeAssertionError extends Error {
  constructor(category) {
    super(category);
    this.name = "SmokeAssertionError";
    this.category = category;
  }
}

await main();
