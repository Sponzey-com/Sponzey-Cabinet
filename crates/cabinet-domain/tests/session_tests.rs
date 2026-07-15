use cabinet_domain::session::{
    Session, SessionId, SessionInstant, SessionStatus, SessionValidationFailure,
};
use cabinet_domain::user::UserId;

#[test]
fn session_starts_created_and_activates_before_expiry() {
    let created_at = SessionInstant::from_epoch_seconds(100);
    let expires_at = created_at.checked_add_seconds(60).expect("valid expiry");
    let session = Session::new_created(
        SessionId::new("session-1").expect("valid session id"),
        UserId::new("user-1").expect("valid user id"),
        created_at,
        expires_at,
    )
    .expect("valid created session");

    assert_eq!(session.status(), SessionStatus::Created);

    let active = session
        .activate(SessionInstant::from_epoch_seconds(100))
        .expect("session can activate before expiry");

    assert_eq!(active.status(), SessionStatus::Active);
    assert_eq!(active.user_id().as_str(), "user-1");
    assert!(
        active
            .validate_at(SessionInstant::from_epoch_seconds(159))
            .is_ok()
    );
}

#[test]
fn expired_and_revoked_sessions_do_not_validate_as_active() {
    let active = active_session();

    let expired = active
        .expire(SessionInstant::from_epoch_seconds(160))
        .expect("active session can expire at expiry time");
    let revoked = active.revoke().expect("active session can revoke");

    assert_eq!(
        expired
            .validate_at(SessionInstant::from_epoch_seconds(160))
            .expect_err("expired session must not validate"),
        SessionValidationFailure::Expired
    );
    assert_eq!(
        revoked
            .validate_at(SessionInstant::from_epoch_seconds(120))
            .expect_err("revoked session must not validate"),
        SessionValidationFailure::Revoked
    );
}

#[test]
fn session_rejects_expiry_not_after_created_at() {
    let error = Session::new_created(
        SessionId::new("session-1").expect("valid session id"),
        UserId::new("user-1").expect("valid user id"),
        SessionInstant::from_epoch_seconds(100),
        SessionInstant::from_epoch_seconds(100),
    )
    .expect_err("expiry must be after creation");

    assert_eq!(error.code(), "SESSION_INVALID_EXPIRY");
}

fn active_session() -> Session {
    Session::new_created(
        SessionId::new("session-1").expect("valid session id"),
        UserId::new("user-1").expect("valid user id"),
        SessionInstant::from_epoch_seconds(100),
        SessionInstant::from_epoch_seconds(160),
    )
    .expect("valid session")
    .activate(SessionInstant::from_epoch_seconds(100))
    .expect("activate")
}
