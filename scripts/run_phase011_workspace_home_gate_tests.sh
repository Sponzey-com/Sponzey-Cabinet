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

"$NODE_BIN" --test \
  scripts/phase011_workspace_home_visual_tests.mjs \
  scripts/phase011_workspace_home_performance_tests.mjs \
  scripts/phase011_workspace_home_gate_tests.mjs
"$NODE_BIN" --experimental-strip-types --test \
  apps/desktop/tests/desktop_tauri_home_transport_tests.ts \
  apps/desktop/tests/desktop_react_home_render_tests.ts \
  apps/desktop/tests/desktop_personal_workspace_home_tests.ts \
  packages/ui/tests/personal_workspace_home_model_tests.ts
cargo test -p cabinet-platform --test workspace_home_command_executor_tests --quiet
cargo test -p cabinet-desktop-shell --test workspace_home_runtime_tests --quiet
