#!/usr/bin/env sh
set -eu

NODE_BIN="${NODE_BIN:-/usr/local/bin/node}"
if [ ! -x "$NODE_BIN" ]; then
  NODE_BIN="node"
fi

"$NODE_BIN" --test scripts/phase011_history_restore_gate_tests.mjs
"$NODE_BIN" --experimental-strip-types --test \
  packages/client-core/tests/local_desktop_command_client_tests.ts \
  packages/ui/tests/restore_flow_model_tests.ts \
  apps/desktop/tests/desktop_document_ux_smoke_tests.ts \
  apps/desktop/tests/desktop_tauri_authoring_transport_tests.ts
cargo test -p cabinet-desktop-shell --test document_authoring_runtime_tests
