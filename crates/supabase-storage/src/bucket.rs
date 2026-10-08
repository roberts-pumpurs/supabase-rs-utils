use bytes::Bytes;
use reqwest::header::{CACHE_CONTROL, CONTENT_TYPE};
use reqwest::{Body, Method};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::StorageError;
use crate::types::{FileObject, FileOptions, ListOptions, ObjectKey, SignedUrl};
use crate::{StorageClient, path};

/// Object operations in one bucket. Create it with [`StorageClient::from`].
///
/// Object paths use `/` between segments, for example `avatars/user-1.png`. Each segment is
/// percent-encoded once. Empty paths, empty segments, `.` and `..` return
/// [`StorageError::InvalidPath`].
#[derive(Debug, Clone, Copy)]
pub struct Bucket<'a> {
    client: &'a StorageClient,
    id: &'a str,
}

#[derive(Serialize)]
struct ListBody<'a> {
    prefix: &'a str,
    #[serde(flatten)]
    options: &'a ListOptions,
}

#[derive(Serialize)]
struct RemoveBody<'a> {
    prefixes: &'a [&'a str],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TransferBody<'a> {
    bucket_id: &'a str,
    source_key: &'a str,
    destination_key: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignBody<'a> {
    expires_in: u64,
    paths: &'a [&'a str],
}

#[derive(Deserialize)]
struct SignedEntry {
    #[serde(default)]
    path: Option<String>,
    #[serde(rename = "signedURL", default)]
    signed_url: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

impl<'a> Bucket<'a> {
    pub(crate) const fn new(client: &'a StorageClient, id: &'a str) -> Self {
        Self { client, id }
    }

    /// Bucket id of this handle.
    #[must_use]
    pub const fn id(&self) -> &'a str {
        self.id
    }

    /// Uploads a new object (`POST /object/<bucket>/<path>`). Set [`FileOptions::upsert`] to
    /// overwrite an existing object.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid path or header, transport failure, non-success status, or
    /// an unexpected body.
    pub async fn upload<B: Into<Body>>(
        &self,
        path: &str,
        body: B,
        options: &FileOptions,
    ) -> Result<ObjectKey, StorageError> {
        self.write(Method::POST, path, body.into(), options).await
    }

    /// Replaces an existing object (`PUT /object/<bucket>/<path>`).
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid path or header, transport failure, non-success status, or
    /// an unexpected body.
    pub async fn update<B: Into<Body>>(
        &self,
        path: &str,
        body: B,
        options: &FileOptions,
    ) -> Result<ObjectKey, StorageError> {
        self.write(Method::PUT, path, body.into(), options).await
    }

    async fn write(
        &self,
        method: Method,
        path: &str,
        body: Body,
        options: &FileOptions,
    ) -> Result<ObjectKey, StorageError> {
        let url = self.object_url(&["object"], path)?;
        let upsert = if options.upsert { "true" } else { "false" };
        let mut request = self
            .client
            .request(method, url)
            .header("x-upsert", upsert)
            .body(body);
        if let Some(content_type) = &options.content_type {
            request = request.header(CONTENT_TYPE, content_type.as_str());
        }
        request = request.header(CACHE_CONTROL, format!("max-age={}", options.cache_control));
        StorageClient::json(request).await
    }

    /// Downloads an object.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid path, transport failure, or non-success status.
    pub async fn download(&self, path: &str) -> Result<Bytes, StorageError> {
        let url = self.object_url(&["object"], path)?;
        let response = StorageClient::send(self.client.request(Method::GET, url)).await?;
        Ok(response.bytes().await?)
    }

    /// Lists objects and folders directly under `prefix` (use `""` for the bucket root).
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid bucket id, transport failure, non-success status, or an
    /// unexpected body.
    pub async fn list(
        &self,
        prefix: &str,
        options: &ListOptions,
    ) -> Result<Vec<FileObject>, StorageError> {
        let url = self.client.url(["object", "list", path::bucket(self.id)?]);
        let body = ListBody { prefix, options };
        StorageClient::json(self.client.request(Method::POST, url).json(&body)).await
    }

    /// Deletes objects and returns the deleted entries. Missing paths are skipped by the API.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid path, transport failure, non-success status, or an
    /// unexpected body.
    pub async fn remove(&self, paths: &[&str]) -> Result<Vec<FileObject>, StorageError> {
        for object in paths {
            path::object(object)?;
        }
        let url = self.client.url(["object", path::bucket(self.id)?]);
        let body = RemoveBody { prefixes: paths };
        StorageClient::json(self.client.request(Method::DELETE, url).json(&body)).await
    }

