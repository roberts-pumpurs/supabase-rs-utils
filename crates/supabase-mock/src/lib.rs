use core::net::SocketAddr;
use core::time::Duration;

pub use mockito;
use mockito::{mock, Matcher};
use serde_json::json;

pub struct SupabaseMockServer {
    pub api_mock: Vec<mockito::Mock>,
}

impl Default for SupabaseMockServer {
    fn default() -> Self {
        Self::new()
    }
}

impl SupabaseMockServer {
    #[must_use]
    pub fn new() -> Self {
        Self { api_mock: vec![] }
    }

    #[must_use]
    pub fn server_address(&self) -> SocketAddr {
        mockito::server_address()
    }

    #[must_use]
    pub fn server_url(&self) -> url::Url {
        mockito::server_url().parse().unwrap()
    }

    pub fn register_jwt(&mut self, jwt: &str) -> &mut Self {
        self.register_jwt_password(jwt).register_jwt_refresh(jwt)
    }

    pub fn register_jwt_password(&mut self, jwt: &str) -> &mut Self {
        self.register_jwt_custom_grant_type(jwt, "password")
    }

    pub fn register_jwt_refresh(&mut self, jwt: &str) -> &mut Self {
        self.register_jwt_custom_grant_type(jwt, "token_refresh")
    }

    fn register_jwt_custom_grant_type(&mut self, jwt: &str, grant_type: &str) -> &mut Self {
        let _m = mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex(format!("grant_type={grant_type}")))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "access_token": jwt,
                    "refresh_token": "some-refresh-token",
                    "expires_in": 3600,
                    "token_type": "bearer",
                    "user": {
                        "id": "user-id",
                        "email": "user@example.com"
                    }
                })
                .to_string(),
            )
            .create();
        self.api_mock.push(_m);
        self
    }
}

#[must_use]
pub fn make_jwt(expires_in: Duration) -> String {
    use jwt_simple::prelude::*;
    jwt_simple::algorithms::ES256kKeyPair::generate()
        .with_key_id("secret")
        .sign(JWTClaims {
            issued_at: None,
            expires_at: Some(expires_in.into()),
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
