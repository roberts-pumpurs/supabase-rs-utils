#![feature(result_flattening)]

mod jwt_expiry;
use std::borrow::Cow;
use std::ops::{Div, Mul};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::prelude::*;
use futures::{FutureExt, Stream};
use jwt_expiry::JwtExpiry;
use jwt_simple::claims::{JWTClaims, NoCustomClaims};
use pin_project::pin_project;
use reqwest::header::{HeaderMap, InvalidHeaderValue};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use simd_json::json;
use thiserror::Error;
pub use {futures, redact, url};

pub const SUPABASE_KEY: &str = "apikey";

pub struct SupabaseAuth {
    url: url::Url,
    api_key: String,
}

impl SupabaseAuth {
    /// Creates a new [`SupabaseAuth`].
    pub fn new(url: url::Url, api_key: String) -> Self {
        Self { url, api_key }
    }

    /// Creates a Stream that will attempt to log in to supabase and periodically refresh the JWT
    ///
    /// # Errors
    ///
    /// This function will return an error if the provided supabase url cannot be joined with the
    /// expected suffix.
    #[tracing::instrument(skip_all, err)]
    pub fn sign_in<'a>(&self, params: TokenBody<'a>) -> Result<RefreshStream<'a>, SignInError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(SUPABASE_KEY, self.api_key.parse()?);
        let client = Client::builder().default_headers(default_headers).build()?;
        Ok(RefreshStream {
            password_url: self
                .url
                .clone()
                .join("/auth/v1/token?grant_type=password")?,
            refresh_url: self
                .url
                .clone()
                .join("/auth/v1/token?grant_type=refresh_token")?,
            api_key: self.api_key.clone(),
            client,
            token_body: params,
            state: RefreshStreamState::PasswordLogin,
        })
    }
}

pub struct TokenBody<'a> {
    pub email: Cow<'a, str>,
    pub password: redact::Secret<Cow<'a, str>>,
}

impl<'a> TokenBody<'a> {
    pub fn new(email: &'a str, password: &'a str) -> Self {
        Self {
            email: Cow::Borrowed(email),
            password: redact::Secret::new(Cow::Borrowed(password)),
        }
    }
}

#[pin_project]
pub struct RefreshStream<'a> {
    password_url: url::Url,
    refresh_url: url::Url,
    pub api_key: String,
    client: Client,
    token_body: TokenBody<'a>,
    #[pin]
    state: RefreshStreamState,
}

#[pin_project]
enum RefreshStreamState {
    PasswordLogin,
    WaitingForResponse(
        #[pin] futures::future::BoxFuture<'static, Result<Response, reqwest::Error>>,
    ),
    ParseJson(#[pin] futures::future::BoxFuture<'static, Result<AuthResponse, RefreshStreamError>>),
    WaitForExpiry {
        refresh_token: String,
        #[pin]
        access_expiry: JwtExpiry,
    },
}

impl<'a> Stream for RefreshStream<'a> {
    type Item = Result<AuthResponse, RefreshStreamError>;

