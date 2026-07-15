#!/usr/bin/env sh
set -eu

NODE_BIN="${NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  if command -v /usr/local/bin/node >/dev/null 2>&1; then
    NODE_BIN="/usr/local/bin/node"
  else
    NODE_BIN="node"
  fi
fi

"$NODE_BIN" scripts/build_desktop_assets.mjs
cd apps/desktop
../../node_modules/.bin/tauri build --debug --bundles app --no-sign --ci
