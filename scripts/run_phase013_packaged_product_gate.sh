#!/usr/bin/env sh
set -eu

scripts/run_desktop_tauri_build.sh

app_binary="target/debug/bundle/macos/Sponzey Cabinet.app/Contents/MacOS/cabinet-desktop-shell"
if [ ! -f "$app_binary" ]; then
  app_binary="$(find apps/desktop/src-tauri/target/debug/bundle/macos -path '*/Sponzey Cabinet.app/Contents/MacOS/*' -type f 2>/dev/null | sort | head -n 1)"
fi
if [ -z "$app_binary" ] || [ ! -f "$app_binary" ]; then
  echo "phase013_packaged_product_gate=failed"
  echo "error_code=PACKAGED_BINARY_MISSING"
  exit 1
fi

node scripts/run_phase013_packaged_product_gate.mjs "$app_binary"
