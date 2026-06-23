use std::path::PathBuf;

use cabinet_core::config::{AppConfig, ExternalEnvironmentSnapshot};
use cabinet_core::first_run::{
    FirstRunDirectoryRole, FirstRunError, FirstRunErrorCode, FirstRunEvent, FirstRunPlan,
    FirstRunState, transition_first_run,
};

fn test_config() -> AppConfig {
    let snapshot = ExternalEnvironmentSnapshot::from_pairs([(
        "SPONZEY_CABINET_APP_DATA_DIR",
        "/tmp/first-run",
    )]);
    AppConfig::from_environment_snapshot(snapshot).expect("test config should be valid")
}

#[test]
fn first_run_plan_includes_all_local_store_directories() {
    let plan = FirstRunPlan::from_config(&test_config());
    let requests = plan.directory_requests();

    assert_eq!(requests.len(), 5);
    assert!(requests.contains(&(
        FirstRunDirectoryRole::MetadataStore,
        PathBuf::from("/tmp/first-run/metadata"),
    )));
    assert!(requests.contains(&(
        FirstRunDirectoryRole::VersionStore,
        PathBuf::from("/tmp/first-run/version-store"),
    )));
    assert!(requests.contains(&(
        FirstRunDirectoryRole::AssetStore,
        PathBuf::from("/tmp/first-run/assets"),
    )));
    assert!(requests.contains(&(
        FirstRunDirectoryRole::SearchIndex,
        PathBuf::from("/tmp/first-run/search-index"),
    )));
    assert!(requests.contains(&(
        FirstRunDirectoryRole::WorkspaceRoot,
        PathBuf::from("/tmp/first-run/workspaces"),
    )));
}

#[test]
fn first_run_transitions_to_completed_through_explicit_events() {
    let resolving = transition_first_run(FirstRunState::NotStarted, FirstRunEvent::Start)
        .expect("start should transition");
    assert_eq!(resolving.next_state, FirstRunState::ResolvingPaths);

    let creating = transition_first_run(resolving.next_state, FirstRunEvent::PathsResolved)
        .expect("paths resolved should transition");
    assert_eq!(creating.next_state, FirstRunState::CreatingStores);

    let writing = transition_first_run(creating.next_state, FirstRunEvent::StoreCreated)
        .expect("store created should transition");
    assert_eq!(writing.next_state, FirstRunState::WritingMetadata);

    let completed = transition_first_run(writing.next_state, FirstRunEvent::MetadataWritten)
        .expect("metadata written should transition");
    assert_eq!(completed.next_state, FirstRunState::Completed);
}

#[test]
fn first_run_rejects_invalid_transition() {
    let error = transition_first_run(FirstRunState::NotStarted, FirstRunEvent::StoreCreated)
        .expect_err("invalid transition should fail");

    assert_eq!(
        error,
        FirstRunError::InvalidTransition {
            state: FirstRunState::NotStarted,
            event: FirstRunEvent::StoreCreated,
        }
    );
}

#[test]
fn first_run_failure_state_carries_error_code_and_retry_policy() {
    let failed = transition_first_run(
        FirstRunState::CreatingStores,
        FirstRunEvent::Fail(FirstRunErrorCode::StoreCreationFailed),
    )
    .expect("failure should transition");

    assert_eq!(
        failed.next_state,
        FirstRunState::Failed {
            error_code: FirstRunErrorCode::StoreCreationFailed,
            retryable: true,
        }
    );
    assert_eq!(
        failed.error_code,
        Some(FirstRunErrorCode::StoreCreationFailed)
    );
    assert_eq!(failed.retryable, true);

    let retrying = transition_first_run(failed.next_state, FirstRunEvent::Retry)
        .expect("retryable failure should transition");
    assert_eq!(retrying.next_state, FirstRunState::Retrying);
}
