#![cfg(all(feature = "rest", feature = "storage"))]
#![expect(clippy::unwrap_used, reason = "tests fail loudly on setup errors")]
#![expect(clippy::tests_outside_test_module, reason = "integration test crate")]

use mockito::Matcher;
use supabase_rp::Client;

const KEY: &str = "sb_publishable_abc123";
const USER_TOKEN: &str = "user-jwt";

/// Storage goes to the storage server, REST to the project server, and the user token reaches
/// storage whether it is applied before or after the storage URL.
#[tokio::test]
async fn storage_url_overrides_only_storage() {
    let mut project = mockito::Server::new_async().await;
    let mut storage = mockito::Server::new_async().await;
    let rest_mock = project
        .mock("GET", Matcher::Regex("^/rest/v1/todos".to_owned()))
        .match_header("apikey", KEY)
        .with_status(200)
        .with_body("[]")
        .expect(2)
        .create_async()
        .await;
    let project_storage = project
        .mock("GET", "/storage/v1/bucket")
        .expect(0)
        .create_async()
        .await;
    let storage_mock = storage
        .mock("GET", "/storage/v1/bucket")
        .match_header("apikey", KEY)
        .match_header("authorization", format!("Bearer {USER_TOKEN}").as_str())
        .with_status(200)
        .with_body("[]")
        .expect(2)
        .create_async()
        .await;

    let client = Client::new(&format!("{}/", project.url()), KEY).unwrap();
    let storage_url = format!("{}/", storage.url());
    let token_first = client
        .with_access_token(USER_TOKEN)
        .unwrap()
        .with_storage_url(&storage_url)
        .unwrap();
    let url_first = client
        .with_storage_url(&storage_url)
        .unwrap()
        .with_access_token(USER_TOKEN)
        .unwrap();

    for user_client in [&token_first, &url_first] {
        user_client.storage().list_buckets().await.unwrap();
        drop(user_client.from("todos").select("id").execute().await);
    }

    rest_mock.assert_async().await;
    project_storage.assert_async().await;
    storage_mock.assert_async().await;
}

#[test]
fn storage_url_rejects_path() {
    let client = Client::new("https://api.example.com/", KEY).unwrap();
    let result = client.with_storage_url("https://abc.supabase.co/x");
    assert!(matches!(
        result,
        Err(supabase_rp::Error::InvalidProjectUrl(_))
    ));
}
