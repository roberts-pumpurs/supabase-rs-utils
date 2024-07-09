use fastwebsockets::WebSocketError;

#[derive(thiserror::Error, Debug)]
pub enum SupabaseRealtimeError {
    #[error("cannot set native certs")]
    CannotSetNativeCertificate,
    #[error("Host string not present in the Stream URL")]
    HostStringNotPresent,
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
}
