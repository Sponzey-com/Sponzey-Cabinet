#!/usr/bin/env sh
set -eu

node --test scripts/phase008_document_runtime_gate_tests.mjs
npm run run:phase008-document-runtime-usecase-tests
npm run run:phase008-document-runtime-adapter-tests
node --test apps/desktop/tests/desktop_local_persistence_flow_tests.ts
