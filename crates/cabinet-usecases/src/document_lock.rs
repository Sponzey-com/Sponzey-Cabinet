use cabinet_domain::document::DocumentId;
use cabinet_domain::document_lock::{
    DocumentLock, DocumentLockError, DocumentLockErrorCode, DocumentLockId, DocumentLockState,
    DocumentLockTimestamp, DocumentLockTransitionContext, transition_document_lock,
};
use cabinet_domain::permission::{Permission, PermissionDecisionResult};
use cabinet_domain::user::UserId;
use cabinet_domain::workspace::WorkspaceId;
use cabinet_ports::document_lock::{
    DocumentLockClock, DocumentLockPermissionCheckError, DocumentLockPermissionChecker,
    DocumentLockRepository, DocumentLockRepositoryError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockDocumentPolicy {
    ttl_millis: u64,
    write_permission: Permission,
    read_permission: Permission,
}

impl LockDocumentPolicy {
    pub const fn new(ttl_millis: u64) -> Result<Self, DocumentLockUsecaseError> {
        if ttl_millis == 0 {
            return Err(DocumentLockUsecaseError::InvalidInput);
        }
        Ok(Self {
            ttl_millis,
            write_permission: Permission::Write,
            read_permission: Permission::Read,
        })
    }

    pub const fn ttl_millis(self) -> u64 {
        self.ttl_millis
    }

    pub const fn write_permission(self) -> Permission {
        self.write_permission
    }

    pub const fn read_permission(self) -> Permission {
        self.read_permission
    }
}

impl Default for LockDocumentPolicy {
    fn default() -> Self {
        Self::new(300_000).expect("default document lock ttl must be non-zero")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockDocumentInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
    lock_id: String,
}

impl LockDocumentInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, document_id: &str, lock_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
            lock_id: lock_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnlockDocumentInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
}

impl UnlockDocumentInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, document_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpireDocumentLockInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
}

impl ExpireDocumentLockInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, document_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetDocumentLockInput {
    actor_user_id: String,
    workspace_id: String,
    document_id: String,
}

