#!/usr/bin/env sh
set -eu

node --test packages/client-core/tests/local_desktop_command_client_tests.ts
node --test apps/desktop/tests/desktop_local_command_facade_tests.ts
node --test apps/desktop/tests/desktop_local_persistence_flow_tests.ts
