#![feature(result_flattening)]

use std::borrow::Cow;
use std::future::Future;
use std::ops::{Div, Mul};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::prelude::*;
use futures::stream::FuturesUnordered;
use futures::{FutureExt, Stream};
use jwt_simple::claims::{JWTClaims, NoCustomClaims};
use pin_project::pin_project;
use reqwest::header::{HeaderMap, InvalidHeaderValue};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use simd_json::json;
use thiserror::Error;
use tokio::task::JoinSet;
pub use {futures, redact, url};

pub const SUPABASE_KEY: &str = "apikey";

pub struct SupabaseAuth {
    url: url::Url,
    api_key: String,
    max_reconnect_attempts: u8,
    reconnect_interval: std::time::Duration,
}

impl SupabaseAuth {
    /// Creates a new [`SupabaseAuth`].
    pub fn new(
        url: url::Url,
        api_key: String,
        max_reconnect_attempts: u8,
        reconnect_interval: std::time::Duration,
    ) -> Self {
        Self {
            url,
            api_key,
            max_reconnect_attempts,
            reconnect_interval,
        }
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
            max_reconnect_attempts: self.max_reconnect_attempts,
            current_reconnect_attempts: 0,
            background_tasks: JoinSet::new(),
            reconnect_interval: self.reconnect_interval,
        })
    }
}

pub struct TokenBody<'a> {
    pub email: &'a str,
    pub password: &'a str,
}

pub struct RefreshStream<'a> {
    password_url: url::Url,
    refresh_url: url::Url,
    pub api_key: String,
    client: Client,
    token_body: TokenBody<'a>,
    max_reconnect_attempts: u8,
    current_reconnect_attempts: u8,
    reconnect_interval: std::time::Duration,
    background_tasks: JoinSet<Result<AuthResponse, RefreshStreamError>>,
}

impl<'a> RefreshStream<'a> {
    fn login_request(&self) -> Result<reqwest::Request, reqwest::Error> {
        self.client
            .post(self.password_url.clone())
            .json(&json!({
                "email": self.token_body.email,
                "password": self.token_body.password,
            }))
            .build()
    }

    fn spawn_login_task(&mut self, delay: Option<std::time::Duration>) {
        let request = match self.login_request() {
            Ok(req) => req,
            Err(e) => {
                tracing::error!(?e, "Failed to build login request");
                return;
            }
        };
        let client = self.client.clone();
        let task = async move {
            if let Some(duration) = delay {
                tokio::time::sleep(duration).await;
            }
            auth_request(client, request).await
        };
        self.background_tasks.spawn(task);
    }

    fn spawn_refresh_task(&mut self, access_token: &AuthResponse) {
        let request = match self
            .client
            .post(self.refresh_url.clone())
            .json(&json!({ "refresh_token": access_token.refresh_token }))
            .build()
        {
            Ok(req) => req,
            Err(e) => {
                tracing::error!(?e, "Failed to build refresh request");
                return;
            }
        };
        let client = self.client.clone();
        let expires_at_ts = access_token.expires_in;
        let task = async move {
            let refresh_in = calculate_refresh_sleep_duration(expires_at_ts);
            tokio::time::sleep(refresh_in).await;
            auth_request(client, request).await
        };
        self.background_tasks.spawn(task);
    }
}

impl<'a> Stream for RefreshStream<'a> {
    type Item = Result<AuthResponse, RefreshStreamError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.background_tasks.poll_join_next(cx) {
            Poll::Ready(Some(Ok(item))) => {
                match &item {
                    Ok(access_token) => {
                        // Reset reconnect attempts on success
                        self.current_reconnect_attempts = 0;
                        // Spawn a task to refresh the token before it expires
                        self.spawn_refresh_task(access_token);
                        cx.waker().wake_by_ref();
                    }
                    Err(err) => {
                        if self.current_reconnect_attempts >= self.max_reconnect_attempts {
                            tracing::error!(
                                ?err,
                                "Max reconnect attempts exceeded; terminating stream"
                            );
                            return Poll::Ready(None);
                        }
                        tracing::warn!(
                            attempts = self.current_reconnect_attempts,
                            max_attempts = self.max_reconnect_attempts,
                            "Login failed; retrying"
                        );
                        self.current_reconnect_attempts += 1;
                        // Spawn a login task with a delay
                        let duration = self.reconnect_interval;
                        self.spawn_login_task(Some(duration));
                        cx.waker().wake_by_ref();
                    }
                }
                return Poll::Ready(Some(item));
            }
            Poll::Ready(Some(Err(join_error))) => {
                tracing::error!(?join_error, "Task panicked; terminating stream");
                return Poll::Ready(None);
            }
            Poll::Ready(None) => {
                // No tasks left; start the initial login attempt
                if self.current_reconnect_attempts >= self.max_reconnect_attempts {
                    tracing::error!("Max reconnect attempts exceeded; terminating stream");
                    return Poll::Ready(None);
                }
                tracing::debug!("No tasks running; attempting initial login");
                self.current_reconnect_attempts += 1;
                self.spawn_login_task(None);
                // Yield control to allow the task to start
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            Poll::Pending => return Poll::Pending,
        }
    }
}

