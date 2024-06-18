mod jwt_expiry;
use std::borrow::Cow;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use base64::prelude::*;
use futures::{FutureExt, Stream};
use jwt_expiry::JwtExpiry;
use jwt_simple::claims::{JWTClaims, NoCustomClaims};
use jwt_simple::token::Token;
use pin_project::pin_project;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

pub struct SupabaseAuth {
    url: url::Url,
}

impl SupabaseAuth {
    pub fn new(url: url::Url) -> Self {
        Self { url }
    }

    pub fn sign_in<'a>(&self, params: TokenBody<'a>) -> RefreshStream<'a> {
        RefreshStream {
            password_url: self
                .url
                .clone()
                .join("/auth/v1/token?grant_type=password")
                .unwrap(),
            refresh_url: self
                .url
                .clone()
                .join("/auth/v1/token?grant_type=token_refresh")
                .unwrap(),
            client: Client::new(),
            token_body: params,
            state: RefreshStreamState::PasswordLogin,
        }
    }
}

pub struct TokenBody<'a> {
    email: Cow<'a, str>,
    password: redact::Secret<Cow<'a, str>>,
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
        refresh_expiry: futures::future::BoxFuture<'static, ()>,
        #[pin]
        access_expiry: futures::future::BoxFuture<'static, ()>,
    },
}

impl<'a> Stream for RefreshStream<'a> {
    type Item = Result<AuthResponse, RefreshStreamError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
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
                RefreshStreamState::WaitingForResponse(fut) => match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(res)) => match res.error_for_status() {
                        Ok(res) => {
                            let json_future = res.json::<AuthResponse>().boxed();
                            this.state.set(RefreshStreamState::ParseJson(json_future));
                        }
                        Err(err) => return Poll::Ready(Some(Err(RefreshStreamError::Reqwest(err)))),
                    },
                    Poll::Ready(Err(err)) => {
                        return Poll::Ready(Some(Err(RefreshStreamError::Reqwest(err))))
                    }
                    Poll::Pending => return Poll::Pending,
                },
                RefreshStreamState::ParseJson(fut) => match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(res)) => {
                        match (parse_jwt(&res.access_token), parse_jwt(&res.refresh_token)) {
                            (Ok(access_token), Ok(refresh_token)) => {
                                let access_jwt_expiry =
                                    JwtExpiry::new(access_token.expires_at.unwrap().into());
                                let refresh_jwt_expiry =
                                    JwtExpiry::new(refresh_token.expires_at.unwrap().into());

                                this.state.set(RefreshStreamState::WaitForExpiry {
                                    refresh_token: res.refresh_token.clone(),
                                    refresh_expiry: Box::pin(refresh_jwt_expiry),
                                    access_expiry: Box::pin(access_jwt_expiry),
                                });

                                cx.waker().wake_by_ref();
                                return Poll::Ready(Some(Ok(res)));
                            }
                            (Err(err), _) | (_, Err(err)) => {
                                return Poll::Ready(Some(Err(RefreshStreamError::JwtParse(err))))
                            }
                        }
                    }
                    Poll::Ready(Err(err)) => {
                        return Poll::Ready(Some(Err(RefreshStreamError::Reqwest(err))))
                    }
                    Poll::Pending => return Poll::Pending,
                },
                RefreshStreamState::WaitForExpiry {
                    refresh_token,
                    refresh_expiry,
                    access_expiry,
                } => {
                    match access_expiry.poll_unpin(cx) {
                        Poll::Ready(()) => {
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
                        Poll::Pending => {}
                    }
                    match refresh_expiry.poll_unpin(cx) {
                        Poll::Ready(()) => this.state.set(RefreshStreamState::PasswordLogin),
                        Poll::Pending => return Poll::Pending,
                    }
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
enum JwtParseError {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64, // in seconds
    token_type: String,
    user: User,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    id: String,
    email: String,
    // Add other user fields if needed
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use futures::StreamExt;
    use rstest::rstest;
    use tokio::test;

    use crate::SupabaseAuth;

    #[rstest]
    #[timeout(Duration::from_secs(3))]
    #[test_log::test(tokio::test)]
    async fn test_connect_and_authenticate() {
        // todo: create a simple mock serivec of supabase?
        let supabase_auth = SupabaseAuth::new("http://127.0.0.1:54321".parse().unwrap());
        let token_body = crate::TokenBody {
            email: std::borrow::Cow::Owned("ttt@ttt.lv".to_string()),
            password: redact::Secret::new(std::borrow::Cow::Owned("pass".to_string())),
        };
        let mut jwt_stream = supabase_auth.sign_in(token_body);
        let res = jwt_stream.next().await.unwrap().unwrap();
        dbg!(res);
        panic!("aaaa");
    }
}
