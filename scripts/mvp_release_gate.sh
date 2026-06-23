#!/usr/bin/env sh
set -eu

cargo fmt --all --check
cargo test --workspace --quiet

sh scripts/check_architecture_boundaries.sh
sh scripts/check_no_git_cli_dependency.sh
sh scripts/check_runtime_config_boundaries.sh
sh scripts/check_first_run_boundaries.sh
sh scripts/check_migration_boundaries.sh
sh scripts/check_logging_boundaries.sh
sh scripts/check_domain_boundaries.sh

node scripts/check_ui_shell.mjs
node scripts/check_editor_integration.mjs
node scripts/check_wikilink_editor.mjs
node scripts/check_attachment_editor.mjs
node scripts/check_current_history_ui.mjs
node scripts/check_search_ui.mjs
node scripts/check_link_ui.mjs
node scripts/check_asset_ui.mjs
node scripts/check_platform_adapter_smoke.mjs
node scripts/check_frontend_boundaries.mjs
sh scripts/check_desktop_shell_boundaries.sh

sh scripts/check_mvp_release_docs.sh

echo "mvp release gate ok"
