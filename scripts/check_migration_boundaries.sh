#!/usr/bin/env sh
set -eu

fail() {
  printf '%s\n' "migration boundary violation: $1" >&2
  exit 1
}

require_file() {
  if [ ! -f "$1" ]; then
    fail "missing required file: $1"
  fi
}

require_file "crates/cabinet-core/src/migration.rs"

if grep -R "std::fs\|tokio::fs\|async_std::fs\|File::\|process::Command\|TcpStream\|UdpSocket\|reqwest\|ureq\|std::env\|env!" crates/cabinet-core/src/migration.rs; then
  fail "migration core must use ports and pure transitions, not perform I/O or environment access"
fi

printf '%s\n' "migration boundaries ok"
