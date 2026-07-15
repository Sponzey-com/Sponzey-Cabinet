use std::collections::HashMap;

use cabinet_domain::user::{User, UserEmail, UserExternalIdentity, UserId, UserLogin, UserStatus};
use cabinet_ports::user_repository::{
    ServerClock, ServerIdGenerator, UserRepository, UserRepositoryError,
};
use cabinet_usecases::user::{
    CreateUserError, CreateUserInput, CreateUserProductEvent, CreateUserProductLogger,
    CreateUserUsecase, ListUsersError, ListUsersInput, ListUsersUsecase, UpdateUserStatusError,
    UpdateUserStatusInput, UpdateUserStatusUsecase,
};

#[derive(Default)]
struct FakeUserRepository {
    users: HashMap<String, User>,
    fail_save: bool,
    fail_list: bool,
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
        if self.fail_save {
            return Err(UserRepositoryError::StorageUnavailable);
        }
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
        if self.fail_list {
            return Err(UserRepositoryError::StorageUnavailable);
        }
        let mut users = self.users.values().cloned().collect::<Vec<_>>();
        users.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(users)
    }
}

#[derive(Default)]
struct FakeIdGenerator {
    generated: Vec<String>,
}

impl ServerIdGenerator for FakeIdGenerator {
    fn generate_user_id(&mut self) -> String {
        let next = format!("user-{}", self.generated.len() + 1);
        self.generated.push(next.clone());
        next
    }
}

struct FakeClock {
    now: String,
}

impl FakeClock {
    fn new(now: &str) -> Self {
        Self {
            now: now.to_string(),
        }
    }
}

impl ServerClock for FakeClock {
    fn now(&self) -> String {
        self.now.clone()
    }
}

#[derive(Default)]
struct FakeProductLogger {
    events: Vec<CreateUserProductEvent>,
}

impl CreateUserProductLogger for FakeProductLogger {
    fn write_product(&mut self, event: CreateUserProductEvent) {
        self.events.push(event);
    }
}

#[test]
fn create_user_generates_identity_persists_active_user_and_logs_masked_id() {
    let mut repository = FakeUserRepository::default();
    let mut id_generator = FakeIdGenerator::default();
    let clock = FakeClock::new("2026-06-25T00:00:00Z");
    let mut logger = FakeProductLogger::default();

    let output = CreateUserUsecase::new()
        .execute(
            CreateUserInput::new("alice", "alice@example.com", "Alice Lee", None),
            &mut repository,
            &clock,
            &mut id_generator,
            &mut logger,
        )
        .expect("user should be created");

    assert_eq!(output.user().id().as_str(), "user-1");
    assert_eq!(output.user().status(), UserStatus::Active);
    assert_eq!(
        logger.events,
        vec![CreateUserProductEvent::UserCreated {
            masked_user_id: "masked:user-1".to_string(),
        }]
    );
}

#[test]
fn create_user_rejects_duplicate_login_or_email_before_save() {
    let mut repository = FakeUserRepository::default();
    let mut id_generator = FakeIdGenerator::default();
    let clock = FakeClock::new("2026-06-25T00:00:00Z");
    let mut logger = FakeProductLogger::default();
    let usecase = CreateUserUsecase::new();

    usecase
        .execute(
            CreateUserInput::new("alice", "alice@example.com", "Alice Lee", None),
            &mut repository,
            &clock,
            &mut id_generator,
            &mut logger,
        )
        .expect("first user");
    let error = usecase
        .execute(
            CreateUserInput::new("alice", "other@example.com", "Other Alice", None),
            &mut repository,
            &clock,
            &mut id_generator,
            &mut logger,
        )
        .expect_err("duplicate login must fail");

    assert_eq!(error, CreateUserError::UserAlreadyExists);
    assert_eq!(
        logger.events.last(),
        Some(&CreateUserProductEvent::UserCreateFailed {
            error_code: "USER_ALREADY_EXISTS",
        })
    );
    assert_eq!(repository.users.len(), 1);
}

#[test]
fn create_user_rejects_duplicate_external_identity_before_save() {
    let mut repository = FakeUserRepository::default();
    let mut id_generator = FakeIdGenerator::default();
    let clock = FakeClock::new("2026-06-25T00:00:00Z");
    let mut logger = FakeProductLogger::default();
    let usecase = CreateUserUsecase::new();

    usecase
        .execute(
            CreateUserInput::new(
                "alice",
                "alice@example.com",
                "Alice Lee",
                Some(("oidc", "subject-1")),
            ),
            &mut repository,
            &clock,
            &mut id_generator,
            &mut logger,
        )
        .expect("first user");
    let error = usecase
        .execute(
            CreateUserInput::new(
                "bob",
                "bob@example.com",
                "Bob Lee",
                Some(("oidc", "subject-1")),
            ),
            &mut repository,
            &clock,
            &mut id_generator,
            &mut logger,
        )
        .expect_err("duplicate external identity must fail");

    assert_eq!(error, CreateUserError::UserAlreadyExists);
    assert_eq!(
        logger.events.last(),
        Some(&CreateUserProductEvent::UserCreateFailed {
            error_code: "USER_ALREADY_EXISTS",
        })
    );
    assert_eq!(repository.users.len(), 1);
}

#[test]
fn create_user_rejects_invalid_profile_before_generating_id() {
    let mut repository = FakeUserRepository::default();
    let mut id_generator = FakeIdGenerator::default();
    let clock = FakeClock::new("2026-06-25T00:00:00Z");
    let mut logger = FakeProductLogger::default();

    let error = CreateUserUsecase::new()
        .execute(
            CreateUserInput::new("ab", "bad-email", " ", None),
            &mut repository,
            &clock,
            &mut id_generator,
            &mut logger,
        )
        .expect_err("invalid profile must fail");

    assert_eq!(error, CreateUserError::InvalidUserInput);
    assert!(id_generator.generated.is_empty());
    assert!(repository.users.is_empty());
}

