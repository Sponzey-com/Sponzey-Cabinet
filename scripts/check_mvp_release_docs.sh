#!/usr/bin/env sh
set -eu

require_file() {
  if [ ! -f "$1" ]; then
    echo "missing required file: $1" >&2
    exit 1
  fi
}

require_text() {
  file="$1"
  pattern="$2"
  if ! rg -q "$pattern" "$file"; then
    echo "missing required text in $file: $pattern" >&2
    exit 1
  fi
}

require_file "MVP_RELEASE.md"
require_file "scripts/mvp_release_gate.sh"

require_text "MVP_RELEASE.md" "Local Data Location"
require_text "MVP_RELEASE.md" "Backup and Export Policy"
require_text "MVP_RELEASE.md" "Known Limitations"
require_text "MVP_RELEASE.md" "Developer Release Gate"
require_text "MVP_RELEASE.md" "Performance and Reliability Evidence"
require_text "MVP_RELEASE.md" "Product Log"
require_text "MVP_RELEASE.md" "Field Debug Log"
require_text "MVP_RELEASE.md" "Development Log"
require_text "MVP_RELEASE.md" "first-run"
require_text "MVP_RELEASE.md" "migration"
require_text "MVP_RELEASE.md" "restore"
require_text "MVP_RELEASE.md" "Git CLI"
require_text "MVP_RELEASE.md" "Node.js"

require_text "scripts/mvp_release_gate.sh" "cargo fmt --all --check"
require_text "scripts/mvp_release_gate.sh" "cargo test --workspace --quiet"
require_text "scripts/mvp_release_gate.sh" "check_architecture_boundaries.sh"
require_text "scripts/mvp_release_gate.sh" "check_no_git_cli_dependency.sh"
require_text "scripts/mvp_release_gate.sh" "check_frontend_boundaries.mjs"
require_text "scripts/mvp_release_gate.sh" "check_platform_adapter_smoke.mjs"
require_text "scripts/mvp_release_gate.sh" "check_desktop_shell_boundaries.sh"

echo "mvp release docs ok"
