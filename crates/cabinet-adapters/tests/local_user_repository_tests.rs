use std::fs;
use std::path::PathBuf;

use cabinet_adapters::local_user_repository::LocalUserRepository;
use cabinet_domain::user::{
    User, UserEmail, UserExternalIdentity, UserId, UserLogin, UserProfile, UserStatus,
    UserTimestamp,
};
use cabinet_ports::user_repository::{UserRepository, UserRepositoryError};

#[test]
fn local_user_repository_persists_user_and_identity_indexes_across_instances() {
    let root = unique_temp_dir("local-user-repository-persist");
    let user = active_user_with_identity("user-2", "bob", Some(("oidc", "subject-2")));

    {
        let mut repository = LocalUserRepository::new(root.clone());
        repository.save(user.clone()).expect("save user");
    }

    let repository = LocalUserRepository::new(root.clone());
    let loaded = repository
        .get_user(user.id())
        .expect("get user")
        .expect("stored user");
    let by_login = repository
        .find_by_identity(
            &UserLogin::new("bob").expect("login"),
            &UserEmail::new("other@example.com").expect("email"),
            None,
        )
        .expect("find by login")
        .expect("login user");
    let by_email = repository
        .find_by_identity(
            &UserLogin::new("other").expect("login"),
            user.profile().email(),
            None,
        )
        .expect("find by email")
        .expect("email user");
    let by_external = repository
        .find_by_identity(
            &UserLogin::new("other").expect("login"),
            &UserEmail::new("other@example.com").expect("email"),
            Some(
                user.profile()
                    .external_identity()
                    .expect("external identity"),
            ),
        )
        .expect("find by external")
        .expect("external user");

    assert_eq!(loaded.id(), user.id());
    assert_eq!(loaded.profile().login(), user.profile().login());
    assert_eq!(by_login.id(), user.id());
    assert_eq!(by_email.id(), user.id());
    assert_eq!(by_external.id(), user.id());
    assert!(!format!("{repository:?}").contains("bob@example.com"));
    cleanup_temp_dir(root);
}

#[test]
fn local_user_repository_updates_status_and_lists_users_in_stable_order() {
    let root = unique_temp_dir("local-user-repository-update");
    let mut repository = LocalUserRepository::new(root.clone());
    let user_b = active_user_with_identity("user-2", "bob", None);
    let user_a = active_user_with_identity("user-1", "alice", None);
    repository.save(user_b).expect("save user b");
    repository.save(user_a.clone()).expect("save user a");

    let suspended = user_a
        .transition_status(
            UserStatus::Suspended,
            UserTimestamp::new("2026-06-25T01:00:00Z").expect("timestamp"),
        )
        .expect("suspend");
    repository
        .update_status(suspended.clone())
        .expect("update status");

    let restarted = LocalUserRepository::new(root.clone());
    let loaded = restarted
        .get_user(suspended.id())
        .expect("get updated")
        .expect("updated user");
    let listed = restarted.list_users().expect("list users");

    assert_eq!(loaded.status(), UserStatus::Suspended);
    assert_eq!(loaded.updated_at().as_str(), "2026-06-25T01:00:00Z");
    assert_eq!(
        listed
            .iter()
            .map(|user| user.id().as_str())
            .collect::<Vec<_>>(),
        vec!["user-1", "user-2"]
    );
    cleanup_temp_dir(root);
}

#[test]
fn local_user_repository_reports_conflict_missing_update_and_corruption() {
    let root = unique_temp_dir("local-user-repository-errors");
    let user = active_user_with_identity("user-1", "alice", None);
    let mut repository = LocalUserRepository::new(root.clone());

    repository.save(user.clone()).expect("save user");
    let duplicate = repository
        .save(user.clone())
        .expect_err("duplicate user id must conflict");
    let missing = repository
        .update_status(active_user_with_identity("missing-user", "missing", None))
        .expect_err("missing update must fail");

    let user_file = fs::read_dir(root.join("users").join("by-id"))
        .expect("by id dir")
        .next()
        .expect("user file")
        .expect("user file entry")
        .path();
    fs::write(user_file, "not-a-user-record").expect("corrupt user file");
    let corrupted = repository
        .get_user(user.id())
        .expect_err("corrupted user record must fail");

    assert_eq!(duplicate, UserRepositoryError::Conflict);
    assert_eq!(missing, UserRepositoryError::NotFound);
    assert_eq!(corrupted, UserRepositoryError::StorageUnavailable);
    cleanup_temp_dir(root);
}

fn active_user_with_identity(user_id: &str, login: &str, identity: Option<(&str, &str)>) -> User {
    User::new(
        UserId::new(user_id).expect("valid user id"),
        UserProfile::new(
            UserLogin::new(login).expect("valid login"),
            UserEmail::new(&format!("{login}@example.com")).expect("valid email"),
            "Test User",
            identity.map(|(provider, subject)| {
                UserExternalIdentity::new(provider, subject).expect("valid external identity")
            }),
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
