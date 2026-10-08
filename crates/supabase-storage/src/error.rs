use reqwest::StatusCode;
use serde::{Deserialize, Deserializer};

/// Errors returned by [`StorageClient`](crate::StorageClient) and [`Bucket`](crate::Bucket).
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The Storage API answered with a non-success HTTP status.
    #[error("storage API error ({status}): {body}")]
    Api {
        /// HTTP status of the response.
        status: StatusCode,
        /// Decoded response body.
        body: ApiErrorBody,
    },
    /// A bucket id or object path is not valid.
    #[error("invalid storage path {path:?}: {reason}")]
    InvalidPath {
        /// The rejected input.
        path: String,
        /// Why the input was rejected.
        reason: PathError,
    },
    /// The project URL cannot carry path segments (for example `mailto:`).
    #[error("project URL cannot be a base URL")]
    InvalidBaseUrl,
    /// A URL could not be parsed or joined.
    #[error(transparent)]
    Url(#[from] url::ParseError),
    /// The HTTP transport failed.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    /// A success response body is not the expected JSON.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// A key, token, or option is not a valid HTTP header value.
    #[error(transparent)]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
}

/// Reason for [`StorageError::InvalidPath`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    /// The path is empty.
    #[error("path is empty")]
    Empty,
    /// The path contains an empty segment (`a//b`, a leading or a trailing `/`).
    #[error("path contains an empty segment")]
    EmptySegment,
    /// The path contains a `.` or `..` segment.
    #[error("path contains a `.` or `..` segment")]
    DotSegment,
}

/// Body of a failed Storage API response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiErrorBody {
    /// The standard Storage error JSON.
    Storage(StorageErrorBody),
    /// Any other body, kept as text.
    Raw(String),
}

impl ApiErrorBody {
    pub(crate) fn decode(text: String) -> Self {
        serde_json::from_str(&text).map_or(Self::Raw(text), Self::Storage)
    }
}

impl core::fmt::Display for ApiErrorBody {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(body) => match &body.error {
                Some(code) => write!(formatter, "{code}: {}", body.message),
                None => formatter.write_str(&body.message),
            },
            Self::Raw(text) => formatter.write_str(text),
        }
    }
}

/// Standard Storage error JSON: `{"statusCode": "404", "error": "not_found", "message": "..."}`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct StorageErrorBody {
    /// Status code reported in the body. The API sends it as a string; numbers are converted.
    #[serde(rename = "statusCode", default, deserialize_with = "string_or_number")]
    pub status_code: Option<String>,
    /// Short error code, for example `not_found` or `Duplicate`.
    #[serde(default)]
    pub error: Option<String>,
    /// Human-readable message.
    pub message: String,
}

fn string_or_number<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Code {
        Text(String),
        Number(u64),
    }
    Ok(
        Option::<Code>::deserialize(deserializer)?.map(|code| match code {
            Code::Text(text) => text,
            Code::Number(number) => number.to_string(),
        }),
    )
}
