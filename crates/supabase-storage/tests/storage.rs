#![expect(
    clippy::tests_outside_test_module,
    reason = "This integration test crate is compiled only for tests"
)]
#![expect(
    clippy::unwrap_used,
    clippy::get_unwrap,
    clippy::panic,
    clippy::default_numeric_fallback,
    reason = "Test fixtures must succeed; JSON literals use untyped numbers"
)]

use mockito::{Matcher, Server, ServerGuard};
use rp_supabase_storage::reqwest::StatusCode;
use rp_supabase_storage::url::Url;
use rp_supabase_storage::{
    ApiErrorBody, BucketOptions, FileOptions, ListOptions, PathError, SortBy, SortColumn,
    SortOrder, StorageClient, StorageError, StorageErrorBody,
};
use serde_json::json;

const KEY: &str = "service-key";

async fn setup() -> (ServerGuard, StorageClient) {
    let server = Server::new_async().await;
    let project = Url::parse(&format!("{}/", server.url())).unwrap();
    let client = StorageClient::new(&project, KEY).unwrap();
    (server, client)
}

fn authed(mock: mockito::Mock, token: &str) -> mockito::Mock {
    mock.match_header("apikey", KEY)
        .match_header("authorization", format!("Bearer {token}").as_str())
}

#[tokio::test]
async fn list_buckets_sends_key_headers_and_decodes() {
    let (mut server, client) = setup().await;
    let mock = authed(server.mock("GET", "/storage/v1/bucket"), KEY)
        .with_body(
            json!([{
                "id": "avatars", "name": "avatars", "owner": "", "public": true,
                "file_size_limit": 1024, "allowed_mime_types": ["image/png"],
                "created_at": "2024-01-01T00:00:00.000Z", "updated_at": "2024-01-02T00:00:00.000Z"
            }])
            .to_string(),
        )
        .create_async()
        .await;
    let buckets = client.list_buckets().await.unwrap();
    mock.assert_async().await;
    assert_eq!(buckets.len(), 1);
    let bucket = buckets.first().unwrap();
    assert_eq!(bucket.id, "avatars");
    assert!(bucket.public);
    assert_eq!(bucket.file_size_limit, Some(1024));
    assert_eq!(
        bucket.allowed_mime_types,
        Some(vec!["image/png".to_owned()])
    );
}

