//! Pure domain model boundary for Sponzey Cabinet.

pub mod ai;
pub mod asset;
pub mod asset_import_operation;
pub mod attachment_operation;
pub mod attachment_snapshot_mutation;
pub mod audit;
pub mod backup;
pub mod canvas;
pub mod collaboration;
pub mod comment;
pub mod connector;
pub mod document;
pub mod document_diff_operation;
pub mod document_diff_query;
pub mod document_lock;
pub mod document_revision;
pub mod embedding;
pub mod field_debug;
pub mod graph;
pub mod group;
pub mod link;
pub mod notification;
pub mod permission;
pub mod projection_repair;
pub mod projection_work;
pub mod realtime;
pub mod retrieval;
pub mod session;
pub mod tool;
pub mod user;
pub mod version;
pub mod webhook;
pub mod workflow;
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
