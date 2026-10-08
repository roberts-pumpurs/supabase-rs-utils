//! Request and response behavior of `FunctionsClient`.
#![expect(
    clippy::tests_outside_test_module,
    reason = "integration tests live in the tests directory"
)]
#![expect(clippy::unwrap_used, reason = "tests fail loudly on unexpected errors")]
#![expect(clippy::panic, reason = "tests fail loudly on unexpected variants")]

use mockito::Matcher;
use rp_supabase_functions::{FunctionsClient, FunctionsError, Method};
use serde::Deserialize;

fn client(server: &mockito::Server) -> FunctionsClient {
    let url = url::Url::parse(&server.url()).unwrap();
    FunctionsClient::new(&url, "anon-key").unwrap()
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Reply {
    ok: bool,
}

#[tokio::test]
async fn json_post_sends_headers_and_decodes() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/functions/v1/hello")
        .match_header("apikey", "anon-key")
        .match_header("authorization", "Bearer anon-key")
        .match_header("content-type", "application/json")
        .match_header("x-region", "eu-west-1")
        .match_body(Matcher::Json(serde_json::json!({"name": "Ada"})))
        .with_body(r#"{"ok":true}"#)
        .create_async()
        .await;

    let reply: Reply = client(&server)
        .invoke("hello")
        .region("eu-west-1")
        .json(&serde_json::json!({"name": "Ada"}))
        .fetch()
        .await
        .unwrap();

    assert_eq!(reply, Reply { ok: true });
    mock.assert_async().await;
}

#[tokio::test]
async fn raw_body_custom_method_and_user_token() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("PUT", "/functions/v1/a%20b")
        .match_header("authorization", "Bearer user-jwt")
        .match_header("content-type", "text/plain")
        .match_header("x-custom", "1")
        .match_body("raw")
        .with_body("done")
        .create_async()
        .await;

    let text = client(&server)
        .with_access_token("user-jwt")
        .unwrap()
        .invoke("a b")
        .method(Method::PUT)
        .header("x-custom", "1")
        .body("raw", "text/plain")
        .text()
        .await
        .unwrap();

    assert_eq!(text, "done");
    mock.assert_async().await;
}

#[tokio::test]
async fn non_success_status_maps_to_http_error() {
    let mut server = mockito::Server::new_async().await;
    for status in [400_u16, 500] {
        let _mock = server
            .mock("POST", "/functions/v1/fail")
            .with_status(usize::from(status))
            .with_body("boom")
            .create_async()
            .await;
        let error = client(&server).invoke("fail").send().await.unwrap_err();
        let FunctionsError::Http { status: got, body } = error else {
            panic!("unexpected error: {error:?}");
        };
        assert_eq!(got.as_u16(), status);
        assert_eq!(body, "boom");
    }
}

#[tokio::test]
async fn relay_header_maps_to_relay_error() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/functions/v1/relay")
        .with_status(502)
        .with_header("x-relay-error", "true")
        .with_body("relay down")
        .create_async()
        .await;

    let error = client(&server).invoke("relay").send().await.unwrap_err();
    let FunctionsError::Relay { status, body } = error else {
        panic!("unexpected error: {error:?}");
    };
    assert_eq!(status.as_u16(), 502);
    assert_eq!(body, "relay down");
}

#[tokio::test]
async fn invalid_json_maps_to_decode_error() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("POST", "/functions/v1/bad")
        .with_body("not json")
        .create_async()
        .await;

    let error = client(&server)
        .invoke("bad")
        .fetch::<Reply>()
        .await
        .unwrap_err();
    assert!(matches!(error, FunctionsError::Decode(_)), "{error:?}");
}

#[tokio::test]
async fn invalid_names_fail_without_request() {
    let server = mockito::Server::new_async().await;
    for name in ["", "a/b"] {
        let error = client(&server).invoke(name).send().await.unwrap_err();
        assert!(
            matches!(&error, FunctionsError::InvalidFunctionName(got) if got == name),
            "{error:?}"
        );
    }
}
