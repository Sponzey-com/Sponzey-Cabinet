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

sh scripts/run_desktop_package_smoke.sh
"$NODE_BIN" scripts/phase011_native_platform_evidence.mjs
