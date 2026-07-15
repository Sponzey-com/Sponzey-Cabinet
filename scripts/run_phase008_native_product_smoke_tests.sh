#!/usr/bin/env sh
set -eu

node --test scripts/phase008_native_product_smoke_gate_tests.mjs
npm run run:phase008-native-product-smoke-runtime-tests
npm run run:phase008-native-product-smoke-ui-tests
