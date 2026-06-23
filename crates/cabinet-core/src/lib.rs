//! Composition-neutral core boundary shared by local app and server entrypoints.

pub mod config;
pub mod first_run;
pub mod logging;
pub mod migration;
pub mod performance;

/// Returns the architectural layer name for smoke tests and diagnostics.
pub const fn layer_name() -> &'static str {
    "core"
}

/// Smoke function proving core can see inner application layers.
pub fn application_layers() -> (&'static str, &'static str, &'static str) {
    (
        cabinet_domain::layer_name(),
        cabinet_ports::layer_name(),
        cabinet_usecases::layer_name(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_layer_can_reference_application_boundaries() {
        assert_eq!(layer_name(), "core");
        assert_eq!(application_layers(), ("domain", "ports", "usecases"));
    }
}
