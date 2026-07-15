#!/usr/bin/env sh
set -eu

cargo test -p cabinet-usecases --test get_current_document_tests
cargo test -p cabinet-usecases --test get_document_history_tests
cargo test -p cabinet-usecases --test update_document_tests
cargo test -p cabinet-usecases --test preview_document_restore_tests
cargo test -p cabinet-usecases --test restore_document_version_tests
