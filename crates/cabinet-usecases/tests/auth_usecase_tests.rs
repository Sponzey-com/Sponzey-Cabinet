use std::collections::HashMap;

use cabinet_domain::session::{Session, SessionId, SessionInstant, SessionStatus};
use cabinet_domain::user::{
    User, UserEmail, UserId, UserLogin, UserProfile, UserStatus, UserTimestamp,
};
use cabinet_ports::auth::{
    CredentialSecret, CredentialVerifier, CredentialVerifierError, IssuedSessionToken,
    PresentedSessionToken, SessionClock, SessionIdGenerator, SessionLookupKey, SessionStore,
    SessionStoreError, TokenIssuer, TokenIssuerError,
};
use cabinet_usecases::auth::{
    AuthError, AuthProductEvent, AuthProductLogger, AuthSessionPolicy, AuthenticateUserInput,
    AuthenticateUserUsecase, ValidateSessionInput, ValidateSessionUsecase,
};

#[derive(Default)]
struct FakeCredentialVerifier {
    users: HashMap<String, User>,
    accepted_password: String,
}

impl CredentialVerifier for FakeCredentialVerifier {
    fn verify(
        &self,
        login: &UserLogin,
        credential: &CredentialSecret,
    ) -> Result<Option<User>, CredentialVerifierError> {
        if credential.expose_secret() != self.accepted_password {
            return Ok(None);
        }
        Ok(self.users.get(login.as_str()).cloned())
    }
}

#[derive(Default)]
struct FakeTokenIssuer {
    next: u32,
}

impl TokenIssuer for FakeTokenIssuer {
    fn issue_token(&mut self) -> Result<IssuedSessionToken, TokenIssuerError> {
        self.next += 1;
        Ok(IssuedSessionToken::new(
            PresentedSessionToken::new(&format!("token-{}", self.next)).expect("valid token"),
            SessionLookupKey::new(&format!("lookup-{}", self.next)).expect("valid lookup key"),
        ))
    }

    fn lookup_key_for(
        &self,
        token: &PresentedSessionToken,
    ) -> Result<SessionLookupKey, TokenIssuerError> {
        SessionLookupKey::new(&format!(
            "lookup-{}",
            token.expose_secret().replace("token-", "")
        ))
        .map_err(|_| TokenIssuerError::InvalidToken)
    }
}

#[derive(Default)]
struct FakeSessionStore {
    sessions: HashMap<String, Session>,
}

