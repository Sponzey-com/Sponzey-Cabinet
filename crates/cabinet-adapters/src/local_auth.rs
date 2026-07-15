use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;

use cabinet_core::server_config::ServerAuthConfig;
use cabinet_domain::session::{Session, SessionId, SessionInstant};
use cabinet_domain::user::{User, UserId, UserLogin};
use cabinet_ports::auth::{
    CredentialSecret, CredentialVerifier, CredentialVerifierError, IssuedSessionToken,
    PresentedSessionToken, SessionLookupKey, SessionStore, SessionStoreError, TokenIssuer,
    TokenIssuerError,
};

use crate::local_atomic_file::write_text_atomically;

#[derive(Clone, PartialEq, Eq)]
pub struct LocalCredentialRecord {
    login: UserLogin,
    password_hash: String,
    user: User,
}

impl fmt::Debug for LocalCredentialRecord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalCredentialRecord")
            .field("login", &self.login.as_str())
            .field("password_hash", &"<redacted>")
            .field("user_id", &self.user.id().as_str())
            .finish()
    }
}

impl LocalCredentialRecord {
    pub fn new(login: UserLogin, password_hash: &str, user: User) -> Self {
        Self {
            login,
            password_hash: password_hash.to_string(),
            user,
        }
    }
}

pub trait PasswordHashVerifier {
    fn verify_password(&self, credential: &CredentialSecret, password_hash: &str) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticPasswordHashVerifier;

impl PasswordHashVerifier for StaticPasswordHashVerifier {
    fn verify_password(&self, credential: &CredentialSecret, password_hash: &str) -> bool {
        password_hash == format!("hash:{}", credential.expose_secret())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCredentialVerifier<H> {
    records_by_login: HashMap<String, LocalCredentialRecord>,
    hash_verifier: H,
}

impl<H> LocalCredentialVerifier<H> {
    pub fn new(records: Vec<LocalCredentialRecord>, hash_verifier: H) -> Self {
        Self {
            records_by_login: records
                .into_iter()
                .map(|record| (record.login.as_str().to_string(), record))
                .collect(),
            hash_verifier,
        }
    }
}

impl<H> CredentialVerifier for LocalCredentialVerifier<H>
where
    H: PasswordHashVerifier,
{
    fn verify(
        &self,
        login: &UserLogin,
        credential: &CredentialSecret,
    ) -> Result<Option<User>, CredentialVerifierError> {
        let Some(record) = self.records_by_login.get(login.as_str()) else {
            return Ok(None);
        };
        if !self
            .hash_verifier
            .verify_password(credential, &record.password_hash)
        {
            return Ok(None);
        }
        Ok(Some(record.user.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalOpaqueTokenIssuer {
    auth_config: ServerAuthConfig,
    next: u64,
}

impl LocalOpaqueTokenIssuer {
    pub const fn new(auth_config: ServerAuthConfig) -> Self {
        Self {
            auth_config,
            next: 0,
        }
    }
}

impl TokenIssuer for LocalOpaqueTokenIssuer {
    fn issue_token(&mut self) -> Result<IssuedSessionToken, TokenIssuerError> {
        self.next += 1;
        let raw_token = format!(
            "cabinet.{}.{}",
            self.auth_config.token_byte_length(),
            self.next
        );
        let token =
            PresentedSessionToken::new(&raw_token).map_err(|_| TokenIssuerError::Unavailable)?;
        let lookup_key = self.lookup_key_for(&token)?;
        Ok(IssuedSessionToken::new(token, lookup_key))
    }

    fn lookup_key_for(
        &self,
        token: &PresentedSessionToken,
    ) -> Result<SessionLookupKey, TokenIssuerError> {
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in self
            .auth_config
            .token_secret()
            .expose_secret()
            .as_bytes()
            .iter()
            .chain(token.expose_secret().as_bytes())
        {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        SessionLookupKey::new(&format!("lookup-{hash:016x}"))
            .map_err(|_| TokenIssuerError::InvalidToken)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InMemorySessionStore {
    sessions: HashMap<String, Session>,
}

impl SessionStore for InMemorySessionStore {
    fn create_session(
        &mut self,
        lookup_key: SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError> {
        if self.sessions.contains_key(lookup_key.as_str()) {
            return Err(SessionStoreError::Conflict);
        }
        self.sessions
            .insert(lookup_key.as_str().to_string(), session);
        Ok(())
    }

    fn get_session(
        &self,
        lookup_key: &SessionLookupKey,
    ) -> Result<Option<Session>, SessionStoreError> {
        Ok(self.sessions.get(lookup_key.as_str()).cloned())
    }

    fn update_session(
        &mut self,
        lookup_key: &SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError> {
        if !self.sessions.contains_key(lookup_key.as_str()) {
            return Err(SessionStoreError::NotFound);
        }
        self.sessions
            .insert(lookup_key.as_str().to_string(), session);
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct LocalSessionStore {
    session_dir: PathBuf,
}

impl fmt::Debug for LocalSessionStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalSessionStore")
            .field("session_dir", &self.session_dir)
            .finish_non_exhaustive()
    }
}

impl LocalSessionStore {
    pub fn new(session_dir: PathBuf) -> Self {
        Self { session_dir }
    }

    fn session_path(&self, lookup_key: &SessionLookupKey) -> PathBuf {
        self.session_dir
            .join(format!("{}.session", hex_encode(lookup_key.as_str())))
    }
}

impl SessionStore for LocalSessionStore {
    fn create_session(
        &mut self,
        lookup_key: SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError> {
        let path = self.session_path(&lookup_key);
        if path.exists() {
            return Err(SessionStoreError::Conflict);
        }
        write_text_atomically(&path, encode_session(&session))
            .map(|_| ())
            .map_err(|_| SessionStoreError::Unavailable)
    }

    fn get_session(
        &self,
        lookup_key: &SessionLookupKey,
    ) -> Result<Option<Session>, SessionStoreError> {
        let path = self.session_path(lookup_key);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path).map_err(|_| SessionStoreError::Unavailable)?;
        decode_session(&content).map(Some)
    }

    fn update_session(
        &mut self,
        lookup_key: &SessionLookupKey,
        session: Session,
    ) -> Result<(), SessionStoreError> {
        let path = self.session_path(lookup_key);
        if !path.exists() {
            return Err(SessionStoreError::NotFound);
        }
        write_text_atomically(&path, encode_session(&session))
            .map(|_| ())
            .map_err(|_| SessionStoreError::Unavailable)
    }
}

fn encode_session(session: &Session) -> String {
    format!(
        "session_id={}\nuser_id={}\nstatus={}\ncreated_at={}\nexpires_at={}\n",
        hex_encode(session.id().as_str()),
        hex_encode(session.user_id().as_str()),
        session.status().as_str(),
        session.created_at().epoch_seconds(),
        session.expires_at().epoch_seconds()
    )
}

fn decode_session(content: &str) -> Result<Session, SessionStoreError> {
    let mut session_id = None;
    let mut user_id = None;
    let mut status = None;
    let mut created_at = None;
    let mut expires_at = None;

    for line in content.lines() {
        let (key, value) = line.split_once('=').ok_or(SessionStoreError::Unavailable)?;
        match key {
            "session_id" => session_id = Some(hex_decode(value)?),
            "user_id" => user_id = Some(hex_decode(value)?),
            "status" => status = Some(value),
            "created_at" => {
                created_at = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| SessionStoreError::Unavailable)?,
                );
            }
            "expires_at" => {
                expires_at = Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| SessionStoreError::Unavailable)?,
                );
            }
            _ => return Err(SessionStoreError::Unavailable),
        }
    }

    let created_at =
        SessionInstant::from_epoch_seconds(created_at.ok_or(SessionStoreError::Unavailable)?);
    let expires_at =
        SessionInstant::from_epoch_seconds(expires_at.ok_or(SessionStoreError::Unavailable)?);
    let created = Session::new_created(
        SessionId::new(&session_id.ok_or(SessionStoreError::Unavailable)?)
            .map_err(|_| SessionStoreError::Unavailable)?,
        UserId::new(&user_id.ok_or(SessionStoreError::Unavailable)?)
            .map_err(|_| SessionStoreError::Unavailable)?,
        created_at,
        expires_at,
    )
    .map_err(|_| SessionStoreError::Unavailable)?;

    match status.ok_or(SessionStoreError::Unavailable)? {
        "Created" => Ok(created),
        "Active" => created
            .activate(created_at)
            .map_err(|_| SessionStoreError::Unavailable),
        "Expired" => created
            .expire(expires_at)
            .map_err(|_| SessionStoreError::Unavailable),
        "Revoked" => created.revoke().map_err(|_| SessionStoreError::Unavailable),
        _ => Err(SessionStoreError::Unavailable),
    }
}

fn hex_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_decode(value: &str) -> Result<String, SessionStoreError> {
    if !value.len().is_multiple_of(2) {
        return Err(SessionStoreError::Unavailable);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| SessionStoreError::Unavailable)?;
    String::from_utf8(bytes).map_err(|_| SessionStoreError::Unavailable)
}
