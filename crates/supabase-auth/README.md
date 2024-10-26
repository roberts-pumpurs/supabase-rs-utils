# rp-supabase-auth

A Rust client library for interacting with Supabase’s Authentication API.

Features

- 	Comprehensive API Coverage: Supports all Supabase Auth endpoints.
- 	Typed Models: Provides strongly-typed request and response models.
- 	Asynchronous: Built on top of tokio and reqwest for async operations.
- 	JWT Refresh Stream: Automatically refreshes JWT tokens using a stream.

```rust
use rp_supabase_auth::auth_client::{new_authenticated_stream, requests};
use rp_supabase_auth::futures::StreamExt as _;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::LoginCredentials;
use rp_supabase_auth::url;
use futures::StreamExt;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = SupabaseAuthConfig {
        api_key: "your-supabase-api-key".to_string(),
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(3),
        url: "https://your-project.supabase.co".parse().unwrap(),
    };
    let login_credentials = LoginCredentials::builder()
        .email("user@example.com".to_string())
        .password("password".to_string())
        .build();

    let mut auth_client_stream = new_authenticated_stream(config, login_credentials).unwrap();
    while let Some(item) = auth_client_stream.next().await {
        tracing::debug!(?item, "new client?");
        let Ok(Ok(client)) = item else {
            continue;
        };
        let result = client
            .build_request(&requests::UserGetRequest)
            .unwrap()
            .execute()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        tracing::info!(data =? result, "user info");
    }
    tracing::error!("realtime connection exited");

}
```
