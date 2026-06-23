use std::path::{Path, PathBuf};

use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunDirectoryRole {
    MetadataStore,
    VersionStore,
    AssetStore,
    SearchIndex,
    WorkspaceRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstRunPlan {
    directory_requests: Vec<(FirstRunDirectoryRole, PathBuf)>,
}

impl FirstRunPlan {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            directory_requests: vec![
                (
                    FirstRunDirectoryRole::MetadataStore,
                    config.storage.metadata_dir.clone(),
                ),
                (
                    FirstRunDirectoryRole::VersionStore,
                    config.storage.version_store_dir.clone(),
                ),
                (
                    FirstRunDirectoryRole::AssetStore,
                    config.storage.asset_store_dir.clone(),
                ),
                (
                    FirstRunDirectoryRole::SearchIndex,
                    config.search.index_dir.clone(),
                ),
                (
                    FirstRunDirectoryRole::WorkspaceRoot,
                    config.local_paths.workspace_root.clone(),
                ),
            ],
        }
    }

    pub fn directory_requests(&self) -> &[(FirstRunDirectoryRole, PathBuf)] {
        &self.directory_requests
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunStoreStatus {
    Created,
    AlreadyPresent,
    NotAttempted,
}

pub trait FirstRunStore {
    fn ensure_directory(
        &mut self,
        role: FirstRunDirectoryRole,
        path: &Path,
    ) -> Result<FirstRunStoreStatus, FirstRunErrorCode>;

    fn write_metadata_marker(
        &mut self,
        metadata_dir: &Path,
    ) -> Result<FirstRunStoreStatus, FirstRunErrorCode>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstRunInitializer {
    config: AppConfig,
}

impl FirstRunInitializer {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn initialize<S: FirstRunStore>(&self, store: &mut S) -> FirstRunInitializationOutcome {
        let mut state = transition_or_failed(
            FirstRunState::NotStarted,
            FirstRunEvent::Start,
            FirstRunErrorCode::PathResolutionFailed,
        );
        state = transition_or_failed(
            state,
            FirstRunEvent::PathsResolved,
            FirstRunErrorCode::PathResolutionFailed,
        );

        let mut created_directories = 0;
        let mut already_present_directories = 0;

        for (role, path) in FirstRunPlan::from_config(&self.config).directory_requests() {
            match store.ensure_directory(*role, path.as_path()) {
                Ok(FirstRunStoreStatus::Created) => created_directories += 1,
                Ok(FirstRunStoreStatus::AlreadyPresent) => already_present_directories += 1,
                Ok(FirstRunStoreStatus::NotAttempted) => {
                    return failed_initialization_outcome(
                        state,
                        FirstRunErrorCode::StoreCreationFailed,
                        created_directories,
                        already_present_directories,
                        FirstRunStoreStatus::NotAttempted,
                    );
                }
                Err(error_code) => {
                    return failed_initialization_outcome(
                        state,
                        error_code,
                        created_directories,
                        already_present_directories,
                        FirstRunStoreStatus::NotAttempted,
                    );
                }
            }
        }

        state = transition_or_failed(
            state,
            FirstRunEvent::StoreCreated,
            FirstRunErrorCode::StoreCreationFailed,
        );

        let metadata_status =
            match store.write_metadata_marker(self.config.storage.metadata_dir.as_path()) {
                Ok(
                    status @ (FirstRunStoreStatus::Created | FirstRunStoreStatus::AlreadyPresent),
                ) => status,
                Ok(FirstRunStoreStatus::NotAttempted) => {
                    return failed_initialization_outcome(
                        state,
                        FirstRunErrorCode::MetadataWriteFailed,
                        created_directories,
                        already_present_directories,
                        FirstRunStoreStatus::NotAttempted,
                    );
                }
                Err(error_code) => {
                    return failed_initialization_outcome(
                        state,
                        error_code,
                        created_directories,
                        already_present_directories,
                        FirstRunStoreStatus::NotAttempted,
                    );
                }
            };

        state = transition_or_failed(
            state,
            FirstRunEvent::MetadataWritten,
            FirstRunErrorCode::MetadataWriteFailed,
        );

        FirstRunInitializationOutcome {
            final_state: state,
            product_event: FirstRunProductEvent::FirstRunCompleted,
            created_directories,
            already_present_directories,
            metadata_status,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunProductEvent {
    FirstRunCompleted,
    FirstRunFailed { error_code: FirstRunErrorCode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstRunInitializationOutcome {
    pub final_state: FirstRunState,
    pub product_event: FirstRunProductEvent,
    pub created_directories: usize,
    pub already_present_directories: usize,
    pub metadata_status: FirstRunStoreStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunState {
    NotStarted,
    ResolvingPaths,
    CreatingStores,
    WritingMetadata,
    Completed,
    Failed {
        error_code: FirstRunErrorCode,
        retryable: bool,
    },
    Retrying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunEvent {
    Start,
    PathsResolved,
    StoreCreated,
    MetadataWritten,
    Fail(FirstRunErrorCode),
    Retry,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunErrorCode {
    PathResolutionFailed,
    StoreCreationFailed,
    MetadataWriteFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstRunTransition {
    pub previous_state: FirstRunState,
    pub event: FirstRunEvent,
    pub next_state: FirstRunState,
    pub retryable: bool,
    pub error_code: Option<FirstRunErrorCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstRunError {
    InvalidTransition {
        state: FirstRunState,
        event: FirstRunEvent,
    },
}

pub fn transition_first_run(
    state: FirstRunState,
    event: FirstRunEvent,
) -> Result<FirstRunTransition, FirstRunError> {
    let next_state = match (state, event) {
        (FirstRunState::NotStarted, FirstRunEvent::Start) => FirstRunState::ResolvingPaths,
        (FirstRunState::Retrying, FirstRunEvent::Start) => FirstRunState::ResolvingPaths,
        (FirstRunState::ResolvingPaths, FirstRunEvent::PathsResolved) => {
            FirstRunState::CreatingStores
        }
        (FirstRunState::CreatingStores, FirstRunEvent::StoreCreated) => {
            FirstRunState::WritingMetadata
        }
        (
            FirstRunState::WritingMetadata,
            FirstRunEvent::MetadataWritten | FirstRunEvent::Complete,
        ) => FirstRunState::Completed,
        (
            FirstRunState::NotStarted
            | FirstRunState::ResolvingPaths
            | FirstRunState::CreatingStores
            | FirstRunState::WritingMetadata
            | FirstRunState::Retrying,
            FirstRunEvent::Fail(error_code),
        ) => FirstRunState::Failed {
            error_code,
            retryable: true,
        },
        (
            FirstRunState::Failed {
                retryable: true, ..
            },
            FirstRunEvent::Retry,
        ) => FirstRunState::Retrying,
        _ => return Err(FirstRunError::InvalidTransition { state, event }),
    };

    let (retryable, error_code) = match next_state {
        FirstRunState::Failed {
            error_code,
            retryable,
        } => (retryable, Some(error_code)),
        _ => (false, None),
    };

    Ok(FirstRunTransition {
        previous_state: state,
        event,
        next_state,
        retryable,
        error_code,
    })
}

fn transition_or_failed(
    state: FirstRunState,
    event: FirstRunEvent,
    fallback_error_code: FirstRunErrorCode,
) -> FirstRunState {
    transition_first_run(state, event)
        .map(|transition| transition.next_state)
        .unwrap_or(FirstRunState::Failed {
            error_code: fallback_error_code,
            retryable: true,
        })
}

fn failed_initialization_outcome(
    state: FirstRunState,
    error_code: FirstRunErrorCode,
    created_directories: usize,
    already_present_directories: usize,
    metadata_status: FirstRunStoreStatus,
) -> FirstRunInitializationOutcome {
    let failed_state = transition_first_run(state, FirstRunEvent::Fail(error_code))
        .map(|transition| transition.next_state)
        .unwrap_or(FirstRunState::Failed {
            error_code,
            retryable: true,
        });

    FirstRunInitializationOutcome {
        final_state: failed_state,
        product_event: FirstRunProductEvent::FirstRunFailed { error_code },
        created_directories,
        already_present_directories,
        metadata_status,
    }
}