#[test]
fn update_user_status_suspends_user_and_logs_masked_id() {
    let mut repository = repository_with_active_user();
    let clock = FakeClock::new("2026-06-25T01:00:00Z");
    let mut logger = FakeProductLogger::default();

    let output = UpdateUserStatusUsecase::new()
        .execute(
            UpdateUserStatusInput::new("user-1", UserStatus::Suspended),
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect("suspend user");

    assert_eq!(output.user().status(), UserStatus::Suspended);
    assert_eq!(
        logger.events,
        vec![CreateUserProductEvent::UserStatusChanged {
            masked_user_id: "masked:user-1".to_string(),
            status: UserStatus::Suspended,
        }]
    );
}

#[test]
fn update_user_status_reports_missing_user() {
    let mut repository = FakeUserRepository::default();
    let clock = FakeClock::new("2026-06-25T01:00:00Z");
    let mut logger = FakeProductLogger::default();

    let error = UpdateUserStatusUsecase::new()
        .execute(
            UpdateUserStatusInput::new("missing", UserStatus::Suspended),
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect_err("missing user must fail");

    assert_eq!(error, UpdateUserStatusError::UserNotFound);
    assert_eq!(
        logger.events,
        vec![CreateUserProductEvent::UserStatusChangeFailed {
            error_code: "USER_NOT_FOUND",
        }]
    );
}

#[test]
fn list_users_returns_safe_user_summaries_without_email_or_external_identity() {
    let repository = repository_with_active_user();

    let output = ListUsersUsecase::new()
        .execute(ListUsersInput::new(), &repository)
        .expect("list users");

    assert_eq!(output.users().len(), 1);
    let user = &output.users()[0];
    assert_eq!(user.user_id(), "user-1");
    assert_eq!(user.login(), "alice");
    assert_eq!(user.display_name(), "Alice Lee");
    assert_eq!(user.status(), UserStatus::Active);
    let rendered = format!("{user:?}");
    assert!(!rendered.contains("alice@example.com"));
    assert!(!rendered.contains("external"));
}

#[test]
fn list_users_maps_repository_failure_to_stable_error_code() {
    let repository = FakeUserRepository {
        fail_list: true,
        ..FakeUserRepository::default()
    };

    let error = ListUsersUsecase::new()
        .execute(ListUsersInput::new(), &repository)
        .expect_err("list users should fail");

    assert_eq!(error, ListUsersError::StorageUnavailable);
    assert_eq!(error.code(), "USER_STORAGE_UNAVAILABLE");
}

#[test]
fn update_user_status_rejects_deleted_to_active_transition() {
    let mut repository = repository_with_active_user();
    let clock = FakeClock::new("2026-06-25T01:00:00Z");
    let mut logger = FakeProductLogger::default();
    let usecase = UpdateUserStatusUsecase::new();

    usecase
        .execute(
            UpdateUserStatusInput::new("user-1", UserStatus::Deleted),
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect("delete user");
    let error = usecase
        .execute(
            UpdateUserStatusInput::new("user-1", UserStatus::Active),
            &mut repository,
            &clock,
            &mut logger,
        )
        .expect_err("deleted user cannot directly reactivate");

    assert_eq!(error, UpdateUserStatusError::InvalidUserStatusTransition);
    assert_eq!(
        repository
            .get_user(&UserId::new("user-1").unwrap())
            .unwrap()
            .unwrap()
            .status(),
        UserStatus::Deleted
    );
}

#[test]
fn product_log_payload_excludes_email_login_and_display_name() {
    let event = CreateUserProductEvent::UserCreated {
        masked_user_id: "masked:user-1".to_string(),
    };

    let rendered = format!("{event:?}");

    assert_eq!(event.event_name(), "user.created");
    assert!(!rendered.contains("alice@example.com"));
    assert!(!rendered.contains("alice"));
    assert!(!rendered.contains("Alice Lee"));
}

#[test]
fn product_log_events_expose_stable_names() {
    assert_eq!(
        CreateUserProductEvent::UserCreated {
            masked_user_id: "masked:user-1".to_string(),
        }
        .event_name(),
        "user.created"
    );
    assert_eq!(
        CreateUserProductEvent::UserStatusChanged {
            masked_user_id: "masked:user-1".to_string(),
            status: UserStatus::Suspended,
        }
        .event_name(),
        "user.status_changed"
    );
    assert_eq!(
        CreateUserProductEvent::UserCreateFailed {
            error_code: "USER_ALREADY_EXISTS",
        }
        .event_name(),
        "user.create.failed"
    );
    assert_eq!(
        CreateUserProductEvent::UserStatusChangeFailed {
            error_code: "USER_NOT_FOUND",
        }
        .event_name(),
        "user.status_change.failed"
    );
}

fn repository_with_active_user() -> FakeUserRepository {
    let mut repository = FakeUserRepository::default();
    let mut id_generator = FakeIdGenerator::default();
    let clock = FakeClock::new("2026-06-25T00:00:00Z");
    let mut logger = FakeProductLogger::default();
    CreateUserUsecase::new()
        .execute(
            CreateUserInput::new("alice", "alice@example.com", "Alice Lee", None),
            &mut repository,
            &clock,
            &mut id_generator,
            &mut logger,
        )
        .expect("seed user");
    repository
}
