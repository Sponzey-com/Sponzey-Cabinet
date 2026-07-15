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

"$NODE_BIN" --test scripts/phase011_recovery_observability_gate_tests.mjs
"$NODE_BIN" --experimental-strip-types --test \
  packages/ui/tests/recovery_observability_model_tests.ts \
  packages/ui/tests/revision_safe_save_coordinator_tests.ts \
  apps/desktop/tests/desktop_document_authoring_controller_tests.ts

cargo test -p cabinet-platform --test startup_repair_smoke
cargo test -p cabinet-usecases --test backup_usecase_tests --test import_markdown_folder_tests --test field_debug_usecase_tests --test guarded_authoring_tests
