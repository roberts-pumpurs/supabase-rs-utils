#![cfg(all(
    feature = "rest",
    feature = "auth",
    feature = "storage",
    feature = "functions"
))]
#![expect(clippy::unwrap_used, reason = "tests fail loudly on setup errors")]
#![expect(clippy::tests_outside_test_module, reason = "integration test crate")]

use mockito::{Matcher, Mock, ServerGuard};
use rp_supabase::Client;

const LEGACY_KEY: &str = "eyJhbGciOiJIUzI1NiJ9.eyJyb2xlIjoiYW5vbiJ9.c2lnbmF0dXJl";
const PUBLISHABLE_KEY: &str = "sb_publishable_abc123";
const USER_TOKEN: &str = "user-jwt";

async fn expect_all(
    server: &mut ServerGuard,
    key: &str,
    rest_auth: Matcher,
    gateway: Matcher,
) -> Vec<Mock> {
    let mut mocks = Vec::new();
    for (method, path, bearer) in [
        ("GET", "/rest/v1/todos", rest_auth.clone()),
        ("GET", "/auth/v1/user", rest_auth),
        ("GET", "/storage/v1/bucket", gateway.clone()),
        ("POST", "/functions/v1/hello", gateway),
    ] {
        mocks.push(
            server
                .mock(method, Matcher::Regex(format!("^{path}")))
                .match_header("apikey", key)
                .match_header("authorization", bearer)
                .with_status(200)
                .with_body("[]")
                .expect(1)
                .create_async()
                .await,
        );
    }
    mocks
}

async fn call_all(client: &Client) {
    // Only the request headers matter; response decoding is out of scope.
    drop(client.from("todos").select("id").execute().await);
    drop(client.auth().get_user().await);
    drop(client.storage().list_buckets().await);
    drop(client.functions().invoke("hello").send().await);
}

async fn check(key: &str, anonymous_gateway_bearer: Matcher) {
    let mut server = mockito::Server::new_async().await;
    let client = Client::new(&format!("{}/", server.url()), key).unwrap();

    let mocks = expect_all(&mut server, key, Matcher::Missing, anonymous_gateway_bearer).await;
    call_all(&client).await;
    for mock in mocks {
        mock.assert_async().await;
        mock.remove_async().await;
    }

    let bearer = Matcher::Exact(format!("Bearer {USER_TOKEN}"));
    let mocks = expect_all(&mut server, key, bearer.clone(), bearer).await;
    call_all(&client.with_access_token(USER_TOKEN).unwrap()).await;
    for mock in mocks {
        mock.assert_async().await;
    }
}

#[tokio::test]
async fn legacy_key_is_bearer_until_user_token() {
    check(LEGACY_KEY, Matcher::Exact(format!("Bearer {LEGACY_KEY}"))).await;
}

#[tokio::test]
async fn publishable_key_is_never_bearer() {
    check(PUBLISHABLE_KEY, Matcher::Missing).await;
}
