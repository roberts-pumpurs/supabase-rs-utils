use core::net::SocketAddr;
use core::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use jsonwebtoken::{DecodingKey, Validation, decode};
pub use mockito;
use mockito::{Matcher, ServerGuard};
use serde::{Deserialize, Serialize};
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
        let expires_at = parsed_jwt.exp;
        let expires_in = expires_at.abs_diff(
            u64::try_from(current_ts.as_secs()).map_err(|_err| JwtParseError::InvalidJwt)?,
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
        let expires_at = parsed_jwt.exp;
        let expires_in = expires_at.abs_diff(
            u64::try_from(current_ts.as_secs()).map_err(|_err| JwtParseError::InvalidJwt)?,
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
    // `iat` and `exp` must be *seconds* since the epoch for JWTs.
    let issued_at = current_ts().as_secs();

    let exp = issued_at
        .checked_add(expires_in.as_secs())
        .ok_or(JwtParseError::InvalidJwt)?;

    let claims = Claims {
        iat: issued_at,
        exp,
    };

    // Build an explicit header so we can keep the `"kid": "secret"` you had.
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("secret".to_owned());

    encode(&header, &claims, &EncodingKey::from_secret(SECRET))
        .map_err(|_| JwtParseError::InvalidJwt)
}

/// Returns the current timestamp.
///
/// # Panics
///
/// This function will panic if the system time is before the Unix epoch.
fn current_ts() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
}

const SECRET: &[u8] = b"SECRET";

/// Same struct we used for encoding; now we just add `Deserialize`.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    iat: u64,
    exp: u64,
}

/// Parse JWT
///
/// # Errors
/// if the JWT cannot be parsed or the claims are invalid
pub fn parse_jwt(token: &str) -> Result<Claims, JwtParseError> {
    // Accept only HS256 and require exp to be in the future.
    let mut validation = Validation::new(Algorithm::HS256);
    validation.required_spec_claims = ["exp".to_string(), "iat".to_string()].into_iter().collect();

    // Perform the decode + signature check
    let data = decode::<Claims>(token, &DecodingKey::from_secret(SECRET), &validation)
        .map_err(|_| JwtParseError::InvalidJwt)?;

    // Optional defense-in-depth: ensure the kid is what we expect.
    if data.header.kid.as_deref() != Some("secret") {
        return Err(JwtParseError::InvalidJwt);
    }

    Ok(data.claims)
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
