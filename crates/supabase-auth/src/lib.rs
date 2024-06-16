mod jwt_expiry;
use std::any::Any;
use std::borrow::{Borrow, Cow};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use futures::{FutureExt, Stream};
use jwt_expiry::JwtExpiry;
use jwt_simple::claims::{JWTClaims, NoCustomClaims};
use jwt_simple::token::Token;
use pin_project::pin_project;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct SupabaseAuth {
    url: url::Url,
}

impl SupabaseAuth {
    async fn token<'a>(&self, params: TokenBody<'a>) -> RefreshStream<'a> {
        todo!()
    }
}

pub struct TokenBody<'a> {
    email: Cow<'a, str>,
    password: redact::Secret<Cow<'a, str>>,
}

#[pin_project]
pub struct RefreshStream<'a> {
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
        access_token: String,
        #[pin]
        access_expiry: futures::future::BoxFuture<'static, ()>,
    },
}

impl<'a> Stream for RefreshStream<'a> {
    type Item = Result<AuthResponse, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.state.as_mut().get_mut() {
                RefreshStreamState::PasswordLogin => {
                    let request_future = this
                        .client
                        .post("/token?grant_type=password")
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
                    Poll::Ready(Ok(res)) => {
                        let res = res.error_for_status();
                        match res {
                            Ok(res) => {
                                let json_future = res.json::<AuthResponse>().boxed();
                                this.state.set(RefreshStreamState::ParseJson(json_future));
                            }
                            Err(_) => todo!(),
                        }
                    }
                    Poll::Ready(Err(err)) => return Poll::Ready(Some(Err(err))),
                    Poll::Pending => return Poll::Pending,
                },
                RefreshStreamState::ParseJson(res) => match res.poll_unpin(cx) {
                    Poll::Ready(Ok(res)) => {
                        let access_token = parse_jwt(&res.access_token).unwrap();
                        let refresh_token = parse_jwt(&res.refresh_token).unwrap();
                        let access_jwt_expiry =
                            JwtExpiry::new(access_token.expires_at.unwrap().into());
                        let refresh_jwt_expiry =
                            JwtExpiry::new(refresh_token.expires_at.unwrap().into());
                        this.state.set(RefreshStreamState::WaitForExpiry {
                            refresh_token: res.refresh_token.clone(),
                            refresh_expiry: Box::pin(refresh_jwt_expiry),
                            access_token: res.access_token.clone(),
                            access_expiry: Box::pin(access_jwt_expiry),
                        });

                        cx.waker().wake_by_ref();
                        return Poll::Ready(Some(Ok(res)))
                    }
                    Poll::Ready(Err(res)) => todo!(),
                    Poll::Pending => todo!(),
                },
                RefreshStreamState::WaitForExpiry {
                    refresh_token,
                    refresh_expiry,
                    access_token: _,
                    access_expiry,
                } => {
                    // todo: check for refresh expiry - if expired then log in

                    match access_expiry.poll_unpin(cx) {
                        Poll::Ready(()) => {
                            let request_future = this
                                .client
                                .post("/token?grant_type=refresh_token")
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
                        Poll::Pending => todo!(),
                    }
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {}

fn parse_jwt(token: &str) -> Result<JWTClaims<NoCustomClaims>, Error> {
    use base64::prelude::*;

    let mut tokens = token.split('.');
    let _header = tokens.next();
    let body = tokens.next().unwrap();
    let body = BASE64_STANDARD.decode(body).unwrap();
    let body = serde_json::from_slice::<JWTClaims<NoCustomClaims>>(&body).unwrap();

    Ok(body)
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64, // in seconds
    token_type: String,
    user: User,
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: String,
    email: String,
    // Add other user fields if needed
}
