#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetImportState {
    Selected,
    Validating,
    Staging,
    Hashing,
    PublishingObject,
    PersistingMetadata,
    Linking,
    Completed,
    ValidationFailed,
    StagingFailed,
    ObjectPublishFailed,
    MetadataPersistFailed,
    LinkFailed,
    Cancelling,
    Cancelled,
    CleanupRequired,
}

impl AssetImportState {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed
                | Self::ValidationFailed
                | Self::StagingFailed
                | Self::ObjectPublishFailed
                | Self::MetadataPersistFailed
                | Self::LinkFailed
                | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetImportEvent {
    Begin,
    ValidationSucceeded,
    ValidationFailed,
    StagingSucceeded,
    StagingFailed,
    HashingSucceeded,
    HashingFailed,
    ObjectPublished,
    ObjectPublishFailed,
    MetadataPersisted,
    MetadataPersistFailed,
    LinkSucceeded,
    LinkFailed,
    CancelRequested,
    CleanupSucceeded,
    CleanupFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetImportSideEffect {
    None,
    Validate,
    Stage,
    Hash,
    PublishObject,
    PersistMetadata,
    Link,
    CleanupStaging,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetImportTransition {
    pub previous_state: AssetImportState,
    pub event: AssetImportEvent,
    pub next_state: AssetImportState,
    pub side_effect: AssetImportSideEffect,
    pub product_log_event: Option<&'static str>,
    pub error_code: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetImportOperationError {
    InvalidOperationId,
    InvalidProgress,
    InvalidTransition {
        state: AssetImportState,
        event: AssetImportEvent,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetImportOperation {
    operation_id: AssetImportOperationId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    state: AssetImportState,
    attempt: u32,
    completed_bytes: u64,
    total_bytes: u64,
}

impl AssetImportOperation {
    pub fn new(
        operation_id: AssetImportOperationId,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        total_bytes: u64,
    ) -> Result<Self, AssetImportOperationError> {
        Self::restore(
            operation_id,
            workspace_id,
            document_id,
            AssetImportState::Selected,
            0,
            0,
            total_bytes,
        )
    }

    pub fn restore(
        operation_id: AssetImportOperationId,
        workspace_id: WorkspaceId,
        document_id: DocumentId,
        state: AssetImportState,
        attempt: u32,
        completed_bytes: u64,
        total_bytes: u64,
    ) -> Result<Self, AssetImportOperationError> {
        if total_bytes == 0 || completed_bytes > total_bytes {
            return Err(AssetImportOperationError::InvalidProgress);
        }
        Ok(Self {
            operation_id,
            workspace_id,
            document_id,
            state,
            attempt,
            completed_bytes,
            total_bytes,
        })
    }

    pub fn apply(
        &mut self,
        event: AssetImportEvent,
        completed_bytes: u64,
    ) -> Result<AssetImportTransition, AssetImportOperationError> {
        if completed_bytes > self.total_bytes || completed_bytes < self.completed_bytes {
            return Err(AssetImportOperationError::InvalidProgress);
        }
        let transition = transition_asset_import(self.state, event)?;
        self.state = transition.next_state;
        self.completed_bytes = completed_bytes;
        if event == AssetImportEvent::Begin {
            self.attempt = self.attempt.saturating_add(1);
        }
        Ok(transition)
    }

    pub fn operation_id(&self) -> &AssetImportOperationId {
        &self.operation_id
    }
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }
    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }
    pub fn state(&self) -> AssetImportState {
        self.state
    }
    pub fn attempt(&self) -> u32 {
        self.attempt
    }
    pub fn completed_bytes(&self) -> u64 {
        self.completed_bytes
    }
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}

pub fn transition_asset_import(
    state: AssetImportState,
    event: AssetImportEvent,
) -> Result<AssetImportTransition, AssetImportOperationError> {
    use AssetImportEvent as E;
    use AssetImportSideEffect as Fx;
    use AssetImportState as S;

    let (next_state, side_effect, product_log_event, error_code) = match (state, event) {
        (S::Selected, E::Begin) => (S::Validating, Fx::Validate, None, None),
        (S::Validating, E::ValidationSucceeded) => (S::Staging, Fx::Stage, None, None),
        (S::Staging, E::StagingSucceeded) => (S::Hashing, Fx::Hash, None, None),
        (S::Hashing, E::HashingSucceeded) => (S::PublishingObject, Fx::PublishObject, None, None),
        (S::PublishingObject, E::ObjectPublished) => {
            (S::PersistingMetadata, Fx::PersistMetadata, None, None)
        }
        (S::PersistingMetadata, E::MetadataPersisted) => (S::Linking, Fx::Link, None, None),
        (S::Linking, E::LinkSucceeded) => {
            (S::Completed, Fx::None, Some("asset.import.completed"), None)
        }
        (S::Validating, E::ValidationFailed) => {
            failure(S::ValidationFailed, "asset.import.validation_failed")
        }
        (S::Staging, E::StagingFailed) | (S::Hashing, E::HashingFailed) => {
            failure(S::StagingFailed, "asset.import.staging_failed")
        }
        (S::PublishingObject, E::ObjectPublishFailed) => {
            failure(S::ObjectPublishFailed, "asset.import.object_publish_failed")
        }
        (S::PersistingMetadata, E::MetadataPersistFailed) => failure(
            S::MetadataPersistFailed,
            "asset.import.metadata_persist_failed",
        ),
        (S::Linking, E::LinkFailed) => failure(S::LinkFailed, "asset.import.link_failed"),
        (S::Selected | S::Validating | S::Staging | S::Hashing, E::CancelRequested) => {
            (S::Cancelling, Fx::CleanupStaging, None, None)
        }
        (S::PublishingObject | S::PersistingMetadata | S::Linking, E::CancelRequested) => (
            S::CleanupRequired,
            Fx::None,
            Some("asset.import.failed"),
            Some("asset.import.cleanup_required"),
        ),
        (S::Cancelling | S::CleanupRequired, E::CleanupSucceeded) => {
            (S::Cancelled, Fx::None, None, None)
        }
        (S::Cancelling | S::CleanupRequired, E::CleanupFailed) => (
            S::CleanupRequired,
            Fx::CleanupStaging,
            Some("asset.import.failed"),
            Some("asset.import.cleanup_required"),
        ),
        _ => return Err(AssetImportOperationError::InvalidTransition { state, event }),
    };

    Ok(AssetImportTransition {
        previous_state: state,
        event,
        next_state,
        side_effect,
        product_log_event,
        error_code,
    })
}

fn failure(
    state: AssetImportState,
    error_code: &'static str,
) -> (
    AssetImportState,
    AssetImportSideEffect,
    Option<&'static str>,
    Option<&'static str>,
) {
    (
        state,
        AssetImportSideEffect::None,
        Some("asset.import.failed"),
        Some(error_code),
    )
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetImportOperationId(String);

impl AssetImportOperationId {
    pub fn new(value: &str) -> Result<Self, AssetImportOperationError> {
        let value = value.trim();
        if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
            return Err(AssetImportOperationError::InvalidOperationId);
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
use crate::document::DocumentId;
use crate::workspace::WorkspaceId;
