#!/usr/bin/env sh
set -eu

requested_port="${1:-5173}"

node scripts/build_web_app.mjs

port="$(
  node -e 'const net = require("node:net"); const start = Number.parseInt(process.argv[1], 10); if (!Number.isInteger(start) || start <= 0 || start > 65535) process.exit(2); function probe(port) { const server = net.createServer(); server.once("error", () => { if (port >= 65535) process.exit(3); probe(port + 1); }); server.listen(port, "127.0.0.1", () => { console.log(port); server.close(); }); } probe(start);' "$requested_port"
)"

echo "Sponzey Cabinet web app running at http://127.0.0.1:${port}"
SPONZEY_CABINET_RUNNER_ANNOUNCED=1 node scripts/run_web_app.mjs "$port"
