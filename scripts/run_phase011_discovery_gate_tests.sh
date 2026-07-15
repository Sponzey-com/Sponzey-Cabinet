#!/usr/bin/env sh
set -eu

NODE_BIN="${NODE_BIN:-}"
if [ -z "$NODE_BIN" ]; then
  if command -v /usr/local/bin/node >/dev/null 2>&1; then
    NODE_BIN="/usr/local/bin/node"
  else
    NODE_BIN="node"
  fi
fi

"$NODE_BIN" --test scripts/phase011_discovery_gate_tests.mjs
"$NODE_BIN" --experimental-strip-types --test \
  packages/ui/tests/local_discovery_panel_model_tests.ts \
  packages/ui/tests/graph_canvas_panel_model_tests.ts \
  apps/desktop/tests/desktop_discovery_smoke_tests.ts

cargo test -p cabinet-usecases --test search_documents_tests --test graph_lite_projection_tests --test list_document_assets_tests
cargo test -p cabinet-adapters --test local_search_index_tests --test local_link_index_tests --test local_graph_projection_store_tests --test local_document_asset_repository_tests
