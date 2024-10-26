# rp-supabase-client

A Rust client for interacting with Supabase’s PostgREST API using authenticated requests.

## Overview

rp-supabase-client simplifies making authenticated requests to Supabase’s PostgREST API. It handles authentication, token refresh, and provides a straightforward API for querying data.

Features

- 	Easy authentication with Supabase.
- 	Automatic token refresh using rp-supabase-auth.
- 	Simple methods for querying and manipulating data.

```rust
use std::time::Duration;
use clap::Parser;
use futures::StreamExt;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::LoginCredentials;
use rp_supabase_client::{new_authenticated, PostgrestResponse};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
struct Args {
    supabase_api_url: url::Url,
    anon_key: String,
    email: String,
    password: String,
    table: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let config = SupabaseAuthConfig {
        api_key: args.anon_key,
        url: args.supabase_api_url,
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(3),
    };

    let login_credentials = LoginCredentials::builder()
        .email(args.email)
        .password(args.password)
        .build();

    let mut client_stream = new_authenticated(config, login_credentials).unwrap();

    while let Some(client_result) = client_stream.next().await {
        if let Ok((client, _token_response)) = client_result {
            let res = client
                .from(&args.table)
                .select("*")
                .build()
                .send()
                .await
                .map(PostgrestResponse::new)
                .unwrap()
                .json::<serde_json::Value>()
                .await;

            println!("Response: {:?}", res);
        }
    }
}
```
