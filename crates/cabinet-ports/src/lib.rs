//! Port interfaces consumed by usecases and implemented by adapters.

pub mod asset_store;
pub mod document_asset_repository;
pub mod document_repository;
pub mod html_renderer;
pub mod link_index;
pub mod markdown_parser;
pub mod search_index;
pub mod version_store;
pub mod workspace_repository;

/// Returns the architectural layer name for smoke tests and diagnostics.
pub const fn layer_name() -> &'static str {
    "ports"
}

/// Smoke function proving this layer can depend inward on domain.
pub fn domain_layer_name() -> &'static str {
    cabinet_domain::layer_name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ports_layer_can_reference_domain_boundary() {
        assert_eq!(layer_name(), "ports");
        assert_eq!(domain_layer_name(), "domain");
    }
}
