#!/usr/bin/env sh
set -eu

fail() {
  printf '%s\n' "desktop shell boundary violation: $1" >&2
  exit 1
}

require_file() {
  if [ ! -f "$1" ]; then
    fail "missing required file: $1"
  fi
}

require_file "apps/desktop/src-tauri/Cargo.toml"
require_file "apps/desktop/src-tauri/src/lib.rs"
require_file "apps/desktop/src-tauri/tauri.conf.json"

manifest="apps/desktop/src-tauri/Cargo.toml"
source="apps/desktop/src-tauri/src/lib.rs"

if ! grep -q 'cabinet-platform' "$manifest"; then
  fail "desktop shell must depend on cabinet-platform boundary"
fi

if grep -Eq 'cabinet-(domain|ports|usecases|adapters|core)' "$manifest"; then
  fail "desktop shell must not directly depend on inner crates except cabinet-platform"
fi

if grep -Eq 'std::(fs|env|net|process)|tokio|reqwest|sqlx|diesel' "$source"; then
  fail "desktop shell source contains direct I/O, environment, network, or DB access"
fi

if grep -Eq 'DocumentLifecycle|VersionStore|SearchIndex|AssetStore|Repository' "$source"; then
  fail "desktop shell source contains business or infrastructure rule names"
fi

printf '%s\n' "desktop shell boundaries ok"
