#!/usr/bin/env sh
set -eu

node scripts/build_desktop_assets.mjs
node scripts/run_phase013_responsive_stress.mjs
