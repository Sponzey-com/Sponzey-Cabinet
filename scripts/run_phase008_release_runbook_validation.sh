#!/usr/bin/env sh
set -eu

node scripts/runbook_validator.mjs .tasks/release/runbook-validation-manifest.json
