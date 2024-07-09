use std::future::Future;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use bytes::Bytes;
use fastwebsockets::{FragmentCollector, Frame, OpCode, WebSocketError};
use http_body_util::Empty;
use hyper::header::{CONNECTION, UPGRADE};
use hyper::upgrade::Upgraded;
use hyper::Request;
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;

struct SpawnExecutor;

impl<Fut> hyper::rt::Executor<Fut> for SpawnExecutor
where
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    fn execute(&self, fut: Fut) {
        tokio::task::spawn(fut);
    }
}

#[derive(thiserror::Error, Debug)]
enum SupabaseRealtimeError {
    #[error("cannot load tls certs")]
    LocalCertificateLoadError,
    #[error("cannot set native certs")]
    CannotSetNativeCertificate,
    #[error("cannot convert domain to server name")]
    UnableConvertDomainToServerName,
    #[error("Host string not present in the Stream URL")]
    HostStringNotPresent,
    #[error("Unable to look up host {host}:{port}")]
    UnableToLookUpHost { host: String, port: u16 },
    #[error("Misconfigured stream URL")]
    MisconfiguredStreamURL,
    #[error("IO Error {0}")]
    IoError(#[from] std::io::Error),
    #[error("Hyper error {0}")]
    HypreError(#[from] hyper::http::Error),
    #[error("WS error {0}")]
    WebsocketError(#[from] WebSocketError),
}

fn tls_connector() -> Result<tokio_rustls::TlsConnector, SupabaseRealtimeError> {
    use tokio_rustls::TlsConnector;

    let mut roots = rustls::RootCertStore::empty();
    let native_certs = rustls_native_certs::load_native_certs().map_err(|err| {
        tracing::error!(?err, "Cannot load native certificates");
        SupabaseRealtimeError::LocalCertificateLoadError
    })?;
    for cert in native_certs {
        roots.add(cert).map_err(|err| {
            tracing::error!(?err, "Cannot set native certificate");
            SupabaseRealtimeError::CannotSetNativeCertificate
        })?;
    }

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(TlsConnector::from(Arc::new(config)))
}

async fn connect(
    url: &url::Url,
) -> Result<FragmentCollector<TokioIo<Upgraded>>, SupabaseRealtimeError> {
    let host = url
        .host_str()
        .ok_or(SupabaseRealtimeError::HostStringNotPresent)?;
    let port = url.port().unwrap_or(443);
    let socket_addr = tokio::net::lookup_host((host, port))
        .await
        .map_err(|err| {
            tracing::error!(?err, "unable to look up host");
            SupabaseRealtimeError::UnableToLookUpHost {
                host: host.to_owned(),
                port,
            }
        })?
        .next();
    let domain = url.domain();
    match (domain, socket_addr) {
        (Some(domain), Some(socket_addr)) => {
            let tcp_stream = TcpStream::connect(&socket_addr).await?;
            let tls_connector = tls_connector().unwrap();
            let domain =
                rustls::pki_types::ServerName::try_from(domain.to_owned()).map_err(|err| {
                    tracing::error!(?err, "unable to convert domain to server name");
                    SupabaseRealtimeError::UnableConvertDomainToServerName
                })?;
            let tls_stream = tls_connector.connect(domain, tcp_stream).await?;

            let req = Request::builder()
                .method("GET")
                .uri(url.as_str()) //stream we want to subscribe to
                .header("Host", url.host_str().unwrap())
                .header(UPGRADE, "websocket")
                .header(CONNECTION, "upgrade")
                .header(
                    "Sec-WebSocket-Key",
                    fastwebsockets::handshake::generate_key(),
                )
                .header("Sec-WebSocket-Version", "13")
                .body(Empty::<Bytes>::new())?;

            let (ws, _) =
                fastwebsockets::handshake::client(&SpawnExecutor, req, tls_stream).await?;
            Ok(FragmentCollector::new(ws))
        }
        params => {
            tracing::error!(?params, "unable to connect to Stream API");

            Err(SupabaseRealtimeError::MisconfiguredStreamURL)
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    let domain = "wss://data-stream.binance.com:9443/ws/btcusdt@bookTicker".parse()?;
    let mut ws = connect(&domain).await?;

    loop {
        let msg = match ws.read_frame().await {
            Ok(msg) => msg,
            Err(e) => {
                println!("Error: {}", e);
                ws.write_frame(Frame::close_raw(vec![].into())).await?;
                break;
            }
        };

        match msg.opcode {
            OpCode::Text => {
                let payload = String::from_utf8(msg.payload.to_vec()).expect("Invalid UTF-8 data");
                // Normally deserialise from json here, print just to show it works
                println!("{:?}", payload);
            }
            OpCode::Close => {
                break;
            }
            _ => {}
        }
    }
    Ok(())
}
