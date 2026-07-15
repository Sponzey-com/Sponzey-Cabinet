#!/usr/bin/env sh
set -eu

node --test scripts/phase013_packaged_product_gate_tests.mjs scripts/desktop_packaged_ui_smoke_tests.mjs
