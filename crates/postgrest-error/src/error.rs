use alloc::{string::String, vec::Vec};
use core::fmt;

use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{Authentication, ErrorCode, ErrorKind};

/// Additional error information returned by `PostgREST`.
///
/// Database errors use text. Ambiguous relationships use structured candidates.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(untagged)]
pub enum ErrorDetails {
    /// A database or server diagnostic.
    Text(String),
    /// Relationships which could satisfy an ambiguous embed.
    AmbiguousEmbeddings(Vec<EmbeddingDetail>),
}

// Select the JSON shape directly, without buffering and cloning untagged candidates.
impl<'de> Deserialize<'de> for ErrorDetails {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DetailsVisitor;

        impl<'de> serde::de::Visitor<'de> for DetailsVisitor {
            type Value = ErrorDetails;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("diagnostic text or an array of relationship candidates")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(ErrorDetails::Text(value.into()))
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(ErrorDetails::Text(value))
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut candidates = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
                while let Some(candidate) = sequence.next_element()? {
                    candidates.push(candidate);
                }
                Ok(ErrorDetails::AmbiguousEmbeddings(candidates))
            }
        }

        deserializer.deserialize_any(DetailsVisitor)
    }
}

/// A candidate relationship in a `PGRST201` ambiguity error.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct EmbeddingDetail {
    /// Relationship cardinality reported by the server.
    pub cardinality: EmbeddingCardinality,
    /// Resources being embedded.
    pub embedding: String,
    /// Constraint and column description identifying this relationship.
    pub relationship: String,
}

/// Relationship cardinality reported in an ambiguous embedding error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmbeddingCardinality {
    /// One source row has at most one target row.
    OneToOne,
    /// One source row can have several target rows.
    OneToMany,
    /// Several source rows can refer to one target row.
    ManyToOne,
    /// Both resources can have several matching rows through a join table.
    ManyToMany,
}

/// Structured error body returned by `PostgREST`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<ErrorDetails>,
    pub hint: Option<String>,
}

impl ErrorResponse {
    /// Returns a canonical five-character SQLSTATE, excluding `PostgREST` codes.
    ///
    /// SQLSTATE uses uppercase ASCII letters and digits. Unknown SQLSTATEs are
    /// accepted; lowercase, custom HTTP codes and `PGRST` codes return `None`.
    #[must_use]
    pub fn sqlstate(&self) -> Option<&str> {
        let code = self.code.as_str();
        (code.len() == 5
            && !code.starts_with("PGRST")
            && !matches!(self.code.kind(), ErrorKind::CustomStatus(_))
            && code
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit()))
        .then_some(code)
    }

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
