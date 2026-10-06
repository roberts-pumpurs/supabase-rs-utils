#![expect(
    clippy::tests_outside_test_module,
    reason = "This integration test crate is compiled only for tests"
)]

use rp_postgrest::{Error, Postgrest, reqwest::StatusCode};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpListener,
};

#[expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "Local HTTP fixture setup and bounded read slices must succeed"
)]
async fn serve(body: &[u8], length: usize) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut response = format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nX-Request-Id: decode-123\r\nContent-Range: 0-0/42\r\nConnection: close\r\n\r\n").into_bytes();
    response.extend_from_slice(body);
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        loop {
            let mut chunk = [0_u8; 1024];
            let count = socket.read(&mut chunk).await.unwrap();
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        socket.write_all(&response).await.unwrap();
        socket.shutdown().await.unwrap();
    });
    (format!("http://{address}"), server)
}

#[expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "Assertions and source matches describe decoding failure evidence"
)]
#[tokio::test]
async fn successful_json_and_shape_failures_retain_metadata() {
    for body in [
        b"not JSON".as_slice(),
        br#"{"id":"not an integer"}"#.as_slice(),
        b"".as_slice(),
    ] {
        let (url, server) = serve(body, body.len()).await;
        let error = Postgrest::new(url)
            .unwrap()
            .from("items")
            .fetch::<Vec<u32>>()
            .await
            .unwrap_err();
        assert_eq!(error.status(), Some(StatusCode::OK));
        assert_eq!(error.url().unwrap().path(), "/items");
        assert_eq!(
            error.response_metadata().unwrap().headers()["content-range"],
            "0-0/42"
        );
        let Error::ResponseDecode { metadata, source } = error else {
            panic!("expected successful response decode failure, got {error:?}");
        };
        assert_eq!(metadata.headers()["x-request-id"], "decode-123");
        assert!(source.is_syntax() || source.is_data() || source.is_eof());
        server.await.unwrap();
    }
}

#[expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "Assertions and source matches describe body read failure evidence"
)]
#[tokio::test]
async fn successful_body_read_failure_retains_metadata() {
    let (url, server) = serve(b"[1", 100).await;
    let error = Postgrest::new(url)
        .unwrap()
        .from("items")
        .fetch::<Vec<u32>>()
        .await
        .unwrap_err();
    let Error::ResponseBody { metadata, source } = error else {
        panic!("expected body read failure, got {error:?}");
    };
    assert_eq!(metadata.status(), StatusCode::OK);
    assert_eq!(metadata.headers()["x-request-id"], "decode-123");
    assert_eq!(metadata.url().path(), "/items");
    assert!(source.is_body() || source.is_decode());
    server.await.unwrap();
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe all supported RPC response shapes"
)]
#[tokio::test]
async fn fetch_decodes_scalar_set_object_and_void_rpc_results() {
    #[derive(serde::Deserialize)]
    struct Record {
        id: u32,
    }

    let (url, server) = serve(b"42", 2).await;
    let value: u32 = Postgrest::new(url)
        .unwrap()
        .rpc_json("number", &())
        .fetch()
        .await
        .unwrap();
    assert_eq!(value, 42);
    server.await.unwrap();
    let (url, server) = serve(b"[1,2]", 5).await;
    let value: Vec<u32> = Postgrest::new(url)
        .unwrap()
        .rpc("numbers", "{}")
        .fetch()
        .await
        .unwrap();
    assert_eq!(value, [1, 2]);
    server.await.unwrap();
    let body = br#"{"id":7}"#;
    let (url, server) = serve(body, body.len()).await;
    let value: Record = Postgrest::new(url)
        .unwrap()
        .rpc("record", "{}")
        .fetch()
        .await
        .unwrap();
    assert_eq!(value.id, 7);
    server.await.unwrap();
    let (url, server) = serve(b"null", 4).await;
    Postgrest::new(url)
        .unwrap()
        .rpc("nothing", "{}")
        .fetch::<()>()
        .await
        .unwrap();
    server.await.unwrap();
}
