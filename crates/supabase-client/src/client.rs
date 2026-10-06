use futures::{Stream, StreamExt as _};
use rp_postgrest::{Postgrest, reqwest};
use rp_supabase_auth::error::AuthError;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::{AccessTokenResponseSchema, LoginCredentials};
use rp_supabase_auth::url;

pub const SUPABASE_KEY: &str = "apikey";

/// Create an authenticated client stream using a default HTTP transport.
///
/// # Errors
/// Returns transport construction, configuration or sign-in setup failures.
pub fn new_authenticated(
    config: SupabaseAuthConfig,
    login_info: LoginCredentials,
) -> Result<
    impl Stream<Item = Result<(Postgrest, AccessTokenResponseSchema), SupabaseClientError>>,
    SupabaseClientError,
> {
    new_authenticated_with_client(config, login_info, reqwest::Client::builder().build()?)
}

/// Create an authenticated stream sharing the supplied HTTP transport for REST,
/// login and every token refresh. Request credentials do not alter its defaults.
///
/// # Errors
/// Returns configuration or sign-in setup failures.
pub fn new_authenticated_with_client(
    config: SupabaseAuthConfig,
    login_info: LoginCredentials,
    http: reqwest::Client,
) -> Result<
    impl Stream<Item = Result<(Postgrest, AccessTokenResponseSchema), SupabaseClientError>>,
    SupabaseClientError,
> {
    let base = anonymous_client_with_client(config.api_key.clone(), &config.url, http.clone())?;
    let auth_stream = rp_supabase_auth::jwt_stream::JwtStream::new(config)
        .sign_in_with_client(login_info, http)?;
    Ok(auth_stream.map(move |item| {
        let item = item?;
        let client = match item.access_token.as_ref() {
            Some(token) => base.clone().auth(token)?,
            None => base.clone(),
        };
        Ok((client, item))
    }))
}

/// Create an anonymous client using a default HTTP transport.
///
/// # Errors
/// Returns transport construction or configuration failures.
pub fn anonymous_client(api_key: String, url: &url::Url) -> Result<Postgrest, SupabaseClientError> {
    anonymous_client_with_client(api_key, url, reqwest::Client::builder().build()?)
}

/// Create an anonymous client retaining the supplied HTTP pool and policies.
///
/// # Errors
/// Returns URL or header configuration failures.
pub fn anonymous_client_with_client(
    api_key: String,
    url: &url::Url,
    http: reqwest::Client,
) -> Result<Postgrest, SupabaseClientError> {
    let url = url.join("rest/v1/")?;
    Ok(Postgrest::new_with_client(url.as_str(), http)?.insert_header(SUPABASE_KEY, api_key)?)
}

#[derive(thiserror::Error, Debug)]
pub enum SupabaseClientError {
    #[error("Jwt Stream closed unexpectedly")]
    JwtStreamClosedUnexpectedly,
    #[error("Refresh stream error")]
    RefreshStreamError(#[from] rp_supabase_auth::jwt_stream::RefreshStreamError),
    #[error("Auth sign in error")]
    AuthSignInError(#[from] rp_supabase_auth::jwt_stream::SignInError),
    #[error("Url parse error {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Auth error {0}")]
    AuthError(#[from] AuthError),
    #[error(transparent)]
    Postgrest(#[from] rp_postgrest::Error),
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
}
