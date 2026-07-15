#!/usr/bin/env sh
set -eu

cargo test -p cabinet-adapters --test local_asset_store_tests
cargo test -p cabinet-adapters --test local_document_asset_repository_tests
cargo test -p cabinet-adapters --test object_storage_adapter_contract_tests
