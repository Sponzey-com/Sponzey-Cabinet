#!/bin/sh
set -eu

cargo test -p cabinet-domain --test canvas_tests
cargo test -p cabinet-usecases --test canvas_usecase_tests
cargo test -p cabinet-ports --test canvas_repository_contract_tests
cargo test -p cabinet-adapters --test local_canvas_repository_tests
cargo test -p cabinet-server --test canvas_runtime_tests
node --test packages/client-core/tests/canvas_client_tests.ts
node --test apps/web/tests/web_canvas_model_tests.ts

cat > .tasks/canvas-product-smoke-result.md <<'RESULT'
# Canvas Product Smoke Result

phase004_canvas_product_smoke=passed
canvas_domain_contract=passed
canvas_usecase_contract=passed
canvas_repository_contract=passed
canvas_runtime_route_contract=passed
canvas_client_contract=passed
canvas_web_model_contract=passed
canvas_relation_graph_projection=passed
RESULT

cat .tasks/canvas-product-smoke-result.md
