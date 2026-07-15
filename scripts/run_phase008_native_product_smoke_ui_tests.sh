#!/usr/bin/env sh
set -eu

node --test \
  apps/desktop/tests/desktop_personal_workspace_shell_tests.ts \
  apps/desktop/tests/desktop_local_persistence_flow_tests.ts \
  apps/desktop/tests/desktop_document_authoring_smoke_tests.ts \
  apps/desktop/tests/desktop_document_ux_smoke_tests.ts \
  apps/desktop/tests/desktop_discovery_smoke_tests.ts \
  apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts \
  apps/desktop/tests/desktop_import_preview_smoke_tests.ts
