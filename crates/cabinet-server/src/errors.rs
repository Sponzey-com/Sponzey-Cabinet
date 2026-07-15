#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerErrorCode {
    RouteNotFound,
    MethodNotAllowed,
    TargetFailed,
    AuthMissingAuthorization,
    AuthMalformedAuthorization,
}

impl ServerErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RouteNotFound => "SERVER_ROUTE_NOT_FOUND",
            Self::MethodNotAllowed => "SERVER_METHOD_NOT_ALLOWED",
            Self::TargetFailed => "SERVER_TARGET_FAILED",
            Self::AuthMissingAuthorization => "SERVER_AUTH_MISSING_AUTHORIZATION",
            Self::AuthMalformedAuthorization => "SERVER_AUTH_MALFORMED_AUTHORIZATION",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerBoundaryError {
    code: ServerErrorCode,
    message: String,
}

impl ServerBoundaryError {
    pub fn new(code: ServerErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub const fn code(&self) -> ServerErrorCode {
        self.code
    }

    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}
