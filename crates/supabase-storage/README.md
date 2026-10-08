# rp-supabase-storage

Async Rust client for the [Supabase Storage](https://supabase.com/docs/guides/storage) API.
It manages buckets, uploads and downloads objects, and creates signed and public URLs.

## Install

```toml
[dependencies]
rp-supabase-storage = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quickstart

```rust,no_run
use rp_supabase_storage::url::Url;
use rp_supabase_storage::{BucketOptions, FileOptions, ListOptions, StorageClient};

# async fn run() -> Result<(), rp_supabase_storage::StorageError> {
let project = Url::parse("https://abc.supabase.co/")?;
let storage = StorageClient::new(&project, "service-role-key")?;

// Create a private bucket that accepts PNG images up to 1 MiB.
let options = BucketOptions {
    public: false,
    file_size_limit: Some(1024 * 1024),
    allowed_mime_types: Some(vec!["image/png".to_owned()]),
};
storage.create_bucket("avatars", &options).await?;

let avatars = storage.from("avatars");

// Upload an object.
let file = FileOptions {
    content_type: Some("image/png".to_owned()),
    cache_control: 3600,
    upsert: true,
};
avatars.upload("users/1.png", vec![0_u8; 16], &file).await?;

// Share it for 60 seconds.
let link = avatars.create_signed_url("users/1.png", 60).await?;
println!("{link}");

// Download, list, and remove.
let bytes = avatars.download("users/1.png").await?;
let entries = avatars.list("users", &ListOptions::default()).await?;
avatars.remove(&["users/1.png"]).await?;
# drop((bytes, entries));
# Ok(())
# }
```

Use `StorageClient::new_with_client` to reuse an existing `reqwest::Client` and its connection pool.

## Auth model

Every request sends the `apikey` header. The `Authorization` header depends on the key and token:

- `StorageClient::new` with a legacy JWT key (anon or service role) also sends
  `Authorization: Bearer <api_key>`. With the service role key, the client bypasses row level
  security (RLS). Use it only on a server.
- `StorageClient::new` with a new-format key (`sb_publishable_...` or `sb_secret_...`) sends no
  `Authorization` header. These keys are not JWTs; the gateway checks the `apikey` header.
- `with_access_token(user_jwt)` returns a client that sends the user's JWT as the bearer token.
  Storage then applies the RLS policies on `storage.objects` and `storage.buckets` for that user.
  Use the anon key plus a user token in code that acts for a user.

```rust,no_run
# use rp_supabase_storage::{url::Url, StorageClient};
# fn run(user_jwt: &str) -> Result<(), rp_supabase_storage::StorageError> {
let storage = StorageClient::new(&Url::parse("https://abc.supabase.co/")?, "anon-key")?;
let as_user = storage.with_access_token(user_jwt)?;
# drop(as_user);
# Ok(())
# }
```

`public_url` sends no request. The URL works only when the bucket is public. For private
buckets, call `create_signed_url` or `create_signed_urls`. Both use the batch signing endpoint,
because the single-object endpoint returns an invalid signature for keys with `?`. The API
returns signed URLs relative to `/storage/v1` with the object key unescaped; this crate rebuilds
each URL from the bucket and the encoded object path, then appends the token. Keys with `?` and
spaces work. The Storage server itself rejects some characters in keys, for example `#`.

`BucketOptions` fields are always sent. On `update_bucket`, `None` removes an existing file size
limit or MIME type list. `FileOptions::default()` sends `Cache-Control: max-age=3600`.

## Object paths

Separate path segments with `/`, for example `users/1.png`. The client percent-encodes each
segment once. It rejects empty paths, empty segments (`a//b`, leading or trailing `/`), `.` and
`..`, tabs, carriage returns, and line feeds with `StorageError::InvalidPath` before it sends a
request.

## Errors

All operations return `StorageError`:

- `Api { status, body }`: the server answered with a non-success status. `body` is
  `ApiErrorBody::Storage` with `status_code`, `error`, and `message` when the server sent the
  standard Storage error JSON, else `ApiErrorBody::Raw` with the body text.
- `InvalidPath`: a bucket id or object path is not valid.
- `MissingSignedToken`: a signed URL from the server has no `?token=` query.
- `SignFailed { path, message }`: `create_signed_url` got no URL for the object, for example
  because it does not exist or the caller cannot read it.
- `Http`, `Json`, `Url`, `InvalidHeader`, `InvalidBaseUrl`: transport, decoding, and input errors.

```rust,no_run
# use rp_supabase_storage::{ApiErrorBody, StorageClient, StorageError, url::Url};
# async fn run(storage: StorageClient) {
match storage.get_bucket("missing").await {
    Err(StorageError::Api { status, body: ApiErrorBody::Storage(body) }) => {
        eprintln!("{status}: {}", body.message);
    }
    other => drop(other),
}
# }
```

## Limits

- No resumable (TUS) uploads. Each upload sends the whole body in one request.
- No image transformations and no S3 protocol support.
- Bucket and object operations only; no analytics or vector buckets.
