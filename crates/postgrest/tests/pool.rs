#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public interfaces."
)]

use core::time::Duration;

use rp_postgrest::{
    Postgrest,
    reqwest::{
        Client,
        header::{HeaderMap, HeaderValue},
    },
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpListener,
};

#[expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "Fixture setup and assertions enforce connection reuse and wire contracts"
)]
#[tokio::test]
async fn clones_reuse_injected_pool_and_keep_default_headers_and_literal_wire_paths() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut requests = Vec::new();
        // Both requests must use this one connection; a rebuilt pool would hang.
        for _ in 0_u8..2_u8 {
            let mut bytes = Vec::new();
            loop {
                let mut chunk = [0_u8; 1024];
                let count = socket.read(&mut chunk).await.unwrap();
                assert_ne!(count, 0);
                bytes.extend_from_slice(&chunk[..count]);
                if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            requests.push(String::from_utf8(bytes).unwrap());
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n[]")
                .await
                .unwrap();
        }
        requests
    });
    let mut defaults = HeaderMap::new();
    defaults.insert("x-injected", HeaderValue::from_static("kept"));
    let http = Client::builder()
        .no_proxy()
        .default_headers(defaults)
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let client = Postgrest::new_with_client(format!("http://{address}/rest/v1"), http)
        .unwrap()
        .insert_header("apikey", "secret")
        .unwrap()
        .auth("token")
        .unwrap();
    client
        .from("typed.probe")
        .execute()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    client
        .clone()
        .rpc("rpc?echo", "{}")
        .method(rp_postgrest::reqwest::Method::GET)
        .execute_checked()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let requests = server.await.unwrap();
    assert!(requests[0].starts_with("GET /rest/v1/typed%2Eprobe HTTP/1.1\r\n"));
    assert!(requests[1].starts_with("GET /rest/v1/rpc/rpc%3Fecho HTTP/1.1\r\n"));
    for request in requests {
        let request = request.to_ascii_lowercase();
        assert!(request.contains("\r\nx-injected: kept\r\n"));
        assert!(request.contains("\r\napikey: secret\r\n"));
        assert!(request.contains("\r\nauthorization: bearer token\r\n"));
    }
}
