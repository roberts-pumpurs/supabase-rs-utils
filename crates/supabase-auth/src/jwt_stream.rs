use core::ops::Div as _;
use core::pin::Pin;
use core::task::{Context, Poll};
use core::time::Duration;

use futures::Stream;
use reqwest::header::InvalidHeaderValue;
use thiserror::Error;
use tokio::task::JoinSet;

use crate::auth_client::requests::{GrantType, TokenRequest};
use crate::auth_client::{ApiClient, Request};
use crate::error::AuthError;
use crate::types::{AccessTokenResponseSchema, ErrorSchema, LoginCredentials, TokenRequestBody};

#[derive(Clone, Debug, PartialEq, Eq, typed_builder::TypedBuilder)]
pub struct SupabaseAuthConfig {
    pub api_key: String,
    pub max_reconnect_attempts: u8,
    pub reconnect_interval: core::time::Duration,
    pub url: url::Url,
}

pub struct JwtStream {
    config: SupabaseAuthConfig,
}

impl JwtStream {
    /// Creates a new [`SupabaseAuth`].
    #[must_use]
    pub const fn new(config: SupabaseAuthConfig) -> Self {
        Self { config }
    }

    /// Creates a Stream that will attempt to log in to supabase and periodically refresh the JWT
    ///
    /// # Errors
    ///
    /// This function will return an error if the provided supabase url cannot be joined with the
    /// expected suffix.
    #[tracing::instrument(skip_all, err)]
    pub fn sign_in(&self, params: LoginCredentials) -> Result<JwtRefreshStream, SignInError> {
        let client =
            ApiClient::new_unauthenticated(self.config.url.clone(), &self.config.api_key).unwrap();
        Ok(JwtRefreshStream {
            api_key: self.config.api_key.clone(),
            client,
            token_body: params,
            max_reconnect_attempts: self.config.max_reconnect_attempts,
            current_reconnect_attempts: 0,
            background_tasks: JoinSet::new(),
            reconnect_interval: self.config.reconnect_interval,
        })
    }
}

pub struct JwtRefreshStream {
    pub api_key: String,
    client: ApiClient,
    token_body: LoginCredentials,
    max_reconnect_attempts: u8,
    current_reconnect_attempts: u8,
    reconnect_interval: core::time::Duration,
    background_tasks: JoinSet<Result<AccessTokenResponseSchema, RefreshStreamError>>,
}

impl JwtRefreshStream {
    fn login_request(
        &self,
    ) -> Result<Request<AccessTokenResponseSchema, ErrorSchema>, RefreshStreamError> {
        let req = self.client.build_request(
            &TokenRequest::builder()
                .grant_type(GrantType::Password)
                .payload(
                    TokenRequestBody::builder()
                        .email(self.token_body.email.clone())
                        .password(self.token_body.password.clone())
                        .phone(self.token_body.phone.clone())
                        .build(),
                )
                .build(),
        )?;
        Ok(req)
    }

    fn spawn_login_task(&mut self, delay: Option<core::time::Duration>) {
        let request = match self.login_request() {
            Ok(req) => req,
            Err(e) => {
                tracing::error!(?e, "Failed to build login request");
                return;
            }
        };
        let task = async move {
            if let Some(duration) = delay {
                tokio::time::sleep(duration).await;
            }
            auth_request(request).await
        };
        self.background_tasks.spawn(task);
    }

    fn spawn_refresh_task(&mut self, access_token: &AccessTokenResponseSchema) {
        // Attempt to extract refresh_token
        let Some(refresh_token) = access_token.refresh_token.clone() else {
            tracing::warn!("`refresh_token` not present");
            return;
        };

        // Attempt to extract expires_in
        let Some(expires_in) = access_token.expires_in else {
            tracing::warn!("`expires_in` not present");
            return;
        };

        // Build the TokenRequestBody
        let token_request_body = TokenRequestBody::builder()
            .refresh_token(refresh_token)
            .build();

        // Build the TokenRequest
        let token_request = TokenRequest::builder()
            .grant_type(GrantType::RefreshToken)
            .payload(token_request_body)
            .build();

        // Attempt to build the request
        let Ok(request) = self.client.build_request(&token_request) else {
            tracing::warn!("could not build refresh task request");
            return;
        };

        // Create the asynchronous task
        let task = async move {
            let refresh_in = calculate_refresh_sleep_duration(expires_in as u64);
            tokio::time::sleep(refresh_in).await;
            auth_request(request).await
        };

        // Spawn the background task
        self.background_tasks.spawn(task);
    }
}

impl Stream for JwtRefreshStream {
    type Item = Result<AccessTokenResponseSchema, RefreshStreamError>;

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
                Poll::Ready(Some(item))
            }
            Poll::Ready(Some(Err(join_error))) => {
                tracing::error!(?join_error, "Task panicked; terminating stream");
                cx.waker().wake_by_ref();
                Poll::Ready(None)
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
                Poll::Pending
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn auth_request(
    request: Request<AccessTokenResponseSchema, ErrorSchema>,
) -> Result<AccessTokenResponseSchema, RefreshStreamError> {
    let res = request.execute().await?.json().await??;
    Ok(res)
}

fn calculate_refresh_sleep_duration(expires_in: u64) -> Duration {
    Duration::from_secs(expires_in).div(2)
}

#[derive(Debug, Error)]
pub enum RefreshStreamError {
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] simd_json::Error),
    #[error("Supabase API error: {0}")]
    SupabaseApiError(String),
    #[error("Auth error: {0}")]
    AuthError(#[from] AuthError),
    #[error("Auth error: {0}")]
    ErrorResponse(#[from] ErrorSchema),
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

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "allow for tests")]
mod auth_tests {
    use core::time::Duration;

