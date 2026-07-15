#!/usr/bin/env sh
set -eu

node --test scripts/phase008_asset_lifecycle_gate_tests.mjs
npm run run:phase008-asset-lifecycle-domain-usecase-tests
npm run run:phase008-asset-lifecycle-adapter-tests
npm run run:phase008-asset-lifecycle-ui-tests
