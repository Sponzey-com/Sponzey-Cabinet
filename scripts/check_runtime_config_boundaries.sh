#!/usr/bin/env sh
set -eu

fail() {
  printf '%s\n' "runtime config boundary violation: $1" >&2
  exit 1
}

require_file() {
  if [ ! -f "$1" ]; then
    fail "missing required file: $1"
  fi
}

require_file "crates/cabinet-core/src/config.rs"

if grep -R "std::env\|process::Command\|dotenv\|env!" crates/cabinet-core/src crates/cabinet-usecases/src crates/cabinet-domain/src; then
  fail "core/usecase/domain source must not read external environment directly"
fi

printf '%s\n' "runtime config boundaries ok"
