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

"$NODE_BIN" --test scripts/phase011_data_settings_gate_tests.mjs
"$NODE_BIN" --experimental-strip-types --test \
  packages/ui/tests/data_ownership_settings_model_tests.ts \
  packages/ui/tests/backup_restore_staging_model_tests.ts \
  packages/ui/tests/import_preview_model_tests.ts \
  packages/ui/tests/ai_citation_tool_scope_model_tests.ts \
  apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts \
  apps/desktop/tests/desktop_import_preview_smoke_tests.ts

cargo test -p cabinet-domain --test field_debug_tests
cargo test -p cabinet-usecases --test field_debug_usecase_tests --test backup_usecase_tests --test import_markdown_folder_tests
