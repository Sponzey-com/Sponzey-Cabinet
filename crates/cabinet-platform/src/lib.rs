//! Platform shell boundary for desktop, web server, and future mobile adapters.

pub mod release_smoke;

/// Returns the architectural layer name for smoke tests and diagnostics.
pub const fn layer_name() -> &'static str {
    "platform"
}

/// Smoke function proving platform code stays outside core application layers.
pub fn outer_layers() -> (&'static str, &'static str, &'static str) {
    (
        cabinet_adapters::layer_name(),
        cabinet_core::layer_name(),
        cabinet_usecases::layer_name(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_layer_references_outer_composition_boundaries() {
        assert_eq!(layer_name(), "platform");
        assert_eq!(outer_layers(), ("adapters", "core", "usecases"));
    }
}
