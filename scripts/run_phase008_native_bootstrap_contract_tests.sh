#!/usr/bin/env sh
set -eu

cargo test -p cabinet-core --test local_desktop_config_tests
cargo test -p cabinet-core --test config_tests
cargo test -p cabinet-platform --test local_desktop_bootstrap_state_tests
