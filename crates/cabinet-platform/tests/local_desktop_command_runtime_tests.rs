use cabinet_platform::local_desktop_runtime::{
    LocalDesktopCommandErrorCode, LocalDesktopCommandEvent, LocalDesktopCommandPayload,
    LocalDesktopCommandState, LocalDesktopRuntimeCommandRequest, LocalDesktopUsecaseInput,
    map_core_local_desktop_command_request, summarize_local_desktop_command_for_product_log,
    transition_local_desktop_command,
};

#[test]
fn local_desktop_command_state_machine_reaches_completed_in_order() {
    let validating = transition_local_desktop_command(
        LocalDesktopCommandState::Idle,
        LocalDesktopCommandEvent::Start,
    );
    let executing = transition_local_desktop_command(
        validating.next_state,
        LocalDesktopCommandEvent::InputValidated,
    );
    let mapping = transition_local_desktop_command(
        executing.next_state,
        LocalDesktopCommandEvent::UsecaseExecuted,
    );
    let completed = transition_local_desktop_command(
        mapping.next_state,
        LocalDesktopCommandEvent::ResultMapped,
    );

    assert_eq!(
        validating.next_state,
        LocalDesktopCommandState::ValidatingInput
    );
    assert_eq!(
        executing.next_state,
        LocalDesktopCommandState::ExecutingUsecase
    );
    assert_eq!(mapping.next_state, LocalDesktopCommandState::MappingResult);
    assert_eq!(completed.next_state, LocalDesktopCommandState::Completed);
    assert_eq!(completed.error_code, None);
}

#[test]
fn local_desktop_command_state_machine_returns_stable_failure_codes() {
    let invalid = transition_local_desktop_command(
        LocalDesktopCommandState::Idle,
        LocalDesktopCommandEvent::ResultMapped,
    );
    let failed = transition_local_desktop_command(
        LocalDesktopCommandState::ExecutingUsecase,
        LocalDesktopCommandEvent::Fail(LocalDesktopCommandErrorCode::UsecaseFailed),
    );

    assert_eq!(invalid.next_state, LocalDesktopCommandState::Failed);
    assert_eq!(
        invalid.error_code,
        Some(LocalDesktopCommandErrorCode::InvalidTransition)
    );
    assert_eq!(invalid.retryable, false);
    assert_eq!(failed.next_state, LocalDesktopCommandState::Failed);
    assert_eq!(
        failed.error_code,
        Some(LocalDesktopCommandErrorCode::UsecaseFailed)
    );
    assert_eq!(failed.retryable, true);
}

#[test]
fn core_local_desktop_command_mapper_maps_workspace_and_document_commands() {
    let bootstrap = map_core_local_desktop_command_request(LocalDesktopRuntimeCommandRequest::new(
        "local_workspace_bootstrap",
        LocalDesktopCommandPayload::Empty,
    ))
    .expect("bootstrap maps");
    let home = map_core_local_desktop_command_request(LocalDesktopRuntimeCommandRequest::new(
        "local_workspace_home",
        LocalDesktopCommandPayload::WorkspaceHome {
            workspace_id: "workspace-1".to_string(),
            recent_documents: 12,
            favorites: 8,
            tags: 10,
            recent_changes: 14,
            unfinished_items: 6,
        },
    ))
    .expect("home maps");
    let current = map_core_local_desktop_command_request(LocalDesktopRuntimeCommandRequest::new(
        "get_current_document",
        LocalDesktopCommandPayload::DocumentIdentity {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
        },
    ))
    .expect("current maps");
    let history = map_core_local_desktop_command_request(LocalDesktopRuntimeCommandRequest::new(
        "get_document_history",
        LocalDesktopCommandPayload::DocumentHistory {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            limit: 20,
        },
    ))
    .expect("history maps");
    let version = map_core_local_desktop_command_request(LocalDesktopRuntimeCommandRequest::new(
        "get_document_version",
        LocalDesktopCommandPayload::DocumentVersion {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-1".to_string(),
        },
    ))
    .expect("version maps");

    assert_eq!(bootstrap, LocalDesktopUsecaseInput::BootstrapWorkspace);
    assert_eq!(
        home,
        LocalDesktopUsecaseInput::WorkspaceHome {
            workspace_id: "workspace-1".to_string(),
            recent_documents: 12,
            favorites: 8,
            tags: 10,
            recent_changes: 14,
            unfinished_items: 6,
        }
    );
    assert_eq!(
        current,
        LocalDesktopUsecaseInput::GetCurrentDocument {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string()
        }
    );
    assert_eq!(
        history,
        LocalDesktopUsecaseInput::GetDocumentHistory {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            limit: 20
        }
    );
    assert_eq!(
        version,
        LocalDesktopUsecaseInput::GetDocumentVersion {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            version_id: "version-1".to_string()
        }
    );
}

