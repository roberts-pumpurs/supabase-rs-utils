#![expect(
    clippy::tests_outside_test_module,
    reason = "This integration test crate is compiled only for tests"
)]

use core::fmt::Write as _;

use rp_postgrest::{
    Count, CountError, Error, Postgrest,
    reqwest::{Method, StatusCode},
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    net::TcpListener,
};

#[expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "Local HTTP fixture setup and bounded read slices must succeed"
)]
async fn serve(
    status: &str,
    range: Option<&str>,
    body: &[u8],
) -> (String, tokio::task::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\nX-Request-Id: counts\r\n",
        body.len()
    );
    if let Some(range) = range {
        write!(response, "Content-Range: {range}\r\n").unwrap();
    }
    response.push_str("\r\n");
    let mut response = response.into_bytes();
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
        String::from_utf8(request).unwrap()
    });
    (format!("http://{address}"), server)
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe server totals independent of page lengths"
)]
#[tokio::test]
async fn counted_reads_report_server_total_not_page_length() {
    for (range, body, expected, data) in [
        ("0-1/42", b"[1,2]".as_slice(), 42, vec![1, 2]),
        ("*/0", b"[]".as_slice(), 0, vec![]),
        ("10-10/99", b"[11]".as_slice(), 99, vec![11]),
    ] {
        let (url, server) = serve("200 OK", Some(range), body).await;
        let counted = Postgrest::new(url)
            .unwrap()
            .from("items")
            .range(10, 11)
            .limit(2)
            .fetch_with_count::<Vec<u32>>(Count::Exact)
            .await
            .unwrap();
        assert_eq!(counted.data, data);
        assert_eq!(counted.count, expected);
        assert_eq!(counted.metadata.count().unwrap(), expected);
        assert_eq!(counted.metadata.status(), StatusCode::OK);
        let request = server.await.unwrap().to_ascii_lowercase();
        assert!(request.starts_with("get /items?limit=2 "));
        assert!(request.contains("range: 10-11\r\n"));
        assert!(request.contains("prefer: count=exact\r\n"));
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe HEAD count request configuration"
)]
#[tokio::test]
async fn count_uses_head_and_preserves_read_configuration() {
    let (url, server) = serve("206 Partial Content", Some("3-4/57"), b"").await;
    let count = Postgrest::new(url)
        .unwrap()
        .from("items")
        .schema("private")
        .eq("id", "7")
        .range(3, 4)
        .limit(2)
        .execute_count(Count::Planned)
        .await
        .unwrap();
    assert_eq!(count, 57);
    let request = server.await.unwrap().to_ascii_lowercase();
    assert!(request.starts_with("head /items?id=eq.7&limit=2 "));
    assert!(request.contains("accept-profile: private\r\n"));
    assert!(request.contains("range: 3-4\r\n"));
    assert!(request.contains("prefer: count=planned\r\n"));
}

