use std::borrow::Cow;

use jwt_simple::claims::NoCustomClaims;

pub struct SupabaseAuth {
    url: url::Url,
}

impl SupabaseAuth {
    async fn token<'a>(&self, params: TokenBody<'a>) -> RefreshStream<'a> {
        todo!()
    }
}

pub enum TokenBody<'a> {
    Email {
        email: Cow<'a, str>,
        password: redact::Secret<Cow<'a, str>>,
    },
}

pub struct RefreshStream<'a> {
    client: reqwest::Client,
    token_body: TokenBody<'a>,
    // add extra state variables here
    refresh_at: ..
}

impl<'a> futures::Stream for RefreshStream<'a> {
    type Item = jwt_simple::claims::JWTClaims<NoCustomClaims>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // todo: call the url /token endpoint using `reqwest`
        // the returnend data looks like this: {
        //   "access_token": "jwt-token-representing-the-user",
        //   "token_type": "bearer",
        //   "expires_in": 3600,
        //   "refresh_token": "a-refresh-token"
        // }
        // todo: parse the returned result using `jwt_simple`
        // if successful: returen the new auth token
        // and store the `refresh_at` and 
        // wake up when we need to refresh the token again
        todo!()
    }
}
