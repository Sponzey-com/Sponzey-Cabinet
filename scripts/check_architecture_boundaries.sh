#!/usr/bin/env sh
set -eu

fail() {
  printf '%s\n' "architecture boundary violation: $1" >&2
  exit 1
}

require_file() {
  if [ ! -f "$1" ]; then
    fail "missing required file: $1"
  fi
}

require_dir() {
  if [ ! -d "$1" ]; then
    fail "missing required directory: $1"
  fi
}

require_file "Cargo.toml"

for crate in cabinet-domain cabinet-ports cabinet-usecases cabinet-core cabinet-adapters cabinet-platform; do
  require_dir "crates/$crate"
  require_file "crates/$crate/Cargo.toml"
  require_file "crates/$crate/src/lib.rs"
done

domain_manifest="crates/cabinet-domain/Cargo.toml"
if grep -Eq 'cabinet-(ports|usecases|core|adapters|platform)|tauri|react|codemirror|tokio|reqwest|sqlx|diesel' "$domain_manifest"; then
  fail "cabinet-domain depends on an outer layer or framework"
fi

usecases_manifest="crates/cabinet-usecases/Cargo.toml"
if grep -Eq 'cabinet-(core|adapters|platform)|tauri|react|codemirror|reqwest|sqlx|diesel' "$usecases_manifest"; then
  fail "cabinet-usecases depends on an adapter, platform, or framework"
fi

ports_manifest="crates/cabinet-ports/Cargo.toml"
if grep -Eq 'cabinet-(usecases|core|adapters|platform)|tauri|react|codemirror|reqwest|sqlx|diesel' "$ports_manifest"; then
  fail "cabinet-ports depends on an outer layer or framework"
fi

printf '%s\n' "architecture boundaries ok"
