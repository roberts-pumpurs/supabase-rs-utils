#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public auth interfaces."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Test setup and assertions retain failure details."
)]

extern crate alloc;

use alloc::sync::Arc;
use std::sync::{Mutex, PoisonError};

use mockito::Matcher;
use rp_supabase_auth::{
    auth_client::{
        ApiClient,
        requests::{GrantType, TokenRequest, VerifyGetRequest},
    },
    types::TokenRequestBody,
};

const PASSWORD: &str = "secret-password-value";
const ACCESS_TOKEN: &str = "secret-access-token-value";
const REFRESH_TOKEN: &str = "secret-refresh-token-value";
const VERIFY_TOKEN: &str = "secret-verify-token-value";

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Keep this the only test in this binary. Other tests that run first without a
// subscriber can cache callsite interest and hide events from this capture.
#[tokio::test]
async fn logs_do_not_contain_secrets() {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let mut server = mockito::Server::new_async().await;
    let token = server
        .mock("POST", "/auth/v1/token")
        .match_query(Matcher::UrlEncoded("grant_type".into(), "password".into()))
        .with_body(format!(
            r#"{{"access_token":"{ACCESS_TOKEN}","refresh_token":"{REFRESH_TOKEN}"}}"#
        ))
        .create_async()
        .await;
    let verify = server
        .mock("GET", "/auth/v1/verify")
        .match_query(Matcher::Any)
        .with_status(400)
        .with_body(format!(r#"{{"msg":"bad token {VERIFY_TOKEN}"}}"#))
        .create_async()
        .await;
    let url = url::Url::parse(&server.url()).unwrap();
    let client = ApiClient::new_unauthenticated(&url, "key").unwrap();

    let request = TokenRequest::builder()
        .grant_type(GrantType::Password)
        .payload(
            TokenRequestBody::builder()
                .email(Some("user@example.com".into()))
                .password(Some(PASSWORD.into()))
                .build(),
        )
        .build();
    client
        .build_request(&request)
        .unwrap()
        .execute()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let verify_request = VerifyGetRequest::builder()
        .token(VERIFY_TOKEN.into())
        .verification_type("signup".into())
        .redirect_to(None)
        .build();
    client
        .build_request(&verify_request)
        .unwrap()
        .execute()
        .await
        .unwrap()
        .ok()
        .await
        .unwrap_err();

    token.assert_async().await;
    verify.assert_async().await;

    let logs = String::from_utf8(
        capture
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone(),
    )
    .unwrap();
    assert!(logs.contains("/auth/v1/verify"), "no request logs captured");
    for secret in [PASSWORD, ACCESS_TOKEN, REFRESH_TOKEN, VERIFY_TOKEN] {
        assert!(!logs.contains(secret), "logs contain {secret}: {logs}");
    }
}
