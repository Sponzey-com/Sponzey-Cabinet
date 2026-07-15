const USER_LOGIN_MIN_LEN: usize = 3;
const USER_LOGIN_MAX_LEN: usize = 64;
const USER_DISPLAY_NAME_MAX_LEN: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    id: UserId,
    profile: UserProfile,
    status: UserStatus,
    created_at: UserTimestamp,
    updated_at: UserTimestamp,
}

impl User {
    pub fn new(id: UserId, profile: UserProfile, created_at: UserTimestamp) -> Self {
        Self {
            id,
            profile,
            status: UserStatus::Active,
            updated_at: created_at.clone(),
            created_at,
        }
    }

    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn profile(&self) -> &UserProfile {
        &self.profile
    }

    pub const fn status(&self) -> UserStatus {
        self.status
    }

    pub fn created_at(&self) -> &UserTimestamp {
        &self.created_at
    }

    pub fn updated_at(&self) -> &UserTimestamp {
        &self.updated_at
    }

    pub fn transition_status(
        &self,
        next_status: UserStatus,
        changed_at: UserTimestamp,
    ) -> Result<Self, UserStatusTransitionError> {
        if !self.status.can_transition_to(next_status) {
            return Err(UserStatusTransitionError {
                from: self.status,
                to: next_status,
            });
        }

        let mut next_user = self.clone();
        next_user.status = next_status;
        next_user.updated_at = changed_at;
        Ok(next_user)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserId {
    value: String,
}

impl UserId {
    pub fn new(value: &str) -> Result<Self, UserError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(UserError::EmptyId);
        }
        if trimmed.chars().any(char::is_control) {
            return Err(UserError::InvalidId);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserLogin {
    value: String,
}

impl UserLogin {
    pub fn new(value: &str) -> Result<Self, UserError> {
        let trimmed = value.trim();
        let len = trimmed.chars().count();
        if len < USER_LOGIN_MIN_LEN {
            return Err(UserError::LoginTooShort {
                min: USER_LOGIN_MIN_LEN,
            });
        }
        if len > USER_LOGIN_MAX_LEN {
            return Err(UserError::LoginTooLong {
                max: USER_LOGIN_MAX_LEN,
            });
        }
        if !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
        {
            return Err(UserError::InvalidLoginCharacter);
        }
        Ok(Self {
            value: trimmed.to_ascii_lowercase(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserEmail {
    value: String,
}

impl UserEmail {
    pub fn new(value: &str) -> Result<Self, UserError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(UserError::EmptyEmail);
        }
        let Some((local, domain)) = trimmed.split_once('@') else {
            return Err(UserError::InvalidEmail);
        };
        if local.is_empty() || domain.is_empty() || domain.contains('@') || !domain.contains('.') {
            return Err(UserError::InvalidEmail);
        }
        if trimmed.chars().any(char::is_control) || trimmed.chars().any(char::is_whitespace) {
            return Err(UserError::InvalidEmail);
        }
        Ok(Self {
            value: trimmed.to_ascii_lowercase(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserExternalIdentity {
    provider: String,
    subject: String,
}

impl UserExternalIdentity {
    pub fn new(provider: &str, subject: &str) -> Result<Self, UserError> {
        let provider = provider.trim();
        let subject = subject.trim();
        if provider.is_empty() {
            return Err(UserError::EmptyExternalProvider);
        }
        if subject.is_empty() {
            return Err(UserError::EmptyExternalSubject);
        }
        if provider.chars().any(char::is_control) || subject.chars().any(char::is_control) {
            return Err(UserError::InvalidExternalIdentity);
        }
        Ok(Self {
            provider: provider.to_string(),
            subject: subject.to_string(),
        })
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    login: UserLogin,
    email: UserEmail,
    display_name: String,
    external_identity: Option<UserExternalIdentity>,
}

impl UserProfile {
    pub fn new(
        login: UserLogin,
        email: UserEmail,
        display_name: &str,
        external_identity: Option<UserExternalIdentity>,
    ) -> Result<Self, UserError> {
        let display_name = display_name.trim();
        if display_name.is_empty() {
            return Err(UserError::EmptyDisplayName);
        }
        if display_name.chars().count() > USER_DISPLAY_NAME_MAX_LEN {
            return Err(UserError::DisplayNameTooLong {
                max: USER_DISPLAY_NAME_MAX_LEN,
            });
        }
        if display_name.chars().any(char::is_control) {
            return Err(UserError::InvalidDisplayNameCharacter);
        }
        Ok(Self {
            login,
            email,
            display_name: display_name.to_string(),
            external_identity,
        })
    }

    pub fn login(&self) -> &UserLogin {
        &self.login
    }

    pub fn email(&self) -> &UserEmail {
        &self.email
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn external_identity(&self) -> Option<&UserExternalIdentity> {
        self.external_identity.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Suspended,
    Deleted,
}

impl UserStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Suspended => "Suspended",
            Self::Deleted => "Deleted",
        }
    }

    const fn can_transition_to(self, next: Self) -> bool {
        match (self, next) {
            (Self::Active, Self::Active)
            | (Self::Active, Self::Suspended)
            | (Self::Active, Self::Deleted)
            | (Self::Suspended, Self::Suspended)
            | (Self::Suspended, Self::Active)
            | (Self::Suspended, Self::Deleted)
            | (Self::Deleted, Self::Deleted) => true,
            (Self::Deleted, Self::Active) | (Self::Deleted, Self::Suspended) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserTimestamp {
    value: String,
}

impl UserTimestamp {
    pub fn new(value: &str) -> Result<Self, UserError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(UserError::EmptyTimestamp);
        }
        if trimmed.chars().any(char::is_whitespace) || trimmed.chars().any(char::is_control) {
            return Err(UserError::InvalidTimestamp);
        }
        Ok(Self {
            value: trimmed.to_string(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStatusTransitionError {
    from: UserStatus,
    to: UserStatus,
}

impl UserStatusTransitionError {
    pub const fn from(&self) -> UserStatus {
        self.from
    }

    pub const fn to(&self) -> UserStatus {
        self.to
    }

    pub const fn code(&self) -> &'static str {
        "INVALID_USER_STATUS_TRANSITION"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserError {
    EmptyId,
    InvalidId,
    LoginTooShort { min: usize },
    LoginTooLong { max: usize },
    InvalidLoginCharacter,
    EmptyEmail,
    InvalidEmail,
    EmptyDisplayName,
    DisplayNameTooLong { max: usize },
    InvalidDisplayNameCharacter,
    EmptyExternalProvider,
    EmptyExternalSubject,
    InvalidExternalIdentity,
    EmptyTimestamp,
    InvalidTimestamp,
}
