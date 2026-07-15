//! Self-host server boundary.

pub mod adapter;
pub mod auth;
pub mod bootstrap;
pub mod collaboration_realtime;
pub mod composition;
pub mod e2e_http;
pub mod errors;
pub mod health;
pub mod package_smoke;
pub mod runtime;

/// Returns the architectural layer name for smoke tests and diagnostics.
pub const fn layer_name() -> &'static str {
    "server"
}

/// Returns the inward application boundary used by this server shell.
pub fn inward_layer_name() -> &'static str {
    cabinet_core::layer_name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_boundary_depends_on_core_boundary() {
        assert_eq!(layer_name(), "server");
        assert_eq!(inward_layer_name(), "core");
    }
}
