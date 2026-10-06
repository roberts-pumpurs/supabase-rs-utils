pub mod requests;
use core::marker::PhantomData;

use futures::{Stream, StreamExt as _};
use requests::AuthModuleRequest;
use reqwest::header;
use tracing::instrument;

use crate::error::AuthError;
use crate::jwt_stream::{RefreshStreamError, SupabaseAuthConfig};
use crate::types::LoginCredentials;
use crate::{SUPABASE_KEY, jwt_stream};

#[derive(Clone, Debug)]
pub struct ApiClient {
    inner: reqwest::Client,
    url: url::Url,
    headers: header::HeaderMap,
}

/// Creates a new authenticated stream.
///
/// # Errors
///
/// Returns an error if sign-in fails or the stream cannot be created.
pub fn new_authenticated_stream(
    config: SupabaseAuthConfig,
    login_info: LoginCredentials,
) -> Result<
    impl Stream<Item = Result<Result<ApiClient, AuthError>, RefreshStreamError>>,
    RefreshStreamError,
> {
    new_authenticated_stream_with_client(config, login_info, default_client()?)
}

/// Creates an authenticated stream retaining one configured HTTP transport.
///
/// # Errors
/// Returns URL, credential header or sign-in setup failures.
pub fn new_authenticated_stream_with_client(
    config: SupabaseAuthConfig,
    login_info: LoginCredentials,
    http: reqwest::Client,
) -> Result<
    impl Stream<Item = Result<Result<ApiClient, AuthError>, RefreshStreamError>>,
    RefreshStreamError,
> {
    let url = config.url.clone();
    let api_key = config.api_key.clone();
    let auth_stream = jwt_stream::JwtStream::new(config)
        .sign_in_with_client(login_info, http.clone())
        .map_err(RefreshStreamError::from)?;
    let client_stream = auth_stream
        .map(move |item| {
            let url = url.clone();
            let api_key = api_key.clone();

            let res = item
                .map(|item| {
                    if let Some(access_token) = item.access_token.as_ref() {
                        let client = ApiClient::new_authenticated_with_client(
                            &url,
                            &api_key,
                            access_token,
                            http.clone(),
                        );
                        return Some(client);
                    }
                    None
                })
                .transpose();
            res
        })
        .filter_map(futures::future::ready);

    Ok(client_stream)
}

impl ApiClient {
    /// Create a new unauthenticated API client.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL cannot be joined or the client cannot be created.
    pub fn new_unauthenticated(url: &url::Url, api_key: &str) -> Result<Self, AuthError> {
        Self::new_unauthenticated_with_client(url, api_key, default_client()?)
    }

    /// Create a new authenticated API client.
    ///
    /// # Errors
    ///
    /// Returns an error if the URL cannot be joined or the client cannot be created.
    pub fn new_authenticated(
        url: &url::Url,
        api_key: &str,
        token: &str,
    ) -> Result<Self, AuthError> {
        Self::new_authenticated_with_client(url, api_key, token, default_client()?)
    }

    /// Create an unauthenticated client retaining the supplied HTTP transport.
    ///
    /// # Errors
    /// Returns URL or credential header failures.
    pub fn new_unauthenticated_with_client(
        url: &url::Url,
        api_key: &str,
        http: reqwest::Client,
    ) -> Result<Self, AuthError> {
        Ok(Self {
            url: url.join("/auth/v1/")?,
            inner: http,
            headers: base_headers(api_key)?,
        })
    }

    /// Create an authenticated client without altering shared transport defaults.
    ///
    /// # Errors
    /// Returns URL or credential header failures.
    pub fn new_authenticated_with_client(
        url: &url::Url,
        api_key: &str,
        token: &str,
        http: reqwest::Client,
    ) -> Result<Self, AuthError> {
        let mut client = Self::new_unauthenticated_with_client(url, api_key, http)?;
        let mut authorization = header::HeaderValue::from_str(&format!("Bearer {token}"))?;
        authorization.set_sensitive(true);
        client.headers.insert(header::AUTHORIZATION, authorization);
        Ok(client)
    }

