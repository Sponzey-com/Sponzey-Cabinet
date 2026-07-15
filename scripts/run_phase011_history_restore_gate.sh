#!/usr/bin/env sh
set -eu

NODE_BIN="${NODE_BIN:-/usr/local/bin/node}"
if [ ! -x "$NODE_BIN" ]; then
  NODE_BIN="node"
fi

sh scripts/run_phase011_history_restore_gate_tests.sh
"$NODE_BIN" scripts/phase011_history_restore_gate.mjs
