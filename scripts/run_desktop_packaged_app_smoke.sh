#!/usr/bin/env sh
set -eu

scripts/run_desktop_tauri_build.sh

app_binary="$(
  find target/debug/bundle/macos apps/desktop/src-tauri/target/debug/bundle/macos \
    -path '*/Sponzey Cabinet.app/Contents/MacOS/*' \
    -type f 2>/dev/null | sort | head -n 1
)"

if [ -z "$app_binary" ]; then
  echo "packaged_app_binary_found=false"
  exit 1
fi

echo "packaged_app_binary_found=true"
echo "packaged_app_binary=$app_binary"
"$app_binary" --packaged-smoke
