#!/usr/bin/env sh
set -eu

cargo run --quiet -p cabinet-server --bin cabinet-server -- "$@"
