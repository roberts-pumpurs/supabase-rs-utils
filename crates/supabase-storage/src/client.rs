use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use reqwest::{Method, RequestBuilder, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

use crate::error::{ApiErrorBody, StorageError};
use crate::types::{BucketInfo, BucketOptions};
use crate::{Bucket, path};

/// Client for the Supabase Storage API at `<project>/storage/v1`.
///
/// Cloning is cheap; clones share the HTTP connection pool.
#[derive(Debug, Clone)]
pub struct StorageClient {
    http: reqwest::Client,
    base: Url,
    headers: HeaderMap,
}

#[derive(Serialize)]
struct BucketBody<'a> {
    id: &'a str,
    name: &'a str,
    #[serde(flatten)]
    options: &'a BucketOptions,
}

impl StorageClient {
    /// Creates a client for the project at `project_url` (for example `https://abc.supabase.co/`).
    ///
    /// Requests send `apikey: <api_key>` and `Authorization: Bearer <api_key>`.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL cannot be a base or `api_key` is not a valid header value.
    pub fn new(project_url: &Url, api_key: &str) -> Result<Self, StorageError> {
        Self::new_with_client(project_url, api_key, reqwest::Client::new())
    }

    /// Like [`StorageClient::new`], but reuses the connection pool of `http`.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL cannot be a base or `api_key` is not a valid header value.
    pub fn new_with_client(
        project_url: &Url,
        api_key: &str,
        http: reqwest::Client,
    ) -> Result<Self, StorageError> {
        let base = project_url.join("storage/v1")?;
        if base.cannot_be_a_base() {
            return Err(StorageError::InvalidBaseUrl);
        }
        let mut headers = HeaderMap::new();
        headers.insert("apikey", sensitive(api_key)?);
        headers.insert(AUTHORIZATION, sensitive(&format!("Bearer {api_key}"))?);
        Ok(Self {
            http,
            base,
            headers,
        })
    }

    /// Returns a copy that sends `Authorization: Bearer <token>` (a user JWT) so that row level
    /// security policies apply to that user. The `apikey` header stays unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error when `token` is not a valid header value.
    pub fn with_access_token(&self, token: &str) -> Result<Self, StorageError> {
        let mut next = self.clone();
        next.headers
            .insert(AUTHORIZATION, sensitive(&format!("Bearer {token}"))?);
        Ok(next)
    }

    /// Returns a handle for object operations in `bucket`. No request is sent.
    #[must_use]
    pub const fn from<'a>(&'a self, bucket: &'a str) -> Bucket<'a> {
        Bucket::new(self, bucket)
    }

    /// Lists all buckets visible to the current credentials.
    ///
    /// # Errors
    ///
    /// Returns an error on transport failure, non-success status, or an unexpected body.
    pub async fn list_buckets(&self) -> Result<Vec<BucketInfo>, StorageError> {
        Self::json(self.request(Method::GET, self.url(["bucket"]))).await
    }

    /// Fetches one bucket.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid id, transport failure, non-success status, or an unexpected body.
    pub async fn get_bucket(&self, id: &str) -> Result<BucketInfo, StorageError> {
        let url = self.url(["bucket", path::bucket(id)?]);
        Self::json(self.request(Method::GET, url)).await
    }

    /// Creates a bucket whose id and name are `id`.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid id, transport failure, or non-success status.
    pub async fn create_bucket(
        &self,
        id: &str,
        options: &BucketOptions,
    ) -> Result<(), StorageError> {
        let body = BucketBody {
            id: path::bucket(id)?,
            name: id,
            options,
        };
        Self::send(self.request(Method::POST, self.url(["bucket"])).json(&body)).await?;
        Ok(())
    }

    /// Replaces the settings of a bucket.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid id, transport failure, or non-success status.
    pub async fn update_bucket(
        &self,
        id: &str,
        options: &BucketOptions,
    ) -> Result<(), StorageError> {
        let url = self.url(["bucket", path::bucket(id)?]);
        let body = BucketBody {
            id,
            name: id,
            options,
        };
        Self::send(self.request(Method::PUT, url).json(&body)).await?;
        Ok(())
    }

    /// Deletes all objects in a bucket and keeps the bucket.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid id, transport failure, or non-success status.
    pub async fn empty_bucket(&self, id: &str) -> Result<(), StorageError> {
        let url = self.url(["bucket", path::bucket(id)?, "empty"]);
        Self::send(
            self.request(Method::POST, url)
                .json(&serde_json::Map::new()),
        )
        .await?;
        Ok(())
    }

    /// Deletes a bucket. The API rejects this when the bucket is not empty; call
    /// [`StorageClient::empty_bucket`] first.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid id, transport failure, or non-success status.
    pub async fn delete_bucket(&self, id: &str) -> Result<(), StorageError> {
        let url = self.url(["bucket", path::bucket(id)?]);
        Self::send(self.request(Method::DELETE, url)).await?;
        Ok(())
    }

    /// Builds `<base>/<segments...>`; each segment is percent-encoded once.
    pub(crate) fn url<'a, I: IntoIterator<Item = &'a str>>(&self, segments: I) -> Url {
        let mut url = self.base.clone();
        // The constructor rejects cannot-be-a-base URLs, so this always succeeds.
        if let Ok(mut path) = url.path_segments_mut() {
            path.pop_if_empty().extend(segments);
        }
        url
    }

    /// Turns a URL relative to `/storage/v1` into an absolute URL.
    pub(crate) fn absolute(&self, relative: &str) -> Result<Url, StorageError> {
        if let Ok(url) = Url::parse(relative) {
            return Ok(url);
        }
        let base = self.base.as_str().trim_end_matches('/');
        let separator = if relative.starts_with('/') { "" } else { "/" };
        Ok(Url::parse(&format!("{base}{separator}{relative}"))?)
    }

    pub(crate) fn request(&self, method: Method, url: Url) -> RequestBuilder {
        self.http.request(method, url).headers(self.headers.clone())
    }

    pub(crate) async fn send(request: RequestBuilder) -> Result<Response, StorageError> {
        let response = request.send().await?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let body = ApiErrorBody::decode(response.text().await?);
        Err(StorageError::Api { status, body })
    }

    pub(crate) async fn json<T: DeserializeOwned>(
        request: RequestBuilder,
    ) -> Result<T, StorageError> {
        let bytes = Self::send(request).await?.bytes().await?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

fn sensitive(value: &str) -> Result<HeaderValue, StorageError> {
    let mut header = HeaderValue::from_str(value)?;
    header.set_sensitive(true);
    Ok(header)
}
