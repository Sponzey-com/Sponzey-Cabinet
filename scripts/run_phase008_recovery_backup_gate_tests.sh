#!/usr/bin/env sh
set -eu

node --test scripts/phase008_recovery_backup_gate_tests.mjs
npm run run:phase008-recovery-backup-recovery-tests
npm run run:phase008-recovery-backup-package-tests
npm run run:phase008-recovery-backup-ui-tests
