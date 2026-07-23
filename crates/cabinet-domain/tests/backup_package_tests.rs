use cabinet_domain::backup::{
    BackupDataClass, BackupDataOwnership, BackupManifestEntry, BackupPackageError,
    BackupPackageManifest, RestoreEvent, RestoreSideEffectRequest, RestoreState,
    RestoreWorkflowStateMachine,
};

const CHECKSUM_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CHECKSUM_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn backup_package_manifest_requires_every_authoritative_and_rebuildable_data_class() {
    let manifest = complete_manifest();

    assert_eq!(manifest.schema_version(), 1);
    assert_eq!(manifest.entries().len(), 8);
    assert_eq!(
        manifest
            .entry(BackupDataClass::CanvasRecords)
            .expect("canvas entry")
            .record_count(),
        23
    );
    assert_eq!(
        manifest
            .entry(BackupDataClass::GraphRebuildMetadata)
            .expect("graph metadata")
            .ownership(),
        BackupDataOwnership::Rebuildable
    );
}

#[test]
fn backup_package_manifest_accepts_optional_non_zero_creation_time() {
    let legacy = complete_manifest();
    assert_eq!(legacy.created_at_epoch_ms(), None);

    let timestamped = legacy
        .with_created_at_epoch_ms(1_784_064_000_000)
        .expect("valid creation time");
    assert_eq!(timestamped.created_at_epoch_ms(), Some(1_784_064_000_000));
    assert_eq!(
        complete_manifest().with_created_at_epoch_ms(0),
        Err(BackupPackageError::InvalidCreatedAt)
    );
}

#[test]
fn backup_package_manifest_rejects_unknown_schema_missing_class_and_duplicate_class() {
    let entries = complete_entries();
    assert_eq!(
        BackupPackageManifest::new(2, entries.clone()),
        Err(BackupPackageError::UnsupportedSchemaVersion)
    );

    let mut missing = entries.clone();
    missing.retain(|entry| entry.data_class() != BackupDataClass::AssetAssociations);
    assert_eq!(
        BackupPackageManifest::new(1, missing),
        Err(BackupPackageError::MissingDataClass(
            BackupDataClass::AssetAssociations
        ))
    );

    let mut duplicate = entries;
    duplicate.push(entry(
        BackupDataClass::CurrentDocuments,
        BackupDataOwnership::Authoritative,
        1,
    ));
    assert_eq!(
        BackupPackageManifest::new(1, duplicate),
        Err(BackupPackageError::DuplicateDataClass(
            BackupDataClass::CurrentDocuments
        ))
    );
}

#[test]
fn backup_manifest_entry_rejects_invalid_checksum_and_ownership() {
    assert_eq!(
        BackupManifestEntry::new(
            BackupDataClass::AssetObjects,
            BackupDataOwnership::Authoritative,
            1,
            16,
            "not-a-sha256",
        ),
        Err(BackupPackageError::InvalidChecksum)
    );
    assert_eq!(
        BackupManifestEntry::new(
            BackupDataClass::CanvasRecords,
            BackupDataOwnership::Rebuildable,
            1,
            16,
            CHECKSUM_A,
        ),
        Err(BackupPackageError::InvalidOwnership)
    );
}

#[test]
fn restore_workflow_enforces_preview_validation_confirmation_apply_and_reopen_order() {
    let previewing = transition(RestoreState::Requested, RestoreEvent::StartPreview);
    assert_eq!(previewing.next_state(), RestoreState::Previewing);
    assert_eq!(
        previewing.side_effect_request(),
        Some(RestoreSideEffectRequest::BuildPreview)
    );

    let validating = transition(previewing.next_state(), RestoreEvent::PreviewBuilt);
    let awaiting_confirmation = transition(validating.next_state(), RestoreEvent::ValidationPassed);
    let staging = transition(awaiting_confirmation.next_state(), RestoreEvent::Confirm);
    let applying = transition(staging.next_state(), RestoreEvent::StageCompleted);
    let reopening = transition(applying.next_state(), RestoreEvent::ApplyCompleted);
    let completed = transition(reopening.next_state(), RestoreEvent::ReopenCompleted);

    assert_eq!(validating.next_state(), RestoreState::Validating);
    assert_eq!(
        awaiting_confirmation.next_state(),
        RestoreState::AwaitingConfirmation
    );
    assert_eq!(staging.next_state(), RestoreState::Staging);
    assert_eq!(applying.next_state(), RestoreState::Applying);
    assert_eq!(reopening.next_state(), RestoreState::Reopening);
    assert_eq!(completed.next_state(), RestoreState::Completed);
    assert_eq!(
        completed.product_log_event_name(),
        Some("restore.completed")
    );
    assert!(completed.next_state().is_terminal());

    let early_confirm =
        RestoreWorkflowStateMachine::transition(RestoreState::Previewing, RestoreEvent::Confirm)
            .expect_err("confirmation cannot bypass validation");
    assert_eq!(early_confirm.code(), "RESTORE_INVALID_TRANSITION");
}

