import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";

const root = process.cwd();
const publicDir = join(root, process.env.SPONZEY_CABINET_WEB_PUBLIC_DIR ?? "apps/web/public");
const port = Number.parseInt(process.argv[2] ?? "5173", 10);
const requireExactPort = process.env.SPONZEY_CABINET_REQUIRE_EXACT_PORT === "1";

if (!Number.isInteger(port) || port <= 0 || port > 65535) {
  console.error("Usage: node scripts/run_web_app.mjs [port]");
  process.exit(2);
}

const contentTypes = new Map([
  [".html", "text/html; charset=utf-8"],
  [".css", "text/css; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
]);

function createStaticServer() {
  return createServer(async (request, response) => {
    const url = new URL(request.url ?? "/", `http://${request.headers.host ?? "localhost"}`);
    const pathname = url.pathname === "/" ? "/index.html" : url.pathname;
    const relative = normalize(pathname).replace(/^(\.\.[/\\])+/, "").replace(/^[/\\]/, "");
    const filePath = join(publicDir, relative);

    try {
      const body = await readFile(filePath);
      response.writeHead(200, {
        "content-type": contentTypes.get(extname(filePath)) ?? "application/octet-stream",
        "cache-control": "no-store",
      });
      response.end(body);
    } catch {
      response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      response.end("Not found");
    }
  });
}

function startServer(candidatePort, attemptsRemaining = 20) {
  const server = createStaticServer();
  server.once("error", (error) => {
    if (
      error.code === "EADDRINUSE" &&
      !requireExactPort &&
      attemptsRemaining > 0 &&
      candidatePort < 65535
    ) {
      const nextPort = candidatePort + 1;
      console.error(`Port ${candidatePort} is unavailable; trying ${nextPort}`);
      startServer(nextPort, attemptsRemaining - 1);
      return;
    }

    if (error.code === "EADDRINUSE" && requireExactPort) {
      console.error(`Port ${candidatePort} is unavailable; exact port is required`);
      process.exit(1);
    }

    console.error(error);
    process.exit(1);
  });

  server.listen(candidatePort, "127.0.0.1", () => {
    const address = server.address();
    const activePort = typeof address === "object" && address ? address.port : candidatePort;
    if (process.env.SPONZEY_CABINET_RUNNER_ANNOUNCED !== "1") {
      console.log(`Sponzey Cabinet web app running at http://127.0.0.1:${activePort}`);
    }
  });
}

startServer(port);
