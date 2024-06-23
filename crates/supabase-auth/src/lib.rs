mod jwt_expiry;
use std::borrow::Cow;
use std::ops::Div;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::prelude::*;
use futures::{FutureExt, Stream};
use jwt_expiry::JwtExpiry;
use jwt_simple::claims::{JWTClaims, NoCustomClaims};
use pin_project::pin_project;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
pub use {redact, url};

pub struct SupabaseAuth {
    url: url::Url,
}

impl SupabaseAuth {
    pub fn new(url: url::Url) -> Self {
        Self { url }
    }

    pub fn sign_in<'a>(&self, params: TokenBody<'a>) -> Result<RefreshStream<'a>, url::ParseError> {
        Ok(RefreshStream {
            password_url: self
                .url
                .clone()
                .join("/auth/v1/token?grant_type=password")?,
            refresh_url: self
                .url
                .clone()
                .join("/auth/v1/token?grant_type=token_refresh")?,
            client: Client::new(),
            token_body: params,
            state: RefreshStreamState::PasswordLogin,
        })
    }
}

pub struct TokenBody<'a> {
    pub email: Cow<'a, str>,
    pub password: redact::Secret<Cow<'a, str>>,
}

#[pin_project]
pub struct RefreshStream<'a> {
    password_url: url::Url,
    refresh_url: url::Url,
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
    ParseJson(#[pin] futures::future::BoxFuture<'static, Result<AuthResponse, reqwest::Error>>),
    WaitForExpiry {
        refresh_token: String,
        #[pin]
        access_expiry: JwtExpiry,
    },
}

impl<'a> Stream for RefreshStream<'a> {
    type Item = Result<AuthResponse, RefreshStreamError>;

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
                                let json_future = res.json::<AuthResponse>().boxed();
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
                                // and divide by `2` just to be on the safe side
                                Duration::from_secs(expires_at_ts - current_ts).div(2)
                            } else {
                                Duration::from_secs(0)
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
                        return Poll::Ready(Some(Err(RefreshStreamError::Reqwest(err))))
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
    #[error("JWT parse error: {0}")]
    JwtParse(#[from] JwtParseError),
}

#[derive(Debug, Error)]
pub enum JwtParseError {
    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("Invalid JWT")]
    InvalidJwt,

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),
}

fn parse_jwt(token: &str) -> Result<JWTClaims<NoCustomClaims>, JwtParseError> {
    let mut tokens = token.split('.');
    let _header = tokens.next();
    let body = tokens.next().ok_or(JwtParseError::InvalidJwt)?;
    let body = BASE64_STANDARD.decode(body)?;
    let body = serde_json::from_slice::<JWTClaims<NoCustomClaims>>(&body)?;

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
#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::time::Duration;

    use futures::StreamExt;
    use mockito::{mock, Matcher};
    use rstest::rstest;
    use supabase_mock::{make_jwt, SupabaseMockServer};
    use tokio::time::timeout;

    use super::*;

    #[rstest]
    #[tokio::test]
    async fn test_successful_password_login() {
        let access_token = make_jwt(Duration::from_secs(3600));
        let mut m = SupabaseMockServer::new();
        let m = m.register_jwt_password(&access_token);
        let supabase_auth = SupabaseAuth::new(m.server_url());
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
    #[tokio::test]
    async fn test_password_login_error() {
        let _m = mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_string()))
            .with_status(400)
            .create();

        let url = mockito::server_url();
        let supabase_auth = SupabaseAuth::new(url.parse().unwrap());
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
    #[tokio::test]
    async fn test_jwt_parsing_error() {
        let mut m = SupabaseMockServer::new();
        let m = m.register_jwt_password(&"invalid-jwt");
        let supabase_auth = SupabaseAuth::new(m.server_url());
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
        match response {
            Err(RefreshStreamError::JwtParse(_)) => (),
            _ => panic!("Expected JwtParse error"),
        }
    }

    #[rstest]
    #[tokio::test]
    async fn test_retry_on_login_error() {
        let _m1 = mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_string()))
            .with_status(500)
            .create();
        let mut m = SupabaseMockServer::new();
        let m = m.register_jwt_password(&make_jwt(Duration::from_secs(3600)));
        let supabase_auth = SupabaseAuth::new(m.server_url());
        let token_body = TokenBody {
            email: Cow::Borrowed("user@example.com"),
            password: redact::Secret::new(Cow::Borrowed("password")),
        };

        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        let response = stream.next().await.unwrap();
        assert!(response.is_err());
        let response = timeout(Duration::from_secs(10), stream.next())
            .await
            .unwrap()
            .unwrap();

        assert!(response.is_ok());
        let auth_response = response.unwrap();
        assert_eq!(auth_response.refresh_token, "some-refresh-token");
        assert_eq!(auth_response.user.email, "user@example.com");
    }

    #[rstest]
    #[tokio::test]
    async fn test_use_refresh_token_on_expiry() {
        let first_access_token = make_jwt(Duration::from_secs(1));
        let mut m = SupabaseMockServer::new();
        let m = m.register_jwt_password(&first_access_token);
        let new_access_token = make_jwt(Duration::from_secs(3600));
        let m = m.register_jwt_refresh(&new_access_token);
        let supabase_auth = SupabaseAuth::new(m.server_url());
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
        assert!(response1.is_ok());
        let auth_response1 = response1.unwrap();
        assert_eq!(auth_response1.access_token, first_access_token);
        assert_eq!(auth_response1.user.email, "user@example.com");

        // Wait for token to expire and refresh
        let response2 = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();
        assert!(response2.is_ok());
        let auth_response2 = response2.unwrap();
        assert_eq!(auth_response2.access_token, new_access_token);
        assert_eq!(auth_response2.user.email, "user@example.com");
    }
}
