// Adapted from postgrest-rs / rp-postgrest 2.1.0 (MIT OR Apache-2.0).
use reqwest::{StatusCode, Url, header::HeaderMap};
use rp_postgrest_error::{DecodeError, PostgrestError};

/// Configuration rejected before sending a request.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The base URL could not be parsed.
    #[error("invalid PostgREST URL: {0}")]
    Url(#[from] url::ParseError),
    /// Only absolute HTTP(S) URLs without credentials, query or fragment are supported.
    #[error("PostgREST requires an HTTP(S) base URL without credentials, query or fragment")]
    InvalidBaseUrl,
    /// URL parsers normalize dot-only segments; they cannot name a resource safely.
    #[error("dot-only resource names cannot be represented by the HTTP URL parser")]
    DotOnlyResource,
    /// A JSON text path requires at least one key.
    #[error("JSON text paths require at least one key")]
    EmptyJsonPath,
    /// A header name was invalid.
    #[error("invalid header name: {0}")]
    HeaderName(#[from] reqwest::header::InvalidHeaderName),
    /// A header value was invalid.
    #[error("invalid header value: {0}")]
    HeaderValue(#[from] reqwest::header::InvalidHeaderValue),
    /// Prefer directives must be representable as HTTP-visible ASCII.
    #[error("Prefer directives must contain only HTTP-visible ASCII")]
    InvalidPreference,
    /// The default HTTP transport could not be configured.
    #[error("HTTP client configuration failed: {0}")]
    Client(#[source] reqwest::Error),
}

/// Authoritative HTTP metadata retained when a response fails.
#[derive(Clone, Debug)]
pub struct ResponseMetadata {
    status: StatusCode,
    headers: HeaderMap,
    url: Url,
}
impl ResponseMetadata {
    pub(crate) fn take_from_response(response: &mut reqwest::Response) -> Self {
        Self {
            status: response.status(),
            headers: core::mem::take(response.headers_mut()),
            url: response.url().clone(),
        }
    }
    /// Observed HTTP status.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        self.status
    }
    /// Response headers.
    #[must_use]
    pub const fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    /// Effective URL after redirects.
    #[must_use]
    pub const fn url(&self) -> &Url {
        &self.url
    }
    /// Extracts the server total from Content-Range without counting returned rows.
    ///
    /// # Errors
    /// Rejects missing, unavailable, malformed or overflowing totals.
    pub fn count(&self) -> Result<u64, crate::CountError> {
        crate::count::response_count(self.headers())
    }
}

/// A configuration, request, or response failure, without nested result layers.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[expect(
    clippy::error_impl_error,
    reason = "Error is the established public protocol error type"
)]
pub enum Error {
    /// Invalid client or request configuration.
    #[error(transparent)]
    Configuration(#[from] ConfigError),
    /// Deferred payload serialization failure.
    #[error("JSON serialization failed: {0}")]
    Serialization(#[source] serde_json::Error),
    /// No usable HTTP response was received.
    #[error("PostgREST request failed: {0}")]
    Request(#[source] reqwest::Error),
    /// Reading a successful or unsuccessful response body failed.
    #[error("reading PostgREST response body failed: {source}")]
    ResponseBody {
        metadata: Box<ResponseMetadata>,
        source: reqwest::Error,
    },
    /// A structured server error; observed HTTP status is authoritative.
    #[error("{source}")]
    Postgrest {
        metadata: Box<ResponseMetadata>,
        source: Box<PostgrestError>,
    },
    /// A malformed error envelope, retaining exact body bytes in its source.
    #[error("{source}")]
    Decode {
        metadata: Box<ResponseMetadata>,
        source: Box<DecodeError>,
    },
    /// A successful response did not match the requested JSON shape.
    #[error("decoding successful PostgREST response failed: {source}")]
    ResponseDecode {
        metadata: Box<ResponseMetadata>,
        source: serde_json::Error,
    },
    /// A requested row count was missing or invalid.
    #[error("reading PostgREST row count failed: {source}")]
    Count {
        metadata: Box<ResponseMetadata>,
        source: crate::CountError,
    },
}
impl Error {
    /// The decoded structured server error without destructuring this error.
    #[must_use]
    pub const fn postgrest_error(&self) -> Option<&PostgrestError> {
        match self {
            Self::Postgrest { source, .. } => Some(source),
            Self::Configuration(_)
            | Self::Serialization(_)
            | Self::Request(_)
            | Self::ResponseBody { .. }
            | Self::Decode { .. }
            | Self::ResponseDecode { .. }
            | Self::Count { .. } => None,
        }
    }
    /// The decoded server response, including code, details and hint.
    #[must_use]
    pub fn postgrest_body(&self) -> Option<&rp_postgrest_error::ErrorResponse> {
        self.postgrest_error().map(PostgrestError::response)
    }
    /// Metadata when a response was observed.
    #[must_use]
    pub const fn response_metadata(&self) -> Option<&ResponseMetadata> {
        match self {
            Self::ResponseBody { metadata, .. }
            | Self::Postgrest { metadata, .. }
            | Self::Decode { metadata, .. }
            | Self::ResponseDecode { metadata, .. }
            | Self::Count { metadata, .. } => Some(metadata),
            Self::Configuration(_) | Self::Serialization(_) | Self::Request(_) => None,
        }
    }
    /// Authoritative observed HTTP status, if available.
    #[must_use]
    pub fn status(&self) -> Option<StatusCode> {
        self.response_metadata()
            .map(ResponseMetadata::status)
            .or_else(|| match self {
                Self::Request(source) => source.status(),
                Self::Configuration(_)
                | Self::Serialization(_)
                | Self::ResponseBody { .. }
                | Self::Postgrest { .. }
                | Self::Decode { .. }
                | Self::ResponseDecode { .. }
                | Self::Count { .. } => None,
            })
    }
    /// Effective response URL, or the failed request URL.
    #[must_use]
    pub fn url(&self) -> Option<&Url> {
        self.response_metadata()
            .map(ResponseMetadata::url)
            .or_else(|| match self {
                Self::Request(source) => source.url(),
                Self::Configuration(_)
                | Self::Serialization(_)
                | Self::ResponseBody { .. }
                | Self::Postgrest { .. }
                | Self::Decode { .. }
                | Self::ResponseDecode { .. }
                | Self::Count { .. } => None,
            })
    }
}
