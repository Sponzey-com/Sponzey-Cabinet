#!/usr/bin/env sh
set -eu

cargo test -p cabinet-adapters --test local_document_repository_tests
cargo test -p cabinet-adapters --test local_version_store_tests
