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

    /// Returns the server URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the server URL cannot be parsed.
    pub fn server_url(&self) -> Result<url::Url, url::ParseError> {
        self.mockito_server.url().parse()
    }

    /// Registers a JWT token for both password and refresh grant types.
    ///
    /// # Errors
    ///
    /// Returns an error if the JWT token cannot be parsed or does not have an expiration time.
    pub fn register_jwt(&mut self, jwt: &str) -> Result<&mut Self, JwtParseError> {
        self.register_jwt_password(jwt)?.register_jwt_refresh(jwt)
    }

    /// Registers a JWT token for password authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the JWT token cannot be parsed or does not have an expiration time.
    pub fn register_jwt_password(&mut self, jwt: &str) -> Result<&mut Self, JwtParseError> {
        let parsed_jwt = parse_jwt(jwt)?;
        let current_ts = current_ts();
        let expires_at = parsed_jwt.expires_at.ok_or(JwtParseError::InvalidJwt)?;
        let expires_in = expires_at.as_millis().abs_diff(
            u64::try_from(current_ts.as_millis()).map_err(|_err| JwtParseError::InvalidJwt)?,
        );
        self.register_jwt_custom_grant_type(jwt, "password", Duration::from_millis(expires_in));
        Ok(self)
    }

    /// Registers a JWT token for refresh token authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the JWT token cannot be parsed or does not have an expiration time.
    pub fn register_jwt_refresh(&mut self, jwt: &str) -> Result<&mut Self, JwtParseError> {
        let parsed_jwt = parse_jwt(jwt)?;
        let current_ts = current_ts();
        let expires_at = parsed_jwt.expires_at.ok_or(JwtParseError::InvalidJwt)?;
        let expires_in = expires_at.as_millis().abs_diff(
            u64::try_from(current_ts.as_millis()).map_err(|_err| JwtParseError::InvalidJwt)?,
        );
        self.register_jwt_custom_grant_type(
            jwt,
            "refresh_token",
            Duration::from_millis(expires_in),
        );
        Ok(self)
    }

    fn register_jwt_custom_grant_type(
        &mut self,
        jwt: &str,
        grant_type: &str,
        expires_in: Duration,
    ) {
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
        let body = simd_json::to_string(&body).unwrap_or_else(|_| "{}".to_owned());
        let mock = self
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex(format!("grant_type={grant_type}")))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();
        self.api_mock.push(mock);
    }
}

/// Creates a new JWT token with the specified expiration time.
///
/// # Errors
///
/// Returns an error if the JWT key pair cannot be generated or the JWT token cannot be signed.
pub fn make_jwt(expires_in: Duration) -> Result<String, JwtParseError> {
    use jwt_simple::prelude::*;
    let current_ts = current_ts();
    let will_expire_at = current_ts
        .checked_add(expires_in)
        .ok_or(JwtParseError::InvalidJwt)?;
    let key_pair = jwt_simple::algorithms::ES256kKeyPair::generate();
    key_pair
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
        .map_err(|_err| JwtParseError::InvalidJwt)
}

/// Returns the current timestamp.
///
/// # Errors
///
/// This function will panic if the system time is before the Unix epoch.
fn current_ts() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
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
