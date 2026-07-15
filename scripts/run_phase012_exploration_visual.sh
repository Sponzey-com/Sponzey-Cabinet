#!/usr/bin/env sh
set -eu
node scripts/build_desktop_assets.mjs
node scripts/run_phase012_exploration_visual.mjs
