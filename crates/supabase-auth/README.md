# rp-supabase-auth

Typed asynchronous requests to Supabase Auth and a JWT refresh stream.

```toml
[dependencies]
rp-supabase-auth = "0.8"
```

## Configured HTTP transport

Supply a `reqwest::Client` when your application needs its own timeout, proxy, TLS policy, default headers, or connection pool. The supplied client is reused for login, refresh, and every auth client emitted by the stream.

```rust,no_run
use rp_supabase_auth::auth_client::new_authenticated_stream_with_client;
use rp_supabase_auth::futures::StreamExt as _;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::LoginCredentials;
use std::time::Duration;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let http = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()?;
let config = SupabaseAuthConfig {
    api_key: "your-supabase-api-key".to_owned(),
    max_reconnect_attempts: 5,
    reconnect_interval: Duration::from_secs(3),
    url: "https://your-project.supabase.co".parse()?,
};
let credentials = LoginCredentials::builder()
    .email("user@example.com".to_owned())
    .password("password".to_owned())
    .build();
let mut clients = new_authenticated_stream_with_client(config, credentials, http)?;
while let Some(item) = clients.next().await {
    match item {
        Ok(Ok(client)) => {
            let response = client
                .build_request(&rp_supabase_auth::auth_client::requests::UserGetRequest)?
                .execute()
                .await?
                .json()
                .await?;
            tracing::debug!(?response, "user response");
        }
        Ok(Err(error)) => tracing::warn!(?error, "auth client configuration failed"),
        Err(error) => tracing::warn!(?error, "token refresh failed"),
    }
}
# Ok(())
# }
```

Add a direct `reqwest = { version = "0.13", default-features = false, features = ["rustls"] }` dependency for this example. Select the TLS features your application needs.

`JwtStream::sign_in_with_client` accepts the same configured transport for a token-only stream. `ApiClient::new_authenticated_with_client` and `new_unauthenticated_with_client` accept it for individual auth clients. The constructors without `_with_client` build a default transport as a convenience.

API-key, JSON content-type, and bearer headers apply to each request. They do not mutate the shared client's defaults. Caller default headers coexist with required protocol headers, and successive tokens do not change another client's bearer header. These constructors do not rebuild the supplied client or implicitly override its compression policy.

For REST clients, `rp-supabase-client` 0.8 provides `anonymous_client_with_client` and `new_authenticated_with_client`. The authenticated stream reuses the supplied transport for login, refresh, and emitted REST clients. Auth request errors keep this crate's auth semantics; REST execution uses the owned `rp-postgrest` 3.0 canonical error interface. The HTTP client is a constructor argument, not a field in `SupabaseAuthConfig`.
