#!/usr/bin/env sh
set -eu

node --test scripts/phase008_command_bridge_gate_tests.mjs
npm run run:phase008-command-bridge-contract-tests
cargo test -p cabinet-desktop-shell local_desktop_command
