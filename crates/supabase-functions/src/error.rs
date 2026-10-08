use reqwest::StatusCode;

/// Errors returned by [`crate::FunctionsClient`] and [`crate::InvokeBuilder`].
#[derive(Debug, thiserror::Error)]
pub enum FunctionsError {
    /// The function name is empty, `.` or `..`, or contains `/`, tab, CR, or LF.
    #[error(
        "invalid function name {0:?}: it must be non-empty, must not be '.' or '..', and must not contain '/', tab, CR, or LF"
    )]
    InvalidFunctionName(String),
    /// The project URL cannot hold the functions path.
    #[error("invalid project URL: {0}")]
    Url(#[from] url::ParseError),
    /// The project URL cannot be a base URL (for example `mailto:`).
    #[error("project URL cannot be a base URL")]
    UrlNotBase,
    /// A header name is not valid.
    #[error("invalid header name: {0}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),
    /// A header value is not valid.
    #[error("invalid header value: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    /// The request body cannot be serialized to JSON.
    #[error("failed to serialize request body: {0}")]
    Serialize(#[source] serde_json::Error),
    /// The request did not complete (connection, TLS, timeout, or body read failure).
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    /// The Supabase relay failed to reach the function (`x-relay-error: true`).
    #[error("relay error {status}: {body}")]
    Relay {
        /// HTTP status returned by the relay.
        status: StatusCode,
        /// Response body text.
        body: String,
    },
    /// The function returned a non-2xx status.
    #[error("function returned {status}: {body}")]
    Http {
        /// HTTP status returned by the function.
        status: StatusCode,
        /// Response body text.
        body: String,
    },
    /// The response body is not valid JSON for the requested type.
    #[error("failed to decode response body: {0}")]
    Decode(#[source] serde_json::Error),
}
