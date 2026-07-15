#!/usr/bin/env sh
set -eu

node --test scripts/phase008_native_bootstrap_gate_tests.mjs
npm run run:phase008-native-bootstrap-contract-tests
cargo test -p cabinet-adapters --test local_first_run_store_tests
cargo test -p cabinet-adapters --test local_setup_health_checker_tests
