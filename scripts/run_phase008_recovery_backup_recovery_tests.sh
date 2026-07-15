#!/usr/bin/env sh
set -eu

cargo test -p cabinet-core --test migration_tests
cargo test -p cabinet-platform --test startup_repair_smoke
cargo test -p cabinet-platform --test data_preservation_smoke
cargo test -p cabinet-platform --test phase002_migration_fixture_smoke
cargo test -p cabinet-adapters --test local_setup_health_checker_tests
cargo test -p cabinet-adapters --test local_migration_store_tests
cargo test -p cabinet-adapters --test local_phase002_migration_fixture_tests
