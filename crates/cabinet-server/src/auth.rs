use cabinet_usecases::auth::{ValidateSessionInput, ValidateSessionOutput};

use crate::errors::{ServerBoundaryError, ServerErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthHeaderMapper;

impl AuthHeaderMapper {
    pub const fn new() -> Self {
        Self
    }

    pub fn authorization_header_to_input(
        &self,
        authorization_header: Option<&str>,
    ) -> Result<ValidateSessionInput, ServerBoundaryError> {
        let Some(header) = authorization_header else {
            return Err(ServerBoundaryError::new(
                ServerErrorCode::AuthMissingAuthorization,
                "authorization header is required",
            ));
        };
        let Some(token) = header.strip_prefix("Bearer ") else {
            return Err(ServerBoundaryError::new(
                ServerErrorCode::AuthMalformedAuthorization,
                "authorization header must use bearer token",
            ));
        };
        if token.trim().is_empty() || token.contains(char::is_whitespace) {
            return Err(ServerBoundaryError::new(
                ServerErrorCode::AuthMalformedAuthorization,
                "authorization bearer token is malformed",
            ));
        }
        Ok(ValidateSessionInput::new(token))
    }
}

impl Default for AuthHeaderMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedActorContext {
    user_id: String,
}

impl AuthenticatedActorContext {
    pub fn from_validate_output(output: ValidateSessionOutput) -> Self {
        Self {
            user_id: output.actor().user_id().to_string(),
        }
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub const fn has_permission(&self, _permission: &str) -> bool {
        false
    }
}
