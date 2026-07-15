use cabinet_domain::user::{User, UserEmail, UserExternalIdentity, UserId, UserLogin};

pub trait UserRepository {
    fn find_by_identity(
        &self,
        login: &UserLogin,
        email: &UserEmail,
        external_identity: Option<&UserExternalIdentity>,
    ) -> Result<Option<User>, UserRepositoryError>;

    fn get_user(&self, user_id: &UserId) -> Result<Option<User>, UserRepositoryError>;

    fn save(&mut self, user: User) -> Result<(), UserRepositoryError>;

    fn update_status(&mut self, user: User) -> Result<(), UserRepositoryError>;

    fn list_users(&self) -> Result<Vec<User>, UserRepositoryError>;
}

pub trait ServerClock {
    fn now(&self) -> String;
}

pub trait ServerIdGenerator {
    fn generate_user_id(&mut self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRepositoryError {
    Conflict,
    NotFound,
    StorageUnavailable,
}

impl UserRepositoryError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Conflict => "user_repository.conflict",
            Self::NotFound => "user_repository.not_found",
            Self::StorageUnavailable => "user_repository.storage_unavailable",
        }
    }
}
