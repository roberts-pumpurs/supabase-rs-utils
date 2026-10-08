#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public auth interfaces."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Test setup and assertions retain failure details."
)]

use mockito::{Matcher, Mock, ServerGuard};
use pretty_assertions::assert_eq;
use rp_supabase_auth::auth_client::ApiClient;
use rp_supabase_auth::auth_client::requests::{OtpRequest, UserUpdateRequest, VerifyPostRequest};
use rp_supabase_auth::error::AuthError;
use rp_supabase_auth::types::{LoginCredentials, SignupPayload, SignupResponse};

const USER: &str = r#"{
    "id": "7d1e5b4c-7f1e-4d0a-9a39-0c2b4c4f2d11",
    "aud": "authenticated",
    "role": "authenticated",
    "email": "user@example.com",
    "email_confirmed_at": "2026-10-01T12:00:00Z",
    "phone": "",
    "app_metadata": {"provider": "email", "providers": ["email"]},
    "user_metadata": {},
    "created_at": "2026-10-01T12:00:00Z",
    "updated_at": "2026-10-01T12:00:00Z"
}"#;

fn session() -> String {
    format!(
        r#"{{"access_token":"new-access","token_type":"bearer","expires_in":3600,"expires_at":1791900000,"refresh_token":"new-refresh","user":{USER}}}"#
    )
}

async fn server() -> ServerGuard {
    mockito::Server::new_async().await
}

fn anonymous(server: &ServerGuard) -> ApiClient {
    ApiClient::new_unauthenticated(&url::Url::parse(&server.url()).unwrap(), "anon-key").unwrap()
}

fn authenticated(server: &ServerGuard) -> ApiClient {
    ApiClient::new_authenticated(
        &url::Url::parse(&server.url()).unwrap(),
        "anon-key",
        "user-jwt",
    )
    .unwrap()
}

/// Mock that requires the apikey header and no bearer token.
fn anonymous_mock(server: &mut ServerGuard, method: &str, path: &str) -> Mock {
    server
        .mock(method, path)
        .match_header("apikey", "anon-key")
        .match_header("authorization", Matcher::Missing)
}

/// Mock that requires the apikey header and the user bearer token.
fn bearer_mock(server: &mut ServerGuard, method: &str, path: &str) -> Mock {
    server
        .mock(method, path)
        .match_header("apikey", "anon-key")
        .match_header("authorization", "Bearer user-jwt")
}

#[tokio::test]
async fn sign_in_with_password_posts_credentials_and_decodes_session() {
    let mut server = server().await;
    let mock = anonymous_mock(&mut server, "POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded("grant_type".into(), "password".into()))
        .match_body(Matcher::PartialJsonString(
            r#"{"email":"user@example.com","password":"secret"}"#.into(),
        ))
        .with_body(session())
        .create_async()
        .await;

    let session = anonymous(&server)
        .sign_in_with_password(
            &LoginCredentials::builder()
                .email("user@example.com".to_owned())
                .password("secret".to_owned())
                .build(),
        )
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(session.access_token.as_deref(), Some("new-access"));
    assert_eq!(session.refresh_token.as_deref(), Some("new-refresh"));
    assert_eq!(session.expires_in, Some(3600));
    assert_eq!(
        session.user.unwrap().email.as_deref(),
        Some("user@example.com")
    );
}

#[tokio::test]
async fn refresh_session_posts_refresh_token() {
    let mut server = server().await;
    let mock = anonymous_mock(&mut server, "POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded(
            "grant_type".into(),
            "refresh_token".into(),
        ))
        .match_body(Matcher::PartialJsonString(
            r#"{"refresh_token":"old-refresh"}"#.into(),
        ))
        .with_body(session())
        .create_async()
        .await;

    let session = anonymous(&server)
        .refresh_session("old-refresh")
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(session.access_token.as_deref(), Some("new-access"));
}

#[expect(
    clippy::panic,
    reason = "Any other response variant fails this decoding test"
)]
#[tokio::test]
async fn sign_up_posts_payload_and_decodes_session() {
    let mut server = server().await;
    let mock = anonymous_mock(&mut server, "POST", "/auth/v1/signup")
        .match_body(Matcher::PartialJsonString(
            r#"{"email":"new@example.com","password":"secret"}"#.into(),
        ))
        .with_body(session())
        .create_async()
        .await;

    let response = anonymous(&server)
        .sign_up(
            SignupPayload::builder()
                .email("new@example.com".to_owned())
                .password("secret".to_owned())
                .build(),
        )
        .await
        .unwrap();

    mock.assert_async().await;
    let SignupResponse::Session(session) = response else {
        panic!("expected a session, got {response:?}");
    };
    assert_eq!(session.access_token.as_deref(), Some("new-access"));
}

#[expect(
    clippy::panic,
    reason = "Any other response variant fails this decoding test"
)]
#[tokio::test]
async fn sign_up_with_confirmation_required_decodes_user() {
    let mut server = server().await;
    let _mock = anonymous_mock(&mut server, "POST", "/auth/v1/signup")
        .with_body(USER)
        .create_async()
        .await;

    let response = anonymous(&server)
        .sign_up(
            SignupPayload::builder()
                .email("user@example.com".to_owned())
                .password("secret".to_owned())
                .build(),
        )
        .await
        .unwrap();

    let SignupResponse::ConfirmationRequired(user) = response else {
        panic!("expected a confirmation-required user, got {response:?}");
    };
    assert_eq!(user.email.as_deref(), Some("user@example.com"));
}

