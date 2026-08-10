use alloc::{string::String, vec::Vec};
use core::fmt;

use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{Authentication, ErrorCode, ErrorKind};

/// Structured error body returned by `PostgREST`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<String>,
    pub hint: Option<String>,
}

impl ErrorResponse {
    /// Returns the HTTP status implied by this body, when one is known.
    ///
    /// This method includes the message-sensitive mappings used by
    /// `PostgREST`. It remains a fallback for cases where the observed HTTP
    /// status is unavailable.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "ErrorKind is non-exhaustive and future kinds use code-only inference"
    )]
    #[must_use]
    pub fn inferred_status(&self, authentication: Authentication) -> Option<StatusCode> {
        match self.code.kind() {
            ErrorKind::Postgres(crate::PostgresErrorCode::CardinalityViolation) => {
                if self.message.ends_with("requires a WHERE clause") {
                    Some(StatusCode::BAD_REQUEST)
                } else {
                    Some(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
            ErrorKind::Postgres(crate::PostgresErrorCode::InvalidParameterValue) => {
                if self.message.starts_with("role") && self.message.ends_with("does not exist") {
                    Some(StatusCode::UNAUTHORIZED)
                } else {
                    Some(StatusCode::BAD_REQUEST)
                }
            }
            ErrorKind::Postgres(crate::PostgresErrorCode::UndefinedFunction) => {
                if self.message.starts_with("function xmlagg(") {
                    Some(StatusCode::NOT_ACCEPTABLE)
                } else {
                    Some(StatusCode::NOT_FOUND)
                }
            }
            _ => self.code.inferred_status(authentication),
        }
    }
}

/// A decoded `PostgREST` error paired with its authoritative HTTP status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgrestError {
    status: StatusCode,
    response: ErrorResponse,
}

impl PostgrestError {
    /// Decodes a structured `PostgREST` error from an HTTP response body.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] when the body is not a valid structured
    /// `PostgREST` error. The error retains both the status and original body.
    pub fn from_slice(status: StatusCode, body: &[u8]) -> Result<Self, DecodeError> {
        let response = serde_json::from_slice(body)
            .map_err(|source| DecodeError::new(status, body, source))?;
        Ok(Self::from_response(status, response))
    }

    /// Decodes an owned HTTP response body without copying malformed evidence.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] when the body is not a valid structured
    /// `PostgREST` error. The error takes ownership of the original body.
    pub fn from_vec(status: StatusCode, body: Vec<u8>) -> Result<Self, DecodeError> {
        match serde_json::from_slice(&body) {
            Ok(response) => Ok(Self::from_response(status, response)),
            Err(source) => Err(DecodeError::from_owned(status, body, source)),
        }
    }

    /// Creates an error from an observed HTTP status and decoded response.
    #[must_use]
    pub const fn from_response(status: StatusCode, response: ErrorResponse) -> Self {
        Self { status, response }
    }

    /// Returns the authoritative status observed on the HTTP response.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the structured `PostgREST` response body.
    #[must_use]
    pub const fn response(&self) -> &ErrorResponse {
        &self.response
    }

    /// Returns the exact error code from the response body.
    #[must_use]
    pub const fn code(&self) -> &ErrorCode {
        &self.response.code
    }

    /// Returns the semantic classification of the response code.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        self.code().kind()
    }

    /// Returns the HTTP status implied by the response code, when known.
    ///
    /// This does not replace [`Self::status`], which is authoritative.
    #[must_use]
    pub fn inferred_status(&self, authentication: Authentication) -> Option<StatusCode> {
        self.response.inferred_status(authentication)
    }

    /// Consumes the error and returns its authoritative status and response.
    #[must_use]
    pub fn into_parts(self) -> (StatusCode, ErrorResponse) {
        (self.status, self.response)
    }
}

impl fmt::Display for PostgrestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "PostgREST request failed with {} [{}]: {}",
            self.status, self.response.code, self.response.message
        )
    }
}

impl core::error::Error for PostgrestError {}

/// Failure to decode a structured `PostgREST` error body.
#[derive(Debug)]
pub struct DecodeError {
    status: StatusCode,
    body: Vec<u8>,
    source: serde_json::Error,
}

impl DecodeError {
    fn new(status: StatusCode, body: &[u8], source: serde_json::Error) -> Self {
        Self::from_owned(status, body.to_vec(), source)
    }

    const fn from_owned(status: StatusCode, body: Vec<u8>, source: serde_json::Error) -> Self {
        Self {
            status,
            body,
            source,
        }
    }

    /// Returns the authoritative status observed on the HTTP response.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the original undecodable response body.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to decode PostgREST error response with status {}: {}",
            self.status, self.source
        )
    }
}

impl core::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&self.source)
    }
}
