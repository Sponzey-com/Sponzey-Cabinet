#!/usr/bin/env sh
set -eu

fail() {
  printf '%s\n' "logging boundary violation: $1" >&2
  exit 1
}

require_file() {
  if [ ! -f "$1" ]; then
    fail "missing required file: $1"
  fi
}

require_file "crates/cabinet-core/src/logging.rs"

if grep -R "std::fs\|tokio::fs\|async_std::fs\|File::\|process::Command\|TcpStream\|UdpSocket\|reqwest\|ureq\|std::env\|env!" crates/cabinet-core/src/logging.rs; then
  fail "logging core must define event models and ports, not perform I/O or environment access"
fi

printf '%s\n' "logging boundaries ok"
