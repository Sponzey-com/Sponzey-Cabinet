//! Pure domain model boundary for Sponzey Cabinet.

pub mod asset;
pub mod document;
pub mod link;
pub mod version;
pub mod workspace;

/// Returns the architectural layer name for smoke tests and diagnostics.
pub const fn layer_name() -> &'static str {
    "domain"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_layer_name_is_stable() {
        assert_eq!(layer_name(), "domain");
    }
}
