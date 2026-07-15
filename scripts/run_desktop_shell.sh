#!/usr/bin/env sh
set -eu

mode="${1:-smoke}"

case "$mode" in
  smoke)
    echo "Running Sponzey Cabinet internal desktop shell smoke..."
    echo "Product UI launcher: scripts/run_desktop_app.sh"
    command="${2:-open_workspace}"
    cargo run --quiet -p cabinet-desktop-shell -- --shell-smoke "$command"
    ;;
  web)
    port="${SPONZEY_CABINET_DESKTOP_PORT:-5174}"
    echo "Starting Sponzey Cabinet browser preview..."
    exec scripts/run_web_app.sh "$port"
    ;;
  *)
    echo "Usage: scripts/run_desktop_shell.sh [smoke|web]" >&2
    echo "Product UI launcher: scripts/run_desktop_app.sh" >&2
    exit 2
    ;;
esac
