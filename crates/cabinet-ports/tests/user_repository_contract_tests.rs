use std::collections::HashMap;

use cabinet_domain::user::{User, UserEmail, UserExternalIdentity, UserId, UserLogin, UserStatus};
use cabinet_ports::user_repository::{
    ServerClock, ServerIdGenerator, UserRepository, UserRepositoryError,
};

#[derive(Default)]
struct FakeUserRepository {
    users: HashMap<String, User>,
}

impl UserRepository for FakeUserRepository {
    fn find_by_identity(
        &self,
        login: &UserLogin,
        email: &UserEmail,
        external_identity: Option<&UserExternalIdentity>,
    ) -> Result<Option<User>, UserRepositoryError> {
        Ok(self
            .users
            .values()
            .find(|user| {
                let external_identity_matches = external_identity
                    .and_then(|identity| {
                        user.profile()
                            .external_identity()
                            .map(|existing| existing == identity)
                    })
                    .unwrap_or(false);

                user.profile().login() == login
                    || user.profile().email() == email
                    || external_identity_matches
            })
            .cloned())
    }

    fn get_user(&self, user_id: &UserId) -> Result<Option<User>, UserRepositoryError> {
        Ok(self.users.get(user_id.as_str()).cloned())
    }

    fn save(&mut self, user: User) -> Result<(), UserRepositoryError> {
        if self.users.contains_key(user.id().as_str()) {
            return Err(UserRepositoryError::Conflict);
        }
        self.users.insert(user.id().as_str().to_string(), user);
        Ok(())
    }

    fn update_status(&mut self, user: User) -> Result<(), UserRepositoryError> {
        if !self.users.contains_key(user.id().as_str()) {
            return Err(UserRepositoryError::NotFound);
        }
        self.users.insert(user.id().as_str().to_string(), user);
        Ok(())
    }

    fn list_users(&self) -> Result<Vec<User>, UserRepositoryError> {
        let mut users = self.users.values().cloned().collect::<Vec<_>>();
        users.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(users)
    }
}

struct FakeClock;

impl ServerClock for FakeClock {
    fn now(&self) -> String {
        "2026-06-25T00:00:00Z".to_string()
    }
}

#[derive(Default)]
struct FakeIdGenerator {
    next: u32,
}

impl ServerIdGenerator for FakeIdGenerator {
    fn generate_user_id(&mut self) -> String {
        self.next += 1;
        format!("user-{}", self.next)
    }
}

#[test]
fn user_repository_contract_finds_by_login_or_email_and_updates_status() {
    let mut repository = FakeUserRepository::default();
    let mut id_generator = FakeIdGenerator::default();
    let clock = FakeClock;
    let user = cabinet_domain::user::User::new(
        UserId::new(&id_generator.generate_user_id()).expect("valid id"),
        cabinet_domain::user::UserProfile::new(
            UserLogin::new("alice").expect("valid login"),
            UserEmail::new("alice@example.com").expect("valid email"),
            "Alice Lee",
            Some(UserExternalIdentity::new("oidc", "subject-1").expect("valid external identity")),
        )
        .expect("valid profile"),
        cabinet_domain::user::UserTimestamp::new(&clock.now()).expect("valid timestamp"),
    );

    repository.save(user.clone()).expect("save user");

    assert!(
        repository
            .find_by_identity(
                &UserLogin::new("alice").expect("valid login"),
                &UserEmail::new("other@example.com").expect("valid email"),
                None,
            )
            .expect("lookup")
            .is_some()
    );
    assert!(
        repository
            .find_by_identity(
                &UserLogin::new("other").expect("valid login"),
                &UserEmail::new("alice@example.com").expect("valid email"),
                None,
            )
            .expect("lookup")
            .is_some()
    );
    assert!(
        repository
            .find_by_identity(
                &UserLogin::new("other").expect("valid login"),
                &UserEmail::new("other@example.com").expect("valid email"),
                Some(&UserExternalIdentity::new("oidc", "subject-1").expect("valid identity")),
            )
            .expect("lookup")
            .is_some()
    );

    let suspended = user
        .transition_status(
            UserStatus::Suspended,
            cabinet_domain::user::UserTimestamp::new("2026-06-25T01:00:00Z").unwrap(),
        )
        .expect("suspend");
    repository
        .update_status(suspended.clone())
        .expect("update status");

    assert_eq!(
        repository
            .get_user(suspended.id())
            .expect("get")
            .expect("user")
            .status(),
        UserStatus::Suspended
    );
    assert_eq!(repository.list_users().expect("list").len(), 1);
}