    use futures::StreamExt as _;
    use mockito::Matcher;
    use pretty_assertions::assert_eq;
    use rp_supabase_mock::{SupabaseMockServer, make_jwt};
    use rstest::rstest;
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
        let mut ms = SupabaseMockServer::new().await;
        let ms = ms.register_jwt_password(&access_token);
        let config = SupabaseAuthConfig {
            url: ms.server_url(),
            api_key: "api-key".to_owned(),
            max_reconnect_attempts: 1,
            reconnect_interval: Duration::from_secs(1),
        };
        let supabase_auth = JwtStream::new(config);
        let token_body = LoginCredentials::builder()
            .email("user@example.com".to_owned())
            .password("password".to_owned())
            .build();

        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        let response = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();

        dbg!(&response);
        assert!(response.is_ok());
        let auth_response = response.unwrap();
        assert_eq!(auth_response.access_token.unwrap(), access_token);
        assert_eq!(auth_response.refresh_token.unwrap(), "some-refresh-token");
        assert_eq!(
            auth_response.user.unwrap().email.unwrap(),
            "user@example.com"
        );
    }

    #[rstest]
    #[test(tokio::test)]
    #[timeout(ms(100))]
    async fn test_password_login_error() {
        let mut ms = SupabaseMockServer::new().await;
        let _m1 = ms
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_owned()))
            .with_status(400)
            .create();

        let config = SupabaseAuthConfig {
            url: ms.server_url(),
            api_key: "api-key".to_owned(),
            max_reconnect_attempts: 2,
            reconnect_interval: Duration::from_secs(1),
        };
        let supabase_auth = JwtStream::new(config);
        let token_body = LoginCredentials::builder()
            .email("user@example.com".to_owned())
            .password("password".to_owned())
            .build();

        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        let response = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();

        response.unwrap_err();
    }
    #[rstest]
    #[test(tokio::test)]
    #[timeout(ms(100))]
    async fn test_password_login_error_no_retries() {
        let mut ms = SupabaseMockServer::new().await;
        let _m1 = ms
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_owned()))
            .with_status(400)
            .create();

        let config = SupabaseAuthConfig {
            url: ms.server_url(),
            api_key: "api-key".to_owned(),
            max_reconnect_attempts: 1,
            reconnect_interval: Duration::from_secs(1),
        };
        let supabase_auth = JwtStream::new(config);
        let token_body = LoginCredentials::builder()
            .email("user@example.com".to_owned())
            .password("password".to_owned())
            .build();

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
        let mut ms = SupabaseMockServer::new().await;
        let _m1 = ms
            .mockito_server
            .mock("POST", "/auth/v1/token")
            .match_query(Matcher::Regex("grant_type=password".to_owned()))
            .with_status(500)
            .create();
        let config = SupabaseAuthConfig {
            url: ms.server_url(),
            api_key: "api-key".to_owned(),
            max_reconnect_attempts: 2,
            reconnect_interval: Duration::from_millis(20),
        };
        let supabase_auth = JwtStream::new(config);
        let token_body = LoginCredentials::builder()
            .email("user@example.com".to_owned())
            .password("password".to_owned())
            .build();

        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        let response = stream.next().await.unwrap();
        response.unwrap_err();
        ms.register_jwt_password(&make_jwt(Duration::from_secs(3600)));
        let response = timeout(Duration::from_secs(10), stream.next())
            .await
            .unwrap()
            .unwrap();

        dbg!(&response);
        assert!(response.is_ok());
        let auth_response = response.unwrap();
        assert_eq!(auth_response.refresh_token.unwrap(), "some-refresh-token");
        assert_eq!(
            auth_response.user.unwrap().email.unwrap(),
            "user@example.com"
        );
    }

    #[rstest]
    #[test_log::test(tokio::test)]
    #[timeout(ms(3_000))]
    async fn test_use_refresh_token_on_expiry() {
        // setup
        let mut ms = SupabaseMockServer::new().await;
        let first_access_token = make_jwt(Duration::from_millis(5));
        ms.register_jwt_password(&first_access_token);

        let new_access_token = make_jwt(Duration::from_secs(3600));
        ms.register_jwt_refresh(&new_access_token);
        let config = SupabaseAuthConfig {
            url: ms.server_url(),
            api_key: "api-key".to_owned(),
            max_reconnect_attempts: 1,
            reconnect_interval: Duration::from_millis(20),
        };
        let supabase_auth = JwtStream::new(config);

        // action
        let token_body = LoginCredentials::builder()
            .email("user@example.com".to_owned())
            .password("password".to_owned())
            .build();
        let mut stream = supabase_auth.sign_in(token_body).unwrap();

        // Get the initial token
        let response1 = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();
        dbg!(&response1);
        assert!(response1.is_ok());
        let auth_response1 = response1.unwrap();
        assert_eq!(auth_response1.access_token.unwrap(), first_access_token);
        assert_eq!(
            auth_response1.user.unwrap().email.unwrap(),
            "user@example.com"
        );

        // Wait for token to expire and refresh
        let response2 = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap();
        dbg!(&response2);
        assert!(response2.is_ok());
        let auth_response2 = response2.unwrap();
        assert_eq!(auth_response2.access_token.unwrap(), new_access_token);
        assert_eq!(
            auth_response2.user.unwrap().email.unwrap(),
            "user@example.com"
        );
    }
}
