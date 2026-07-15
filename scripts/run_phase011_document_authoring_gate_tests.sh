#!/usr/bin/env sh
set -eu

NODE_BIN="${NODE_BIN:-/usr/local/bin/node}"
if [ ! -x "$NODE_BIN" ]; then
  NODE_BIN="node"
fi

"$NODE_BIN" --test \
  scripts/phase011_authoring_browser_tests.mjs \
  scripts/phase011_document_authoring_gate_tests.mjs
"$NODE_BIN" --experimental-strip-types --test \
  apps/desktop/tests/desktop_entry_authoring_contract_tests.ts \
  apps/desktop/tests/desktop_react_home_render_tests.ts \
  apps/desktop/tests/desktop_react_navigator_render_tests.ts \
  apps/desktop/tests/desktop_tauri_authoring_transport_tests.ts \
  apps/desktop/tests/desktop_document_authoring_controller_tests.ts \
  apps/desktop/tests/desktop_react_authoring_workbench_tests.ts \
  packages/client-core/tests/document_authoring_command_client_tests.ts \
  packages/ui/tests/revision_safe_save_coordinator_tests.ts
cargo test -p cabinet-platform --test document_authoring_command_executor_tests --quiet
cargo test -p cabinet-desktop-shell --test document_authoring_runtime_tests --quiet
