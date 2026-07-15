use cabinet_domain::user::{
    User, UserEmail, UserExternalIdentity, UserId, UserLogin, UserProfile, UserStatus,
    UserTimestamp,
};
use cabinet_ports::user_repository::{
    ServerClock, ServerIdGenerator, UserRepository, UserRepositoryError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateUserInput {
    login: String,
    email: String,
    display_name: String,
    external_identity: Option<(String, String)>,
}

impl CreateUserInput {
    pub fn new(
        login: &str,
        email: &str,
        display_name: &str,
        external_identity: Option<(&str, &str)>,
    ) -> Self {
        Self {
            login: login.to_string(),
            email: email.to_string(),
            display_name: display_name.to_string(),
            external_identity: external_identity
                .map(|(provider, subject)| (provider.to_string(), subject.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateUserOutput {
    user: User,
}

impl CreateUserOutput {
    pub fn user(&self) -> &User {
        &self.user
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateUserStatusInput {
    user_id: String,
    next_status: UserStatus,
}

impl UpdateUserStatusInput {
    pub fn new(user_id: &str, next_status: UserStatus) -> Self {
        Self {
            user_id: user_id.to_string(),
            next_status,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateUserStatusOutput {
    user: User,
}

impl UpdateUserStatusOutput {
    pub fn user(&self) -> &User {
        &self.user
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateUserProductEvent {
    UserCreated {
        masked_user_id: String,
    },
    UserStatusChanged {
        masked_user_id: String,
        status: UserStatus,
    },
    UserCreateFailed {
        error_code: &'static str,
    },
    UserStatusChangeFailed {
        error_code: &'static str,
    },
}

impl CreateUserProductEvent {
    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::UserCreated { .. } => "user.created",
            Self::UserStatusChanged { .. } => "user.status_changed",
            Self::UserCreateFailed { .. } => "user.create.failed",
            Self::UserStatusChangeFailed { .. } => "user.status_change.failed",
        }
    }
}

pub trait CreateUserProductLogger {
    fn write_product(&mut self, event: CreateUserProductEvent);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateUserUsecase;

impl CreateUserUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: CreateUserInput,
        repository: &mut impl UserRepository,
        clock: &impl ServerClock,
        id_generator: &mut impl ServerIdGenerator,
        product_logger: &mut impl CreateUserProductLogger,
    ) -> Result<CreateUserOutput, CreateUserError> {
        let profile = match build_user_profile(input) {
            Ok(profile) => profile,
            Err(error) => {
                product_logger.write_product(CreateUserProductEvent::UserCreateFailed {
                    error_code: error.code(),
                });
                return Err(error);
            }
        };

        match repository.find_by_identity(
            profile.login(),
            profile.email(),
            profile.external_identity(),
        ) {
            Ok(Some(_)) => {
                product_logger.write_product(CreateUserProductEvent::UserCreateFailed {
                    error_code: CreateUserError::UserAlreadyExists.code(),
                });
                return Err(CreateUserError::UserAlreadyExists);
            }
            Ok(None) => {}
            Err(error) => {
                let usecase_error = CreateUserError::from_repository_error(error);
                product_logger.write_product(CreateUserProductEvent::UserCreateFailed {
                    error_code: usecase_error.code(),
                });
                return Err(usecase_error);
            }
        }

        let user_id = UserId::new(&id_generator.generate_user_id()).map_err(|_| {
            product_logger.write_product(CreateUserProductEvent::UserCreateFailed {
                error_code: CreateUserError::InvalidUserInput.code(),
            });
            CreateUserError::InvalidUserInput
        })?;
        let created_at = UserTimestamp::new(&clock.now()).map_err(|_| {
            product_logger.write_product(CreateUserProductEvent::UserCreateFailed {
                error_code: CreateUserError::InvalidUserInput.code(),
            });
            CreateUserError::InvalidUserInput
        })?;
        let user = User::new(user_id, profile, created_at);

        if let Err(error) = repository.save(user.clone()) {
            let usecase_error = CreateUserError::from_repository_error(error);
            product_logger.write_product(CreateUserProductEvent::UserCreateFailed {
                error_code: usecase_error.code(),
            });
            return Err(usecase_error);
        }

        product_logger.write_product(CreateUserProductEvent::UserCreated {
            masked_user_id: mask_user_id(user.id()),
        });
        Ok(CreateUserOutput { user })
    }
}

impl Default for CreateUserUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateUserStatusUsecase;

impl UpdateUserStatusUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        input: UpdateUserStatusInput,
        repository: &mut impl UserRepository,
        clock: &impl ServerClock,
        product_logger: &mut impl CreateUserProductLogger,
    ) -> Result<UpdateUserStatusOutput, UpdateUserStatusError> {
        let user_id = UserId::new(&input.user_id).map_err(|_| {
            product_logger.write_product(CreateUserProductEvent::UserStatusChangeFailed {
                error_code: UpdateUserStatusError::InvalidUserInput.code(),
            });
            UpdateUserStatusError::InvalidUserInput
        })?;
        let user = match repository.get_user(&user_id) {
            Ok(Some(user)) => user,
            Ok(None) => {
                product_logger.write_product(CreateUserProductEvent::UserStatusChangeFailed {
                    error_code: UpdateUserStatusError::UserNotFound.code(),
                });
                return Err(UpdateUserStatusError::UserNotFound);
            }
            Err(error) => {
                let usecase_error = UpdateUserStatusError::from_repository_error(error);
                product_logger.write_product(CreateUserProductEvent::UserStatusChangeFailed {
                    error_code: usecase_error.code(),
                });
                return Err(usecase_error);
            }
        };
        let changed_at = UserTimestamp::new(&clock.now()).map_err(|_| {
            product_logger.write_product(CreateUserProductEvent::UserStatusChangeFailed {
                error_code: UpdateUserStatusError::InvalidUserInput.code(),
            });
            UpdateUserStatusError::InvalidUserInput
        })?;
        let updated_user = match user.transition_status(input.next_status, changed_at) {
            Ok(user) => user,
            Err(_) => {
                product_logger.write_product(CreateUserProductEvent::UserStatusChangeFailed {
                    error_code: UpdateUserStatusError::InvalidUserStatusTransition.code(),
                });
                return Err(UpdateUserStatusError::InvalidUserStatusTransition);
            }
        };

        if let Err(error) = repository.update_status(updated_user.clone()) {
            let usecase_error = UpdateUserStatusError::from_repository_error(error);
            product_logger.write_product(CreateUserProductEvent::UserStatusChangeFailed {
                error_code: usecase_error.code(),
            });
            return Err(usecase_error);
        }

        product_logger.write_product(CreateUserProductEvent::UserStatusChanged {
            masked_user_id: mask_user_id(updated_user.id()),
            status: updated_user.status(),
        });
        Ok(UpdateUserStatusOutput { user: updated_user })
    }
}

impl Default for UpdateUserStatusUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListUsersInput;

impl ListUsersInput {
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ListUsersInput {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUserSummary {
    user_id: String,
    login: String,
    display_name: String,
    status: UserStatus,
}

impl ListUserSummary {
    fn from_user(user: &User) -> Self {
        Self {
            user_id: user.id().as_str().to_string(),
            login: user.profile().login().as_str().to_string(),
            display_name: user.profile().display_name().to_string(),
            status: user.status(),
        }
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub fn login(&self) -> &str {
        &self.login
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub const fn status(&self) -> UserStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUsersOutput {
    users: Vec<ListUserSummary>,
}

impl ListUsersOutput {
    pub fn users(&self) -> &[ListUserSummary] {
        &self.users
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListUsersUsecase;

impl ListUsersUsecase {
    pub const fn new() -> Self {
        Self
    }

    pub fn execute(
        &self,
        _input: ListUsersInput,
        repository: &impl UserRepository,
    ) -> Result<ListUsersOutput, ListUsersError> {
        let users = repository
            .list_users()
            .map_err(ListUsersError::from_repository_error)?
            .iter()
            .map(ListUserSummary::from_user)
            .collect();
        Ok(ListUsersOutput { users })
    }
}

impl Default for ListUsersUsecase {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateUserError {
    InvalidUserInput,
    UserAlreadyExists,
    StorageUnavailable,
}

impl CreateUserError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidUserInput => "INVALID_USER_INPUT",
            Self::UserAlreadyExists => "USER_ALREADY_EXISTS",
            Self::StorageUnavailable => "USER_STORAGE_UNAVAILABLE",
        }
    }

    fn from_repository_error(error: UserRepositoryError) -> Self {
        match error {
            UserRepositoryError::Conflict => Self::UserAlreadyExists,
            UserRepositoryError::NotFound | UserRepositoryError::StorageUnavailable => {
                Self::StorageUnavailable
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListUsersError {
    StorageUnavailable,
}

impl ListUsersError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::StorageUnavailable => "USER_STORAGE_UNAVAILABLE",
        }
    }

    const fn from_repository_error(_error: UserRepositoryError) -> Self {
        Self::StorageUnavailable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateUserStatusError {
    InvalidUserInput,
    UserNotFound,
    InvalidUserStatusTransition,
    StorageUnavailable,
}

impl UpdateUserStatusError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidUserInput => "INVALID_USER_INPUT",
            Self::UserNotFound => "USER_NOT_FOUND",
            Self::InvalidUserStatusTransition => "INVALID_USER_STATUS_TRANSITION",
            Self::StorageUnavailable => "USER_STORAGE_UNAVAILABLE",
        }
    }

    fn from_repository_error(error: UserRepositoryError) -> Self {
        match error {
            UserRepositoryError::NotFound => Self::UserNotFound,
            UserRepositoryError::Conflict | UserRepositoryError::StorageUnavailable => {
                Self::StorageUnavailable
            }
        }
    }
}

fn build_user_profile(input: CreateUserInput) -> Result<UserProfile, CreateUserError> {
    let login = UserLogin::new(&input.login).map_err(|_| CreateUserError::InvalidUserInput)?;
    let email = UserEmail::new(&input.email).map_err(|_| CreateUserError::InvalidUserInput)?;
    let external_identity = input
        .external_identity
        .map(|(provider, subject)| UserExternalIdentity::new(&provider, &subject))
        .transpose()
        .map_err(|_| CreateUserError::InvalidUserInput)?;
    UserProfile::new(login, email, &input.display_name, external_identity)
        .map_err(|_| CreateUserError::InvalidUserInput)
}

fn mask_user_id(user_id: &UserId) -> String {
    format!("masked:{}", user_id.as_str())
}