#[test]
fn restore_workflow_exposes_cancel_cleanup_and_rollback_paths() {
    let cancelled = transition(RestoreState::AwaitingConfirmation, RestoreEvent::Cancel);
    assert_eq!(cancelled.next_state(), RestoreState::Cancelled);
    assert_eq!(
        cancelled.product_log_event_name(),
        Some("restore.cancelled")
    );

    let cleanup = transition(RestoreState::Staging, RestoreEvent::StageFailed);
    assert_eq!(cleanup.next_state(), RestoreState::CleanupRequired);
    assert_eq!(
        cleanup.side_effect_request(),
        Some(RestoreSideEffectRequest::CleanupStaging)
    );
    let cleaned = transition(cleanup.next_state(), RestoreEvent::CleanupCompleted);
    assert_eq!(cleaned.next_state(), RestoreState::Failed);

    let rollback = transition(RestoreState::Applying, RestoreEvent::ApplyFailed);
    assert_eq!(rollback.next_state(), RestoreState::RollbackRequired);
    assert_eq!(
        rollback.side_effect_request(),
        Some(RestoreSideEffectRequest::RollbackApply)
    );
    let rolled_back = transition(rollback.next_state(), RestoreEvent::RollbackCompleted);
    assert_eq!(rolled_back.next_state(), RestoreState::RolledBack);

    let recovery_required = transition(rollback.next_state(), RestoreEvent::RollbackFailed);
    assert_eq!(
        recovery_required.next_state(),
        RestoreState::RecoveryRequired
    );
    assert_eq!(
        recovery_required.product_log_event_name(),
        Some("restore.recovery_required")
    );
    let retry = transition(
        recovery_required.next_state(),
        RestoreEvent::RecoveryRequested,
    );
    assert_eq!(retry.next_state(), RestoreState::RollbackRequired);
    assert_eq!(
        retry.side_effect_request(),
        Some(RestoreSideEffectRequest::RollbackApply)
    );
    assert!(rolled_back.next_state().is_terminal());

    assert!(
        RestoreWorkflowStateMachine::transition(RestoreState::Applying, RestoreEvent::Cancel)
            .is_err()
    );
}

fn complete_manifest() -> BackupPackageManifest {
    BackupPackageManifest::new(1, complete_entries()).expect("complete manifest")
}

fn complete_entries() -> Vec<BackupManifestEntry> {
    vec![
        entry(
            BackupDataClass::CurrentDocuments,
            BackupDataOwnership::Authoritative,
            10,
        ),
        entry(
            BackupDataClass::VersionHistory,
            BackupDataOwnership::Authoritative,
            30,
        ),
        entry(
            BackupDataClass::CanvasRecords,
            BackupDataOwnership::Authoritative,
            23,
        ),
        entry(
            BackupDataClass::AssetMetadata,
            BackupDataOwnership::Authoritative,
            8,
        ),
        entry(
            BackupDataClass::AssetObjects,
            BackupDataOwnership::Authoritative,
            7,
        ),
        entry(
            BackupDataClass::AssetAssociations,
            BackupDataOwnership::Authoritative,
            9,
        ),
        entry(
            BackupDataClass::GraphRebuildMetadata,
            BackupDataOwnership::Rebuildable,
            10,
        ),
        entry(
            BackupDataClass::SearchRebuildMetadata,
            BackupDataOwnership::Rebuildable,
            10,
        ),
    ]
}

fn entry(
    data_class: BackupDataClass,
    ownership: BackupDataOwnership,
    record_count: u64,
) -> BackupManifestEntry {
    BackupManifestEntry::new(
        data_class,
        ownership,
        record_count,
        record_count * 16,
        if record_count.is_multiple_of(2) {
            CHECKSUM_B
        } else {
            CHECKSUM_A
        },
    )
    .expect("manifest entry")
}

fn transition(
    state: RestoreState,
    event: RestoreEvent,
) -> cabinet_domain::backup::RestoreTransition {
    RestoreWorkflowStateMachine::transition(state, event).expect("valid restore transition")
}
