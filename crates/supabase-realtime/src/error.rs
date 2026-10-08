use fastwebsockets::WebSocketError;
use rp_supabase_auth::error::AuthError;

use crate::message::postgres_changes::PostgresDataChangeEvent;

/// Errors produced by the realtime connection and its streams.
#[derive(thiserror::Error, Debug)]
pub enum SupabaseRealtimeError {
    #[error("cannot load the native TLS root certificates")]
    CannotSetNativeCertificate,
    #[error("the realtime URL has no host")]
    HostStringNotPresent,
    #[error("websocket processing failed")]
    WsProcessingError,
    #[error("HTTP error: {0}")]
    HyperError(#[from] hyper::http::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("cannot load TLS certificates")]
    LocalCertificateLoadError,
    #[error("the realtime URL is misconfigured")]
    MisconfiguredStreamURL,
    #[error("cannot convert the domain to a TLS server name")]
    UnableConvertDomainToServerName,
    #[error("cannot look up host {host}:{port}")]
    UnableToLookUpHost { host: String, port: u16 },
    #[error("websocket error: {0}")]
    WebsocketError(#[from] WebSocketError),
    #[error("cannot parse URL: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("JSON error: {0}")]
    SerdeJsonError(#[from] simd_json::Error),
    #[error("cannot send a message to the connection task")]
    MpscSendError,
    #[error("the JWT stream closed before it produced an access token")]
    JwtStreamClosedUnexpectedly,
    #[error("token refresh failed: {0}")]
    RefreshStreamError(#[from] rp_supabase_auth::jwt_stream::RefreshStreamError),
    #[error("sign in failed: {0}")]
    AuthSignInError(#[from] AuthError),
    #[error("{event:?} change has no `{field}` field")]
    MissingChangeRecord {
        event: PostgresDataChangeEvent,
        field: &'static str,
    },
}
