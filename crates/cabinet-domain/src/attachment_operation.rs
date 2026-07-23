#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentOperationState {
    Selected,
    Validating,
    Staging,
    Hashing,
    PublishingObject,
    PersistingMetadata,
    PreparingRevision,
    Associating,
    Projecting,
    Verifying,
    Completed,
    Cancelled,
    Failed,
    Conflict,
    RecoveryRequired,
}

impl AttachmentOperationState {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentOperationEvent {
    ValidationRequested,
    ValidationSucceeded,
    ValidationFailed,
    StagingCompleted,
    StagingFailed,
    HashCompleted,
    HashFailed,
    ObjectPublished,
    ObjectPublishFailed,
    MetadataPersisted,
    MetadataPersistFailed,
    RevisionPrepared,
    RevisionConflict,
    PrimaryCommitSucceeded,
    PrimaryCommitFailed,
    ProjectionCompleted,
    ProjectionFailed,
    ReadbackMatched,
    ReadbackMismatch,
    CancelRequested,
    RetryRequested,
    RepairRequested,
    PermanentFailureRecorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentOperationSideEffect {
    None,
    ValidateSource,
    StageSource,
    HashContent,
    PublishObject,
    PersistMetadata,
    LoadCurrentRevision,
    ReloadCurrentRevision,
    CommitAssociation,
    ProjectAssociation,
    RepairProjection,
    VerifyReadback,
    RepairAndVerify,
    CleanupStaging,
    PersistTerminalResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachmentOperationTransition {
    pub previous_state: AttachmentOperationState,
    pub event: AttachmentOperationEvent,
    pub next_state: AttachmentOperationState,
    pub side_effect: AttachmentOperationSideEffect,
    pub product_log_event: Option<&'static str>,
    pub error_code: Option<&'static str>,
    pub retryable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentOperationError {
    InvalidTransition {
        state: AttachmentOperationState,
        event: AttachmentOperationEvent,
    },
}

pub fn transition_attachment_operation(
    state: AttachmentOperationState,
    event: AttachmentOperationEvent,
) -> Result<AttachmentOperationTransition, AttachmentOperationError> {
    use AttachmentOperationEvent as E;
    use AttachmentOperationSideEffect as Fx;
    use AttachmentOperationState as S;

    let (next_state, side_effect, product_log_event, error_code, retryable) = match (state, event) {
        (S::Selected, E::ValidationRequested) => (
            S::Validating,
            Fx::ValidateSource,
            Some("document.attachment.started"),
            None,
            false,
        ),
        (S::Validating, E::ValidationSucceeded) => success(S::Staging, Fx::StageSource),
        (S::Staging, E::StagingCompleted) => success(S::Hashing, Fx::HashContent),
        (S::Hashing, E::HashCompleted) => success(S::PublishingObject, Fx::PublishObject),
        (S::PublishingObject, E::ObjectPublished) => {
            success(S::PersistingMetadata, Fx::PersistMetadata)
        }
        (S::PersistingMetadata, E::MetadataPersisted) => {
            success(S::PreparingRevision, Fx::LoadCurrentRevision)
        }
        (S::PreparingRevision, E::RevisionPrepared) => {
            success(S::Associating, Fx::CommitAssociation)
        }
        (S::Associating, E::PrimaryCommitSucceeded) => {
            success(S::Projecting, Fx::ProjectAssociation)
        }
        (S::Projecting, E::ProjectionCompleted) => success(S::Verifying, Fx::VerifyReadback),
        (S::Verifying, E::ReadbackMatched) => (
            S::Completed,
            Fx::PersistTerminalResult,
            Some("document.attachment.completed"),
            None,
            false,
        ),
        (
            S::Selected
            | S::Validating
            | S::Staging
            | S::Hashing
            | S::PublishingObject
            | S::PersistingMetadata
            | S::PreparingRevision,
            E::CancelRequested,
        ) => (
            S::Cancelled,
            Fx::CleanupStaging,
            Some("document.attachment.cancelled"),
            None,
            false,
        ),
        (S::Associating | S::Projecting | S::Verifying, E::CancelRequested) => {
            recovery(Fx::RepairAndVerify, "ATTACHMENT_POST_COMMIT_RECOVERY")
        }
        (S::Validating, E::ValidationFailed) => failed("ATTACHMENT_VALIDATION_FAILED"),
        (S::Staging, E::StagingFailed) => failed("ATTACHMENT_STAGING_FAILED"),
        (S::Hashing, E::HashFailed) => failed("ATTACHMENT_HASH_FAILED"),
        (S::PublishingObject, E::ObjectPublishFailed) => failed("ATTACHMENT_OBJECT_PUBLISH_FAILED"),
        (S::PersistingMetadata, E::MetadataPersistFailed) => recovery(
            Fx::RepairAndVerify,
            "ATTACHMENT_METADATA_RECONCILIATION_REQUIRED",
        ),
        (S::PreparingRevision, E::RevisionConflict) => (
            S::Conflict,
            Fx::None,
            Some("document.attachment.failed"),
            Some("DOCUMENT_CURRENT_CONFLICT"),
            true,
        ),
        (S::Associating, E::PrimaryCommitFailed) => recovery(
            Fx::RepairAndVerify,
            "ATTACHMENT_ASSOCIATION_RECOVERY_REQUIRED",
        ),
        (S::Projecting, E::ProjectionFailed) => recovery(
            Fx::RepairProjection,
            "ATTACHMENT_PROJECTION_RECOVERY_REQUIRED",
        ),
        (S::Verifying, E::ReadbackMismatch) => {
            recovery(Fx::RepairAndVerify, "ATTACHMENT_READBACK_MISMATCH")
        }
        (S::Conflict, E::RetryRequested) => (
            S::PreparingRevision,
            Fx::ReloadCurrentRevision,
            None,
            None,
            true,
        ),
        (S::RecoveryRequired, E::RepairRequested) => {
            (S::Projecting, Fx::RepairProjection, None, None, true)
        }
        (S::Completed, E::ReadbackMatched) => replay(S::Completed),
        (S::Cancelled, E::CancelRequested) => replay(S::Cancelled),
        (S::Failed, E::PermanentFailureRecorded) => replay(S::Failed),
        _ => {
            return Err(AttachmentOperationError::InvalidTransition { state, event });
        }
    };

    Ok(AttachmentOperationTransition {
        previous_state: state,
        event,
        next_state,
        side_effect,
        product_log_event,
        error_code,
        retryable,
    })
}

const fn success(
    state: AttachmentOperationState,
    side_effect: AttachmentOperationSideEffect,
) -> (
    AttachmentOperationState,
    AttachmentOperationSideEffect,
    Option<&'static str>,
    Option<&'static str>,
    bool,
) {
    (state, side_effect, None, None, false)
}

const fn failed(
    error_code: &'static str,
) -> (
    AttachmentOperationState,
    AttachmentOperationSideEffect,
    Option<&'static str>,
    Option<&'static str>,
    bool,
) {
    (
        AttachmentOperationState::Failed,
        AttachmentOperationSideEffect::CleanupStaging,
        Some("document.attachment.failed"),
        Some(error_code),
        false,
    )
}

const fn recovery(
    side_effect: AttachmentOperationSideEffect,
    error_code: &'static str,
) -> (
    AttachmentOperationState,
    AttachmentOperationSideEffect,
    Option<&'static str>,
    Option<&'static str>,
    bool,
) {
    (
        AttachmentOperationState::RecoveryRequired,
        side_effect,
        Some("document.attachment.recovery_required"),
        Some(error_code),
        true,
    )
}

const fn replay(
    state: AttachmentOperationState,
) -> (
    AttachmentOperationState,
    AttachmentOperationSideEffect,
    Option<&'static str>,
    Option<&'static str>,
    bool,
) {
    (
        state,
        AttachmentOperationSideEffect::None,
        None,
        None,
        false,
    )
}