#[test]
fn workspace_home_mapper_rejects_empty_workspace_and_out_of_range_limits() {
    for (workspace_id, recent_documents) in [("", 10), ("workspace-1", 0), ("workspace-1", 101)] {
        let error = map_core_local_desktop_command_request(LocalDesktopRuntimeCommandRequest::new(
            "local_workspace_home",
            LocalDesktopCommandPayload::WorkspaceHome {
                workspace_id: workspace_id.to_string(),
                recent_documents,
                favorites: 10,
                tags: 10,
                recent_changes: 10,
                unfinished_items: 10,
            },
        ))
        .expect_err("invalid workspace home payload must fail");

        assert_eq!(error.error_code, LocalDesktopCommandErrorCode::InvalidInput);
    }
}

#[test]
fn workspace_home_product_log_summary_excludes_raw_workspace_id() {
    let request = LocalDesktopRuntimeCommandRequest::new(
        "local_workspace_home",
        LocalDesktopCommandPayload::WorkspaceHome {
            workspace_id: "private-workspace-id".to_string(),
            recent_documents: 12,
            favorites: 8,
            tags: 10,
            recent_changes: 14,
            unfinished_items: 6,
        },
    );

    let summary = summarize_local_desktop_command_for_product_log(&request);

    assert!(summary.workspace_id_present);
    assert_eq!(summary.result_limit, Some(50));
    assert!(!format!("{summary:?}").contains("private-workspace-id"));
}

#[test]
fn core_local_desktop_command_mapper_maps_update_without_leaking_raw_body_to_summary() {
    let request = LocalDesktopRuntimeCommandRequest::new(
        "update_current_document",
        LocalDesktopCommandPayload::DocumentUpdate {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            title: "Source".to_string(),
            path: "docs/source.md".to_string(),
            body: "raw document body fixture must not be logged".to_string(),
            expected_version_id: "version-1".to_string(),
        },
    );
    let summary = summarize_local_desktop_command_for_product_log(&request);
    let mapped = map_core_local_desktop_command_request(request).expect("update maps");

    assert_eq!(
        mapped,
        LocalDesktopUsecaseInput::UpdateCurrentDocument {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            title: "Source".to_string(),
            path: "docs/source.md".to_string(),
            body: "raw document body fixture must not be logged".to_string(),
            expected_version_id: "version-1".to_string(),
        }
    );
    assert_eq!(summary.command_name, "update_current_document");
    assert_eq!(summary.body_byte_len, Some(44));
    assert!(!format!("{summary:?}").contains("raw document body fixture"));
    assert!(!format!("{summary:?}").contains("docs/source.md"));
}

#[test]
fn local_desktop_command_mapper_covers_remaining_phase009_commands() {
    let cases = [
        (
            "preview_document_restore",
            LocalDesktopCommandPayload::DocumentVersion {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
                version_id: "version-1".to_string(),
            },
        ),
        (
            "restore_document_version",
            LocalDesktopCommandPayload::DocumentVersion {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
                version_id: "version-1".to_string(),
            },
        ),
        (
            "search_documents",
            LocalDesktopCommandPayload::Search {
                workspace_id: "workspace-1".to_string(),
                text: "needle".to_string(),
                limit: 10,
            },
        ),
        (
            "search_assets",
            LocalDesktopCommandPayload::Search {
                workspace_id: "workspace-1".to_string(),
                text: "needle".to_string(),
                limit: 10,
            },
        ),
        (
            "get_link_overview",
            LocalDesktopCommandPayload::DocumentIdentity {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
            },
        ),
        (
            "get_graph_projection",
            LocalDesktopCommandPayload::GraphProjection {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
                depth: 2,
                direction: "both".to_string(),
                include_unresolved: true,
                include_assets: false,
                node_limit: 120,
                edge_limit: 240,
            },
        ),
        (
            "list_document_assets",
            LocalDesktopCommandPayload::DocumentIdentity {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
            },
        ),
        (
            "attach_document_asset",
            LocalDesktopCommandPayload::AssetAttachment {
                workspace_id: "workspace-1".to_string(),
                document_id: "doc-1".to_string(),
                asset_id: "asset-1".to_string(),
                label: "Reference".to_string(),
                file_name: "private-source-name.pdf".to_string(),
                media_type: "application/pdf".to_string(),
                byte_size: 42,
            },
        ),
        (
            "create_backup",
            LocalDesktopCommandPayload::Workspace {
                workspace_id: "workspace-1".to_string(),
            },
        ),
        (
            "preview_import",
            LocalDesktopCommandPayload::ImportPreview {
                workspace_id: "workspace-1".to_string(),
                source_label: "private/source/path".to_string(),
                file_count: 3,
            },
        ),
        (
            "preview_restore",
            LocalDesktopCommandPayload::RestorePackage {
                workspace_id: "workspace-1".to_string(),
                package_label: "private/backup/path.zip".to_string(),
            },
        ),
        (
            "apply_restore",
            LocalDesktopCommandPayload::RestorePackage {
                workspace_id: "workspace-1".to_string(),
                package_label: "private/backup/path.zip".to_string(),
            },
        ),
    ];

    let mapped_names = cases
        .into_iter()
        .map(|(command_name, payload)| {
            let input = map_core_local_desktop_command_request(
                LocalDesktopRuntimeCommandRequest::new(command_name, payload),
            )
            .expect("command maps");
            input.phase009_name()
        })
        .collect::<Vec<_>>();

    assert_eq!(
        mapped_names,
        [
            "PreviewDocumentRestore",
            "RestoreDocumentVersion",
            "SearchDocuments",
            "SearchAssets",
            "GetLinkOverview",
            "GetGraphProjection",
            "ListDocumentAssets",
            "AttachDocumentAsset",
            "CreateBackup",
            "PreviewImport",
            "PreviewRestore",
            "ApplyRestore",
        ]
    );
}

