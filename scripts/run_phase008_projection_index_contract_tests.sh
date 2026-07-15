#!/usr/bin/env sh
set -eu

cargo test -p cabinet-usecases --test search_documents_tests
cargo test -p cabinet-usecases --test graph_lite_projection_tests
cargo test -p cabinet-usecases --test permission_aware_graph_tests
cargo test -p cabinet-usecases --test list_document_assets_tests
cargo test -p cabinet-adapters --test local_search_index_tests
cargo test -p cabinet-adapters --test local_link_index_tests
cargo test -p cabinet-adapters --test local_graph_projection_store_tests
cargo test -p cabinet-adapters --test local_markdown_parser_tests
cargo test -p cabinet-adapters --test local_document_asset_repository_tests
