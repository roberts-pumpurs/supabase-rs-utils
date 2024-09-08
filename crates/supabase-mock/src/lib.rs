use core::net::SocketAddr;
use core::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

pub use mockito;
use mockito::{Matcher, ServerGuard};
use serde_json::json;

pub struct SupabaseMockServer {
    pub api_mock: Vec<mockito::Mock>,
    pub mockito_server: ServerGuard,
}

impl SupabaseMockServer {
    #[must_use]
    pub async fn new() -> Self {
        let server = mockito::Server::new_async().await;
        Self {
            mockito_server: server,
            api_mock: vec![],
        }
    }

    #[must_use]
    pub fn server_address(&self) -> SocketAddr {
        self.mockito_server.socket_address()
    }

    #[must_use]
    pub fn server_url(&self) -> url::Url {
        self.mockito_server.url().parse().unwrap()
    }

    pub fn register_jwt(&mut self, jwt: &str) -> &mut Self {
        self.register_jwt_password(jwt).register_jwt_refresh(jwt)
    }

    pub fn register_jwt_password(&mut self, jwt: &str) -> &mut Self {
        self.register_jwt_custom_grant_type(jwt, "password")
    }

    pub fn register_jwt_refresh(&mut self, jwt: &str) -> &mut Self {
        self.register_jwt_custom_grant_type(jwt, "refresh_token")
    }

    fn register_jwt_custom_grant_type(&mut self, jwt: &str, grant_type: &str) -> &mut Self {
        let mock = self
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex(format!("grant_type={grant_type}")))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": jwt,
                    "refresh_token": "some-refresh-token",
                    "expires_in": 3_600_i32,
                    "token_type": "bearer",
                    "user": {
                        "id": "user-id",
                        "email": "user@example.com"
                    }
                })
                .to_string(),
            )
            .create();
        self.api_mock.push(mock);
        self
    }
}

#[must_use]
pub fn make_jwt(expires_in: Duration) -> String {
    use jwt_simple::prelude::*;
    let current_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    let will_expire_at = current_ts + expires_in;
    jwt_simple::algorithms::ES256kKeyPair::generate()
        .with_key_id("secret")
        .sign(JWTClaims {
            issued_at: None,
            expires_at: Some(Duration::new(
                will_expire_at.as_secs(),
                will_expire_at.subsec_nanos(),
            )),
            invalid_before: None,
            issuer: None,
            subject: None,
            audiences: None,
            jwt_id: None,
            nonce: None,
            custom: NoCustomClaims {},
        })
        .unwrap()
}