#[test]
fn asset_search_mapper_and_product_log_summary_hide_raw_query() {
    let request = LocalDesktopRuntimeCommandRequest::new(
        "search_assets",
        LocalDesktopCommandPayload::Search {
            workspace_id: "workspace-1".to_string(),
            text: "private attachment query".to_string(),
            limit: 12,
        },
    );
    let summary = summarize_local_desktop_command_for_product_log(&request);
    let mapped = map_core_local_desktop_command_request(request).expect("asset search maps");

    assert_eq!(
        mapped,
        LocalDesktopUsecaseInput::SearchAssets {
            workspace_id: "workspace-1".to_string(),
            text: "private attachment query".to_string(),
            limit: 12,
        }
    );
    assert_eq!(summary.command_name, "search_assets");
    assert_eq!(summary.result_limit, Some(12));
    assert!(summary.workspace_id_present);
    assert!(!format!("{summary:?}").contains("private attachment query"));
}

#[test]
fn local_desktop_command_summary_hides_asset_import_and_restore_paths() {
    let request = LocalDesktopRuntimeCommandRequest::new(
        "attach_document_asset",
        LocalDesktopCommandPayload::AssetAttachment {
            workspace_id: "workspace-1".to_string(),
            document_id: "doc-1".to_string(),
            asset_id: "asset-1".to_string(),
            label: "Reference".to_string(),
            file_name: "/Users/example/private/source.pdf".to_string(),
            media_type: "application/pdf".to_string(),
            byte_size: 42,
        },
    );
    let summary = summarize_local_desktop_command_for_product_log(&request);

    assert_eq!(summary.command_name, "attach_document_asset");
    assert_eq!(summary.asset_byte_len, Some(42));
    assert!(!format!("{summary:?}").contains("/Users/example/private"));
    assert!(!format!("{summary:?}").contains("source.pdf"));
    assert!(!format!("{summary:?}").contains("asset content"));
}

#[test]
fn core_local_desktop_command_mapper_rejects_invalid_payload_and_unsupported_command() {
    let mismatch = map_core_local_desktop_command_request(LocalDesktopRuntimeCommandRequest::new(
        "get_current_document",
        LocalDesktopCommandPayload::Empty,
    ))
    .expect_err("payload mismatch fails");
    let empty_id = map_core_local_desktop_command_request(LocalDesktopRuntimeCommandRequest::new(
        "get_current_document",
        LocalDesktopCommandPayload::DocumentIdentity {
            workspace_id: "".to_string(),
            document_id: "doc-1".to_string(),
        },
    ))
    .expect_err("empty id fails");
    let unsupported =
        map_core_local_desktop_command_request(LocalDesktopRuntimeCommandRequest::new(
            "unsupported_future_command",
            LocalDesktopCommandPayload::Empty,
        ))
        .expect_err("unsupported command fails");

    assert_eq!(
        mismatch.error_code,
        LocalDesktopCommandErrorCode::InvalidInput
    );
    assert_eq!(
        empty_id.error_code,
        LocalDesktopCommandErrorCode::InvalidInput
    );
    assert_eq!(
        unsupported.error_code,
        LocalDesktopCommandErrorCode::UnsupportedCommand
    );
}