    /// Moves (renames) an object inside this bucket.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid path, transport failure, or non-success status.
    pub async fn move_object(&self, from: &str, to: &str) -> Result<(), StorageError> {
        let request = self.transfer("move", from, to)?;
        StorageClient::send(request).await?;
        Ok(())
    }

    /// Copies an object inside this bucket and returns the key of the copy.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid path, transport failure, non-success status, or an
    /// unexpected body.
    pub async fn copy_object(&self, from: &str, to: &str) -> Result<ObjectKey, StorageError> {
        StorageClient::json(self.transfer("copy", from, to)?).await
    }

    fn transfer(
        &self,
        action: &str,
        from: &str,
        to: &str,
    ) -> Result<reqwest::RequestBuilder, StorageError> {
        path::object(from)?;
        path::object(to)?;
        let body = TransferBody {
            bucket_id: path::bucket(self.id)?,
            source_key: from,
            destination_key: to,
        };
        let url = self.client.url(["object", action]);
        Ok(self.client.request(Method::POST, url).json(&body))
    }

    /// Creates an absolute URL that grants read access to one object for `expires_in` seconds.
    ///
    /// This uses the batch signing endpoint. The single-object endpoint signs a key that still
    /// contains percent escapes when the name has a `?`, so its URL fails with an invalid
    /// signature.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid path, transport failure, non-success status, an
    /// unexpected body, or [`StorageError::SignFailed`] when the object cannot be signed.
    pub async fn create_signed_url(
        &self,
        path: &str,
        expires_in: u64,
    ) -> Result<Url, StorageError> {
        let entry = self
            .create_signed_urls(&[path], expires_in)
            .await?
            .into_iter()
            .next();
        match entry {
            Some(SignedUrl { url: Some(url), .. }) => Ok(url),
            Some(SignedUrl { error, .. }) => Err(StorageError::SignFailed {
                path: path.to_owned(),
                message: error.unwrap_or_default(),
            }),
            None => Err(StorageError::SignFailed {
                path: path.to_owned(),
                message: "the response has no entry".to_owned(),
            }),
        }
    }

    /// Creates signed URLs for several objects in one request. Paths that cannot be signed
    /// have `url: None` and an `error`.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid path, transport failure, non-success status, or an
    /// unexpected body.
    pub async fn create_signed_urls(
        &self,
        paths: &[&str],
        expires_in: u64,
    ) -> Result<Vec<SignedUrl>, StorageError> {
        for object in paths {
            path::object(object)?;
        }
        let url = self.client.url(["object", "sign", path::bucket(self.id)?]);
        let body = SignBody { expires_in, paths };
        let entries: Vec<SignedEntry> =
            StorageClient::json(self.client.request(Method::POST, url).json(&body)).await?;
        entries
            .into_iter()
            .zip(paths)
            .map(|(entry, requested)| {
                let url = entry
                    .signed_url
                    .map(|signed| {
                        self.signed_url(entry.path.as_deref().unwrap_or(requested), &signed)
                    })
                    .transpose()?;
                Ok(SignedUrl {
                    path: entry.path,
                    url,
                    error: entry.error,
                })
            })
            .collect()
    }

    /// Builds `<base>/object/sign/<bucket>/<path>?token=<token>` from the known path. The API
    /// returns the object key unescaped, so a key with `?` or `#` breaks a plain URL parse.
    /// The token follows the last `?token=` in `signed`.
    fn signed_url(&self, path: &str, signed: &str) -> Result<Url, StorageError> {
        let (_, token) = signed
            .rsplit_once("?token=")
            .ok_or_else(|| StorageError::MissingSignedToken(signed.to_owned()))?;
        let mut url = self.object_url(&["object", "sign"], path)?;
        url.set_query(Some(&format!("token={token}")));
        Ok(url)
    }

    /// Returns the public URL of an object. No request is sent and the URL works only when the
    /// bucket is public; use [`Bucket::create_signed_url`] for private buckets.
    ///
    /// # Errors
    ///
    /// Returns an error on an invalid bucket id or object path.
    pub fn public_url(&self, path: &str) -> Result<Url, StorageError> {
        self.object_url(&["object", "public"], path)
    }

    fn object_url(&self, prefix: &[&str], path: &str) -> Result<Url, StorageError> {
        let bucket = path::bucket(self.id)?;
        let segments = path::object(path)?;
        Ok(self.client.url(
            prefix
                .iter()
                .copied()
                .chain(core::iter::once(bucket))
                .chain(segments),
        ))
    }
}
