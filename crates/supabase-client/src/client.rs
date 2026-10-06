use core::marker::PhantomData;

use futures::{Stream, StreamExt as _};
use postgrest::{Postgrest, reqwest};
use rp_supabase_auth::error::AuthError;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::{AccessTokenResponseSchema, LoginCredentials};
use rp_supabase_auth::url;
use tracing::instrument;

pub struct PostgerstResponse<T> {
    response: reqwest::Response,
    result: PhantomData<T>,
}

pub const SUPABASE_KEY: &str = "apikey";

/// Create a new authenticated supabase client stream
///
/// # Errors
/// - the client cannot be constructed
/// - the login url is invalid
pub fn new_authenticated(
    config: SupabaseAuthConfig,
    login_info: LoginCredentials,
) -> Result<
    impl Stream<Item = Result<(Postgrest, AccessTokenResponseSchema), SupabaseClientError>>,
    SupabaseClientError,
> {
    let base = anonymous_client(config.api_key.clone(), &config.url)?;
    let auth_stream = rp_supabase_auth::jwt_stream::JwtStream::new(config).sign_in(login_info)?;
    let client_stream = auth_stream.map(move |item| {
        item.map(|item| {
            let mut client = base.clone();
            if let Some(access_token) = item.access_token.as_ref() {
                client = client.auth(access_token);
            }
            (client, item)
        })
        .map_err(SupabaseClientError::from)
    });

    Ok(client_stream)
}

/// Create a new anonymous supabase client
///
/// # Errors
/// - the url is invalid
pub fn anonymous_client(api_key: String, url: &url::Url) -> Result<Postgrest, SupabaseClientError> {
    let url = url.join("rest/v1/")?;
    let postgrest = Postgrest::new(url).insert_header(SUPABASE_KEY, api_key);
    Ok(postgrest)
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
}

impl<T> PostgerstResponse<T> {
    #[must_use]
    pub const fn new(response: reqwest::Response) -> Self {
        Self {
            response,
            result: PhantomData,
        }
    }

    /// Only check if the returtned HTTP response is of error type; don't parse the data
    ///
    /// Useful when you don't care about the actual response besides if it was an error.
    #[instrument(name = "response_ok", skip(self), err)]
    pub fn ok(self) -> Result<(), ResponseError> {
        self.response.error_for_status()?;
        Ok(())
    }

    /// Check if the returned HTTP result is an error;
    /// Only parse the error type if we received an error.
    ///
    /// Useful when you don't care about the actual response besides if it was an error.
    #[instrument(name = "parse_response_json_err", skip(self), err)]
    pub async fn json_err(
        self,
    ) -> Result<Result<(), rp_postgrest_error::PostgrestError>, ResponseError> {
        let status = self.response.status();
        if status.is_success() {
            Ok(Ok(()))
        } else {
            let bytes = self.response.bytes().await?.to_vec();
            let error = parse_postgrest_error(bytes, status)?;
            Ok(Err(error))
        }
    }

    /// Parse the response json
    #[instrument(name = "parse_response_json", skip(self), err)]
    pub async fn json(self) -> Result<Result<T, rp_postgrest_error::PostgrestError>, ResponseError>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = self.response.status();
        let bytes = self.response.bytes().await?;
        if status.is_success() {
            let json = String::from_utf8_lossy(bytes.as_ref());
            tracing::debug!(response_body = %json, "Response JSON");

            let result = serde_json::from_slice::<T>(&bytes)?;
            Ok(Ok(result))
        } else {
            let error = parse_postgrest_error(bytes.to_vec(), status)?;
            Ok(Err(error))
        }
    }
}

fn parse_postgrest_error(
    bytes: Vec<u8>,
    status: reqwest::StatusCode,
) -> Result<rp_postgrest_error::PostgrestError, ResponseError> {
    let json = String::from_utf8_lossy(bytes.as_ref());
    tracing::error!(
        status = %status,
        body = %json,
        "Failed to execute request"
    );

    Ok(rp_postgrest_error::PostgrestError::from_vec(status, bytes)?)
}

#[derive(thiserror::Error, Debug)]
pub enum ResponseError {
    #[error("JSON response decode error {0}")]
    Json(#[from] serde_json::Error),
    #[error("PostgREST error response decode error {0}")]
    PostgrestDecode(#[from] rp_postgrest_error::DecodeError),
    #[error("reqwest {0}")]
    Request(#[from] reqwest::Error),
}