#[expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "Assertions and source matches describe invalid count evidence"
)]
#[tokio::test]
async fn missing_unavailable_and_invalid_counts_retain_evidence() {
    for (range, expected) in [
        (None, CountError::MissingContentRange),
        (Some("0-0/*"), CountError::UnavailableTotal),
        (Some("0-0/no"), CountError::InvalidContentRange),
        (
            Some("0-0/18446744073709551616"),
            CountError::InvalidContentRange,
        ),
        (Some("2-1/5"), CountError::InvalidContentRange),
        (Some("0-0/+5"), CountError::InvalidContentRange),
        (Some("0-0/5/6"), CountError::InvalidContentRange),
    ] {
        let (url, server) = serve("200 OK", range, b"[]").await;
        let error = Postgrest::new(url)
            .unwrap()
            .from("items")
            .fetch_with_count::<Vec<u32>>(Count::Exact)
            .await
            .unwrap_err();
        assert_eq!(error.status(), Some(StatusCode::OK));
        let Error::Count { metadata, source } = error else {
            panic!("expected count error, got {error:?}");
        };
        assert_eq!(source, expected);
        assert_eq!(metadata.headers()["x-request-id"], "counts");
        assert_eq!(metadata.url().path(), "/items");
        server.await.unwrap();
    }
    let (url, server) = serve("200 OK", None, b"").await;
    let error = Postgrest::new(url)
        .unwrap()
        .from("items")
        .execute_count(Count::Exact)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        Error::Count {
            source: CountError::MissingContentRange,
            ..
        }
    ));
    server.await.unwrap();
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe mutation methods and composed count preferences"
)]
#[tokio::test]
async fn affected_counts_preserve_write_methods_and_other_preferences() {
    for method in [Method::POST, Method::PATCH, Method::DELETE] {
        let (url, server) = serve("204 No Content", Some("*/7"), b"").await;
        let builder = Postgrest::new(url).unwrap().from("items");
        let builder = match method {
            Method::POST => builder.upsert("{}"),
            Method::PATCH => builder.update("{}"),
            _ => builder.delete(),
        };
        let count = builder
            .schema("private")
            .insert_header("prefer", "tx=rollback")
            .execute_count(Count::Estimated)
            .await
            .unwrap();
        assert_eq!(count, 7);
        let request = server.await.unwrap();
        assert!(
            request.starts_with(&format!("{method} /items ")),
            "Counted writes must preserve the original request method"
        );
        let request = request.to_ascii_lowercase();
        assert!(request.contains("return=minimal"));
        assert!(!request.contains("return=representation"));
        assert!(request.contains("count=estimated"));
        assert!(request.contains("tx=rollback"));
        assert!(request.contains("content-profile: private"));
        if method == Method::POST {
            assert!(request.contains("resolution=merge-duplicates"));
        }
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe minimal and counted void decoding"
)]
#[tokio::test]
async fn minimal_writes_and_counted_void_preserve_204_decoding() {
    let (url, server) = serve("204 No Content", None, b"").await;
    Postgrest::new(url)
        .unwrap()
        .from("items")
        .insert("{}")
        .return_minimal()
        .fetch::<()>()
        .await
        .unwrap();
    assert!(server.await.unwrap().contains("return=minimal"));
    let (url, server) = serve("204 No Content", Some("*/2"), b"").await;
    let counted = Postgrest::new(url)
        .unwrap()
        .from("items")
        .delete()
        .return_minimal()
        .fetch_with_count::<()>(Count::Exact)
        .await
        .unwrap();
    assert_eq!(counted.count, 2);
    server.await.unwrap();
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe RPC count method preservation"
)]
#[tokio::test]
async fn count_rpc_does_not_change_its_method() {
    for method in [Method::POST, Method::GET] {
        let (url, server) = serve("200 OK", Some("0-0/1"), b"1").await;
        let count = Postgrest::new(url)
            .unwrap()
            .rpc("number", "{}")
            .method(method.clone())
            .execute_count(Count::Exact)
            .await
            .unwrap();
        assert_eq!(count, 1);
        assert!(
            server
                .await
                .unwrap()
                .starts_with(&format!("{method} /rpc/number ")),
            "RPC counts must preserve the configured request method"
        );
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe count preference replacement"
)]
#[test]
fn count_helpers_replace_only_the_count_preference() {
    let request = Postgrest::new("http://localhost")
        .unwrap()
        .from("items")
        .upsert("{}")
        .insert_header("prefer", "tx=rollback,count=planned")
        .exact_count()
        .planned_count()
        .estimated_count()
        .return_minimal()
        .limit(0)
        .build()
        .unwrap()
        .build()
        .unwrap();
    let prefer = request.headers()["prefer"].to_str().unwrap();
    assert_eq!(
        prefer,
        "return=minimal,resolution=merge-duplicates,tx=rollback,count=estimated"
    );
    assert_eq!(request.url().query(), Some("limit=0"));
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe checked and decode errors for counted fetches"
)]
#[tokio::test]
async fn counted_fetch_keeps_checked_errors_and_decode_errors() {
    let (url, server) = serve("200 OK", Some("0-0/1"), b"not json").await;
    let error = Postgrest::new(url)
        .unwrap()
        .from("items")
        .fetch_with_count::<Vec<u32>>(Count::Exact)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::ResponseDecode { .. }));
    server.await.unwrap();
    let body = br#"{"code":"23505","message":"duplicate","details":null,"hint":null}"#;
    let (url, server) = serve("409 Conflict", Some("*/1"), body).await;
    let error = Postgrest::new(url)
        .unwrap()
        .from("items")
        .insert("{}")
        .execute_count(Count::Exact)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Postgrest { .. }));
    server.await.unwrap();
}

struct Fails;
impl serde::Serialize for Fails {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("count serialization failure"))
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "Assertions describe deferred serialization failures"
)]
#[tokio::test]
async fn count_and_minimal_operations_preserve_deferred_serialization_errors() {
    let client = Postgrest::new("http://127.0.0.1:1").unwrap();
    let error = client
        .from("items")
        .insert_json(&Fails)
        .return_minimal()
        .execute_count(Count::Exact)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Serialization(_)));
    let error = client
        .from("items")
        .update_json(&Fails)
        .fetch_with_count::<Vec<u32>>(Count::Exact)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Serialization(_)));
}
