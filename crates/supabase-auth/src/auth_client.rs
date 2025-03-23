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
}

pub fn new_authenticated_stream(
    config: SupabaseAuthConfig,
    login_info: LoginCredentials,
) -> Result<
    impl Stream<Item = Result<Result<ApiClient, AuthError>, RefreshStreamError>>,
    RefreshStreamError,
> {
    let url = config.url.clone();
    let api_key = config.api_key.clone();
    let auth_stream = jwt_stream::JwtStream::new(config)
        .sign_in(login_info)
        .unwrap();
    let client_stream = auth_stream
        .map(move |item| {
            let url = url.clone();
            let api_key = api_key.clone();

            let res = item
                .map(|item| {
                    if let Some(access_token) = item.access_token.as_ref() {
                        let client = ApiClient::new_authenticated(url, &api_key, access_token);
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
    pub fn new_unauthenticated(url: url::Url, api_key: &str) -> Result<Self, AuthError> {
        let url = url.join("/auth/v1/")?;
        let authenticated_client = unauthenticated_client(api_key)?;
        Ok(Self {
            url,
            inner: authenticated_client,
        })
    }

    pub fn new_authenticated(url: url::Url, api_key: &str, token: &str) -> Result<Self, AuthError> {
        let url = url.join("/auth/v1/")?;
        let authenticated_client = authenticated_client(api_key, token)?;
        Ok(Self {
            url,
            inner: authenticated_client,
        })
    }

    #[instrument(name = "build_request", skip(self, request))]
    pub fn build_request<T>(&self, request: &T) -> Result<Request<T::Res, T::Error>, AuthError>
    where
        T: AuthModuleRequest + core::fmt::Debug,
    {
        let endpoint = request.path(&self.url)?;
        let method = T::METHOD;
        let client = self.inner.clone();
        let payload = simd_json::to_vec(&request.payload())?;
        let reqwest_req = client.request(method, endpoint.as_str()).body(payload);

        Ok(Request {
            request: reqwest_req,
            result: PhantomData,
            err: PhantomData,
        })
    }
}

/// Encalpsulated HTTP request for the  API
pub struct Request<T, E> {
    request: reqwest::RequestBuilder,
    result: PhantomData<T>,
    err: PhantomData<E>,
}

impl<T, E> Request<T, E> {
    /// execute an  API request
    #[instrument(name = "execute_request", skip(self))]
    pub async fn execute(self) -> Result<Response<T, E>, AuthError> {
        let (client, request) = self.request.build_split();
        let request = request?;

        // Capture the current span
        let span = tracing::Span::current();
        span.record("method", request.method().as_str());
        span.record("url", request.url().as_str());

        // execute the request
        let response = client.execute(request).await?;

        Ok(Response {
            response,
            result: PhantomData,
            err: PhantomData,
            span,
        })
    }
}

/// The raw response of the  API request
pub struct Response<T, E> {
    response: reqwest::Response,
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
        self.response.error_for_status()?;
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
        let status = self.response.status();
        if status.is_success() {
            Ok(Ok(()))
        } else {
            let bytes = self.response.bytes().await?.to_vec();
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
        let status = self.response.status();
        let mut bytes = self.response.bytes().await?.to_vec();
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

fn unauthenticated_client(api_key: &str) -> Result<reqwest::Client, AuthError> {
    const KEEP_ALIVE_INTERVAL: core::time::Duration = core::time::Duration::from_secs(15);

    let headers = base_headers(api_key)?;

    let temp_client = reqwest::Client::builder()
        .use_rustls_tls()
        .http2_keep_alive_interval(KEEP_ALIVE_INTERVAL)
        .http2_keep_alive_while_idle(true)
        .default_headers(headers)
        .build()?;
    Ok(temp_client)
}

fn authenticated_client(api_key: &str, token: &str) -> Result<reqwest::Client, AuthError> {
    const KEEP_ALIVE_INTERVAL: core::time::Duration = core::time::Duration::from_secs(15);

    let mut headers = base_headers(api_key)?;
    headers.insert(
        "Authorization",
        header::HeaderValue::from_str(&format!("Bearer {token}"))?,
    );

    let temp_client = reqwest::Client::builder()
        .use_rustls_tls()
        .http2_keep_alive_interval(KEEP_ALIVE_INTERVAL)
        .http2_keep_alive_while_idle(true)
        .default_headers(headers)
        .build()?;
    Ok(temp_client)
}

fn base_headers(api_key: &str) -> Result<header::HeaderMap, AuthError> {
    let mut headers = header::HeaderMap::new();
    headers.insert(SUPABASE_KEY, header::HeaderValue::from_str(api_key)?);
    headers.insert(
        "Accept",
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        "Accept-Encoding",
        header::HeaderValue::from_static("gzip, deflate"),
    );
    headers.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/json"),
    );
    Ok(headers)
}