#[tokio::test]
async fn sign_in_with_otp_posts_email_without_unset_fields() {
    let mut server = server().await;
    let mock = anonymous_mock(&mut server, "POST", "/auth/v1/otp")
        .match_body(Matcher::JsonString(
            r#"{"email":"user@example.com","create_user":false}"#.into(),
        ))
        .with_body(r#"{"message_id":null}"#)
        .create_async()
        .await;

    let response = anonymous(&server)
        .sign_in_with_otp(
            &OtpRequest::builder()
                .email("user@example.com".to_owned())
                .create_user(false)
                .build(),
        )
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(response.message_id, None);
}

#[tokio::test]
async fn verify_otp_posts_type_and_token_and_decodes_session() {
    let mut server = server().await;
    let mock = anonymous_mock(&mut server, "POST", "/auth/v1/verify")
        .match_body(Matcher::JsonString(
            r#"{"type":"email","token":"123456","email":"user@example.com"}"#.into(),
        ))
        .with_body(session())
        .create_async()
        .await;

    let session = anonymous(&server)
        .verify_otp(
            &VerifyPostRequest::builder()
                .verification_type("email".to_owned())
                .token("123456".to_owned())
                .email("user@example.com".to_owned())
                .build(),
        )
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(session.access_token.as_deref(), Some("new-access"));
}

#[tokio::test]
async fn reset_password_for_email_posts_recover_and_accepts_empty_object() {
    let mut server = server().await;
    let mock = anonymous_mock(&mut server, "POST", "/auth/v1/recover")
        .match_body(Matcher::JsonString(
            r#"{"email":"user@example.com"}"#.into(),
        ))
        .with_body("{}")
        .create_async()
        .await;

    anonymous(&server)
        .reset_password_for_email("user@example.com")
        .await
        .unwrap();

    mock.assert_async().await;
}

#[tokio::test]
async fn get_user_sends_bearer_and_decodes_user() {
    let mut server = server().await;
    let mock = bearer_mock(&mut server, "GET", "/auth/v1/user")
        .with_body(USER)
        .create_async()
        .await;

    let user = authenticated(&server).get_user().await.unwrap();

    mock.assert_async().await;
    assert_eq!(user.email.as_deref(), Some("user@example.com"));
    assert_eq!(user.role.as_deref(), Some("authenticated"));
}

#[tokio::test]
async fn update_user_puts_only_set_fields() {
    let mut server = server().await;
    let mock = bearer_mock(&mut server, "PUT", "/auth/v1/user")
        .match_body(Matcher::JsonString(r#"{"password":"new-secret"}"#.into()))
        .with_body(USER)
        .create_async()
        .await;

    let user = authenticated(&server)
        .update_user(
            &UserUpdateRequest::builder()
                .password("new-secret".to_owned())
                .build(),
        )
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(user.email.as_deref(), Some("user@example.com"));
}

#[tokio::test]
async fn sign_out_posts_logout_and_accepts_no_content() {
    let mut server = server().await;
    let mock = bearer_mock(&mut server, "POST", "/auth/v1/logout")
        .match_query(Matcher::Missing)
        .with_status(204)
        .create_async()
        .await;

    authenticated(&server).sign_out().await.unwrap();

    mock.assert_async().await;
}

#[rstest::rstest]
#[case::legacy(
    r#"{"code":400,"error_code":"invalid_credentials","msg":"Invalid login credentials"}"#,
    Some(400_i32)
)]
#[case::api_version_2024_01_01(
    r#"{"code":"invalid_credentials","message":"Invalid login credentials"}"#,
    None
)]
#[tokio::test]
async fn gotrue_error_body_maps_to_api_error_with_status_and_message(
    #[case] body: &str,
    #[case] code: Option<i32>,
) {
    let mut server = server().await;
    let _mock = anonymous_mock(&mut server, "POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded("grant_type".into(), "password".into()))
        .with_status(400)
        .with_body(body)
        .create_async()
        .await;

    let error = anonymous(&server)
        .sign_in_with_password(
            &LoginCredentials::builder()
                .email("user@example.com".to_owned())
                .password("wrong".to_owned())
                .build(),
        )
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Supabase Auth returned 400 Bad Request: Invalid login credentials; code: invalid_credentials"
    );
    #[expect(
        clippy::panic,
        reason = "Any other error variant fails this error-mapping test"
    )]
    let AuthError::Api {
        status,
        error: body,
    } = error
    else {
        panic!("expected API error, got {error:?}");
    };
    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(body.msg.as_deref(), Some("Invalid login credentials"));
    assert_eq!(body.error_code.as_deref(), Some("invalid_credentials"));
    assert_eq!(body.code, code);
}

#[expect(
    clippy::panic,
    reason = "Any other error variant fails this error-mapping test"
)]
#[tokio::test]
async fn non_json_error_body_keeps_status_and_text() {
    let mut server = server().await;
    let _mock = bearer_mock(&mut server, "GET", "/auth/v1/user")
        .with_status(502)
        .with_body("upstream unavailable")
        .create_async()
        .await;

    let error = authenticated(&server).get_user().await.unwrap_err();

    let AuthError::Api {
        status,
        error: body,
    } = error
    else {
        panic!("expected API error, got {error:?}");
    };
    assert_eq!(status, reqwest::StatusCode::BAD_GATEWAY);
    assert_eq!(body.msg.as_deref(), Some("upstream unavailable"));
}
