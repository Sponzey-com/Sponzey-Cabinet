#!/usr/bin/env sh
set -eu

node --test scripts/phase008_release_gate_tests.mjs
node --test scripts/security_log_scanner_tests.mjs
node --test scripts/runbook_validator_tests.mjs
npm run run:phase008-release-security-scan
npm run run:phase008-release-runbook-validation
