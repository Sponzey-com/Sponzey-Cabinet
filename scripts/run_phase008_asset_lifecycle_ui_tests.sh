#!/usr/bin/env sh
set -eu

node --test apps/desktop/tests/desktop_discovery_smoke_tests.ts
node --test apps/desktop/tests/desktop_import_preview_smoke_tests.ts
node --test apps/desktop/tests/desktop_backup_restore_staging_smoke_tests.ts