impl SessionStore for FakeSessionStore {
    fn create_session(
        &mut self,
        lookup_key: SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError> {
        self.sessions
            .insert(lookup_key.as_str().to_string(), session);
        Ok(())
    }

    fn get_session(
        &self,
        lookup_key: &SessionLookupKey,
    ) -> Result<Option<Session>, SessionStoreError> {
        Ok(self.sessions.get(lookup_key.as_str()).cloned())
    }

    fn update_session(
        &mut self,
        lookup_key: &SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError> {
        self.sessions
            .insert(lookup_key.as_str().to_string(), session);
        Ok(())
    }
}

struct FakeClock {
    now: SessionInstant,
}

impl FakeClock {
    fn at(seconds: u64) -> Self {
        Self {
            now: SessionInstant::from_epoch_seconds(seconds),
        }
    }
}

impl SessionClock for FakeClock {
    fn now(&self) -> SessionInstant {
        self.now
    }
}

#[derive(Default)]
struct FakeSessionIdGenerator {
    next: u32,
}

impl SessionIdGenerator for FakeSessionIdGenerator {
    fn generate_session_id(&mut self) -> String {
        self.next += 1;
        format!("session-{}", self.next)
    }
}

#[derive(Default)]
struct FakeProductLogger {
    events: Vec<AuthProductEvent>,
}

impl AuthProductLogger for FakeProductLogger {
    fn write_product(&mut self, event: AuthProductEvent) {
        self.events.push(event);
    }
}

#[test]
fn authenticate_active_user_creates_active_session_and_returns_token() {
    let mut credentials = credentials_with_user(active_user("user-1", "alice"));
    let mut issuer = FakeTokenIssuer::default();
    let mut store = FakeSessionStore::default();
    let clock = FakeClock::at(100);
    let mut ids = FakeSessionIdGenerator::default();
    let mut logger = FakeProductLogger::default();

    let output = AuthenticateUserUsecase::new(AuthSessionPolicy::new(60).expect("valid policy"))
        .execute(
            AuthenticateUserInput::new("alice", "correct-password"),
            &mut credentials,
            &mut issuer,
            &mut store,
            &clock,
            &mut ids,
            &mut logger,
        )
        .expect("auth succeeds");

    assert_eq!(output.user_id(), "user-1");
    assert_eq!(output.session_status(), SessionStatus::Active);
    assert_eq!(output.token().expose_secret(), "token-1");
    assert_eq!(
        store
            .sessions
            .get("lookup-1")
            .expect("stored session")
            .status(),
        SessionStatus::Active
    );
    assert_eq!(
        logger.events,
        vec![
            AuthProductEvent::SessionCreated {
                masked_user_id: "masked:user-1".to_string(),
                status: SessionStatus::Active,
            },
            AuthProductEvent::LoginSucceeded {
                masked_user_id: "masked:user-1".to_string(),
            },
        ]
    );
}

#[test]
fn authenticate_rejects_invalid_credentials_without_session_or_token() {
    let mut credentials = credentials_with_user(active_user("user-1", "alice"));
    let mut issuer = FakeTokenIssuer::default();
    let mut store = FakeSessionStore::default();
    let clock = FakeClock::at(100);
    let mut ids = FakeSessionIdGenerator::default();
    let mut logger = FakeProductLogger::default();

    let error = AuthenticateUserUsecase::new(AuthSessionPolicy::new(60).expect("valid policy"))
        .execute(
            AuthenticateUserInput::new("alice", "wrong-password"),
            &mut credentials,
            &mut issuer,
            &mut store,
            &clock,
            &mut ids,
            &mut logger,
        )
        .expect_err("invalid credential must fail");

    assert_eq!(error, AuthError::InvalidCredential);
    assert!(store.sessions.is_empty());
    assert_eq!(issuer.next, 0);
    assert_eq!(
        logger.events,
        vec![AuthProductEvent::LoginFailed {
            failure_category: "invalid_credential",
            error_code: "AUTH_INVALID_CREDENTIAL",
        }]
    );
}

#[test]
fn authenticate_rejects_suspended_and_deleted_users() {
    for status in [UserStatus::Suspended, UserStatus::Deleted] {
        let mut credentials = credentials_with_user(user_with_status("user-1", "alice", status));
        let mut issuer = FakeTokenIssuer::default();
        let mut store = FakeSessionStore::default();
        let clock = FakeClock::at(100);
        let mut ids = FakeSessionIdGenerator::default();
        let mut logger = FakeProductLogger::default();

        let error = AuthenticateUserUsecase::new(AuthSessionPolicy::new(60).expect("valid policy"))
            .execute(
                AuthenticateUserInput::new("alice", "correct-password"),
                &mut credentials,
                &mut issuer,
                &mut store,
                &clock,
                &mut ids,
                &mut logger,
            )
            .expect_err("inactive user must fail");

        assert_eq!(error, AuthError::UserNotActive);
        assert!(store.sessions.is_empty());
    }
}

#[test]
fn validate_session_returns_actor_for_active_unexpired_session() {
    let mut store = FakeSessionStore::default();
    store.sessions.insert(
        "lookup-1".to_string(),
        active_session("session-1", "user-1", 100, 160),
    );
    let issuer = FakeTokenIssuer::default();
    let clock = FakeClock::at(120);
    let mut logger = FakeProductLogger::default();

    let output = ValidateSessionUsecase::new()
        .execute(
            ValidateSessionInput::new("token-1"),
            &issuer,
            &mut store,
            &clock,
            &mut logger,
        )
        .expect("valid session");

    assert_eq!(output.actor().user_id(), "user-1");
    assert_eq!(output.session_status(), SessionStatus::Active);
    assert!(logger.events.is_empty());
}

#[test]
fn validate_session_expires_active_session_after_ttl() {
    let mut store = FakeSessionStore::default();
    store.sessions.insert(
        "lookup-1".to_string(),
        active_session("session-1", "user-1", 100, 160),
    );
    let issuer = FakeTokenIssuer::default();
    let clock = FakeClock::at(160);
    let mut logger = FakeProductLogger::default();

    let error = ValidateSessionUsecase::new()
        .execute(
            ValidateSessionInput::new("token-1"),
            &issuer,
            &mut store,
            &clock,
            &mut logger,
        )
        .expect_err("expired session must fail");

    assert_eq!(error, AuthError::SessionExpired);
    assert_eq!(
        store.sessions.get("lookup-1").expect("session").status(),
        SessionStatus::Expired
    );
    assert_eq!(
        logger.events,
        vec![AuthProductEvent::SessionExpired {
            masked_user_id: "masked:user-1".to_string(),
        }]
    );
}

#[test]
fn validate_session_rejects_revoked_session() {
    let mut store = FakeSessionStore::default();
    store.sessions.insert(
        "lookup-1".to_string(),
        active_session("session-1", "user-1", 100, 160)
            .revoke()
            .expect("revoke"),
    );
    let issuer = FakeTokenIssuer::default();
    let clock = FakeClock::at(120);
    let mut logger = FakeProductLogger::default();

    let error = ValidateSessionUsecase::new()
        .execute(
            ValidateSessionInput::new("token-1"),
            &issuer,
            &mut store,
            &clock,
            &mut logger,
        )
        .expect_err("revoked session must fail");

    assert_eq!(error, AuthError::SessionRevoked);
    assert_eq!(
        logger.events,
        vec![AuthProductEvent::SessionRevoked {
            masked_user_id: "masked:user-1".to_string(),
        }]
    );
}

#[test]
fn auth_product_log_payload_excludes_password_token_and_session_id() {
    let event = AuthProductEvent::LoginFailed {
        failure_category: "invalid_credential",
        error_code: "AUTH_INVALID_CREDENTIAL",
    };
    let rendered = format!("{event:?}");

    assert_eq!(event.event_name(), "auth.login.failed");
    assert!(!rendered.contains("correct-password"));
    assert!(!rendered.contains("token-1"));
    assert!(!rendered.contains("session-1"));
}

fn credentials_with_user(user: User) -> FakeCredentialVerifier {
    let mut users = HashMap::new();
    users.insert(user.profile().login().as_str().to_string(), user);
    FakeCredentialVerifier {
        users,
        accepted_password: "correct-password".to_string(),
    }
}

fn active_user(user_id: &str, login: &str) -> User {
    user_with_status(user_id, login, UserStatus::Active)
}

fn user_with_status(user_id: &str, login: &str, status: UserStatus) -> User {
    let user = User::new(
        UserId::new(user_id).expect("valid user id"),
        UserProfile::new(
            UserLogin::new(login).expect("valid login"),
            UserEmail::new(&format!("{login}@example.com")).expect("valid email"),
            "Test User",
            None,
        )
        .expect("valid profile"),
        UserTimestamp::new("2026-06-25T00:00:00Z").expect("valid timestamp"),
    );
    if status == UserStatus::Active {
        return user;
    }
    user.transition_status(status, UserTimestamp::new("2026-06-25T01:00:00Z").unwrap())
        .expect("status transition")
}

fn active_session(
    session_id: &str,
    user_id: &str,
    created_at_seconds: u64,
    expires_at_seconds: u64,
) -> Session {
    Session::new_created(
        SessionId::new(session_id).expect("valid session id"),
        UserId::new(user_id).expect("valid user id"),
        SessionInstant::from_epoch_seconds(created_at_seconds),
        SessionInstant::from_epoch_seconds(expires_at_seconds),
    )
    .expect("valid session")
    .activate(SessionInstant::from_epoch_seconds(created_at_seconds))
    .expect("activate")
}