impl GetDocumentLockInput {
    pub fn new(actor_user_id: &str, workspace_id: &str, document_id: &str) -> Self {
        Self {
            actor_user_id: actor_user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            document_id: document_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLockOutput {
    status: DocumentLockViewStatus,
    lock: Option<DocumentLock>,
}

impl DocumentLockOutput {
    pub const fn status(&self) -> DocumentLockViewStatus {
        self.status
    }

    pub fn lock(&self) -> Option<&DocumentLock> {
        self.lock.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLockViewStatus {
    Unlocked,
    Locked,
    Expired,
}

impl DocumentLockViewStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unlocked => "unlocked",
            Self::Locked => "locked",
            Self::Expired => "expired",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLockUsecaseError {
    InvalidInput,
    Unauthorized,
    AlreadyLocked,
    NotOwner,
    LockNotFound,
    LockExpired,
    LockNotExpired,
    StorageUnavailable,
    Conflict,
}

impl DocumentLockUsecaseError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "DOCUMENT_LOCK_INVALID_INPUT",
            Self::Unauthorized => "DOCUMENT_LOCK_UNAUTHORIZED",
            Self::AlreadyLocked => "DOCUMENT_LOCK_ALREADY_LOCKED",
            Self::NotOwner => "DOCUMENT_LOCK_NOT_OWNER",
            Self::LockNotFound => "DOCUMENT_LOCK_NOT_FOUND",
            Self::LockExpired => "DOCUMENT_LOCK_EXPIRED",
            Self::LockNotExpired => "DOCUMENT_LOCK_NOT_EXPIRED",
            Self::StorageUnavailable => "DOCUMENT_LOCK_STORAGE_UNAVAILABLE",
            Self::Conflict => "DOCUMENT_LOCK_CONFLICT",
        }
    }

    const fn from_transition_error(error: DocumentLockErrorCode) -> Self {
        match error {
            DocumentLockErrorCode::AlreadyLocked => Self::AlreadyLocked,
            DocumentLockErrorCode::NotOwner => Self::NotOwner,
            DocumentLockErrorCode::Expired => Self::LockExpired,
            DocumentLockErrorCode::NotLocked => Self::LockNotFound,
            DocumentLockErrorCode::NotExpired => Self::LockNotExpired,
        }
    }

    const fn from_repository_error(error: DocumentLockRepositoryError) -> Self {
        match error {
            DocumentLockRepositoryError::StorageUnavailable
            | DocumentLockRepositoryError::CorruptedState => Self::StorageUnavailable,
            DocumentLockRepositoryError::Conflict => Self::Conflict,
        }
    }

    const fn from_permission_error(error: DocumentLockPermissionCheckError) -> Self {
        match error {
            DocumentLockPermissionCheckError::StorageUnavailable => Self::StorageUnavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentLockProductEvent {
    LockAcquired {
        masked_actor_id: String,
        document_id: String,
        lock_id: String,
        state: &'static str,
    },
    LockReleased {
        masked_actor_id: String,
        document_id: String,
        lock_id: String,
        state: &'static str,
    },
    LockExpired {
        masked_actor_id: String,
        document_id: String,
        lock_id: String,
        state: &'static str,
    },
    LockConflict {
        masked_actor_id: String,
        document_id: String,
        error_code: &'static str,
        state: &'static str,
    },
}

impl DocumentLockProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::LockAcquired { .. } => "document.lock.acquired",
            Self::LockReleased { .. } => "document.lock.released",
            Self::LockExpired { .. } => "document.lock.expired",
            Self::LockConflict { .. } => "document.lock.conflict",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLockFieldDebugEvent {
    current_state: &'static str,
    requested_event: &'static str,
    permission_decision: &'static str,
    lock_expiry_status: &'static str,
}

impl DocumentLockFieldDebugEvent {
    pub const fn current_state(&self) -> &'static str {
        self.current_state
    }

    pub const fn requested_event(&self) -> &'static str {
        self.requested_event
    }

    pub const fn permission_decision(&self) -> &'static str {
        self.permission_decision
    }

    pub const fn lock_expiry_status(&self) -> &'static str {
        self.lock_expiry_status
    }
}

pub trait DocumentLockUsecaseLogger {
    fn write_product(&mut self, event: DocumentLockProductEvent);
    fn write_field_debug(&mut self, event: DocumentLockFieldDebugEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockDocumentUsecase {
    policy: LockDocumentPolicy,
}

impl LockDocumentUsecase {
    pub const fn new(policy: LockDocumentPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        self,
        input: LockDocumentInput,
        permission_checker: &impl DocumentLockPermissionChecker,
        repository: &mut impl DocumentLockRepository,
        clock: &impl DocumentLockClock,
        logger: &mut impl DocumentLockUsecaseLogger,
    ) -> Result<DocumentLockOutput, DocumentLockUsecaseError> {
        let ids = ParsedLockDocumentInput::from_lock_input(input)?;
        ensure_document_permission(
            permission_checker,
            logger,
            &ids.actor_user_id,
            &ids.workspace_id,
            &ids.document_id,
            self.policy.write_permission(),
            "lock_requested",
        )?;

        let now = clock.now();
        let current_lock = repository
            .get_document_lock(&ids.workspace_id, &ids.document_id)
            .map_err(DocumentLockUsecaseError::from_repository_error)?;

        if let Some(lock) = current_lock.as_ref() {
            if lock.is_expired_at(now) {
                expire_existing_lock(
                    repository,
                    logger,
                    &ids.workspace_id,
                    lock,
                    &ids.actor_user_id,
                )?;
            }
        }

        let current_lock = current_lock.filter(|lock| !lock.is_expired_at(now));
        let transition = transition_document_lock(DocumentLockTransitionContext::lock_requested(
            current_lock.as_ref(),
            ids.actor_user_id.clone(),
            now,
        ));
        if let Err(failure) = transition {
            let error = DocumentLockUsecaseError::from_transition_error(failure.error_code());
            logger.write_product(DocumentLockProductEvent::LockConflict {
                masked_actor_id: mask_id(ids.actor_user_id.as_str()),
                document_id: ids.document_id.as_str().to_string(),
                error_code: error.code(),
                state: lock_state_label(current_lock.as_ref()),
            });
            logger.write_field_debug(DocumentLockFieldDebugEvent {
                current_state: lock_state_label(current_lock.as_ref()),
                requested_event: "lock_requested",
                permission_decision: "allowed",
                lock_expiry_status: lock_expiry_label(current_lock.as_ref(), now),
            });
            return Err(error);
        }

        let lock = DocumentLock::new(
            ids.lock_id,
            ids.document_id.clone(),
            ids.actor_user_id.clone(),
            now,
            now.plus_millis(self.policy.ttl_millis()),
        )
        .map_err(map_domain_error)?;
        repository
            .save_document_lock(&ids.workspace_id, lock.clone())
            .map_err(DocumentLockUsecaseError::from_repository_error)?;
        logger.write_product(DocumentLockProductEvent::LockAcquired {
            masked_actor_id: mask_id(ids.actor_user_id.as_str()),
            document_id: ids.document_id.as_str().to_string(),
            lock_id: lock.lock_id().as_str().to_string(),
            state: "locked",
        });
        logger.write_field_debug(DocumentLockFieldDebugEvent {
            current_state: "locked",
            requested_event: "lock_requested",
            permission_decision: "allowed",
            lock_expiry_status: "active",
        });

        Ok(DocumentLockOutput {
            status: DocumentLockViewStatus::Locked,
            lock: Some(lock),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnlockDocumentUsecase;

impl UnlockDocumentUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        self,
        input: UnlockDocumentInput,
        permission_checker: &impl DocumentLockPermissionChecker,
        repository: &mut impl DocumentLockRepository,
        clock: &impl DocumentLockClock,
        logger: &mut impl DocumentLockUsecaseLogger,
    ) -> Result<DocumentLockOutput, DocumentLockUsecaseError> {
        let ids = ParsedDocumentInput::from_unlock_input(input)?;
        ensure_document_permission(
            permission_checker,
            logger,
            &ids.actor_user_id,
            &ids.workspace_id,
            &ids.document_id,
            Permission::Write,
            "unlock_requested",
        )?;

        let now = clock.now();
        let current_lock = repository
            .get_document_lock(&ids.workspace_id, &ids.document_id)
            .map_err(DocumentLockUsecaseError::from_repository_error)?;
        let transition = transition_document_lock(DocumentLockTransitionContext::unlock_requested(
            current_lock.as_ref(),
            ids.actor_user_id.clone(),
            now,
        ));
        let lock = match (transition, current_lock.as_ref()) {
            (Ok(_), Some(lock)) => lock,
            (Err(failure), Some(lock))
                if failure.error_code() == DocumentLockErrorCode::Expired =>
            {
                expire_existing_lock(
                    repository,
                    logger,
                    &ids.workspace_id,
                    lock,
                    &ids.actor_user_id,
                )?;
                return Err(log_conflict(
                    logger,
                    &ids.actor_user_id,
                    &ids.document_id,
                    DocumentLockUsecaseError::LockExpired,
                    "unlock_requested",
                    current_lock.as_ref(),
                    now,
                ));
            }
            (Err(failure), _) => {
                return Err(log_conflict(
                    logger,
                    &ids.actor_user_id,
                    &ids.document_id,
                    DocumentLockUsecaseError::from_transition_error(failure.error_code()),
                    "unlock_requested",
                    current_lock.as_ref(),
                    now,
                ));
            }
            (Ok(_), None) => unreachable!("unlock transition cannot succeed without a lock"),
        };

        repository
            .delete_document_lock(&ids.workspace_id, &ids.document_id)
            .map_err(DocumentLockUsecaseError::from_repository_error)?;
        logger.write_product(DocumentLockProductEvent::LockReleased {
            masked_actor_id: mask_id(ids.actor_user_id.as_str()),
            document_id: ids.document_id.as_str().to_string(),
            lock_id: lock.lock_id().as_str().to_string(),
            state: "unlocked",
        });
        logger.write_field_debug(DocumentLockFieldDebugEvent {
            current_state: "unlocked",
            requested_event: "unlock_requested",
            permission_decision: "allowed",
            lock_expiry_status: "active",
        });

        Ok(DocumentLockOutput {
            status: DocumentLockViewStatus::Unlocked,
            lock: None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpireDocumentLockUsecase;

impl ExpireDocumentLockUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        self,
        input: ExpireDocumentLockInput,
        permission_checker: &impl DocumentLockPermissionChecker,
        repository: &mut impl DocumentLockRepository,
        clock: &impl DocumentLockClock,
        logger: &mut impl DocumentLockUsecaseLogger,
    ) -> Result<DocumentLockOutput, DocumentLockUsecaseError> {
        let ids = ParsedDocumentInput::from_expire_input(input)?;
        ensure_document_permission(
            permission_checker,
            logger,
            &ids.actor_user_id,
            &ids.workspace_id,
            &ids.document_id,
            Permission::Write,
            "lock_expired",
        )?;

        let now = clock.now();
        let current_lock = repository
            .get_document_lock(&ids.workspace_id, &ids.document_id)
            .map_err(DocumentLockUsecaseError::from_repository_error)?;
        transition_document_lock(DocumentLockTransitionContext::lock_expired(
            current_lock.as_ref(),
            ids.actor_user_id.clone(),
            now,
        ))
        .map_err(|failure| {
            log_conflict(
                logger,
                &ids.actor_user_id,
                &ids.document_id,
                DocumentLockUsecaseError::from_transition_error(failure.error_code()),
                "lock_expired",
                current_lock.as_ref(),
                now,
            )
        })?;

        let lock = current_lock
            .as_ref()
            .ok_or(DocumentLockUsecaseError::LockNotFound)?;
        expire_existing_lock(
            repository,
            logger,
            &ids.workspace_id,
            lock,
            &ids.actor_user_id,
        )?;
        Ok(DocumentLockOutput {
            status: DocumentLockViewStatus::Expired,
            lock: None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetDocumentLockUsecase;

impl GetDocumentLockUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        self,
        input: GetDocumentLockInput,
        permission_checker: &impl DocumentLockPermissionChecker,
        repository: &mut impl DocumentLockRepository,
        clock: &impl DocumentLockClock,
        logger: &mut impl DocumentLockUsecaseLogger,
    ) -> Result<DocumentLockOutput, DocumentLockUsecaseError> {
        let ids = ParsedDocumentInput::from_get_input(input)?;
        ensure_document_permission(
            permission_checker,
            logger,
            &ids.actor_user_id,
            &ids.workspace_id,
            &ids.document_id,
            Permission::Read,
            "lock_read_requested",
        )?;

        let now = clock.now();
        let current_lock = repository
            .get_document_lock(&ids.workspace_id, &ids.document_id)
            .map_err(DocumentLockUsecaseError::from_repository_error)?;
        let Some(lock) = current_lock else {
            logger.write_field_debug(DocumentLockFieldDebugEvent {
                current_state: "unlocked",
                requested_event: "lock_read_requested",
                permission_decision: "allowed",
                lock_expiry_status: "none",
            });
            return Ok(DocumentLockOutput {
                status: DocumentLockViewStatus::Unlocked,
                lock: None,
            });
        };

        if lock.is_expired_at(now) {
            expire_existing_lock(
                repository,
                logger,
                &ids.workspace_id,
                &lock,
                &ids.actor_user_id,
            )?;
            return Ok(DocumentLockOutput {
                status: DocumentLockViewStatus::Expired,
                lock: None,
            });
        }

        logger.write_field_debug(DocumentLockFieldDebugEvent {
            current_state: "locked",
            requested_event: "lock_read_requested",
            permission_decision: "allowed",
            lock_expiry_status: "active",
        });
        Ok(DocumentLockOutput {
            status: DocumentLockViewStatus::Locked,
            lock: Some(lock),
        })
    }
}

struct ParsedLockDocumentInput {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
    lock_id: DocumentLockId,
}

impl ParsedLockDocumentInput {
    fn from_lock_input(input: LockDocumentInput) -> Result<Self, DocumentLockUsecaseError> {
        Ok(Self {
            actor_user_id: UserId::new(&input.actor_user_id)
                .map_err(|_| DocumentLockUsecaseError::InvalidInput)?,
            workspace_id: WorkspaceId::new(&input.workspace_id)
                .map_err(|_| DocumentLockUsecaseError::InvalidInput)?,
            document_id: DocumentId::new(&input.document_id)
                .map_err(|_| DocumentLockUsecaseError::InvalidInput)?,
            lock_id: DocumentLockId::new(&input.lock_id)
                .map_err(|_| DocumentLockUsecaseError::InvalidInput)?,
        })
    }
}

struct ParsedDocumentInput {
    actor_user_id: UserId,
    workspace_id: WorkspaceId,
    document_id: DocumentId,
}

impl ParsedDocumentInput {
    fn from_unlock_input(input: UnlockDocumentInput) -> Result<Self, DocumentLockUsecaseError> {
        Self::new(input.actor_user_id, input.workspace_id, input.document_id)
    }

    fn from_expire_input(input: ExpireDocumentLockInput) -> Result<Self, DocumentLockUsecaseError> {
        Self::new(input.actor_user_id, input.workspace_id, input.document_id)
    }

    fn from_get_input(input: GetDocumentLockInput) -> Result<Self, DocumentLockUsecaseError> {
        Self::new(input.actor_user_id, input.workspace_id, input.document_id)
    }

    fn new(
        actor_user_id: String,
        workspace_id: String,
        document_id: String,
    ) -> Result<Self, DocumentLockUsecaseError> {
        Ok(Self {
            actor_user_id: UserId::new(&actor_user_id)
                .map_err(|_| DocumentLockUsecaseError::InvalidInput)?,
            workspace_id: WorkspaceId::new(&workspace_id)
                .map_err(|_| DocumentLockUsecaseError::InvalidInput)?,
            document_id: DocumentId::new(&document_id)
                .map_err(|_| DocumentLockUsecaseError::InvalidInput)?,
        })
    }
}

fn ensure_document_permission(
    permission_checker: &impl DocumentLockPermissionChecker,
    logger: &mut impl DocumentLockUsecaseLogger,
    actor_user_id: &UserId,
    workspace_id: &WorkspaceId,
    document_id: &DocumentId,
    permission: Permission,
    requested_event: &'static str,
) -> Result<(), DocumentLockUsecaseError> {
    let decision = permission_checker
        .check_document_permission(actor_user_id, workspace_id, document_id, permission)
        .map_err(DocumentLockUsecaseError::from_permission_error)?;
    let permission_decision = if decision.result() == PermissionDecisionResult::Allowed {
        "allowed"
    } else {
        "denied"
    };
    logger.write_field_debug(DocumentLockFieldDebugEvent {
        current_state: "permission_checked",
        requested_event,
        permission_decision,
        lock_expiry_status: "unknown",
    });
    if decision.result() != PermissionDecisionResult::Allowed {
        let error = DocumentLockUsecaseError::Unauthorized;
        logger.write_product(DocumentLockProductEvent::LockConflict {
            masked_actor_id: mask_id(actor_user_id.as_str()),
            document_id: document_id.as_str().to_string(),
            error_code: error.code(),
            state: "permission_checked",
        });
        return Err(error);
    }
    Ok(())
}

fn expire_existing_lock(
    repository: &mut impl DocumentLockRepository,
    logger: &mut impl DocumentLockUsecaseLogger,
    workspace_id: &WorkspaceId,
    lock: &DocumentLock,
    actor_user_id: &UserId,
) -> Result<(), DocumentLockUsecaseError> {
    repository
        .delete_document_lock(workspace_id, lock.document_id())
        .map_err(DocumentLockUsecaseError::from_repository_error)?;
    logger.write_product(DocumentLockProductEvent::LockExpired {
        masked_actor_id: mask_id(actor_user_id.as_str()),
        document_id: lock.document_id().as_str().to_string(),
        lock_id: lock.lock_id().as_str().to_string(),
        state: "expired",
    });
    Ok(())
}

fn log_conflict(
    logger: &mut impl DocumentLockUsecaseLogger,
    actor_user_id: &UserId,
    document_id: &DocumentId,
    error: DocumentLockUsecaseError,
    requested_event: &'static str,
    current_lock: Option<&DocumentLock>,
    now: DocumentLockTimestamp,
) -> DocumentLockUsecaseError {
    logger.write_product(DocumentLockProductEvent::LockConflict {
        masked_actor_id: mask_id(actor_user_id.as_str()),
        document_id: document_id.as_str().to_string(),
        error_code: error.code(),
        state: lock_state_label(current_lock),
    });
    logger.write_field_debug(DocumentLockFieldDebugEvent {
        current_state: lock_state_label(current_lock),
        requested_event,
        permission_decision: "allowed",
        lock_expiry_status: lock_expiry_label(current_lock, now),
    });
    error
}

fn map_domain_error(error: DocumentLockError) -> DocumentLockUsecaseError {
    match error {
        DocumentLockError::EmptyLockId
        | DocumentLockError::InvalidLockId
        | DocumentLockError::InvalidExpiry => DocumentLockUsecaseError::InvalidInput,
    }
}

fn lock_state_label(lock: Option<&DocumentLock>) -> &'static str {
    match lock.map(|_| DocumentLockState::Locked) {
        Some(DocumentLockState::Locked) => "locked",
        Some(DocumentLockState::Unlocked) | None => "unlocked",
    }
}

fn lock_expiry_label(lock: Option<&DocumentLock>, now: DocumentLockTimestamp) -> &'static str {
    match lock {
        Some(lock) if lock.is_expired_at(now) => "expired",
        Some(_) => "active",
        None => "none",
    }
}

fn mask_id(value: &str) -> String {
    match value.len() {
        0 => "masked:empty".to_string(),
        1..=4 => "masked:short".to_string(),
        len => format!("masked:{}", &value[len - 4..]),
    }
}