#[tokio::test]
async fn with_access_token_swaps_bearer_only() {
    let (mut server, client) = setup().await;
    let mock = authed(server.mock("DELETE", "/storage/v1/bucket/b"), "user-jwt")
        .with_body(r#"{"message":"Successfully deleted"}"#)
        .create_async()
        .await;
    client
        .with_access_token("user-jwt")
        .unwrap()
        .delete_bucket("b")
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn new_format_keys_send_no_default_bearer() {
    let mut server = Server::new_async().await;
    let project = Url::parse(&format!("{}/", server.url())).unwrap();
    for key in ["sb_publishable_abc", "sb_secret_abc"] {
        let client = StorageClient::new(&project, key).unwrap();
        let anonymous = server
            .mock("GET", "/storage/v1/bucket")
            .match_header("apikey", key)
            .match_header("authorization", Matcher::Missing)
            .with_body("[]")
            .create_async()
            .await;
        client.list_buckets().await.unwrap();
        anonymous.assert_async().await;
        anonymous.remove_async().await;
        let as_user = server
            .mock("GET", "/storage/v1/bucket")
            .match_header("apikey", key)
            .match_header("authorization", "Bearer user-jwt")
            .with_body("[]")
            .create_async()
            .await;
        client
            .with_access_token("user-jwt")
            .unwrap()
            .list_buckets()
            .await
            .unwrap();
        as_user.assert_async().await;
        as_user.remove_async().await;
    }
}

#[tokio::test]
async fn bucket_crud_requests() {
    let (mut server, client) = setup().await;
    let options = BucketOptions {
        public: true,
        file_size_limit: Some(10),
        allowed_mime_types: Some(vec!["text/plain".to_owned()]),
    };
    let expected = json!({
        "id": "docs", "name": "docs", "public": true,
        "file_size_limit": 10, "allowed_mime_types": ["text/plain"]
    });
    let create = server
        .mock("POST", "/storage/v1/bucket")
        .match_body(Matcher::Json(expected.clone()))
        .with_body(r#"{"name":"docs"}"#)
        .create_async()
        .await;
    let update = server
        .mock("PUT", "/storage/v1/bucket/docs")
        .match_body(Matcher::Json(expected))
        .with_body(r#"{"message":"ok"}"#)
        .create_async()
        .await;
    let empty = server
        .mock("POST", "/storage/v1/bucket/docs/empty")
        .match_body(Matcher::Json(json!({})))
        .with_body(r#"{"message":"ok"}"#)
        .create_async()
        .await;
    let get = server
        .mock("GET", "/storage/v1/bucket/docs")
        .with_body(
            json!({
                "id": "docs", "name": "docs", "public": false,
                "created_at": "2024-01-01T00:00:00Z", "updated_at": "2024-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .create_async()
        .await;
    client.create_bucket("docs", &options).await.unwrap();
    client.update_bucket("docs", &options).await.unwrap();
    client.empty_bucket("docs").await.unwrap();
    let info = client.get_bucket("docs").await.unwrap();
    assert_eq!(info.owner, None);
    assert_eq!(info.file_size_limit, None);
    create.assert_async().await;
    update.assert_async().await;
    empty.assert_async().await;
    get.assert_async().await;
}

#[tokio::test]
async fn bucket_options_none_is_sent_as_null() {
    let (mut server, client) = setup().await;
    let update = server
        .mock("PUT", "/storage/v1/bucket/docs")
        .match_body(Matcher::Json(json!({
            "id": "docs", "name": "docs", "public": false,
            "file_size_limit": null, "allowed_mime_types": null
        })))
        .with_body(r#"{"message":"ok"}"#)
        .create_async()
        .await;
    client
        .update_bucket("docs", &BucketOptions::default())
        .await
        .unwrap();
    update.assert_async().await;
}

#[tokio::test]
async fn upload_sends_options_as_headers_and_encodes_segments() {
    let (mut server, client) = setup().await;
    let mock = authed(
        server.mock("POST", "/storage/v1/object/my%20bucket/dir/a%20b%3F%23.txt"),
        KEY,
    )
    .match_header("content-type", "text/plain")
    .match_header("cache-control", "max-age=60")
    .match_header("x-upsert", "true")
    .match_body("hello")
    .with_body(r#"{"Id":"uuid-1","Key":"my bucket/dir/a b?#.txt"}"#)
    .create_async()
    .await;
    let options = FileOptions {
        content_type: Some("text/plain".to_owned()),
        cache_control: 60,
        upsert: true,
    };
    let key = client
        .from("my bucket")
        .upload("dir/a b?#.txt", "hello", &options)
        .await
        .unwrap();
    mock.assert_async().await;
    assert_eq!(key.id.as_deref(), Some("uuid-1"));
    assert_eq!(key.key, "my bucket/dir/a b?#.txt");
}

#[tokio::test]
async fn update_uses_put_and_default_cache_control() {
    let (mut server, client) = setup().await;
    let mock = server
        .mock("PUT", "/storage/v1/object/b/f.bin")
        .match_header("x-upsert", "false")
        .match_header("cache-control", "max-age=3600")
        .match_header("content-type", Matcher::Missing)
        .with_body(r#"{"Id":"i","Key":"b/f.bin"}"#)
        .create_async()
        .await;
    client
        .from("b")
        .update("f.bin", vec![1_u8, 2], &FileOptions::default())
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn download_returns_bytes() {
    let (mut server, client) = setup().await;
    let mock = authed(server.mock("GET", "/storage/v1/object/b/x/y.bin"), KEY)
        .with_body([0_u8, 159, 146, 150])
        .create_async()
        .await;
    let bytes = client.from("b").download("x/y.bin").await.unwrap();
    mock.assert_async().await;
    assert_eq!(bytes.as_ref(), &[0_u8, 159, 146, 150]);
}

#[tokio::test]
async fn list_sends_prefix_and_options() {
    let (mut server, client) = setup().await;
    let mock = server
        .mock("POST", "/storage/v1/object/list/b")
        .match_body(Matcher::Json(json!({
            "prefix": "dir", "limit": 5, "offset": 2,
            "sortBy": {"column": "created_at", "order": "desc"}, "search": "a"
        })))
        .with_body(
            json!([
                {"name": "sub", "id": null, "updated_at": null, "created_at": null,
                 "last_accessed_at": null, "metadata": null},
                {"name": "a.txt", "id": "1", "updated_at": "2024-01-01T00:00:00.000Z",
                 "created_at": "2024-01-01T00:00:00.000Z", "last_accessed_at": "2024-01-01T00:00:00.000Z",
                 "metadata": {"size": 3, "mimetype": "text/plain"}}
            ])
            .to_string(),
        )
        .create_async()
        .await;
    let options = ListOptions {
        limit: Some(5),
        offset: Some(2),
        sort_by: Some(SortBy {
            column: SortColumn::CreatedAt,
            order: SortOrder::Desc,
        }),
        search: Some("a".to_owned()),
    };
    let entries = client.from("b").list("dir", &options).await.unwrap();
    mock.assert_async().await;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries.first().unwrap().id, None);
    let file = entries.get(1).unwrap();
    assert_eq!(file.metadata.as_ref().unwrap().get("size"), Some(&json!(3)));
}

#[tokio::test]
async fn list_default_options_send_only_prefix() {
    let (mut server, client) = setup().await;
    let mock = server
        .mock("POST", "/storage/v1/object/list/b")
        .match_body(Matcher::Json(json!({"prefix": ""})))
        .with_body("[]")
        .create_async()
        .await;
    let entries = client
        .from("b")
        .list("", &ListOptions::default())
        .await
        .unwrap();
    mock.assert_async().await;
    assert!(entries.is_empty());
}

#[tokio::test]
async fn remove_move_copy_requests() {
    let (mut server, client) = setup().await;
    let remove = server
        .mock("DELETE", "/storage/v1/object/b")
        .match_body(Matcher::Json(json!({"prefixes": ["a.txt", "d/b.txt"]})))
        .with_body(r#"[{"name":"a.txt","bucket_id":"b"}]"#)
        .create_async()
        .await;
    let transfer = json!({"bucketId": "b", "sourceKey": "a.txt", "destinationKey": "c.txt"});
    let moved = server
        .mock("POST", "/storage/v1/object/move")
        .match_body(Matcher::Json(transfer.clone()))
        .with_body(r#"{"message":"Successfully moved"}"#)
        .create_async()
        .await;
    let copied = server
        .mock("POST", "/storage/v1/object/copy")
        .match_body(Matcher::Json(transfer))
        .with_body(r#"{"Key":"b/c.txt"}"#)
        .create_async()
        .await;
    let bucket = client.from("b");
    let removed = bucket.remove(&["a.txt", "d/b.txt"]).await.unwrap();
    assert_eq!(removed.first().unwrap().bucket_id.as_deref(), Some("b"));
    bucket.move_object("a.txt", "c.txt").await.unwrap();
    let key = bucket.copy_object("a.txt", "c.txt").await.unwrap();
    assert_eq!(key.key, "b/c.txt");
    assert_eq!(key.id, None);
    remove.assert_async().await;
    moved.assert_async().await;
    copied.assert_async().await;
}

#[tokio::test]
async fn signed_url_is_absolute() {
    let (mut server, client) = setup().await;
    let mock = server
        .mock("POST", "/storage/v1/object/sign/b/d/a%20b.png")
        .match_body(Matcher::Json(json!({"expiresIn": 60})))
        .with_body(r#"{"signedURL":"/object/sign/b/d/a%20b.png?token=t1"}"#)
        .create_async()
        .await;
    let url = client
        .from("b")
        .create_signed_url("d/a b.png", 60)
        .await
        .unwrap();
    mock.assert_async().await;
    assert_eq!(
        url.as_str(),
        format!(
            "{}/storage/v1/object/sign/b/d/a%20b.png?token=t1",
            server.url()
        )
    );
}

#[tokio::test]
async fn signed_urls_keep_per_path_errors() {
    let (mut server, client) = setup().await;
    let mock = server
        .mock("POST", "/storage/v1/object/sign/b")
        .match_body(Matcher::Json(json!({"expiresIn": 30, "paths": ["a", "missing"]})))
        .with_body(
            json!([
                {"path": "a", "signedURL": "/object/sign/b/a?token=t", "error": null},
                {"path": "missing", "signedURL": null, "error": "Either the object does not exist or you do not have access to it"}
            ])
            .to_string(),
        )
        .create_async()
        .await;
    let urls = client
        .from("b")
        .create_signed_urls(&["a", "missing"], 30)
        .await
        .unwrap();
    mock.assert_async().await;
    let ok = urls.first().unwrap();
    assert_eq!(
        ok.url.as_ref().unwrap().as_str(),
        format!("{}/storage/v1/object/sign/b/a?token=t", server.url())
    );
    let missing = urls.get(1).unwrap();
    assert_eq!(missing.url, None);
    assert!(missing.error.is_some());
}

#[tokio::test]
async fn signed_urls_with_reserved_characters_use_encoded_path() {
    let (mut server, client) = setup().await;
    let batch = server
        .mock("POST", "/storage/v1/object/sign/b")
        .with_body(
            json!([
                {"path": "dir/q?.txt", "signedURL": "/object/sign/b/dir/q?.txt?token=t.q", "error": null},
                {"path": "h#1.txt", "signedURL": "/object/sign/b/h#1.txt?token=t.h", "error": null}
            ])
            .to_string(),
        )
        .create_async()
        .await;
    let single = server
        .mock("POST", "/storage/v1/object/sign/b/dir/q%3F.txt")
        .with_body(r#"{"signedURL":"/object/sign/b/dir/q?.txt?token=t.s"}"#)
        .create_async()
        .await;
    let bucket = client.from("b");
    let urls = bucket
        .create_signed_urls(&["dir/q?.txt", "h#1.txt"], 30)
        .await
        .unwrap();
    let url = bucket.create_signed_url("dir/q?.txt", 30).await.unwrap();
    batch.assert_async().await;
    single.assert_async().await;
    let base = format!("{}/storage/v1/object/sign/b", server.url());
    let got: Vec<_> = urls
        .iter()
        .map(|entry| entry.url.as_ref().unwrap().as_str())
        .collect();
    assert_eq!(
        got,
        [
            format!("{base}/dir/q%3F.txt?token=t.q"),
            format!("{base}/h%231.txt?token=t.h"),
        ]
    );
    assert_eq!(url.as_str(), format!("{base}/dir/q%3F.txt?token=t.s"));
}

#[tokio::test]
async fn signed_url_without_token_is_an_error() {
    let (mut server, client) = setup().await;
    let _mock = server
        .mock("POST", "/storage/v1/object/sign/b/a")
        .with_body(r#"{"signedURL":"/object/sign/b/a"}"#)
        .create_async()
        .await;
    let error = client
        .from("b")
        .create_signed_url("a", 30)
        .await
        .unwrap_err();
    assert!(
        matches!(error, StorageError::MissingSignedToken(_)),
        "{error:?}"
    );
}

#[tokio::test]
async fn public_url_needs_no_request() {
    let (server, client) = setup().await;
    let url = client.from("pub").public_url("img/a b.png").unwrap();
    assert_eq!(
        url.as_str(),
        format!(
            "{}/storage/v1/object/public/pub/img/a%20b.png",
            server.url()
        )
    );
}

#[tokio::test]
async fn invalid_paths_are_rejected_before_sending() {
    let (_server, client) = setup().await;
    let bucket = client.from("b");
    for (path, reason) in [
        ("", PathError::Empty),
        ("a//b", PathError::EmptySegment),
        ("/a", PathError::EmptySegment),
        ("a/", PathError::EmptySegment),
        ("a/./b", PathError::DotSegment),
        ("../a", PathError::DotSegment),
        ("a\tb", PathError::ControlCharacter),
        ("a/b\r", PathError::ControlCharacter),
        ("a\nb", PathError::ControlCharacter),
    ] {
        let error = bucket.public_url(path).unwrap_err();
        assert!(
            matches!(&error, StorageError::InvalidPath { path: rejected, reason: got } if rejected == path && *got == reason),
            "{path:?}: {error:?}"
        );
    }
    let error = bucket.remove(&["ok", ".."]).await.unwrap_err();
    assert!(matches!(
        error,
        StorageError::InvalidPath {
            reason: PathError::DotSegment,
            ..
        }
    ));
    let error = client.get_bucket("").await.unwrap_err();
    assert!(matches!(
        error,
        StorageError::InvalidPath {
            reason: PathError::Empty,
            ..
        }
    ));
}

#[tokio::test]
async fn api_error_json_is_decoded() {
    let (mut server, client) = setup().await;
    let mock = server
        .mock("GET", "/storage/v1/bucket/missing")
        .with_status(400)
        .with_body(
            r#"{"statusCode":"404","error":"Bucket not found","message":"Bucket not found"}"#,
        )
        .create_async()
        .await;
    let error = client.get_bucket("missing").await.unwrap_err();
    mock.assert_async().await;
    let StorageError::Api { status, body } = error else {
        panic!("unexpected error: {error:?}");
    };
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        ApiErrorBody::Storage(StorageErrorBody {
            status_code: Some("404".to_owned()),
            error: Some("Bucket not found".to_owned()),
            message: "Bucket not found".to_owned(),
        })
    );
}

#[tokio::test]
async fn api_error_non_json_keeps_text() {
    let (mut server, client) = setup().await;
    let mock = server
        .mock("GET", "/storage/v1/object/b/f")
        .with_status(502)
        .with_body("Bad Gateway")
        .create_async()
        .await;
    let error = client.from("b").download("f").await.unwrap_err();
    mock.assert_async().await;
    let StorageError::Api { status, body } = error else {
        panic!("unexpected error: {error:?}");
    };
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body, ApiErrorBody::Raw("Bad Gateway".to_owned()));
}
