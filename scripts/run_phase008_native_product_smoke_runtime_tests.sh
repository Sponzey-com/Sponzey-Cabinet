#!/usr/bin/env sh
set -eu

cargo test -p cabinet-platform --test clean_install_smoke
cargo test -p cabinet-platform --test mvp_end_to_end_smoke
cargo test -p cabinet-platform --test data_preservation_smoke
cargo test -p cabinet-platform --test startup_repair_smoke
