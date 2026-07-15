#!/usr/bin/env sh
set -eu

NODE_BIN="${NODE_BIN:-/usr/local/bin/node}"
if [ ! -x "$NODE_BIN" ]; then
  NODE_BIN="node"
fi

"$NODE_BIN" scripts/build_desktop_assets.mjs
"$NODE_BIN" scripts/run_phase011_authoring_browser.mjs
