use crate::document::DocumentId;
use crate::user::UserId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLock {
    lock_id: DocumentLockId,
    document_id: DocumentId,
    owner_user_id: UserId,
    acquired_at: DocumentLockTimestamp,
    expires_at: DocumentLockTimestamp,
}

impl DocumentLock {
    pub fn new(
        lock_id: DocumentLockId,
        document_id: DocumentId,
        owner_user_id: UserId,
        acquired_at: DocumentLockTimestamp,
        expires_at: DocumentLockTimestamp,
    ) -> Result<Self, DocumentLockError> {
        if expires_at <= acquired_at {
            return Err(DocumentLockError::InvalidExpiry);
        }
        Ok(Self {
            lock_id,
            document_id,
            owner_user_id,
            acquired_at,
            expires_at,
        })
    }

    pub fn lock_id(&self) -> &DocumentLockId {
        &self.lock_id
    }

    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub fn owner_user_id(&self) -> &UserId {
        &self.owner_user_id
    }

    pub const fn acquired_at(&self) -> DocumentLockTimestamp {
        self.acquired_at
    }

    pub const fn expires_at(&self) -> DocumentLockTimestamp {
        self.expires_at
    }

    pub fn is_owned_by(&self, actor_user_id: &UserId) -> bool {
        &self.owner_user_id == actor_user_id
    }

    pub const fn is_expired_at(&self, now: DocumentLockTimestamp) -> bool {
        now.millis_since_epoch >= self.expires_at.millis_since_epoch
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLockId {
    value: String,
}

impl DocumentLockId {
    pub fn new(value: &str) -> Result<Self, DocumentLockError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(DocumentLockError::EmptyLockId);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(DocumentLockError::InvalidLockId);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentLockTimestamp {
    millis_since_epoch: u64,
}

impl DocumentLockTimestamp {
    pub const fn from_millis(millis_since_epoch: u64) -> Self {
        Self { millis_since_epoch }
    }

    pub const fn as_millis(self) -> u64 {
        self.millis_since_epoch
    }

    pub const fn plus_millis(self, millis: u64) -> Self {
        Self {
            millis_since_epoch: self.millis_since_epoch.saturating_add(millis),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLockState {
    Unlocked,
    Locked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLockEvent {
    LockRequested,
    UnlockRequested,
    LockExpired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentLockErrorCode {
    AlreadyLocked,
    NotOwner,
    Expired,
    NotLocked,
    NotExpired,
}

impl DocumentLockErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlreadyLocked => "DOCUMENT_LOCK_ALREADY_LOCKED",
            Self::NotOwner => "DOCUMENT_LOCK_NOT_OWNER",
            Self::Expired => "DOCUMENT_LOCK_EXPIRED",
            Self::NotLocked => "DOCUMENT_LOCK_NOT_FOUND",
            Self::NotExpired => "DOCUMENT_LOCK_NOT_EXPIRED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentLockError {
    EmptyLockId,
    InvalidLockId,
    InvalidExpiry,
}

#[derive(Debug, Clone)]
pub struct DocumentLockTransitionContext<'lock> {
    current_lock: Option<&'lock DocumentLock>,
    actor_user_id: UserId,
    now: DocumentLockTimestamp,
    event: DocumentLockEvent,
}

impl<'lock> DocumentLockTransitionContext<'lock> {
    pub fn lock_requested(
        current_lock: Option<&'lock DocumentLock>,
        actor_user_id: UserId,
        now: DocumentLockTimestamp,
    ) -> Self {
        Self {
            current_lock,
            actor_user_id,
            now,
            event: DocumentLockEvent::LockRequested,
        }
    }

    pub fn unlock_requested(
        current_lock: Option<&'lock DocumentLock>,
        actor_user_id: UserId,
        now: DocumentLockTimestamp,
    ) -> Self {
        Self {
            current_lock,
            actor_user_id,
            now,
            event: DocumentLockEvent::UnlockRequested,
        }
    }

    pub fn lock_expired(
        current_lock: Option<&'lock DocumentLock>,
        actor_user_id: UserId,
        now: DocumentLockTimestamp,
    ) -> Self {
        Self {
            current_lock,
            actor_user_id,
            now,
            event: DocumentLockEvent::LockExpired,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentLockTransition {
    previous_state: DocumentLockState,
    next_state: DocumentLockState,
    event: DocumentLockEvent,
}

impl DocumentLockTransition {
    pub const fn previous_state(self) -> DocumentLockState {
        self.previous_state
    }

    pub const fn next_state(self) -> DocumentLockState {
        self.next_state
    }

    pub const fn event(self) -> DocumentLockEvent {
        self.event
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentLockTransitionFailure {
    current_state: DocumentLockState,
    event: DocumentLockEvent,
    error_code: DocumentLockErrorCode,
}

impl DocumentLockTransitionFailure {
    pub const fn current_state(self) -> DocumentLockState {
        self.current_state
    }

    pub const fn event(self) -> DocumentLockEvent {
        self.event
    }

    pub const fn error_code(self) -> DocumentLockErrorCode {
        self.error_code
    }
}

pub fn transition_document_lock(
    context: DocumentLockTransitionContext<'_>,
) -> Result<DocumentLockTransition, DocumentLockTransitionFailure> {
    let current_state = match context.current_lock {
        Some(_) => DocumentLockState::Locked,
        None => DocumentLockState::Unlocked,
    };

    match context.event {
        DocumentLockEvent::LockRequested => match context.current_lock {
            Some(lock) if !lock.is_expired_at(context.now) => Err(failure(
                current_state,
                context.event,
                DocumentLockErrorCode::AlreadyLocked,
            )),
            Some(_) | None => Ok(success(
                current_state,
                DocumentLockState::Locked,
                context.event,
            )),
        },
        DocumentLockEvent::UnlockRequested => {
            let Some(lock) = context.current_lock else {
                return Err(failure(
                    current_state,
                    context.event,
                    DocumentLockErrorCode::NotLocked,
                ));
            };
            if lock.is_expired_at(context.now) {
                return Err(failure(
                    current_state,
                    context.event,
                    DocumentLockErrorCode::Expired,
                ));
            }
            if !lock.is_owned_by(&context.actor_user_id) {
                return Err(failure(
                    current_state,
                    context.event,
                    DocumentLockErrorCode::NotOwner,
                ));
            }
            Ok(success(
                DocumentLockState::Locked,
                DocumentLockState::Unlocked,
                context.event,
            ))
        }
        DocumentLockEvent::LockExpired => {
            let Some(lock) = context.current_lock else {
                return Err(failure(
                    current_state,
                    context.event,
                    DocumentLockErrorCode::NotLocked,
                ));
            };
            if !lock.is_expired_at(context.now) {
                return Err(failure(
                    current_state,
                    context.event,
                    DocumentLockErrorCode::NotExpired,
                ));
            }
            Ok(success(
                DocumentLockState::Locked,
                DocumentLockState::Unlocked,
                context.event,
            ))
        }
    }
}

const fn success(
    previous_state: DocumentLockState,
    next_state: DocumentLockState,
    event: DocumentLockEvent,
) -> DocumentLockTransition {
    DocumentLockTransition {
        previous_state,
        next_state,
        event,
    }
}

const fn failure(
    current_state: DocumentLockState,
    event: DocumentLockEvent,
    error_code: DocumentLockErrorCode,
) -> DocumentLockTransitionFailure {
    DocumentLockTransitionFailure {
        current_state,
        event,
        error_code,
    }
}
