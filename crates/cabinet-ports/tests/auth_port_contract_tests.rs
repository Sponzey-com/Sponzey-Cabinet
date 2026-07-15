use std::collections::HashMap;

use cabinet_domain::session::{Session, SessionId, SessionInstant, SessionStatus};
use cabinet_domain::user::{User, UserEmail, UserId, UserLogin, UserProfile, UserTimestamp};
use cabinet_ports::auth::{
    CredentialSecret, CredentialVerifier, CredentialVerifierError, IssuedSessionToken,
    PresentedSessionToken, SessionClock, SessionIdGenerator, SessionLookupKey, SessionStore,
    SessionStoreError, TokenIssuer, TokenIssuerError,
};

#[derive(Default)]
struct FakeCredentialVerifier {
    users_by_login: HashMap<String, User>,
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
        Ok(self.users_by_login.get(login.as_str()).cloned())
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
            SessionLookupKey::new(&format!("lookup-{}", self.next)).expect("valid lookup"),
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

struct FakeClock;

impl SessionClock for FakeClock {
    fn now(&self) -> SessionInstant {
        SessionInstant::from_epoch_seconds(100)
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

#[test]
fn auth_ports_can_be_replaced_by_fakes_without_raw_secret_debug_output() {
    let mut credentials = FakeCredentialVerifier {
        users_by_login: HashMap::new(),
        accepted_password: "correct-password".to_string(),
    };
    let user = active_user("user-1", "alice");
    credentials
        .users_by_login
        .insert("alice".to_string(), user.clone());
    let mut issuer = FakeTokenIssuer::default();
    let mut store = FakeSessionStore::default();
    let clock = FakeClock;
    let mut ids = FakeSessionIdGenerator::default();

    let verified = credentials
        .verify(
            &UserLogin::new("alice").expect("valid login"),
            &CredentialSecret::new("correct-password").expect("valid password"),
        )
        .expect("verify")
        .expect("known user");
    let issued = issuer.issue_token().expect("issue token");
    let session = Session::new_created(
        SessionId::new(&ids.generate_session_id()).expect("valid session id"),
        verified.id().clone(),
        clock.now(),
        clock.now().checked_add_seconds(60).expect("valid expiry"),
    )
    .expect("valid session")
    .activate(clock.now())
    .expect("activate");

    store
        .create_session(issued.lookup_key().clone(), session)
        .expect("store");

    let loaded = store
        .get_session(issued.lookup_key())
        .expect("lookup")
        .expect("session");

    assert_eq!(loaded.status(), SessionStatus::Active);
    assert!(!format!("{:?}", issued.token()).contains("token-1"));
    assert!(!format!("{:?}", issued.lookup_key()).contains("lookup-1"));
    assert!(
        !format!("{:?}", CredentialSecret::new("correct-password").unwrap())
            .contains("correct-password")
    );
}

fn active_user(user_id: &str, login: &str) -> User {
    User::new(
        UserId::new(user_id).expect("valid user id"),
        UserProfile::new(
            UserLogin::new(login).expect("valid login"),
            UserEmail::new(&format!("{login}@example.com")).expect("valid email"),
            "Test User",
            None,
        )
        .expect("valid profile"),
        UserTimestamp::new("2026-06-25T00:00:00Z").expect("valid timestamp"),
    )
}
