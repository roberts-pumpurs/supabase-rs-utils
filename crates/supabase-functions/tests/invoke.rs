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
    for name in ["", "a/b", ".", "..", "hello\n", "hel\tlo", "hello\r"] {
        let error = client(&server).invoke(name).send().await.unwrap_err();
        assert!(
            matches!(&error, FunctionsError::InvalidFunctionName(got) if got == name),
            "{error:?}"
        );
    }
}

#[tokio::test]
async fn new_format_key_sends_only_apikey() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/functions/v1/hello")
        .match_header("apikey", "sb_publishable_abc")
        .match_header("authorization", Matcher::Missing)
        .create_async()
        .await;
    let url = url::Url::parse(&server.url()).unwrap();
    FunctionsClient::new(&url, "sb_publishable_abc")
        .unwrap()
        .invoke("hello")
        .send()
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn new_format_key_uses_access_token_override() {
    let mut server = mockito::Server::new_async().await;
    let mock = server
        .mock("POST", "/functions/v1/hello")
        .match_header("apikey", "sb_secret_abc")
        .match_header("authorization", "Bearer user-jwt")
        .create_async()
        .await;
    let url = url::Url::parse(&server.url()).unwrap();
    FunctionsClient::new(&url, "sb_secret_abc")
        .unwrap()
        .with_access_token("user-jwt")
        .unwrap()
        .invoke("hello")
        .send()
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn cross_origin_redirect_is_not_followed() {
    let mut first = mockito::Server::new_async().await;
    let mut second = mockito::Server::new_async().await;
    let target = format!("{}/functions/v1/hello", second.url());
    let redirect = first
        .mock("POST", "/functions/v1/hello")
        .with_status(302)
        .with_header("location", &target)
        .create_async()
        .await;
    let leaked = second
        .mock("POST", Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let leaked_get = second
        .mock("GET", Matcher::Any)
        .expect(0)
        .create_async()
        .await;

    let error = client(&first).invoke("hello").send().await.unwrap_err();

    assert!(matches!(error, FunctionsError::Http { status, .. } if status == 302));
    redirect.assert_async().await;
    leaked.assert_async().await;
    leaked_get.assert_async().await;
}

#[tokio::test]
async fn same_origin_redirect_is_followed() {
    let mut server = mockito::Server::new_async().await;
    let redirect = server
        .mock("GET", "/functions/v1/old")
        .with_status(302)
        .with_header("location", "/functions/v1/new")
        .create_async()
        .await;
    let target = server
        .mock("GET", "/functions/v1/new")
        .match_header("apikey", "anon-key")
        .with_body(r#"{"ok":true}"#)
        .create_async()
        .await;

    let reply: Reply = client(&server)
        .invoke("old")
        .method(Method::GET)
        .fetch()
        .await
        .unwrap();

    assert_eq!(reply, Reply { ok: true });
    redirect.assert_async().await;
    target.assert_async().await;
}

#[test]
fn credential_headers_are_redacted_in_debug() {
    let url = url::Url::parse("https://abc.supabase.co/").unwrap();
    let builder = FunctionsClient::new(&url, "sb_publishable_x")
        .unwrap()
        .invoke("hello")
        .header("Authorization", "Bearer secret-token")
        .header("APIKEY", "secret-key");

    let debug = format!("{builder:?}");

    assert!(!debug.contains("secret-token"), "{debug}");
    assert!(!debug.contains("secret-key"), "{debug}");
}

struct FailingSerialize;

impl serde::Serialize for FailingSerialize {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("refused"))
    }
}

#[tokio::test]
async fn first_builder_error_wins_and_sends_nothing() {
    let mut server = mockito::Server::new_async().await;
    let any = server
        .mock("POST", Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let api = client(&server);

    let name_error = api
        .invoke("hello")
        .header("bad name", "v")
        .header("x-ok", "v")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap_err();
    assert!(
        matches!(name_error, FunctionsError::HeaderName(_)),
        "{name_error:?}"
    );

    let value_error = api
        .invoke("hello")
        .header("x-bad", "line\nbreak")
        .header("bad name", "v")
        .body("raw", "text/plain")
        .send()
        .await
        .unwrap_err();
    assert!(
        matches!(value_error, FunctionsError::HeaderValue(_)),
        "{value_error:?}"
    );

    let serialize_error = api
        .invoke("hello")
        .json(&FailingSerialize)
        .header("bad name", "v")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap_err();
    assert!(
        matches!(serialize_error, FunctionsError::Serialize(_)),
        "{serialize_error:?}"
    );

    any.assert_async().await;
}
