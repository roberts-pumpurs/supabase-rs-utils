// Adapted from rp-postgrest 2.1.0 (MIT OR Apache-2.0).
#![expect(
    clippy::tests_outside_test_module,
    reason = "This integration test crate is compiled only for tests"
)]

use core::future::pending;
use core::time::Duration;

use rp_postgrest::reqwest::StatusCode;
use rp_postgrest::rp_postgrest_error::{ErrorKind, PostgrestErrorCode};
use rp_postgrest::{Error, Postgrest};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpListener;

#[expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "Local HTTP fixture setup and bounded read slices must succeed"
)]
async fn serve_once(
    status_line: &str,
    content_type: &str,
    body: &[u8],
    declared_length: usize,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut response = format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {declared_length}\r\nX-Request-Id: request-123\r\nConnection: close\r\n\r\n"
    )
    .into_bytes();
    response.extend_from_slice(body);

    let server = tokio::spawn(async move {
        let (mut connection, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        loop {
            let mut chunk = [0_u8; 1024];
            let read = connection.read(&mut chunk).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        connection.write_all(&response).await.unwrap();
        connection.shutdown().await.unwrap();
    });

    (format!("http://{address}"), server)
}

#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Assertions describe the successful response contract"
)]
#[tokio::test]
async fn checked_execution_preserves_success_response() {
    let body = br#"{"ok":true}"#;
    let (url, server) = serve_once("201 Created", "application/json", body, body.len()).await;

    let response = Postgrest::new(url)
        .unwrap()
        .from("items")
        .insert(r#"{"name":"rust"}"#)
        .execute_checked()
        .await
        .expect("success response should be returned");

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(response.headers()["x-request-id"], "request-123");
    assert_eq!(response.url().path(), "/items");
    assert_eq!(response.bytes().await.unwrap().as_ref(), body);
    server.await.unwrap();
}

#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Assertions describe the raw response contract"
)]
#[tokio::test]
async fn raw_execution_preserves_non_success_response() {
    let body = b"raw upstream failure";
    let (url, server) = serve_once("502 Bad Gateway", "text/plain", body, body.len()).await;

    let response = Postgrest::new(url)
        .unwrap()
        .from("items")
        .execute()
        .await
        .expect("raw execution should not interpret the status");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(response.headers()["x-request-id"], "request-123");
    assert_eq!(response.url().path(), "/items");
    assert_eq!(response.bytes().await.unwrap().as_ref(), body);
    server.await.unwrap();
}

#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "Assertions and exhaustive source matches describe structured error evidence"
)]
#[tokio::test]
async fn checked_execution_decodes_structured_postgrest_error() {
    let body = br#"{"code":"PGRST201","message":"ambiguous relationship","details":[{"cardinality":"many-to-one","embedding":"items with owners","relationship":"items_owner_fkey using items(owner_id) and owners(id)"}],"hint":"disambiguate"}"#;
    let (url, server) =
        serve_once("300 Multiple Choices", "application/json", body, body.len()).await;

    let error = Postgrest::new(url)
        .unwrap()
        .from("items")
        .select("*")
        .execute_checked()
        .await
        .expect_err("non-success response should be an error");

    assert_eq!(error.status(), Some(StatusCode::MULTIPLE_CHOICES));
    assert_eq!(error.url().unwrap().path(), "/items");
    assert_eq!(error.url().unwrap().query(), Some("select=*"));
    let metadata = error.response_metadata().unwrap();
    assert_eq!(metadata.status(), StatusCode::MULTIPLE_CHOICES);
    assert_eq!(metadata.headers()["x-request-id"], "request-123");

    let Error::Postgrest { metadata, source } = error else {
        panic!("expected structured PostgREST error, got {error:?}");
    };
    assert_eq!(metadata.url().path(), "/items");
    assert_eq!(source.status(), StatusCode::MULTIPLE_CHOICES);
    assert_eq!(source.code().as_ref(), "PGRST201");
    assert_eq!(
        source.kind(),
        ErrorKind::Postgrest(PostgrestErrorCode::AmbiguousEmbedding)
    );
    assert_eq!(source.response().message, "ambiguous relationship");
    let details = source.response().details.as_ref().unwrap();
    let rp_postgrest::rp_postgrest_error::ErrorDetails::AmbiguousEmbeddings(candidates) = details
    else {
        panic!("expected real PGRST201 details array, got {details:?}");
    };
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].embedding, "items with owners");
    assert_eq!(
        candidates[0].relationship,
        "items_owner_fkey using items(owner_id) and owners(id)"
    );
    server.await.unwrap();
}

