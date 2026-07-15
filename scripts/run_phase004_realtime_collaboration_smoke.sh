#!/bin/sh
set -eu

node --test packages/client-core/tests/realtime_client_tests.ts
cargo test -p cabinet-domain --test realtime_tests
cargo test -p cabinet-ports --test realtime_contract_tests
cargo test -p cabinet-adapters --test local_realtime_adapter_tests
cargo test -p cabinet-server --test collaboration_realtime_command_mapper_tests
cargo test -p cabinet-server --test collaboration_realtime_executor_tests
cargo test -p cabinet-server --test collaboration_realtime_runtime_target_tests
cargo test -p cabinet-server --test split_realtime_server_target_tests

cat > .tasks/realtime-collaboration-smoke-result.md <<'RESULT'
# Realtime Collaboration Smoke Result

phase004_realtime_collaboration_smoke=passed
client_realtime_contract=passed
domain_realtime_state_machine=passed
ports_realtime_contract=passed
local_realtime_adapter=passed
server_command_mapper=passed
server_command_executor=passed
server_runtime_target=passed
server_split_dispatch=passed
RESULT

cat .tasks/realtime-collaboration-smoke-result.md
