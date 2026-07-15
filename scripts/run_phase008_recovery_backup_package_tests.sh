#!/usr/bin/env sh
set -eu

cargo test -p cabinet-domain --test backup_job_tests
cargo test -p cabinet-usecases --test backup_usecase_tests
cargo test -p cabinet-usecases --test import_markdown_folder_tests
cargo test -p cabinet-usecases --test export_markdown_tests
cargo test -p cabinet-usecases --test preview_document_restore_tests
cargo test -p cabinet-usecases --test restore_document_version_tests
cargo test -p cabinet-adapters --test local_backup_store_tests
