#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public auth interfaces."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Test setup and assertions retain failure details."
)]

use core::time::Duration;
use futures::StreamExt as _;
use mockito::Matcher;
use rp_supabase_auth::{
    auth_client::{
        ApiClient, new_authenticated_stream_with_client,
        requests::{GrantType, TokenRequest},
    },
    jwt_stream::{JwtStream, SupabaseAuthConfig},
    types::{LoginCredentials, TokenRequestBody},
};

fn http() -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        "x-transport",
        reqwest::header::HeaderValue::from_static("retained"),
    );
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_static("Bearer original"),
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

fn request() -> TokenRequest {
    TokenRequest::builder()
        .grant_type(GrantType::Password)
        .payload(TokenRequestBody::builder().build())
        .build()
}

fn config(url: &str) -> SupabaseAuthConfig {
    SupabaseAuthConfig {
        api_key: "key".into(),
        url: url::Url::parse(url).unwrap(),
        max_reconnect_attempts: 2,
        reconnect_interval: Duration::from_millis(1),
    }
}

#[tokio::test]
async fn configured_api_clients_keep_headers_and_do_not_mutate_shared_bearer() {
    let mut server = mockito::Server::new_async().await;
    let authenticated = server
        .mock("POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded("grant_type".into(), "password".into()))
        .match_header("apikey", "key")
        .match_header("x-transport", "retained")
        .match_header("authorization", "Bearer user")
        .match_header("accept", "application/json")
        .match_header("content-type", "application/json")
        .with_body("{}")
        .create_async()
        .await;
    let unauthenticated = server
        .mock("POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded("grant_type".into(), "password".into()))
        .match_header("apikey", "key")
        .match_header("x-transport", "retained")
        .match_header("authorization", "Bearer original")
        .with_body("{}")
        .create_async()
        .await;
    let url = url::Url::parse(&server.url()).unwrap();
    let transport = http();
    ApiClient::new_authenticated_with_client(&url, "key", "user", transport.clone())
        .unwrap()
        .build_request(&request())
        .unwrap()
        .execute()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    ApiClient::new_unauthenticated_with_client(&url, "key", transport)
        .unwrap()
        .build_request(&request())
        .unwrap()
        .execute()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    authenticated.assert_async().await;
    unauthenticated.assert_async().await;
}

#[tokio::test]
async fn sign_in_configured_transport_preserves_redirect_policy() {
    let mut server = mockito::Server::new_async().await;
    let login = server
        .mock("POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded("grant_type".into(), "password".into()))
        .match_header("x-transport", "retained")
        .with_status(302)
        .with_header("location", "/unexpected")
        .with_body(r#"{"msg":"redirect rejected"}"#)
        .create_async()
        .await;
    let target = server
        .mock("GET", "/unexpected")
        .with_body(r#"{"access_token":"unexpected-follow"}"#)
        .expect(0)
        .create_async()
        .await;
    let mut stream = JwtStream::new(config(&server.url()))
        .sign_in_with_client(LoginCredentials::builder().build(), http())
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(3), stream.next())
        .await
        .unwrap()
        .unwrap();
    result.unwrap_err();
    login.assert_async().await;
    target.assert_async().await;
}

#[tokio::test]
async fn authenticated_auth_stream_reuses_transport_after_refresh() {
    let mut server = mockito::Server::new_async().await;
    let login = server
        .mock("POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded("grant_type".into(), "password".into()))
        .match_header("x-transport", "retained")
        .match_header("authorization", "Bearer original")
        .with_body(r#"{"access_token":"first","refresh_token":"refresh","expires_in":0}"#)
        .create_async()
        .await;
    let refresh = server
        .mock("POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded(
            "grant_type".into(),
            "refresh_token".into(),
        ))
        .match_header("x-transport", "retained")
        .match_header("authorization", "Bearer original")
        .match_body(Matcher::Regex(r#""refresh_token":"refresh""#.into()))
        .with_body(r#"{"access_token":"second"}"#)
        .create_async()
        .await;
    let user_request = server
        .mock("POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded("grant_type".into(), "password".into()))
        .match_header("x-transport", "retained")
        .match_header("authorization", "Bearer second")
        .match_header("apikey", "key")
        .with_body("{}")
        .create_async()
        .await;
    let stream = new_authenticated_stream_with_client(
        config(&server.url()),
        LoginCredentials::builder().build(),
        http(),
    )
    .unwrap();
    futures::pin_mut!(stream);
    let _first = tokio::time::timeout(Duration::from_secs(3), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let second = tokio::time::timeout(Duration::from_secs(3), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    second
        .build_request(&request())
        .unwrap()
        .execute()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    login.assert_async().await;
    refresh.assert_async().await;
    user_request.assert_async().await;
}

#[expect(
    clippy::panic,
    reason = "An unexpected auth error fails this timeout contract test"
)]
#[tokio::test]
async fn configured_sign_in_preserves_total_timeout() {
    let mut server = mockito::Server::new_async().await;
    let _slow = server
        .mock("POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded("grant_type".into(), "password".into()))
        .with_chunked_body(|writer| {
            std::thread::sleep(Duration::from_millis(150));
            writer.write_all(b"{}")
        })
        .create_async()
        .await;
    let transport = reqwest::Client::builder()
        .timeout(Duration::from_millis(20))
        .build()
        .unwrap();
    let mut stream = JwtStream::new(config(&server.url()))
        .sign_in_with_client(LoginCredentials::builder().build(), transport)
        .unwrap();
    let error = tokio::time::timeout(Duration::from_secs(3), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    let rp_supabase_auth::jwt_stream::RefreshStreamError::AuthError(
        rp_supabase_auth::error::AuthError::Reqwest(source),
    ) = error
    else {
        panic!("expected configured timeout");
    };
    assert!(source.is_timeout());
}

#[tokio::test]
async fn default_client_does_not_follow_cross_origin_redirect() {
    let mut origin = mockito::Server::new_async().await;
    let mut other = mockito::Server::new_async().await;
    let redirect = origin
        .mock("POST", "/auth/v1/recover")
        .with_status(302)
        .with_header("location", &format!("{}/steal", other.url()))
        .create_async()
        .await;
    let leaked = other.mock("GET", "/steal").expect(0).create_async().await;
    let client =
        ApiClient::new_unauthenticated(&url::Url::parse(&origin.url()).unwrap(), "anon-key")
            .unwrap();

    let result = client.reset_password_for_email("user@example.com").await;

    assert!(result.is_err());
    redirect.assert_async().await;
    leaked.assert_async().await;
}

#[tokio::test]
async fn default_client_follows_same_origin_redirect() {
    let mut server = mockito::Server::new_async().await;
    let redirect = server
        .mock("POST", "/auth/v1/recover")
        .with_status(302)
        .with_header("location", "/moved")
        .create_async()
        .await;
    let target = server
        .mock("GET", "/moved")
        .match_header("apikey", "anon-key")
        .with_body("{}")
        .create_async()
        .await;
    let client =
        ApiClient::new_unauthenticated(&url::Url::parse(&server.url()).unwrap(), "anon-key")
            .unwrap();

    client
        .reset_password_for_email("user@example.com")
        .await
        .unwrap();

    redirect.assert_async().await;
    target.assert_async().await;
}