#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Assertions describe authoritative status and borrowed structured error evidence"
)]
#[tokio::test]
async fn structured_response_accessor_uses_observed_status_not_error_code() {
    let body = br#"{"code":"23505","message":"duplicate key","details":null,"hint":null}"#;
    for (status_line, status) in [
        ("400 Bad Request", StatusCode::BAD_REQUEST),
        ("300 Multiple Choices", StatusCode::MULTIPLE_CHOICES),
    ] {
        let (url, server) = serve_once(status_line, "application/json", body, body.len()).await;
        let error = Postgrest::new(url)
            .unwrap()
            .from("items")
            .execute_checked()
            .await
            .expect_err("structured non-success response should be an error");

        let (observed_status, response) = error.postgrest_response().unwrap();
        assert_eq!(observed_status, status);
        assert_eq!(error.status(), Some(observed_status));
        assert_eq!(
            error
                .postgrest_error()
                .unwrap()
                .inferred_status(rp_postgrest::rp_postgrest_error::Authentication::Unknown),
            Some(StatusCode::CONFLICT)
        );
        assert_eq!(response.message, "duplicate key");
        assert_eq!(response.code.as_str(), "23505");
        server.await.unwrap();
    }
}

#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Assertions and source matches describe malformed error evidence"
)]
#[tokio::test]
async fn checked_execution_preserves_malformed_error_body() {
    let body = b"upstream failure: \xff";
    let (url, server) = serve_once("502 Bad Gateway", "text/plain", body, body.len()).await;

    let error = Postgrest::new(url)
        .unwrap()
        .from("items")
        .execute_checked()
        .await
        .expect_err("malformed non-success response should be an error");

    assert_eq!(error.status(), Some(StatusCode::BAD_GATEWAY));
    assert!(error.postgrest_response().is_none());
    assert_eq!(error.url().unwrap().path(), "/items");
    assert_eq!(
        error.response_metadata().unwrap().headers()["x-request-id"],
        "request-123"
    );
    let Error::Decode { metadata, source } = error else {
        panic!("expected decode error, got {error:?}");
    };
    assert_eq!(metadata.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(source.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(source.body(), body);
    server.await.unwrap();
}

#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Assertions and source matches describe truncated body evidence"
)]
#[tokio::test]
async fn checked_execution_preserves_status_when_body_read_fails() {
    let body = b"short";
    let (url, server) = serve_once("503 Service Unavailable", "text/plain", body, 100).await;

    let error = Postgrest::new(url)
        .unwrap()
        .from("items")
        .execute_checked()
        .await
        .expect_err("truncated response body should be an error");

    assert_eq!(error.status(), Some(StatusCode::SERVICE_UNAVAILABLE));
    assert!(error.postgrest_response().is_none());
    let Error::ResponseBody { metadata, source } = error else {
        panic!("expected response body error, got {error:?}");
    };
    assert_eq!(metadata.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(metadata.headers()["x-request-id"], "request-123");
    assert_eq!(metadata.url().path(), "/items");
    assert!(source.is_body() || source.is_decode());
    server.await.unwrap();
}

#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "Assertions and source matches describe transport failure evidence"
)]
#[tokio::test]
async fn checked_execution_preserves_request_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (_connection, _) = listener.accept().await.unwrap();
        pending::<()>().await;
    });
    let client = rp_postgrest::reqwest::Client::builder()
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();

    let request = Postgrest::new_with_client(format!("http://{address}"), client)
        .unwrap()
        .from("items")
        .execute_checked();
    let result = tokio::time::timeout(Duration::from_secs(5), request).await;
    server.abort();
    let error = result
        .expect("request exceeded the independent test deadline")
        .expect_err("silent server should time out");

    assert_eq!(error.status(), None);
    assert_eq!(error.url().unwrap().path(), "/items");
    assert!(error.response_metadata().is_none());
    assert!(error.postgrest_response().is_none());
    let Error::Request(source) = error else {
        panic!("expected request error, got {error:?}");
    };
    assert!(source.is_timeout());
}
