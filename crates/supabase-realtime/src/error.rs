use fastwebsockets::WebSocketError;

#[derive(thiserror::Error, Debug)]
pub enum SupabaseRealtimeError {
    #[error("cannot set native certs")]
    CannotSetNativeCertificate,
    #[error("Host string not present in the Stream URL")]
    HostStringNotPresent,
    #[error("Websocket processing Error")]
    WsProcessingError,
    #[error("Hyper error {0}")]
    HypreError(#[from] hyper::http::Error),
    #[error("IO Error {0}")]
    IoError(#[from] std::io::Error),
    #[error("cannot load tls certs")]
    LocalCertificateLoadError,
    #[error("Misconfigured stream URL")]
    MisconfiguredStreamURL,
    #[error("cannot convert domain to server name")]
    UnableConvertDomainToServerName,
    #[error("Unable to look up host {host}:{port}")]
    UnableToLookUpHost { host: String, port: u16 },
    #[error("WS error {0}")]
    WebsocketError(#[from] WebSocketError),
    #[error("Url parse error {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Serde json error {0}")]
    SerdeJsonError(#[from] simd_json::Error),
    #[error("Mpsc send error")]
    MpscSendError,
    #[error("Jwt Stream closed unexpectedly")]
    JwtStreamClosedUnexpectedly,
    #[error("Refresh stream error")]
    RefreshStreamError(#[from] supabase_auth::RefreshStreamError),
    #[error("Auth sign in error")]
    AuthSignInError(#[from] supabase_auth::SignInError),
}
