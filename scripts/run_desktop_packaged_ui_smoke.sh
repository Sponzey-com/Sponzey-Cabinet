#!/usr/bin/env sh
set -eu

if [ "${1:-}" != "--reuse-existing" ]; then
  scripts/run_desktop_tauri_build.sh
fi

app_binary="target/debug/bundle/macos/Sponzey Cabinet.app/Contents/MacOS/cabinet-desktop-shell"

if [ ! -f "$app_binary" ]; then
  app_binary="$(
    find apps/desktop/src-tauri/target/debug/bundle/macos \
      -path '*/Sponzey Cabinet.app/Contents/MacOS/*' \
      -type f 2>/dev/null | sort | head -n 1
  )"
fi

if [ -z "$app_binary" ]; then
  echo "phase012_packaged_ui_smoke=failed"
  echo "error_code=PHASE012_PACKAGED_UI_BINARY_MISSING"
  exit 1
fi

node scripts/desktop_packaged_ui_smoke.mjs "$app_binary"
