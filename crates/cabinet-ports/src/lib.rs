//! Port interfaces consumed by usecases and implemented by adapters.

pub mod ai;
pub mod asset_association_catalog;
pub mod asset_availability;
pub mod asset_external_open;
pub mod asset_import_operation_repository;
pub mod asset_import_source;
pub mod asset_metadata_catalog;
pub mod asset_object_publisher;
pub mod asset_preview;
pub mod asset_search_index;
pub mod asset_staging;
pub mod asset_store;
pub mod audit_log;
pub mod auth;
pub mod backup_catalog;
pub mod backup_package;
pub mod backup_restore;
pub mod backup_store;
pub mod canvas_catalog;
pub mod canvas_graph_projection;
pub mod canvas_recovery;
pub mod canvas_repository;
pub mod canvas_viewport_query;
pub mod collaboration;
pub mod comment_repository;
pub mod committed_version_record_reader;
pub mod connector;
pub mod current_document_attachment_projection;
pub mod current_document_projection_catalog;
pub mod current_document_revision_projection;
pub mod current_document_version;
pub mod document_asset_repository;
pub mod document_existence;
pub mod document_link_catalog;
pub mod document_lock;
pub mod document_mutation_fingerprint;
pub mod document_navigator;
pub mod document_repository;
pub mod document_revision_commit;
pub mod document_revision_metadata;
pub mod document_title_reader;
pub mod embedding;
pub mod field_debug;
pub mod graph_projection;
pub mod group_repository;
pub mod html_renderer;
pub mod imported_asset_document_link;
pub mod link_index;
pub mod link_target_resolver;
pub mod markdown_parser;
pub mod notification;
pub mod object_storage;
pub mod permission_aware_query;
pub mod permission_policy_repository;
pub mod projection_repair;
pub mod projection_work;
pub mod projection_worker;
pub mod projection_writer;
pub mod realtime;
pub mod retrieval;
pub mod review_workflow;
pub mod search_index;
pub mod server_health;
pub mod user_repository;
pub mod version_preparation;
pub mod version_publication;
pub mod version_store;
pub mod webhook;
pub mod workspace_home;
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
