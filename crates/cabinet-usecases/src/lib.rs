//! Application usecase boundary.

pub mod document;
pub mod export;
pub mod graph;
pub mod import;
pub mod search;
pub mod workspace;

/// Returns the architectural layer name for smoke tests and diagnostics.
pub const fn layer_name() -> &'static str {
    "usecases"
}

/// Smoke function proving this layer depends only on inward contracts.
pub fn inward_layers() -> (&'static str, &'static str) {
    (cabinet_domain::layer_name(), cabinet_ports::layer_name())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usecases_layer_depends_on_domain_and_ports() {
        assert_eq!(layer_name(), "usecases");
        assert_eq!(inward_layers(), ("domain", "ports"));
    }
}
