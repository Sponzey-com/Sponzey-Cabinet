#!/usr/bin/env sh
set -eu

node --test packages/ui/tests/backup_restore_staging_model_tests.ts packages/ui/tests/import_preview_model_tests.ts packages/ui/tests/restore_flow_model_tests.ts
node --test apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts apps/desktop/tests/desktop_import_preview_smoke_tests.ts
