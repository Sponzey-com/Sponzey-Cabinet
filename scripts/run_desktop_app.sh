#!/usr/bin/env sh
set -eu

dev_port="5173"

NODE_BIN="${NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  if command -v /usr/local/bin/node >/dev/null 2>&1; then
    NODE_BIN="/usr/local/bin/node"
  else
    NODE_BIN="node"
  fi
fi

"$NODE_BIN" scripts/build_desktop_assets.mjs

SPONZEY_CABINET_WEB_PUBLIC_DIR=apps/desktop/dist \
SPONZEY_CABINET_RUNNER_ANNOUNCED=1 \
SPONZEY_CABINET_REQUIRE_EXACT_PORT=1 \
"$NODE_BIN" scripts/run_web_app.mjs "$dev_port" &
web_pid="$!"

cleanup() {
  kill "$web_pid" 2>/dev/null || true
  wait "$web_pid" 2>/dev/null || true
}

trap cleanup EXIT INT TERM

"$NODE_BIN" -e '
const url = process.argv[1];
const deadline = Date.now() + 10000;
async function waitForServer() {
  let lastError;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url, { cache: "no-store" });
      if (response.ok) return;
      lastError = new Error(`HTTP ${response.status}`);
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw lastError ?? new Error(`Timed out waiting for ${url}`);
}
waitForServer().catch((error) => {
  console.error(error.message);
  process.exit(1);
});
' "http://127.0.0.1:${dev_port}/"

echo "Launching Sponzey Cabinet desktop app with UI server at http://127.0.0.1:${dev_port}"
cargo run -p cabinet-desktop-shell
