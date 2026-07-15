use cabinet_domain::session::{Session, SessionId, SessionStatus, SessionValidationFailure};
use cabinet_domain::user::{UserId, UserLogin, UserStatus};
use cabinet_ports::auth::{
    CredentialSecret, CredentialVerifier, CredentialVerifierError, PresentedSessionToken,
    SessionClock, SessionIdGenerator, SessionStore, SessionStoreError, TokenIssuer,
    TokenIssuerError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthSessionPolicy {
    session_ttl_seconds: u32,
}

impl AuthSessionPolicy {
    pub fn new(session_ttl_seconds: u32) -> Result<Self, AuthPolicyError> {
        if session_ttl_seconds == 0 {
            return Err(AuthPolicyError::InvalidSessionTtl);
        }
        Ok(Self {
            session_ttl_seconds,
        })
    }

    pub const fn session_ttl_seconds(self) -> u32 {
        self.session_ttl_seconds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthPolicyError {
    InvalidSessionTtl,
}

#[derive(Clone, PartialEq, Eq)]
pub struct AuthenticateUserInput {
    login: String,
    credential: CredentialSecret,
}

impl AuthenticateUserInput {
    pub fn new(login: &str, credential: &str) -> Self {
        Self {
            login: login.to_string(),
            credential: CredentialSecret::new(credential).expect("credential input must be valid"),
        }
    }

    pub fn login(&self) -> &str {
        &self.login
    }
}

impl std::fmt::Debug for AuthenticateUserInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthenticateUserInput")
            .field("login", &self.login)
            .field("credential", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticateUserOutput {
    user_id: String,
    token: PresentedSessionToken,
    session_status: SessionStatus,
}

impl AuthenticateUserOutput {
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn token(&self) -> &PresentedSessionToken {
        &self.token
    }

    pub const fn session_status(&self) -> SessionStatus {
        self.session_status
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ValidateSessionInput {
    token: PresentedSessionToken,
}

impl ValidateSessionInput {
    pub fn new(token: &str) -> Self {
        Self {
            token: PresentedSessionToken::new(token).expect("session token input must be valid"),
        }
    }

    pub fn token(&self) -> &PresentedSessionToken {
        &self.token
    }
}

impl std::fmt::Debug for ValidateSessionInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ValidateSessionInput")
            .field("token", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedActor {
    user_id: String,
}

impl AuthenticatedActor {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
        }
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateSessionOutput {
    actor: AuthenticatedActor,
    session_status: SessionStatus,
}

impl ValidateSessionOutput {
    pub fn new(actor: AuthenticatedActor, session_status: SessionStatus) -> Self {
        Self {
            actor,
            session_status,
        }
    }

    pub fn active(actor: AuthenticatedActor) -> Self {
        Self {
            actor,
            session_status: SessionStatus::Active,
        }
    }

    pub fn actor(&self) -> &AuthenticatedActor {
        &self.actor
    }

    pub const fn session_status(&self) -> SessionStatus {
        self.session_status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthProductEvent {
    LoginSucceeded {
        masked_user_id: String,
    },
    LoginFailed {
        failure_category: &'static str,
        error_code: &'static str,
    },
    SessionCreated {
        masked_user_id: String,
        status: SessionStatus,
    },
    SessionExpired {
        masked_user_id: String,
    },
    SessionRevoked {
        masked_user_id: String,
    },
    SessionValidationFailed {
        failure_category: &'static str,
        error_code: &'static str,
    },
}

impl AuthProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::LoginSucceeded { .. } => "auth.login.succeeded",
            Self::LoginFailed { .. } => "auth.login.failed",
            Self::SessionCreated { .. } => "auth.session.created",
            Self::SessionExpired { .. } => "session.expired",
            Self::SessionRevoked { .. } => "session.revoked",
            Self::SessionValidationFailed { .. } => "auth.session.validation_failed",
        }
    }
}

pub trait AuthProductLogger {
    fn write_product(&mut self, event: AuthProductEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticateUserUsecase {
    policy: AuthSessionPolicy,
}

impl AuthenticateUserUsecase {
    pub const fn new(policy: AuthSessionPolicy) -> Self {
        Self { policy }
    }

    pub fn execute(
        &self,
        input: AuthenticateUserInput,
        credential_verifier: &mut impl CredentialVerifier,
        token_issuer: &mut impl TokenIssuer,
        session_store: &mut impl SessionStore,
        clock: &impl SessionClock,
        id_generator: &mut impl SessionIdGenerator,
        product_logger: &mut impl AuthProductLogger,
    ) -> Result<AuthenticateUserOutput, AuthError> {
        let login = match UserLogin::new(&input.login) {
            Ok(login) => login,
            Err(_) => {
                log_login_failed(product_logger, AuthError::InvalidCredential);
                return Err(AuthError::InvalidCredential);
            }
        };

        let Some(user) = credential_verifier
            .verify(&login, &input.credential)
            .map_err(|error| {
                let mapped = AuthError::from_credential_error(error);
                log_login_failed(product_logger, mapped);
                mapped
            })?
        else {
            log_login_failed(product_logger, AuthError::InvalidCredential);
            return Err(AuthError::InvalidCredential);
        };

        if user.status() != UserStatus::Active {
            log_login_failed(product_logger, AuthError::UserNotActive);
            return Err(AuthError::UserNotActive);
        }

        let issued_token = token_issuer.issue_token().map_err(|error| {
            let mapped = AuthError::from_token_error(error);
            log_login_failed(product_logger, mapped);
            mapped
        })?;
        let session_id = SessionId::new(&id_generator.generate_session_id()).map_err(|_| {
            log_login_failed(product_logger, AuthError::InvalidSession);
            AuthError::InvalidSession
        })?;
        let now = clock.now();
        let expires_at = now
            .checked_add_seconds(self.policy.session_ttl_seconds())
            .ok_or_else(|| {
                log_login_failed(product_logger, AuthError::InvalidSession);
                AuthError::InvalidSession
            })?;
        let created_session = Session::new_created(session_id, user.id().clone(), now, expires_at)
            .map_err(|_| {
                log_login_failed(product_logger, AuthError::InvalidSession);
                AuthError::InvalidSession
            })?;
        let session = created_session.activate(now).map_err(|_| {
            log_login_failed(product_logger, AuthError::InvalidSession);
            AuthError::InvalidSession
        })?;

        session_store
            .create_session(issued_token.lookup_key().clone(), session.clone())
            .map_err(|error| {
                let mapped = AuthError::from_session_store_error(error);
                log_login_failed(product_logger, mapped);
                mapped
            })?;

        let masked_user_id = mask_user_id(session.user_id());
        product_logger.write_product(AuthProductEvent::SessionCreated {
            masked_user_id: masked_user_id.clone(),
            status: session.status(),
        });
        product_logger.write_product(AuthProductEvent::LoginSucceeded {
            masked_user_id: masked_user_id.clone(),
        });

        Ok(AuthenticateUserOutput {
            user_id: user.id().as_str().to_string(),
            token: issued_token.into_token(),
            session_status: session.status(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidateSessionUsecase;

impl ValidateSessionUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: ValidateSessionInput,
        token_issuer: &impl TokenIssuer,
        session_store: &mut impl SessionStore,
        clock: &impl SessionClock,
        product_logger: &mut impl AuthProductLogger,
    ) -> Result<ValidateSessionOutput, AuthError> {
        let lookup_key = token_issuer
            .lookup_key_for(input.token())
            .map_err(|error| {
                let mapped = AuthError::from_token_error(error);
                product_logger.write_product(AuthProductEvent::SessionValidationFailed {
                    failure_category: mapped.failure_category(),
                    error_code: mapped.code(),
                });
                mapped
            })?;
        let Some(session) = session_store.get_session(&lookup_key).map_err(|error| {
            let mapped = AuthError::from_session_store_error(error);
            product_logger.write_product(AuthProductEvent::SessionValidationFailed {
                failure_category: mapped.failure_category(),
                error_code: mapped.code(),
            });
            mapped
        })?
        else {
            product_logger.write_product(AuthProductEvent::SessionValidationFailed {
                failure_category: AuthError::SessionMissing.failure_category(),
                error_code: AuthError::SessionMissing.code(),
            });
            return Err(AuthError::SessionMissing);
        };

        match session.validate_at(clock.now()) {
            Ok(()) => Ok(ValidateSessionOutput {
                actor: AuthenticatedActor::new(session.user_id().as_str()),
                session_status: session.status(),
            }),
            Err(SessionValidationFailure::Expired) => {
                let expired = if session.status() == SessionStatus::Expired {
                    session.clone()
                } else {
                    session.expire(clock.now()).unwrap_or(session.clone())
                };
                let _ = session_store.update_session(&lookup_key, expired.clone());
                product_logger.write_product(AuthProductEvent::SessionExpired {
                    masked_user_id: mask_user_id(expired.user_id()),
                });
                Err(AuthError::SessionExpired)
            }
            Err(SessionValidationFailure::Revoked) => {
                product_logger.write_product(AuthProductEvent::SessionRevoked {
                    masked_user_id: mask_user_id(session.user_id()),
                });
                Err(AuthError::SessionRevoked)
            }
            Err(SessionValidationFailure::NotActive) => {
                product_logger.write_product(AuthProductEvent::SessionValidationFailed {
                    failure_category: AuthError::InvalidSession.failure_category(),
                    error_code: AuthError::InvalidSession.code(),
                });
                Err(AuthError::InvalidSession)
            }
        }
    }
}

impl Default for ValidateSessionUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    InvalidCredential,
    UserNotActive,
    SessionMissing,
    SessionExpired,
    SessionRevoked,
    SessionStoreUnavailable,
    TokenUnavailable,
    InvalidToken,
    InvalidSession,
}

impl AuthError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidCredential => "AUTH_INVALID_CREDENTIAL",
            Self::UserNotActive => "AUTH_USER_NOT_ACTIVE",
            Self::SessionMissing => "AUTH_SESSION_MISSING",
            Self::SessionExpired => "AUTH_SESSION_EXPIRED",
            Self::SessionRevoked => "AUTH_SESSION_REVOKED",
            Self::SessionStoreUnavailable => "AUTH_SESSION_STORE_UNAVAILABLE",
            Self::TokenUnavailable => "AUTH_TOKEN_UNAVAILABLE",
            Self::InvalidToken => "AUTH_INVALID_TOKEN",
            Self::InvalidSession => "AUTH_INVALID_SESSION",
        }
    }

    pub const fn failure_category(self) -> &'static str {
        match self {
            Self::InvalidCredential => "invalid_credential",
            Self::UserNotActive => "user_not_active",
            Self::SessionMissing => "session_missing",
            Self::SessionExpired => "session_expired",
            Self::SessionRevoked => "session_revoked",
            Self::SessionStoreUnavailable => "session_store_unavailable",
            Self::TokenUnavailable => "token_unavailable",
            Self::InvalidToken => "invalid_token",
            Self::InvalidSession => "invalid_session",
        }
    }

    const fn from_credential_error(error: CredentialVerifierError) -> Self {
        match error {
            CredentialVerifierError::Unavailable => Self::SessionStoreUnavailable,
        }
    }

    const fn from_token_error(error: TokenIssuerError) -> Self {
        match error {
            TokenIssuerError::Unavailable => Self::TokenUnavailable,
            TokenIssuerError::InvalidToken => Self::InvalidToken,
        }
    }

    const fn from_session_store_error(error: SessionStoreError) -> Self {
        match error {
            SessionStoreError::Conflict
            | SessionStoreError::NotFound
            | SessionStoreError::Unavailable => Self::SessionStoreUnavailable,
        }
    }
}

fn log_login_failed(product_logger: &mut impl AuthProductLogger, error: AuthError) {
    product_logger.write_product(AuthProductEvent::LoginFailed {
        failure_category: error.failure_category(),
        error_code: error.code(),
    });
}

fn mask_user_id(user_id: &UserId) -> String {
    format!("masked:{}", user_id.as_str())
}
