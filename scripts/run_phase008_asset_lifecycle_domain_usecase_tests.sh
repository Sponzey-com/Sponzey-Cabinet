#!/usr/bin/env sh
set -eu

cargo test -p cabinet-domain --test asset_tests
cargo test -p cabinet-domain --test asset_lifecycle_tests
cargo test -p cabinet-usecases --test attach_file_to_document_tests
cargo test -p cabinet-usecases --test list_document_assets_tests
