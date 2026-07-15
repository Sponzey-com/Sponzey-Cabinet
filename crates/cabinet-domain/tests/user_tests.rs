use cabinet_domain::user::{
    User, UserEmail, UserExternalIdentity, UserId, UserLogin, UserProfile, UserStatus,
    UserTimestamp,
};

#[test]
fn user_profile_validates_login_email_display_name_and_external_identity() {
    let profile = UserProfile::new(
        UserLogin::new(" alice ").expect("valid login"),
        UserEmail::new("Alice@Example.COM").expect("valid email"),
        " Alice Lee ",
        Some(UserExternalIdentity::new("oidc", "subject-123").expect("valid external identity")),
    )
    .expect("valid profile");

    assert_eq!(profile.login().as_str(), "alice");
    assert_eq!(profile.email().as_str(), "alice@example.com");
    assert_eq!(profile.display_name(), "Alice Lee");
    assert_eq!(
        profile.external_identity().expect("identity").provider(),
        "oidc"
    );
    assert_eq!(
        profile.external_identity().expect("identity").subject(),
        "subject-123"
    );
}

#[test]
fn user_profile_rejects_invalid_values() {
    assert!(UserLogin::new("ab").is_err());
    assert!(UserLogin::new("bad login").is_err());
    assert!(UserEmail::new("not-email").is_err());
    assert!(
        UserProfile::new(
            UserLogin::new("valid-login").expect("valid login"),
            UserEmail::new("valid@example.com").expect("valid email"),
            " ",
            None,
        )
        .is_err()
    );
    assert!(UserExternalIdentity::new(" ", "subject").is_err());
    assert!(UserExternalIdentity::new("oidc", " ").is_err());
}

#[test]
fn user_starts_active_and_allows_suspend_reactivate_and_delete_flow() {
    let user = sample_user();
    assert_eq!(user.status(), UserStatus::Active);

    let suspended = user
        .transition_status(
            UserStatus::Suspended,
            UserTimestamp::new("2026-06-25T01:00:00Z").unwrap(),
        )
        .expect("active can suspend");
    assert_eq!(suspended.status(), UserStatus::Suspended);

    let active = suspended
        .transition_status(
            UserStatus::Active,
            UserTimestamp::new("2026-06-25T02:00:00Z").unwrap(),
        )
        .expect("suspended can reactivate");
    assert_eq!(active.status(), UserStatus::Active);

    let deleted = active
        .transition_status(
            UserStatus::Deleted,
            UserTimestamp::new("2026-06-25T03:00:00Z").unwrap(),
        )
        .expect("active can delete");
    assert_eq!(deleted.status(), UserStatus::Deleted);
}

#[test]
fn deleted_user_cannot_be_reactivated_without_policy_decision() {
    let deleted = sample_user()
        .transition_status(
            UserStatus::Deleted,
            UserTimestamp::new("2026-06-25T01:00:00Z").unwrap(),
        )
        .expect("active can delete");

    let error = deleted
        .transition_status(
            UserStatus::Active,
            UserTimestamp::new("2026-06-25T02:00:00Z").unwrap(),
        )
        .expect_err("deleted user cannot directly reactivate");

    assert_eq!(error.code(), "INVALID_USER_STATUS_TRANSITION");
}

fn sample_user() -> User {
    User::new(
        UserId::new("user-1").expect("valid id"),
        UserProfile::new(
            UserLogin::new("alice").expect("valid login"),
            UserEmail::new("alice@example.com").expect("valid email"),
            "Alice Lee",
            None,
        )
        .expect("valid profile"),
        UserTimestamp::new("2026-06-25T00:00:00Z").expect("valid timestamp"),
    )
}
