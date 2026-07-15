#!/usr/bin/env sh
set -eu

node scripts/build_desktop_assets.mjs
node scripts/run_phase013_action_geometry_baseline.mjs
