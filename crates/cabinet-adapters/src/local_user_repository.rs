use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use cabinet_domain::user::{
    User, UserEmail, UserExternalIdentity, UserId, UserLogin, UserProfile, UserStatus,
    UserTimestamp,
};
use cabinet_ports::user_repository::{UserRepository, UserRepositoryError};

use crate::local_atomic_file::write_text_atomically;

pub const LOCAL_USERS_DIR: &str = "users";
pub const LOCAL_USERS_BY_ID_DIR: &str = "by-id";
pub const LOCAL_USERS_BY_LOGIN_DIR: &str = "by-login";
pub const LOCAL_USERS_BY_EMAIL_DIR: &str = "by-email";
pub const LOCAL_USERS_BY_EXTERNAL_DIR: &str = "by-external";

#[derive(Clone, PartialEq, Eq)]
pub struct LocalUserRepository {
    root: PathBuf,
}

impl fmt::Debug for LocalUserRepository {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalUserRepository")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LocalUserRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn users_root(&self) -> PathBuf {
        self.root.join(LOCAL_USERS_DIR)
    }

    fn user_path(&self, user_id: &UserId) -> PathBuf {
        self.users_root()
            .join(LOCAL_USERS_BY_ID_DIR)
            .join(format!("{}.user", hex_encode(user_id.as_str())))
    }

    fn login_index_path(&self, login: &UserLogin) -> PathBuf {
        self.users_root()
            .join(LOCAL_USERS_BY_LOGIN_DIR)
            .join(format!("{}.idx", hex_encode(login.as_str())))
    }

    fn email_index_path(&self, email: &UserEmail) -> PathBuf {
        self.users_root()
            .join(LOCAL_USERS_BY_EMAIL_DIR)
            .join(format!("{}.idx", hex_encode(email.as_str())))
    }

    fn external_index_path(&self, identity: &UserExternalIdentity) -> PathBuf {
        self.users_root()
            .join(LOCAL_USERS_BY_EXTERNAL_DIR)
            .join(format!(
                "{}-{}.idx",
                hex_encode(identity.provider()),
                hex_encode(identity.subject())
            ))
    }

    fn lookup_index(&self, path: &Path) -> Result<Option<UserId>, UserRepositoryError> {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(UserRepositoryError::StorageUnavailable),
        };
        let user_id = hex_decode(content.trim())?;
        UserId::new(&user_id)
            .map(Some)
            .map_err(|_| UserRepositoryError::StorageUnavailable)
    }

    fn find_by_index(&self, path: &Path) -> Result<Option<User>, UserRepositoryError> {
        let Some(user_id) = self.lookup_index(path)? else {
            return Ok(None);
        };
        self.get_user(&user_id)?
            .ok_or(UserRepositoryError::StorageUnavailable)
            .map(Some)
    }

    fn write_identity_indexes(&self, user: &User) -> Result<(), UserRepositoryError> {
        write_index(&self.login_index_path(user.profile().login()), user.id())?;
        write_index(&self.email_index_path(user.profile().email()), user.id())?;
        if let Some(identity) = user.profile().external_identity() {
            write_index(&self.external_index_path(identity), user.id())?;
        }
        Ok(())
    }
}

impl UserRepository for LocalUserRepository {
    fn find_by_identity(
        &self,
        login: &UserLogin,
        email: &UserEmail,
        external_identity: Option<&UserExternalIdentity>,
    ) -> Result<Option<User>, UserRepositoryError> {
        if let Some(user) = self.find_by_index(&self.login_index_path(login))? {
            return Ok(Some(user));
        }
        if let Some(user) = self.find_by_index(&self.email_index_path(email))? {
            return Ok(Some(user));
        }
        if let Some(identity) = external_identity {
            if let Some(user) = self.find_by_index(&self.external_index_path(identity))? {
                return Ok(Some(user));
            }
        }
        Ok(None)
    }

    fn get_user(&self, user_id: &UserId) -> Result<Option<User>, UserRepositoryError> {
        let path = self.user_path(user_id);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(UserRepositoryError::StorageUnavailable),
        };
        decode_user(&content).map(Some)
    }

    fn save(&mut self, user: User) -> Result<(), UserRepositoryError> {
        if self.user_path(user.id()).exists() {
            return Err(UserRepositoryError::Conflict);
        }
        if self
            .lookup_index(&self.login_index_path(user.profile().login()))?
            .is_some()
            || self
                .lookup_index(&self.email_index_path(user.profile().email()))?
                .is_some()
            || user
                .profile()
                .external_identity()
                .map(|identity| self.lookup_index(&self.external_index_path(identity)))
                .transpose()?
                .flatten()
                .is_some()
        {
            return Err(UserRepositoryError::Conflict);
        }

        write_text_atomically(&self.user_path(user.id()), encode_user(&user))
            .map(|_| ())
            .map_err(|_| UserRepositoryError::StorageUnavailable)?;
        self.write_identity_indexes(&user)
    }

    fn update_status(&mut self, user: User) -> Result<(), UserRepositoryError> {
        let path = self.user_path(user.id());
        if !path.exists() {
            return Err(UserRepositoryError::NotFound);
        }
        write_text_atomically(&path, encode_user(&user))
            .map(|_| ())
            .map_err(|_| UserRepositoryError::StorageUnavailable)
    }

    fn list_users(&self) -> Result<Vec<User>, UserRepositoryError> {
        let user_dir = self.users_root().join(LOCAL_USERS_BY_ID_DIR);
        let entries = match fs::read_dir(user_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(_) => return Err(UserRepositoryError::StorageUnavailable),
        };

        let mut users = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|_| UserRepositoryError::StorageUnavailable)?
                .path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("user") {
                continue;
            }
            let content =
                fs::read_to_string(path).map_err(|_| UserRepositoryError::StorageUnavailable)?;
            users.push(decode_user(&content)?);
        }
        users.sort_by(|left, right| left.id().as_str().cmp(right.id().as_str()));
        Ok(users)
    }
}

