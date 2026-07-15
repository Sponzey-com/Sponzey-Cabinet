#!/usr/bin/env sh
set -eu

node --test scripts/phase008_projection_index_gate_tests.mjs
npm run run:phase008-projection-index-contract-tests
cargo test -p cabinet-platform --test query_performance_benchmarks
node --test apps/desktop/tests/desktop_discovery_smoke_tests.ts
