//! Application usecase boundary.

pub mod ai;
pub mod asset_import;
pub mod asset_lifecycle;
pub mod asset_preview;
pub mod audit;
pub mod auth;
pub mod backup;
pub mod backup_package;
pub mod backup_package_operation;
pub mod backup_recovery;
pub mod backup_restore;
pub mod canvas;
pub mod canvas_lifecycle;
pub mod canvas_mutation;
pub mod canvas_recovery;
pub mod canvas_target_presentation;
pub mod canvas_viewport;
pub mod collaboration;
pub mod comment;
pub mod connector;
pub mod document;
pub mod document_link_catalog_projection;
pub mod document_lock;
pub mod document_navigator;
pub mod export;
pub mod field_debug;
pub mod global_graph;
pub mod graph;
pub mod group;
pub mod guarded_authoring;
pub mod import;
pub mod notification;
pub mod permission;
pub mod permission_query;
pub mod projection_freshness;
pub mod projection_kind_writer_router;
pub mod projection_repair_operation;
pub mod projection_work;
pub mod projection_worker;
pub mod reference_projection_fanout;
pub mod reindex_asset_graph_projection;
pub mod reindex_projection;
pub mod resolved_link_graph_writer;
pub mod restore_projection_rebuild;
pub mod retrieval;
pub mod review_workflow;
pub mod search;
pub mod search_projection_writer;
pub mod semantic;
pub mod server_health;
pub mod tool;
pub mod user;
pub mod versioned_projection_processor;
pub mod webhook;
pub mod workspace;
pub mod workspace_home;
pub mod workspace_home_update;

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
