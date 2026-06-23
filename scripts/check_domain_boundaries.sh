#!/usr/bin/env sh
set -eu

fail() {
  printf '%s\n' "domain boundary violation: $1" >&2
  exit 1
}

require_dir() {
  if [ ! -d "$1" ]; then
    fail "missing required directory: $1"
  fi
}

require_dir "crates/cabinet-domain/src"

if grep -R "std::fs\|tokio::fs\|async_std::fs\|File::\|process::Command\|TcpStream\|UdpSocket\|reqwest\|ureq\|std::env\|env!\|sqlx\|diesel\|tauri\|codemirror\|react\|Logger\|logger" crates/cabinet-domain/src; then
  fail "domain source must not depend on I/O, environment, framework, editor, DB, or logger APIs"
fi

printf '%s\n' "domain boundaries ok"
