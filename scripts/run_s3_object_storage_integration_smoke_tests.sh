#!/usr/bin/env sh
set -eu

node --test scripts/s3_object_storage_integration_smoke_tests.mjs
