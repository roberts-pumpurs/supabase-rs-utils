use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Settings for [`StorageClient::create_bucket`](crate::StorageClient::create_bucket) and
/// [`StorageClient::update_bucket`](crate::StorageClient::update_bucket).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct BucketOptions {
    /// Anyone can read objects without a token when `true`.
    pub public: bool,
    /// Maximum object size in bytes. `None` keeps the server limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size_limit: Option<u64>,
    /// Accepted MIME types, for example `image/png` or `image/*`. `None` accepts all types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mime_types: Option<Vec<String>>,
}

/// A bucket as returned by the Storage API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BucketInfo {
    /// Bucket id.
    pub id: String,
    /// Bucket name.
    pub name: String,
    /// Owner user id, if any.
    #[serde(default)]
    pub owner: Option<String>,
    /// Whether the bucket is public.
    pub public: bool,
    /// Maximum object size in bytes.
    #[serde(default)]
    pub file_size_limit: Option<u64>,
    /// Accepted MIME types.
    #[serde(default)]
    pub allowed_mime_types: Option<Vec<String>>,
    /// Creation time.
    pub created_at: DateTime<Utc>,
    /// Last update time.
    pub updated_at: DateTime<Utc>,
}

/// Settings for [`Bucket::upload`](crate::Bucket::upload) and [`Bucket::update`](crate::Bucket::update).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileOptions {
    /// `Content-Type` of the object. `None` lets the server pick one.
    pub content_type: Option<String>,
    /// Sent as `Cache-Control: max-age=<seconds>`. `None` keeps the server default (3600).
    pub cache_control: Option<u32>,
    /// Overwrite an existing object at the same path. Sent as `x-upsert`.
    pub upsert: bool,
}

/// Key of a stored object.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ObjectKey {
    /// Full key, `<bucket>/<path>`.
    #[serde(rename = "Key")]
    pub key: String,
    /// Object id. Upload responses carry it; copy responses do not.
    #[serde(rename = "Id", default)]
    pub id: Option<String>,
}

/// Settings for [`Bucket::list`](crate::Bucket::list). `None` fields use the server defaults
/// (limit 100, offset 0, sorted by name ascending).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ListOptions {
    /// Maximum number of entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Number of entries to skip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    /// Sort order.
    #[serde(rename = "sortBy", skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<SortBy>,
    /// Return only entries whose name contains this text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

/// Sort order for [`ListOptions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SortBy {
    /// Column to sort by.
    pub column: SortColumn,
    /// Direction.
    pub order: SortOrder,
}

/// Sortable column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SortColumn {
    /// Object name.
    Name,
    /// Last update time.
    UpdatedAt,
    /// Creation time.
    CreatedAt,
    /// Last access time.
    LastAccessedAt,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// Ascending.
    Asc,
    /// Descending.
    Desc,
}

/// An object or folder entry. Folder entries have no id, timestamps, or metadata.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FileObject {
    /// Name relative to the listed prefix (or the full path for `remove` results).
    pub name: String,
    /// Object id. `None` for folders.
    #[serde(default)]
    pub id: Option<String>,
    /// Bucket id, when the API includes it.
    #[serde(default)]
    pub bucket_id: Option<String>,
    /// Creation time.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update time.
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    /// Last access time.
    #[serde(default)]
    pub last_accessed_at: Option<DateTime<Utc>>,
    /// Object metadata (size, mimetype, eTag, cacheControl, ...).
    #[serde(default)]
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

/// One entry of [`Bucket::create_signed_urls`](crate::Bucket::create_signed_urls).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedUrl {
    /// Object path, when the API reports it.
    pub path: Option<String>,
    /// Absolute signed URL. `None` when the API could not sign this path.
    pub url: Option<url::Url>,
    /// Error text for this path, for example when the object does not exist.
    pub error: Option<String>,
}
