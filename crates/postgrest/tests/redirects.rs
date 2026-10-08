#![expect(
    clippy::tests_outside_test_module,
    reason = "Integration tests exercise public interfaces."
)]
#![expect(
    clippy::unwrap_used,
    reason = "Test setup and assertions retain failure details."
)]

use rp_postgrest::{Postgrest, reqwest::StatusCode};

#[tokio::test]
async fn default_client_does_not_follow_cross_origin_redirect() {
    let mut origin = mockito::Server::new_async().await;
    let mut other = mockito::Server::new_async().await;
    let redirect = origin
        .mock("GET", "/items")
        .match_query(mockito::Matcher::Any)
        .with_status(302)
        .with_header("location", &format!("{}/steal", other.url()))
        .create_async()
        .await;
    let leaked = other.mock("GET", "/steal").expect(0).create_async().await;

    let response = Postgrest::new(origin.url())
        .unwrap()
        .insert_header("apikey", "anon-key")
        .unwrap()
        .from("items")
        .select("*")
        .execute()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FOUND);
    redirect.assert_async().await;
    leaked.assert_async().await;
}

#[tokio::test]
async fn default_client_follows_same_origin_redirect() {
    let mut server = mockito::Server::new_async().await;
    let redirect = server
        .mock("GET", "/items")
        .match_query(mockito::Matcher::Any)
        .with_status(302)
        .with_header("location", "/moved")
        .create_async()
        .await;
    let target = server
        .mock("GET", "/moved")
        .match_header("apikey", "anon-key")
        .with_body("[]")
        .create_async()
        .await;

    let response = Postgrest::new(server.url())
        .unwrap()
        .insert_header("apikey", "anon-key")
        .unwrap()
        .from("items")
        .select("*")
        .execute()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    redirect.assert_async().await;
    target.assert_async().await;
}
