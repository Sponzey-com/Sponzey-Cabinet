#!/usr/bin/env sh
set -eu

node --test scripts/phase013_final_release_gate_tests.mjs
desktop_tests="$(find apps/desktop/tests -maxdepth 1 -name '*.ts' ! -name 'desktop_remote_product_smoke.ts' -print | sort)"
# Remote server product smoke is explicitly deferred by the Phase 013 local-personal scope.
node --test $desktop_tests
cargo check --workspace --all-targets
sh scripts/run_phase013_action_geometry_baseline.sh
sh scripts/run_phase013_responsive_stress.sh
sh scripts/run_phase013_query_render_performance.sh
sh scripts/run_phase013_packaged_product_gate.sh
node scripts/run_phase013_final_release_gate.mjs
