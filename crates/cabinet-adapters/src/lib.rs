//! Adapter implementations for external boundaries.

pub mod composite_graph_projection;
pub mod deterministic_embedding_provider;
pub mod durable_asset_association_catalog;
pub mod durable_asset_import_operation_repository;
pub mod durable_asset_metadata_catalog;
pub mod durable_backup_package_store;
pub mod durable_canvas_graph_projection;
pub mod durable_canvas_repository;
pub mod durable_document_link_catalog;
pub mod durable_last_canvas_selection;
pub mod durable_local_graph_projection;
pub mod durable_local_link_index;
pub mod durable_local_search_index;
pub mod durable_projection_repair_repository;
pub mod durable_projection_work_repository;
pub mod fake_ai_provider;
pub mod fake_connector_gateway;
pub mod fake_s3_object_storage;
pub mod fake_webhook_transport;
pub mod guarded_document_revision_commit;
pub mod local_ai_answer_store;
pub mod local_asset_availability_resolver;
pub mod local_asset_external_opener;
pub mod local_asset_import_source;
pub mod local_asset_preview_reader;
pub mod local_asset_search_index;
pub mod local_asset_staging_writer;
pub mod local_asset_store;
pub mod local_atomic_file;
pub mod local_audit_log_store;
pub mod local_auth;
pub mod local_backup_restore_store;
pub mod local_backup_store;
pub mod local_canvas_repository;
pub mod local_comment_repository;
pub mod local_connector_activity_store;
pub mod local_content_addressed_asset_publisher;
pub mod local_create_document_revision_runtime;
pub mod local_current_document_attachment_projection;
pub mod local_current_document_projection_catalog;
pub mod local_current_document_revision_projection;
pub mod local_current_document_version_pointer;
pub mod local_dead_letter_store;
pub mod local_document_asset_repository;
pub mod local_document_lock_repository;
pub mod local_document_mutation_fingerprint;
pub mod local_document_navigator_projection;
pub mod local_document_operation_journal;
pub mod local_document_repository;
pub mod local_document_revision_metadata;
pub mod local_document_store_migration;
pub mod local_event_log_store;
pub mod local_event_subscription_repository;
pub mod local_first_run;
pub mod local_graph_projection;
pub mod local_group_repository;
pub mod local_html_renderer;
pub mod local_imported_asset_document_revision_linker;
pub mod local_link_index;
pub mod local_markdown_parser;
pub mod local_migration;
pub mod local_mutate_document_attachments_runtime;
pub mod local_notification;
pub mod local_object_storage;
pub mod local_permission_policy_repository;
pub mod local_phase002_migration_fixture;
pub mod local_realtime;
pub mod local_restore_document_revision_runtime;
pub mod local_restore_projection_recovery_runtime;
pub mod local_retrieval_source;
pub mod local_review_workflow_repository;
pub mod local_search_index;
pub mod local_setup_health;
pub mod local_update_document_revision_runtime;
pub mod local_user_repository;
pub mod local_vector_index;
pub mod local_version_store;
pub mod local_workspace_home_projection;
pub mod local_workspace_home_query;
pub mod local_workspace_reopener;
pub mod phase011_upgrade_migrator;
pub mod process_local_document_diff_operation_registry;
pub mod static_connector_definition_registry;
pub mod tool_mapper;

/// Returns the architectural layer name for smoke tests and diagnostics.
pub const fn layer_name() -> &'static str {
    "adapters"
}

/// Smoke function proving adapters can depend on port contracts.
pub fn implemented_contract_layers() -> (&'static str, &'static str, &'static str, &'static str) {
    (
        cabinet_domain::layer_name(),
        cabinet_ports::layer_name(),
        cabinet_core::layer_name(),
        cabinet_usecases::layer_name(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapters_layer_references_inward_contracts() {
        assert_eq!(layer_name(), "adapters");
        assert_eq!(
            implemented_contract_layers(),
            ("domain", "ports", "core", "usecases")
        );
    }
}