fn write_index(path: &Path, user_id: &UserId) -> Result<(), UserRepositoryError> {
    write_text_atomically(path, format!("{}\n", hex_encode(user_id.as_str())))
        .map(|_| ())
        .map_err(|_| UserRepositoryError::StorageUnavailable)
}

fn encode_user(user: &User) -> String {
    let (external_provider, external_subject) = user
        .profile()
        .external_identity()
        .map(|identity| (identity.provider(), identity.subject()))
        .unwrap_or(("", ""));
    format!(
        "id={}\nlogin={}\nemail={}\ndisplay_name={}\nexternal_provider={}\nexternal_subject={}\nstatus={}\ncreated_at={}\nupdated_at={}\n",
        hex_encode(user.id().as_str()),
        hex_encode(user.profile().login().as_str()),
        hex_encode(user.profile().email().as_str()),
        hex_encode(user.profile().display_name()),
        hex_encode(external_provider),
        hex_encode(external_subject),
        user.status().as_str(),
        hex_encode(user.created_at().as_str()),
        hex_encode(user.updated_at().as_str())
    )
}

fn decode_user(content: &str) -> Result<User, UserRepositoryError> {
    let mut id = None;
    let mut login = None;
    let mut email = None;
    let mut display_name = None;
    let mut external_provider = None;
    let mut external_subject = None;
    let mut status = None;
    let mut created_at = None;
    let mut updated_at = None;

    for line in content.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or(UserRepositoryError::StorageUnavailable)?;
        match key {
            "id" => id = Some(hex_decode(value)?),
            "login" => login = Some(hex_decode(value)?),
            "email" => email = Some(hex_decode(value)?),
            "display_name" => display_name = Some(hex_decode(value)?),
            "external_provider" => external_provider = Some(hex_decode(value)?),
            "external_subject" => external_subject = Some(hex_decode(value)?),
            "status" => status = Some(value),
            "created_at" => created_at = Some(hex_decode(value)?),
            "updated_at" => updated_at = Some(hex_decode(value)?),
            _ => return Err(UserRepositoryError::StorageUnavailable),
        }
    }

    let external_provider = external_provider.ok_or(UserRepositoryError::StorageUnavailable)?;
    let external_subject = external_subject.ok_or(UserRepositoryError::StorageUnavailable)?;
    let external_identity = match (external_provider.is_empty(), external_subject.is_empty()) {
        (true, true) => None,
        (false, false) => Some(
            UserExternalIdentity::new(&external_provider, &external_subject)
                .map_err(|_| UserRepositoryError::StorageUnavailable)?,
        ),
        _ => return Err(UserRepositoryError::StorageUnavailable),
    };
    let profile = UserProfile::new(
        UserLogin::new(&login.ok_or(UserRepositoryError::StorageUnavailable)?)
            .map_err(|_| UserRepositoryError::StorageUnavailable)?,
        UserEmail::new(&email.ok_or(UserRepositoryError::StorageUnavailable)?)
            .map_err(|_| UserRepositoryError::StorageUnavailable)?,
        &display_name.ok_or(UserRepositoryError::StorageUnavailable)?,
        external_identity,
    )
    .map_err(|_| UserRepositoryError::StorageUnavailable)?;
    let created_at =
        UserTimestamp::new(&created_at.ok_or(UserRepositoryError::StorageUnavailable)?)
            .map_err(|_| UserRepositoryError::StorageUnavailable)?;
    let updated_at =
        UserTimestamp::new(&updated_at.ok_or(UserRepositoryError::StorageUnavailable)?)
            .map_err(|_| UserRepositoryError::StorageUnavailable)?;
    let user = User::new(
        UserId::new(&id.ok_or(UserRepositoryError::StorageUnavailable)?)
            .map_err(|_| UserRepositoryError::StorageUnavailable)?,
        profile,
        created_at,
    );

    match status.ok_or(UserRepositoryError::StorageUnavailable)? {
        "Active" => user
            .transition_status(UserStatus::Active, updated_at)
            .map_err(|_| UserRepositoryError::StorageUnavailable),
        "Suspended" => user
            .transition_status(UserStatus::Suspended, updated_at)
            .map_err(|_| UserRepositoryError::StorageUnavailable),
        "Deleted" => user
            .transition_status(UserStatus::Deleted, updated_at)
            .map_err(|_| UserRepositoryError::StorageUnavailable),
        _ => Err(UserRepositoryError::StorageUnavailable),
    }
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, UserRepositoryError> {
    if !value.len().is_multiple_of(2) {
        return Err(UserRepositoryError::StorageUnavailable);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| UserRepositoryError::StorageUnavailable)?;
    String::from_utf8(bytes).map_err(|_| UserRepositoryError::StorageUnavailable)
}