    #[allow(
        clippy::too_many_lines,
        reason = "poll functions tend to become like this"
    )]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.state.as_mut().get_mut() {
                RefreshStreamState::PasswordLogin => {
                    let request_future = this
                        .client
                        .post(this.password_url.clone())
                        .json(&json!({
                            "email": this.token_body.email,
                            "password": this.token_body.password.expose_secret(),
                        }))
                        .send()
                        .boxed();

                    this.state
                        .set(RefreshStreamState::WaitingForResponse(request_future));
                }
                RefreshStreamState::WaitingForResponse(fut) => {
                    match std::task::ready!(fut.poll_unpin(cx)) {
                        Ok(res) => match res.error_for_status() {
                            Ok(res) => {
                                let json_future = res
                                    .bytes()
                                    .map(|x| {
                                        x.map(|reqwest_bytes| {
                                            let span =
                                                tracing::debug_span!("decoding Auth Response");
                                            let _g = span.entered();
                                            let mut bytes = reqwest_bytes.to_vec();
                                            let response = String::from_utf8_lossy(bytes.as_ref());
                                            tracing::debug!(response = ?response, "auth resp");
                                            simd_json::from_slice::<AuthResponse>(&mut bytes)
                                                .map_err(RefreshStreamError::from)
                                        })
                                        .map_err(RefreshStreamError::from)
                                        .flatten()
                                    })
                                    .boxed();
                                this.state.set(RefreshStreamState::ParseJson(json_future));
                            }
                            Err(err) => {
                                this.state.set(RefreshStreamState::PasswordLogin);
                                return Poll::Ready(Some(Err(RefreshStreamError::Reqwest(err))));
                            }
                        },
                        Err(err) => {
                            this.state.set(RefreshStreamState::PasswordLogin);
                            return Poll::Ready(Some(Err(RefreshStreamError::Reqwest(err))))
                        }
                    }
                }
                RefreshStreamState::ParseJson(fut) => match std::task::ready!(fut.poll_unpin(cx)) {
                    Ok(res) => match parse_jwt(&res.access_token) {
                        Ok(access_token) => {
                            let Some(expires_at) = access_token.expires_at else {
                                tracing::error!("`expires_at` field not present");
                                return Poll::Ready(None)
                            };
                            let expires_at_ts = expires_at.as_secs();
                            // Get the current time as Unix timestamp
                            let current_ts = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_secs();

                            // Calculate the duration until expiration
                            let valid_for = if expires_at_ts > current_ts {
                                // and divide by `2/3` just to be on the safe side
                                Duration::from_secs(expires_at_ts - current_ts)
                                    .mul(3)
                                    .div(2)
                            } else {
                                Duration::from_secs(1)
                            };
                            let access_jwt_expiry = JwtExpiry::new(valid_for);
                            this.state.set(RefreshStreamState::WaitForExpiry {
                                refresh_token: res.refresh_token.clone(),
                                access_expiry: access_jwt_expiry,
                            });

                            cx.waker().wake_by_ref();
                            return Poll::Ready(Some(Ok(res)));
                        }
                        Err(err) => {
                            this.state.set(RefreshStreamState::PasswordLogin);
                            return Poll::Ready(Some(Err(RefreshStreamError::JwtParse(err))))
                        }
                    },
                    Err(err) => {
                        this.state.set(RefreshStreamState::PasswordLogin);
                        return Poll::Ready(Some(Err(err)))
                    }
                },
                RefreshStreamState::WaitForExpiry {
                    refresh_token,
                    access_expiry,
                } => {
                    let _res = std::task::ready!(access_expiry.poll_unpin(cx));
                    let request_future = this
                        .client
                        .post(this.refresh_url.clone())
                        .json(&json!({
                            "refresh_token": refresh_token,
                        }))
                        .send()
                        .boxed();

                    this.state
                        .set(RefreshStreamState::WaitingForResponse(request_future));
                    continue;
                }
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum RefreshStreamError {
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] simd_json::Error),
    #[error("JWT parse error: {0}")]
    JwtParse(#[from] JwtParseError),
}

#[derive(Debug, Error)]
pub enum SignInError {
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
}

#[derive(Debug, Error)]
pub enum JwtParseError {
    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("Invalid JWT")]
    InvalidJwt,

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] simd_json::Error),
}

