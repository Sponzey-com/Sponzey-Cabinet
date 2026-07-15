#!/usr/bin/env sh
set -eu

cargo run --quiet -p cabinet-platform --bin cabinet-local -- "$@"
