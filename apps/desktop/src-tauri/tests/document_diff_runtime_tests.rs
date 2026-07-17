use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

use cabinet_adapters::local_asset_availability_resolver::LocalAssetAvailabilityResolver;
use cabinet_desktop_shell::{
    DesktopDocumentAttachmentDiffDto, DesktopDocumentDiffOperationRequestDto,
    DesktopDocumentDiffOperationRuntime, DesktopDocumentDiffOperationTokenRequestDto,
    DesktopDocumentDiffRequestDto, DesktopDocumentDiffRuntime, DesktopDocumentMutationRequestDto,
    DesktopDocumentMutationRuntime,
};
use cabinet_domain::asset::{AssetId, AssetReference};
use cabinet_domain::version::AttachmentSnapshotState;
use cabinet_usecases::attachment_diff::compare_attachment_snapshots;
use cabinet_usecases::document_diff::DiffPolicy;
use cabinet_usecases::resolve_attachment_diff_availability::{
    ResolveAttachmentDiffAvailabilityInput, ResolveAttachmentDiffAvailabilityUsecase,
};

#[test]
fn attachment_diff_mapper_preserves_labels_without_serializing_asset_identity() {
    let temp = TempRoot::new("attachment-mapper");
    let left = AttachmentSnapshotState::known(vec![
        attachment('a', "초안"),
        attachment('b', "제거 자료"),
        attachment('c', "유지 자료"),
    ])
    .unwrap();
    let right = AttachmentSnapshotState::known(vec![
        attachment('a', "최종안"),
        attachment('c', "유지 자료"),
        attachment('d', "추가 자료"),
    ])
    .unwrap();

    let resolved = ResolveAttachmentDiffAvailabilityUsecase::new()
        .execute(
            ResolveAttachmentDiffAvailabilityInput::new(
                "workspace-1",
                compare_attachment_snapshots(&left, &right),
            ),
            &LocalAssetAvailabilityResolver::new(temp.path.clone()),
        )
        .unwrap();
    let dto = DesktopDocumentAttachmentDiffDto::from_resolved(&resolved);

    assert_eq!(dto.kind, "known");
    assert_eq!(dto.added[0].label, "추가 자료");
    assert_eq!(dto.added[0].availability, "missing");
    assert_eq!(dto.removed[0].label, "제거 자료");
    assert_eq!(dto.removed[0].availability, "missing");
    assert_eq!(dto.relabeled[0].before_label, "초안");
    assert_eq!(dto.relabeled[0].after_label, "최종안");
    assert_eq!(dto.relabeled[0].availability, "missing");
    assert_eq!(dto.unchanged_count, 1);
    let json = serde_json::to_string(&dto).unwrap();
    assert!(!json.contains("assetId"));
    assert!(!json.contains(&"a".repeat(64)));
}

#[test]
fn current_to_version_diff_returns_bounded_hunks_without_storage_metadata() {
    let temp = TempRoot::new("current-version");
    let mutation = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    let diff = DesktopDocumentDiffRuntime::new(temp.path.clone(), 4096).unwrap();

    let created = mutation.execute(create_request("operation-1", "# 이전 제목\n이전 본문\n"));
    let first_version = created.data.unwrap().current_version_id;
    let updated = mutation.execute(update_request(
        "operation-2",
        &first_version,
        "# 현재 제목\n현재 본문\n",
    ));
    assert!(updated.ok);

    let response = diff.execute(DesktopDocumentDiffRequestDto::CurrentToVersion {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        version_token: first_version,
    });

    assert!(response.ok);
    let data = response.data.unwrap();
    assert_eq!(data.kind, "complete");
    assert!(data.added_count > 0);
    assert!(data.removed_count > 0);
    assert!(!data.hunks.is_empty());
    assert_eq!(data.attachment_diff.kind, "known");
    assert!(data.attachment_diff.added.is_empty());
    assert!(data.attachment_diff.removed.is_empty());
    assert!(data.attachment_diff.relabeled.is_empty());
    assert_eq!(data.attachment_diff.unchanged_count, 0);
    let title = data.title_delta.as_ref().expect("title delta");
    assert_eq!(title.kind, "changed");
    assert_eq!(title.before.as_deref(), Some("현재 제목"));
    assert_eq!(title.after.as_deref(), Some("이전 제목"));

    let json = serde_json::to_value(data).unwrap();
    let text = json.to_string();
    assert!(!text.contains("snapshotRef"));
    assert!(!text.contains("path"));
    assert!(!text.contains("assetId"));
    assert!(text.contains("oldLineNumber"));
    assert!(text.contains("newLineNumber"));
}

