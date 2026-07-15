use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_auth::{
    InMemorySessionStore, LocalCredentialRecord, LocalCredentialVerifier, LocalOpaqueTokenIssuer,
    LocalSessionStore, StaticPasswordHashVerifier,
};
use cabinet_core::server_config::ServerConfigInput;
use cabinet_domain::session::{Session, SessionId, SessionInstant, SessionStatus};
use cabinet_domain::user::{User, UserEmail, UserId, UserLogin, UserProfile, UserTimestamp};
use cabinet_ports::auth::{
    CredentialSecret, CredentialVerifier, SessionLookupKey, SessionStore, SessionStoreError,
    TokenIssuer,
};

#[test]
fn local_credential_verifier_returns_user_only_for_matching_password() {
    let verifier = LocalCredentialVerifier::new(
        vec![LocalCredentialRecord::new(
            UserLogin::new("alice").expect("valid login"),
            "hash:correct-password",
            active_user("user-1", "alice"),
        )],
        StaticPasswordHashVerifier,
    );

    let valid = verifier
        .verify(
            &UserLogin::new("alice").expect("valid login"),
            &CredentialSecret::new("correct-password").expect("valid credential"),
        )
        .expect("verify");
    let invalid = verifier
        .verify(
            &UserLogin::new("alice").expect("valid login"),
            &CredentialSecret::new("wrong-password").expect("valid credential"),
        )
        .expect("verify");

    assert!(valid.is_some());
    assert!(invalid.is_none());
    assert!(!format!("{verifier:?}").contains("hash:correct-password"));
}

#[test]
fn local_token_issuer_uses_config_without_exposing_secret_in_token_or_debug() {
    let config = ServerConfigInput::local_dev_defaults()
        .with_auth_token_secret("0123456789abcdef0123456789abcdef")
        .with_auth_token_byte_length(32)
        .validate()
        .expect("valid config");
    let mut issuer = LocalOpaqueTokenIssuer::new(config.auth().clone());

    let issued = issuer.issue_token().expect("issue token");
    let lookup = issuer.lookup_key_for(issued.token()).expect("lookup key");

    assert_eq!(lookup, *issued.lookup_key());
    assert!(!issued.token().expose_secret().contains("0123456789abcdef"));
    assert!(!format!("{issued:?}").contains("0123456789abcdef"));
    assert!(!format!("{:?}", issued.token()).contains(issued.token().expose_secret()));
}

#[test]
fn in_memory_session_store_creates_updates_and_looks_up_by_lookup_key() {
    let mut store = InMemorySessionStore::default();
    let lookup = SessionLookupKey::new("lookup-1").expect("valid lookup");
    let session = active_session("session-1", "user-1");

    store
        .create_session(lookup.clone(), session.clone())
        .expect("create");

    assert_eq!(
        store
            .get_session(&lookup)
            .expect("lookup")
            .expect("session")
            .status(),
        SessionStatus::Active
    );

    store
        .update_session(&lookup, session.revoke().expect("revoke"))
        .expect("update");

    assert_eq!(
        store
            .get_session(&lookup)
            .expect("lookup")
            .expect("session")
            .status(),
        SessionStatus::Revoked
    );
}

#[test]
fn local_session_store_persists_create_and_update_across_instances() {
    let session_dir = unique_temp_dir("local-session-store-persist");
    let lookup = SessionLookupKey::new("lookup-1").expect("valid lookup");
    let session = active_session("session-1", "user-1");

    {
        let mut store = LocalSessionStore::new(session_dir.clone());
        store
            .create_session(lookup.clone(), session.clone())
            .expect("create session");
    }

    {
        let mut store = LocalSessionStore::new(session_dir.clone());
        let loaded = store
            .get_session(&lookup)
            .expect("lookup")
            .expect("session");
        assert_eq!(loaded.status(), SessionStatus::Active);
        store
            .update_session(&lookup, loaded.revoke().expect("revoke"))
            .expect("update");
    }

    let store = LocalSessionStore::new(session_dir.clone());
    let loaded = store
        .get_session(&lookup)
        .expect("lookup after restart")
        .expect("session after restart");

    assert_eq!(loaded.status(), SessionStatus::Revoked);
    assert!(!format!("{store:?}").contains("lookup-1"));
    cleanup_temp_dir(session_dir);
}

#[test]
fn local_session_store_reports_conflict_and_missing_update_without_raw_lookup_output() {
    let session_dir = unique_temp_dir("local-session-store-conflict");
    let lookup = SessionLookupKey::new("lookup-dup").expect("valid lookup");
    let missing = SessionLookupKey::new("lookup-missing").expect("valid lookup");
    let session = active_session("session-1", "user-1");
    let mut store = LocalSessionStore::new(session_dir.clone());

    store
        .create_session(lookup.clone(), session.clone())
        .expect("create session");
    let duplicate = store
        .create_session(lookup.clone(), session.clone())
        .expect_err("duplicate create must conflict");
    let missing_update = store
        .update_session(&missing, session)
        .expect_err("missing update must fail");

    assert_eq!(duplicate, SessionStoreError::Conflict);
    assert_eq!(missing_update, SessionStoreError::NotFound);
    assert!(!format!("{store:?}").contains("lookup-dup"));
    assert!(!format!("{store:?}").contains("lookup-missing"));
    cleanup_temp_dir(session_dir);
}

#[test]
fn local_session_store_reports_unavailable_for_corrupted_session_file() {
    let session_dir = unique_temp_dir("local-session-store-corrupt");
    let lookup = SessionLookupKey::new("lookup-corrupt").expect("valid lookup");
    let session = active_session("session-1", "user-1");
    let mut store = LocalSessionStore::new(session_dir.clone());
    store
        .create_session(lookup.clone(), session)
        .expect("create session");

    let session_file = fs::read_dir(&session_dir)
        .expect("session dir")
        .next()
        .expect("session file")
        .expect("session file entry")
        .path();
    fs::write(session_file, "not-a-session-file").expect("corrupt session file");

    let error = store
        .get_session(&lookup)
        .expect_err("corrupted session file must fail");

    assert_eq!(error, SessionStoreError::Unavailable);
    cleanup_temp_dir(session_dir);
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

fn unique_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sponzey-cabinet-{name}-{}", std::process::id()));
    cleanup_temp_dir(dir.clone());
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn cleanup_temp_dir(dir: PathBuf) {
    if dir.exists() {
        fs::remove_dir_all(dir).expect("remove temp dir");
    }
}

fn active_session(session_id: &str, user_id: &str) -> Session {
    Session::new_created(
        SessionId::new(session_id).expect("valid session id"),
        UserId::new(user_id).expect("valid user id"),
        SessionInstant::from_epoch_seconds(100),
        SessionInstant::from_epoch_seconds(160),
    )
    .expect("valid session")
    .activate(SessionInstant::from_epoch_seconds(100))
    .expect("activate")
}
