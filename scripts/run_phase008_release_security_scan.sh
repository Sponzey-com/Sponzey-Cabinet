#!/usr/bin/env sh
set -eu

node scripts/security_log_scanner.mjs .tasks/release/security-log-policy-manifest.json