async fn auth_request(
    client: Client,
    request: reqwest::Request,
) -> Result<AuthResponse, RefreshStreamError> {
    let result = client.execute(request).await?;
    if result.status().is_success() {
        let mut vec = result.bytes().await?.to_vec();
        {
            let response = String::from_utf8_lossy(&vec);
            tracing::debug!(?response, "auth response");
        }
        let result = simd_json::from_slice::<AuthResponse>(&mut vec)?;
        return Ok(result)
    }
    let error = result.text().await?;
    tracing::warn!(?error, "auth erorr response");
    Err(RefreshStreamError::SupabaseApiError(error))
}

fn calculate_refresh_sleep_duration(expires_at_ts: u64) -> Duration {
    // Get the current time as Unix timestamp
    let current_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    // Calculate the duration until expiration
    let refresh_in = if expires_at_ts > current_ts {
        // and divide by `2/3` just to be on the safe side
        Duration::from_secs(expires_at_ts - current_ts)
            .mul(3)
            .div(2)
    } else {
        Duration::from_secs(1)
    };
    refresh_in
}

#[derive(Debug, Error)]
pub enum RefreshStreamError {
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] simd_json::Error),
    #[error("Supabase API erorr: {0}")]
    SupabaseApiError(String),
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
mod auth_tests {
    use std::time::Duration;

    use futures::StreamExt;
    use mockito::Matcher;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use supabase_mock::{make_jwt, SupabaseMockServer};
    use test_log::test;
    use tokio::time::timeout;

    use super::*;

    fn ms(ms: u32) -> Duration {
        Duration::from_millis(ms.into())
    }

    #[rstest]
    #[test(tokio::test)]
    #[timeout(ms(5_000))]
    async fn test_successful_password_login() {
        let access_token = make_jwt(Duration::from_secs(3600));
        let mut m = SupabaseMockServer::new().await;
        let m = m.register_jwt_password(&access_token);
        let supabase_auth = SupabaseAuth::new(
            m.server_url(),
            "api-key".to_string(),
            1,
            Duration::from_secs(1),
        );
        let token_body = TokenBody {
            email: "user@example.com",
            password: "password",
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
    #[timeout(ms(100))]
    async fn test_password_login_error() {
        let mut m = SupabaseMockServer::new().await;
        let _m1 = m
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_string()))
            .with_status(400)
            .create();

        let supabase_auth = SupabaseAuth::new(
            m.server_url(),
            "api-key".to_string(),
            2,
            Duration::from_secs(1),
        );
        let token_body = TokenBody {
            email: ("user@example.com"),
            password: "password",
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
    #[timeout(ms(100))]
    async fn test_password_login_error_no_retries() {
        let mut m = SupabaseMockServer::new().await;
        let _m1 = m
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_string()))
            .with_status(400)
            .create();

        let supabase_auth = SupabaseAuth::new(
            m.server_url(),
            "api-key".to_string(),
            1,
            Duration::from_secs(1),
        );
        let token_body = TokenBody {
            email: ("user@example.com"),
            password: "password",
        };

        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        let response = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap();

        assert!(response.is_none());
    }

    #[rstest]
    #[test(tokio::test)]
    #[timeout(ms(100))]
    async fn test_retry_on_login_error() {
        let mut m = SupabaseMockServer::new().await;
        let _m1 = m
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_string()))
            .with_status(500)
            .create();
        let supabase_auth = SupabaseAuth::new(
            m.server_url(),
            "api-key".to_string(),
            2,
            Duration::from_millis(20),
        );
        let token_body = TokenBody {
            email: "user@example.com",
            password: "password",
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
    #[timeout(ms(3_000))]
    async fn test_use_refresh_token_on_expiry() {
        // setup
        let mut m = SupabaseMockServer::new().await;
        let first_access_token = make_jwt(Duration::from_millis(5));
        m.register_jwt_password(&first_access_token);

        let new_access_token = make_jwt(Duration::from_secs(3600));
        m.register_jwt_refresh(&new_access_token);
        let supabase_auth = SupabaseAuth::new(
            m.server_url(),
            "api-key".to_string(),
            1,
            Duration::from_millis(20),
        );

        // action
        let token_body = TokenBody {
            email: "user@example.com",
            password: "password",
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
