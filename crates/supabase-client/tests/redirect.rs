#![cfg(feature = "client")]
#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public interfaces."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Test setup and assertions retain failure details."
)]

use mockito::Matcher;
use rp_supabase_client::anonymous_client;
use rp_supabase_client::rp_supabase_auth::url::Url;

#[tokio::test]
async fn default_client_does_not_follow_cross_origin_redirect() {
    let mut origin = mockito::Server::new_async().await;
    let mut other = mockito::Server::new_async().await;
    let target = other
        .mock("GET", Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let redirect = origin
        .mock("GET", "/rest/v1/todos?select=id")
        .with_status(302)
        .with_header("location", &format!("{}/rest/v1/todos", other.url()))
        .expect(1)
        .create_async()
        .await;
    let url = Url::parse(&format!("{}/", origin.url())).unwrap();
    let client = anonymous_client("anon-key".to_owned(), &url).unwrap();

    drop(client.from("todos").select("id").execute().await);

    redirect.assert_async().await;
    target.assert_async().await;
}
