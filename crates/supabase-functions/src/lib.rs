#![cfg_attr(doc, doc = include_str!("../README.md"))]
mod error;

pub use error::FunctionsError;
pub use reqwest::Method;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

const RELAY_ERROR_HEADER: &str = "x-relay-error";
const REGION_HEADER: &str = "x-region";

/// Client for Supabase Edge Functions at `{project}/functions/v1/`.
///
/// Cloning is cheap: the HTTP connection pool is shared.
#[derive(Debug, Clone)]
pub struct FunctionsClient {
    http: reqwest::Client,
    base: Url,
    headers: HeaderMap,
}

impl FunctionsClient {
    /// Creates a client for the project base URL, for example `https://abc.supabase.co/`.
    ///
    /// The client sends `apikey` and `Authorization: Bearer <api_key>` on every request.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL cannot hold a path or the key is not a valid header value.
    pub fn new(project_url: &Url, api_key: &str) -> Result<Self, FunctionsError> {
        Self::new_with_client(project_url, api_key, reqwest::Client::new())
    }

    /// Same as [`Self::new`], but reuses the given HTTP client and its connection pool.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL cannot hold a path or the key is not a valid header value.
    pub fn new_with_client(
        project_url: &Url,
        api_key: &str,
        http: reqwest::Client,
    ) -> Result<Self, FunctionsError> {
        if project_url.cannot_be_a_base() {
            return Err(FunctionsError::UrlNotBase);
        }
        let mut root = project_url.clone();
        if !root.path().ends_with('/') {
            let path = format!("{}/", root.path());
            root.set_path(&path);
        }
        let base = root.join("functions/v1/")?;
        let mut headers = HeaderMap::new();
        headers.insert("apikey", sensitive(api_key)?);
        headers.insert(AUTHORIZATION, sensitive(&format!("Bearer {api_key}"))?);
        Ok(Self {
            http,
            base,
            headers,
        })
    }

    /// Returns a copy of the client that sends `Authorization: Bearer <token>`.
    ///
    /// Use this to call functions as a signed-in user.
    ///
    /// # Errors
    ///
    /// Returns an error when the token is not a valid header value.
    pub fn with_access_token(&self, token: &str) -> Result<Self, FunctionsError> {
        let mut next = self.clone();
        next.headers
            .insert(AUTHORIZATION, sensitive(&format!("Bearer {token}"))?);
        Ok(next)
    }

    /// Starts a request to the function `name`. The default method is `POST`.
    ///
    /// The name must be non-empty and must not contain `/`. The builder reports an
    /// invalid name when you call [`InvokeBuilder::send`].
    pub fn invoke(&self, name: &str) -> InvokeBuilder {
        let url = if name.is_empty() || name.contains('/') {
            Err(FunctionsError::InvalidFunctionName(name.to_owned()))
        } else {
            let mut url = self.base.clone();
            url.path_segments_mut()
                .map_err(|()| FunctionsError::UrlNotBase)
                .map(|mut segments| {
                    segments.pop_if_empty().push(name);
                })
                .map(|()| url)
        };
        InvokeBuilder {
            http: self.http.clone(),
            state: url.map(|url| Request {
                url,
                method: Method::POST,
                headers: self.headers.clone(),
                body: None,
            }),
        }
    }
}

fn sensitive(value: &str) -> Result<HeaderValue, FunctionsError> {
    let mut header = HeaderValue::from_str(value)?;
    header.set_sensitive(true);
    Ok(header)
}

#[derive(Debug)]
struct Request {
    url: Url,
    method: Method,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
}

/// Request builder returned by [`FunctionsClient::invoke`].
///
/// Builder errors (invalid name, header, or JSON body) surface from [`Self::send`].
#[derive(Debug)]
#[must_use = "call `send`, `fetch`, or `text` to run the request"]
pub struct InvokeBuilder {
    http: reqwest::Client,
    state: Result<Request, FunctionsError>,
}

impl InvokeBuilder {
    fn update(mut self, apply: impl FnOnce(&mut Request) -> Result<(), FunctionsError>) -> Self {
        if let Ok(request) = self.state.as_mut() {
            if let Err(error) = apply(request) {
                self.state = Err(error);
            }
        }
        self
    }

    /// Sets the HTTP method. The default is `POST`.
    pub fn method(self, method: Method) -> Self {
        self.update(|request| {
            request.method = method;
            Ok(())
        })
    }

    /// Sends `value` as a JSON body with `Content-Type: application/json`.
    pub fn json<T: Serialize + ?Sized>(self, value: &T) -> Self {
        self.update(|request| {
            let bytes = serde_json::to_vec(value).map_err(FunctionsError::Serialize)?;
            request
                .headers
                .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            request.body = Some(bytes);
            Ok(())
        })
    }

    /// Sends raw bytes with the given `Content-Type`.
    pub fn body<B: Into<Vec<u8>>>(self, bytes: B, content_type: &str) -> Self {
        self.update(|request| {
            request
                .headers
                .insert(CONTENT_TYPE, HeaderValue::from_str(content_type)?);
            request.body = Some(bytes.into());
            Ok(())
        })
    }

    /// Sets a request header. A later call with the same name replaces the value.
    pub fn header(self, name: &str, value: &str) -> Self {
        self.update(|request| {
            let name = HeaderName::from_bytes(name.as_bytes())?;
            request.headers.insert(name, HeaderValue::from_str(value)?);
            Ok(())
        })
    }

    /// Runs the function in a specific region, for example `us-east-1`, via the `x-region` header.
    pub fn region(self, region: &str) -> Self {
        self.header(REGION_HEADER, region)
    }

    /// Sends the request and returns the response when the status is 2xx.
    ///
    /// # Errors
    ///
    /// - [`FunctionsError::Relay`] when the response has `x-relay-error: true`.
    /// - [`FunctionsError::Http`] for any other non-2xx status.
    /// - Builder and transport errors.
    pub async fn send(self) -> Result<reqwest::Response, FunctionsError> {
        let request = self.state?;
        let mut builder = self
            .http
            .request(request.method, request.url)
            .headers(request.headers);
        if let Some(body) = request.body {
            builder = builder.body(body);
        }
        let response = builder.send().await?;
        let relay = response
            .headers()
            .get(RELAY_ERROR_HEADER)
            .is_some_and(|value| value.as_bytes() == b"true");
        let status = response.status();
        if relay {
            let body = response.text().await?;
            return Err(FunctionsError::Relay { status, body });
        }
        if !status.is_success() {
            let body = response.text().await?;
            return Err(FunctionsError::Http { status, body });
        }
        Ok(response)
    }

    /// Sends the request and decodes the JSON response body as `T`.
    ///
    /// # Errors
    ///
    /// Errors from [`Self::send`], plus [`FunctionsError::Decode`] for invalid JSON.
    pub async fn fetch<T: DeserializeOwned>(self) -> Result<T, FunctionsError> {
        let bytes = self.send().await?.bytes().await?;
        serde_json::from_slice(&bytes).map_err(FunctionsError::Decode)
    }

    /// Sends the request and returns the response body as text.
    ///
    /// # Errors
    ///
    /// Errors from [`Self::send`].
    pub async fn text(self) -> Result<String, FunctionsError> {
        Ok(self.send().await?.text().await?)
    }
}