    /// Build a request for the API.
    ///
    /// # Errors
    ///
    /// Returns an error if the request cannot be built.
    #[instrument(name = "build_request", skip(self, request))]
    pub fn build_request<T>(&self, request: &T) -> Result<Request<T::Res, T::Error>, AuthError>
    where
        T: AuthModuleRequest + core::fmt::Debug,
    {
        let endpoint = request.path(&self.url)?;
        let method = T::METHOD;
        let client = self.inner.clone();
        let payload = simd_json::to_vec(&request.payload())?;
        let reqwest_req = client
            .request(method, endpoint.as_str())
            .headers(self.headers.clone())
            .body(payload);

        Ok(Request {
            builder: reqwest_req,
            result: PhantomData,
            err: PhantomData,
        })
    }
}

/// Encapsulated HTTP request for the API
pub struct Request<T, E> {
    builder: reqwest::RequestBuilder,
    result: PhantomData<T>,
    err: PhantomData<E>,
}

impl<T, E> Request<T, E> {
    /// Execute an API request.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    #[instrument(name = "execute_request", skip(self))]
    pub async fn execute(self) -> Result<Response<T, E>, AuthError> {
        let (client, request) = self.builder.build_split();
        let request = request?;

        // Capture the current span
        let span = tracing::Span::current();
        span.record("method", request.method().as_str());
        span.record("url", request.url().as_str());

        // execute the request
        let response = client.execute(request).await?;

        Ok(Response {
            raw: response,
            result: PhantomData,
            err: PhantomData,
            span,
        })
    }
}

/// The raw response of the API request
pub struct Response<T, E> {
    raw: reqwest::Response,
    result: PhantomData<T>,
    err: PhantomData<E>,
    // this span carries the context of the `Request`
    span: tracing::Span,
}

impl<T, E> Response<T, E> {
    /// Only check if the returtned HTTP response is of error type; don't parse the data
    ///
    /// Useful when you don't care about the actual response besides if it was an error.
    #[instrument(name = "response_ok", skip(self), err, parent = &self.span)]
    pub fn ok(self) -> Result<(), AuthError> {
        self.raw.error_for_status()?;
        Ok(())
    }

    /// Check if the returned HTTP result is an error;
    /// Only parse the error type if we received an error.
    ///
    /// Useful when you don't care about the actual response besides if it was an error.
    #[instrument(name = "parse_response_json_err", skip(self), err, parent = &self.span)]
    pub async fn json_err(self) -> Result<Result<(), E>, AuthError>
    where
        E: serde::de::DeserializeOwned,
    {
        let status = self.raw.status();
        if status.is_success() {
            Ok(Ok(()))
        } else {
            let bytes = self.raw.bytes().await?.to_vec();
            let res = parse_error::<E>(bytes, status)?;
            Ok(Err(res))
        }
    }

    /// Parse the response json
    #[instrument(name = "parse_response_json", skip(self), err, parent = &self.span)]
    pub async fn json(self) -> Result<Result<T, E>, AuthError>
    where
        T: serde::de::DeserializeOwned,
        E: serde::de::DeserializeOwned,
    {
        let status = self.raw.status();
        let mut bytes = self.raw.bytes().await?.to_vec();
        if status.is_success() {
            let json = String::from_utf8_lossy(bytes.as_ref());
            tracing::debug!(response_body = %json, "Response JSON");

            let result = simd_json::from_slice::<T>(bytes.as_mut())?;
            Ok(Ok(result))
        } else {
            let res = parse_error::<E>(bytes, status)?;
            Ok(Err(res))
        }
    }
}

fn parse_error<E>(mut bytes: Vec<u8>, status: reqwest::StatusCode) -> Result<E, AuthError>
where
    E: serde::de::DeserializeOwned,
{
    let json = String::from_utf8_lossy(bytes.as_ref());
    tracing::error!(
        status = %status,
        body = %json,
        "Failed to execute request"
    );

    let error = simd_json::from_slice::<E>(bytes.as_mut())?;
    Ok(error)
}

fn default_client() -> Result<reqwest::Client, AuthError> {
    const KEEP_ALIVE_INTERVAL: core::time::Duration = core::time::Duration::from_secs(15);

    let temp_client = reqwest::Client::builder()
        .use_rustls_tls()
        .http2_keep_alive_interval(KEEP_ALIVE_INTERVAL)
        .http2_keep_alive_while_idle(true)
        .build()?;
    Ok(temp_client)
}

fn base_headers(api_key: &str) -> Result<header::HeaderMap, AuthError> {
    let mut headers = header::HeaderMap::new();
    let mut key = header::HeaderValue::from_str(api_key)?;
    key.set_sensitive(true);
    headers.insert(SUPABASE_KEY, key);
    headers.insert(
        "Accept",
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/json"),
    );
    Ok(headers)
}
