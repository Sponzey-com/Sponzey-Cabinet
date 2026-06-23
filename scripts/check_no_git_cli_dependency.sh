#!/usr/bin/env sh
set -eu

fail() {
  printf '%s\n' "git cli dependency violation: $1" >&2
  exit 1
}

scan_roots=""
for root in crates apps packages; do
  if [ -d "$root" ]; then
    scan_roots="$scan_roots $root"
  fi
done

if [ -n "$scan_roots" ]; then
  if grep -R -n -E 'std::process::Command|process::Command|Command::new\("git"\)|Command::new\('\''git'\''\)|git2' $scan_roots; then
    fail "source code must not invoke Git CLI or depend on Git implementation libraries"
  fi
fi

printf '%s\n' "git cli dependency check ok"
