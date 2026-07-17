#!/usr/bin/env sh
set -eu

node --test \
  scripts/phase013_final_release_gate_tests.mjs \
  scripts/phase014_completion_evidence_gate_tests.mjs
# Remote server product smoke is explicitly deferred by the Phase 013 local-personal scope.
node scripts/run_phase014_desktop_test_gate.mjs
node scripts/run_phase014_rust_test_gate.mjs
sh scripts/run_phase013_action_geometry_baseline.sh
sh scripts/run_phase013_responsive_stress.sh
sh scripts/run_phase013_query_render_performance.sh
sh scripts/run_phase013_packaged_product_gate.sh
node scripts/run_phase013_final_release_gate.mjs
node scripts/run_phase014_current_scope_audit.mjs
node scripts/run_phase014_completion_evidence_gate.mjs