#[test]
fn version_pair_and_too_large_are_explicit_runtime_outcomes() {
    let temp = TempRoot::new("too-large");
    let mutation = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    let created = mutation.execute(create_request("operation-1", "Left body\n"));
    let first_version = created.data.unwrap().current_version_id;
    let updated = mutation.execute(update_request(
        "operation-2",
        &first_version,
        "Right body\n",
    ));
    let second_version = updated.data.unwrap().current_version_id;
    let diff = DesktopDocumentDiffRuntime::with_policy(
        temp.path.clone(),
        4096,
        DiffPolicy::new(0, 4, 100, 100).unwrap(),
    )
    .unwrap();

    let response = diff.execute(DesktopDocumentDiffRequestDto::Versions {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        left_version_token: first_version,
        right_version_token: second_version,
    });

    assert!(response.ok);
    let data = response.data.unwrap();
    assert_eq!(data.kind, "too_large");
    assert_eq!(data.limit_reason.as_deref(), Some("bytes"));
    assert!(data.hunks.is_empty());
    assert_eq!(data.attachment_diff.kind, "known");
}

#[test]
fn invalid_and_missing_diff_targets_return_stable_errors_without_partial_data() {
    let temp = TempRoot::new("errors");
    let diff = DesktopDocumentDiffRuntime::new(temp.path.clone(), 4096).unwrap();

    let invalid = diff.execute(DesktopDocumentDiffRequestDto::CurrentToVersion {
        workspace_id: "".into(),
        document_id: "doc-1".into(),
        version_token: "version-1".into(),
    });
    assert!(!invalid.ok);
    assert!(invalid.data.is_none());
    assert_eq!(
        invalid.error_code.as_deref(),
        Some("DOCUMENT_DIFF_INVALID_INPUT")
    );

    let missing = diff.execute(DesktopDocumentDiffRequestDto::CurrentToVersion {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        version_token: "version-1".into(),
    });
    assert_eq!(
        missing.error_code.as_deref(),
        Some("DOCUMENT_DIFF_NOT_FOUND")
    );
    assert!(!missing.retryable);
}

#[test]
fn tauri_main_registers_authoritative_document_diff_command() {
    let source = include_str!("../src/main.rs");
    assert!(source.contains("execute_desktop_document_diff"));
    assert!(source.contains("DesktopDocumentDiffRuntime"));
}

