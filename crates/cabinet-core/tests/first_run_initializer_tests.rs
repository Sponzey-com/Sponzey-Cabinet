use std::path::Path;

use cabinet_core::config::{AppConfig, ExternalEnvironmentSnapshot};
use cabinet_core::first_run::{
    FirstRunDirectoryRole, FirstRunErrorCode, FirstRunInitializer, FirstRunProductEvent,
    FirstRunState, FirstRunStore, FirstRunStoreStatus,
};

fn test_config() -> AppConfig {
    let snapshot = ExternalEnvironmentSnapshot::from_pairs([(
        "SPONZEY_CABINET_APP_DATA_DIR",
        "/tmp/first-run-initializer",
    )]);
    AppConfig::from_environment_snapshot(snapshot).expect("test config should be valid")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FakeCall {
    EnsureDirectory(FirstRunDirectoryRole),
    WriteMetadataMarker,
}

#[derive(Debug)]
struct FakeFirstRunStore {
    existing: bool,
    fail_on: Option<FirstRunDirectoryRole>,
    calls: Vec<FakeCall>,
}

impl FakeFirstRunStore {
    fn clean() -> Self {
        Self {
            existing: false,
            fail_on: None,
            calls: Vec::new(),
        }
    }

    fn already_initialized() -> Self {
        Self {
            existing: true,
            fail_on: None,
            calls: Vec::new(),
        }
    }

    fn failing_on(role: FirstRunDirectoryRole) -> Self {
        Self {
            existing: false,
            fail_on: Some(role),
            calls: Vec::new(),
        }
    }
}

impl FirstRunStore for FakeFirstRunStore {
    fn ensure_directory(
        &mut self,
        role: FirstRunDirectoryRole,
        _path: &Path,
    ) -> Result<FirstRunStoreStatus, FirstRunErrorCode> {
        self.calls.push(FakeCall::EnsureDirectory(role));
        if self.fail_on == Some(role) {
            return Err(FirstRunErrorCode::StoreCreationFailed);
        }
        if self.existing {
            Ok(FirstRunStoreStatus::AlreadyPresent)
        } else {
            Ok(FirstRunStoreStatus::Created)
        }
    }

    fn write_metadata_marker(
        &mut self,
        _metadata_dir: &Path,
    ) -> Result<FirstRunStoreStatus, FirstRunErrorCode> {
        self.calls.push(FakeCall::WriteMetadataMarker);
        if self.existing {
            Ok(FirstRunStoreStatus::AlreadyPresent)
        } else {
            Ok(FirstRunStoreStatus::Created)
        }
    }
}

#[test]
fn first_run_initializer_completes_clean_profile() {
    let mut store = FakeFirstRunStore::clean();
    let initializer = FirstRunInitializer::new(test_config());

    let outcome = initializer.initialize(&mut store);

    assert_eq!(outcome.final_state, FirstRunState::Completed);
    assert_eq!(
        outcome.product_event,
        FirstRunProductEvent::FirstRunCompleted
    );
    assert_eq!(outcome.created_directories, 5);
    assert_eq!(outcome.already_present_directories, 0);
    assert_eq!(outcome.metadata_status, FirstRunStoreStatus::Created);
    assert_eq!(
        store.calls,
        vec![
            FakeCall::EnsureDirectory(FirstRunDirectoryRole::MetadataStore),
            FakeCall::EnsureDirectory(FirstRunDirectoryRole::VersionStore),
            FakeCall::EnsureDirectory(FirstRunDirectoryRole::AssetStore),
            FakeCall::EnsureDirectory(FirstRunDirectoryRole::SearchIndex),
            FakeCall::EnsureDirectory(FirstRunDirectoryRole::WorkspaceRoot),
            FakeCall::WriteMetadataMarker,
        ]
    );
}

#[test]
fn first_run_initializer_is_idempotent_for_existing_profile() {
    let mut store = FakeFirstRunStore::already_initialized();
    let initializer = FirstRunInitializer::new(test_config());

    let outcome = initializer.initialize(&mut store);

    assert_eq!(outcome.final_state, FirstRunState::Completed);
    assert_eq!(
        outcome.product_event,
        FirstRunProductEvent::FirstRunCompleted
    );
    assert_eq!(outcome.created_directories, 0);
    assert_eq!(outcome.already_present_directories, 5);
    assert_eq!(outcome.metadata_status, FirstRunStoreStatus::AlreadyPresent);
}

#[test]
fn first_run_initializer_returns_retryable_failed_outcome_when_store_creation_fails() {
    let mut store = FakeFirstRunStore::failing_on(FirstRunDirectoryRole::AssetStore);
    let initializer = FirstRunInitializer::new(test_config());

    let outcome = initializer.initialize(&mut store);

    assert_eq!(
        outcome.final_state,
        FirstRunState::Failed {
            error_code: FirstRunErrorCode::StoreCreationFailed,
            retryable: true,
        }
    );
    assert_eq!(
        outcome.product_event,
        FirstRunProductEvent::FirstRunFailed {
            error_code: FirstRunErrorCode::StoreCreationFailed,
        }
    );
    assert_eq!(outcome.created_directories, 2);
    assert_eq!(outcome.already_present_directories, 0);
    assert!(!store.calls.contains(&FakeCall::WriteMetadataMarker));
}
