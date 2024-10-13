use core::net::SocketAddr;
use core::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use jwt_simple::claims::{JWTClaims, NoCustomClaims};
pub use mockito;
use mockito::{Matcher, ServerGuard};
use simd_json::json;

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
        let parsed_jwt = parse_jwt(jwt).unwrap();
        let current_ts = current_ts();
        let expires_in = parsed_jwt
            .expires_at
            .unwrap()
            .as_millis()
            .abs_diff(current_ts.as_millis() as u64);
        self.register_jwt_custom_grant_type(jwt, "password", Duration::from_millis(expires_in))
    }

    pub fn register_jwt_refresh(&mut self, jwt: &str) -> &mut Self {
        let parsed_jwt = parse_jwt(jwt).unwrap();
        let current_ts = current_ts();
        let expires_in = parsed_jwt
            .expires_at
            .unwrap()
            .as_millis()
            .abs_diff(current_ts.as_millis() as u64);
        self.register_jwt_custom_grant_type(jwt, "refresh_token", Duration::from_millis(expires_in))
    }

    fn register_jwt_custom_grant_type(
        &mut self,
        jwt: &str,
        grant_type: &str,
        expires_in: Duration,
    ) -> &mut Self {
        let body = json!({
            "access_token": jwt,
            "refresh_token": "some-refresh-token",
            "expires_in": expires_in.as_secs(),
            "token_type": "bearer",
            "user": {
                "id": "user-id",
                "email": "user@example.com"
            }
        });
        let body = simd_json::to_string(&body).unwrap();
        let _m = self
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex(format!("grant_type={grant_type}")))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();
        self.api_mock.push(_m);
        self
    }
}

#[must_use]
pub fn make_jwt(expires_in: Duration) -> String {
    use jwt_simple::prelude::*;
    let current_ts = current_ts();

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

fn current_ts() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
}

fn parse_jwt(token: &str) -> Result<JWTClaims<NoCustomClaims>, JwtParseError> {
    use base64::prelude::*;

    let mut tokens = token.split('.');
    let _header = tokens.next();
    let body = tokens.next().ok_or(JwtParseError::InvalidJwt)?;
    let mut body = BASE64_STANDARD.decode(body)?;
    let body = simd_json::from_slice::<JWTClaims<NoCustomClaims>>(body.as_mut_slice())?;

    Ok(body)
}

#[derive(Debug, thiserror::Error)]
pub enum JwtParseError {
    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("Invalid JWT")]
    InvalidJwt,

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] simd_json::Error),
}