#[test]
fn background_diff_accepts_without_waiting_and_polls_authoritative_result() {
    let temp = TempRoot::new("background-complete");
    let mutation = DesktopDocumentMutationRuntime::new(temp.path.clone(), 4096).unwrap();
    let created = mutation.execute(create_request("operation-1", "# 이전\n왼쪽 본문\n"));
    let first_version = created.data.unwrap().current_version_id;
    let updated = mutation.execute(update_request(
        "operation-2",
        &first_version,
        "# 현재\n오른쪽 본문\n",
    ));
    let second_version = updated.data.unwrap().current_version_id;
    let runtime = DesktopDocumentDiffOperationRuntime::with_policy(
        temp.path.clone(),
        4096,
        16,
        DiffPolicy::new(1, 4096, 1000, 1000).unwrap(),
    )
    .unwrap();

    let started_at = Instant::now();
    let accepted = runtime.start(DesktopDocumentDiffOperationRequestDto::Versions {
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        left_version_token: first_version,
        right_version_token: second_version,
    });
    assert!(started_at.elapsed() < Duration::from_millis(300));
    assert!(accepted.ok);
    let accepted_data = accepted.data.unwrap();
    assert_eq!(accepted_data.state, "accepted");
    let token = accepted_data.operation_token;

    let terminal = (0..100)
        .find_map(|_| {
            let response = runtime.status(DesktopDocumentDiffOperationTokenRequestDto {
                operation_token: token.clone(),
            });
            let data = response.data?;
            if matches!(data.state.as_str(), "accepted" | "running") {
                thread::sleep(Duration::from_millis(5));
                None
            } else {
                Some(data)
            }
        })
        .expect("background diff terminal status");

    assert_eq!(terminal.state, "completed");
    let diff = terminal.diff.as_ref().expect("completed diff");
    assert_eq!(diff.kind, "complete");
    assert!(!diff.hunks.is_empty());
    assert_eq!(diff.attachment_diff.kind, "known");
    let json = serde_json::to_string(&terminal).unwrap();
    assert!(!json.contains("snapshotRef"));
    assert!(!json.contains("assetId"));
    assert!(!json.contains("path"));
}

#[test]
fn background_diff_unknown_or_restarted_token_is_expired_and_invalid_token_is_rejected() {
    let temp = TempRoot::new("background-expired");
    let runtime = DesktopDocumentDiffOperationRuntime::new(temp.path.clone(), 4096, 4).unwrap();
    let expired = runtime.status(DesktopDocumentDiffOperationTokenRequestDto {
        operation_token: "opaque-operation-from-previous-process".into(),
    });
    assert!(expired.ok);
    assert_eq!(expired.data.unwrap().state, "expired");

    let cancelled = runtime.cancel(DesktopDocumentDiffOperationTokenRequestDto {
        operation_token: "opaque-operation-from-previous-process".into(),
    });
    assert!(cancelled.ok);
    assert_eq!(cancelled.data.unwrap().state, "expired");

    let invalid = runtime.status(DesktopDocumentDiffOperationTokenRequestDto {
        operation_token: " ".into(),
    });
    assert!(!invalid.ok);
    assert_eq!(
        invalid.error_code.as_deref(),
        Some("DOCUMENT_DIFF_OPERATION_INVALID_INPUT")
    );
}

#[test]
fn tauri_main_registers_background_diff_state_and_commands() {
    let source = include_str!("../src/main.rs");
    assert!(source.contains("DesktopDocumentDiffOperationRuntime"));
    for command in [
        "start_desktop_document_diff_operation",
        "get_desktop_document_diff_operation_status",
        "cancel_desktop_document_diff_operation",
    ] {
        assert!(source.contains(command), "missing {command}");
    }
}

fn create_request(operation_id: &str, body: &str) -> DesktopDocumentMutationRequestDto {
    DesktopDocumentMutationRequestDto::Create {
        operation_id: operation_id.into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        body: body.into(),
        author: "local-user".into(),
        summary: "Create".into(),
    }
}

fn update_request(
    operation_id: &str,
    expected_current_version_id: &str,
    body: &str,
) -> DesktopDocumentMutationRequestDto {
    DesktopDocumentMutationRequestDto::Update {
        operation_id: operation_id.into(),
        workspace_id: "workspace-1".into(),
        document_id: "doc-1".into(),
        expected_current_version_id: expected_current_version_id.into(),
        body: body.into(),
        author: "local-user".into(),
        summary: "Update".into(),
    }
}

fn attachment(seed: char, label: &str) -> AssetReference {
    AssetReference::new(
        AssetId::from_sha256_hex(&seed.to_string().repeat(64)).unwrap(),
        label,
    )
    .unwrap()
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cabinet-desktop-diff-{name}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