#[tracing::instrument(err, skip_all)]
fn parse_jwt(token: &str) -> Result<JWTClaims<NoCustomClaims>, JwtParseError> {
    let mut tokens = token.split('.');
    let _header = tokens.next();
    let body = tokens.next().ok_or(JwtParseError::InvalidJwt)?;
    let mut body = BASE64_STANDARD.decode(body)?;
    let body = simd_json::from_slice::<JWTClaims<NoCustomClaims>>(body.as_mut_slice())?;

    Ok(body)
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub user: User,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct User {
    pub id: String,
    pub email: String,
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::time::Duration;

    use futures::StreamExt;
    use mockito::Matcher;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use supabase_mock::{make_jwt, SupabaseMockServer};
    use test_log::test;
    use tokio::time::timeout;

    use super::*;

    #[rstest]
    #[test(tokio::test)]
    async fn test_successful_password_login() {
        let access_token = make_jwt(Duration::from_secs(3600));
        let mut m = SupabaseMockServer::new().await;
        let m = m.register_jwt_password(&access_token);
        let supabase_auth = SupabaseAuth::new(m.server_url(), "api-key".to_string());
        let token_body = TokenBody {
            email: Cow::Borrowed("user@example.com"),
            password: redact::Secret::new(Cow::Borrowed("password")),
        };

        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        let response = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();

        dbg!(&response);
        assert!(response.is_ok());
        let auth_response = response.unwrap();
        assert_eq!(auth_response.access_token, access_token);
        assert_eq!(auth_response.refresh_token, "some-refresh-token");
        assert_eq!(auth_response.user.email, "user@example.com");
    }

    #[rstest]
    #[test(tokio::test)]
    async fn test_password_login_error() {
        let mut m = SupabaseMockServer::new().await;
        let _m1 = m
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_string()))
            .with_status(400)
            .create();

        let supabase_auth = SupabaseAuth::new(m.server_url(), "api-key".to_string());
        let token_body = TokenBody {
            email: Cow::Borrowed("user@example.com"),
            password: redact::Secret::new(Cow::Borrowed("password")),
        };

        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        let response = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();

        assert!(response.is_err());
    }

    #[rstest]
    #[test(tokio::test)]
    async fn test_jwt_parsing_error() {
        let mut m = SupabaseMockServer::new().await;
        let m = m.register_jwt_password(&"invalid-jwt");
        let supabase_auth = SupabaseAuth::new(m.server_url(), "api-key".to_string());
        let token_body = TokenBody {
            email: Cow::Borrowed("user@example.com"),
            password: redact::Secret::new(Cow::Borrowed("password")),
        };

        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        let response = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();

        assert!(response.is_err());

        dbg!(&response);
        assert!(matches!(response, Err(RefreshStreamError::JwtParse(_))));
    }

    #[rstest]
    #[test(tokio::test)]
    async fn test_retry_on_login_error() {
        let mut m = SupabaseMockServer::new().await;
        let _m1 = m
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_string()))
            .with_status(500)
            .create();
        let supabase_auth = SupabaseAuth::new(m.server_url(), "api-key".to_string());
        let token_body = TokenBody {
            email: Cow::Borrowed("user@example.com"),
            password: redact::Secret::new(Cow::Borrowed("password")),
        };

        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        let response = stream.next().await.unwrap();
        assert!(response.is_err());
        m.register_jwt_password(&make_jwt(Duration::from_secs(3600)));
        let response = timeout(Duration::from_secs(10), stream.next())
            .await
            .unwrap()
            .unwrap();

        dbg!(&response);
        assert!(response.is_ok());
        let auth_response = response.unwrap();
        assert_eq!(auth_response.refresh_token, "some-refresh-token");
        assert_eq!(auth_response.user.email, "user@example.com");
    }

    #[rstest]
    #[test_log::test(tokio::test)]
    async fn test_use_refresh_token_on_expiry() {
        // setup
        let mut m = SupabaseMockServer::new().await;
        let first_access_token = make_jwt(Duration::from_secs(1));
        m.register_jwt_password(&first_access_token);

        let new_access_token = make_jwt(Duration::from_secs(3600));
        m.register_jwt_refresh(&new_access_token);
        let supabase_auth = SupabaseAuth::new(m.server_url(), "api-key".to_string());

        // action
        let token_body = TokenBody {
            email: Cow::Borrowed("user@example.com"),
            password: redact::Secret::new(Cow::Borrowed("password")),
        };
        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        // Get the initial token
        let response1 = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();
        dbg!(&response1);
        assert!(response1.is_ok());
        let auth_response1 = response1.unwrap();
        assert_eq!(auth_response1.access_token, first_access_token);
        assert_eq!(auth_response1.user.email, "user@example.com");

        // Wait for token to expire and refresh
        let response2 = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();
        dbg!(&response2);
        assert!(response2.is_ok());
        let auth_response2 = response2.unwrap();
        assert_eq!(auth_response2.access_token, new_access_token);
        assert_eq!(auth_response2.user.email, "user@example.com");
    }
}
