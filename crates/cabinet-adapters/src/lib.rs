//! Adapter implementations for external boundaries.

pub mod local_asset_store;
pub mod local_atomic_file;
pub mod local_document_asset_repository;
pub mod local_document_repository;
pub mod local_first_run;
pub mod local_html_renderer;
pub mod local_link_index;
pub mod local_markdown_parser;
pub mod local_migration;
pub mod local_search_index;
pub mod local_setup_health;
pub mod local_version_store;

/// Returns the architectural layer name for smoke tests and diagnostics.
pub const fn layer_name() -> &'static str {
    "adapters"
}

/// Smoke function proving adapters can depend on port contracts.
pub fn implemented_contract_layers() -> (&'static str, &'static str, &'static str) {
    (
        cabinet_domain::layer_name(),
        cabinet_ports::layer_name(),
        cabinet_core::layer_name(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapters_layer_references_domain_and_ports_only_for_now() {
        assert_eq!(layer_name(), "adapters");
        assert_eq!(implemented_contract_layers(), ("domain", "ports", "core"));
    }
}
