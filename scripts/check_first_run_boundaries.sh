#!/usr/bin/env sh
set -eu

fail() {
  printf '%s\n' "first-run boundary violation: $1" >&2
  exit 1
}

require_file() {
  if [ ! -f "$1" ]; then
    fail "missing required file: $1"
  fi
}

require_file "crates/cabinet-core/src/first_run.rs"

if grep -R "std::fs\|tokio::fs\|async_std::fs\|File::\|process::Command\|TcpStream\|UdpSocket\|reqwest\|ureq" crates/cabinet-core/src/first_run.rs; then
  fail "first-run core must return plans and transitions, not perform I/O"
fi

printf '%s\n' "first-run boundaries ok"
