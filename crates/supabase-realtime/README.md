# rp-supabase-realtime

A Rust client library for interacting with Supabase’s Realtime API.

## Overview

rp-supabase-realtime is a Rust crate that enables you to connect to Supabase’s Realtime API using WebSockets. It handles authentication, connection management, and provides an easy-to-use interface for subscribing to real-time database changes such as inserts, updates, and deletes.

This crate is ideal for building applications that require real-time data synchronization, like live dashboards, notifications, chat applications, or any system that benefits from immediate data updates.

Features

- 	WebSocket Connection: Establishes a WebSocket connection to Supabase’s Realtime API.
- 	Authentication: Supports JWT authentication, handling token refreshes automatically.
- 	Subscriptions: Allows subscribing to specific tables, rows, or columns with optional filters.
- 	Real-time Events: Listens for INSERT, UPDATE, and DELETE events on your database tables.
- 	Async Support: Built with async/await syntax, powered by tokio and futures.
- 	Error Handling: Provides detailed error types for robust application development.

## Usage

Here’s a basic example demonstrating how to connect to Supabase Realtime API and subscribe to changes on a specific table.

```rust
use core::time::Duration;

use clap::Parser;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::LoginCredentials;
use rp_supabase_auth::url;
use rp_supabase_realtime::futures::StreamExt as _;
use rp_supabase_realtime::message::phx_join;
use rp_supabase_realtime::realtime;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    supabase_api_url: url::Url,

    #[arg(short, long)]
    anon_key: String,

    #[arg(short, long)]
    email: String,

    #[arg(short, long)]
    pass: String,

    /// The Supabase table to subscribe to
    #[arg(short, long)]
    table: String,

    /// The filter to apply on the table (e.g., "id=eq.some-uuid")
    #[arg(short, long)]
    filter: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .from_env()
                .unwrap()
                .add_directive("supabase_auth=info".parse().unwrap())
                .add_directive("supabase_realtime=info".parse().unwrap())
                .add_directive("examples=info".parse().unwrap())
                .add_directive("realtime_example=info".parse().unwrap()),
        )
        .init();
    color_eyre::install().unwrap();

    let args = Args::parse();

    let config = SupabaseAuthConfig {
        api_key: args.anon_key,
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(3),
        url: args.supabase_api_url.clone(),
    };
    let login_credentials = LoginCredentials::builder()
        .email(args.email)
        .password(args.pass)
        .build();
    let (mut realtime, mut client) = realtime::RealtimeConnection::new(config)
        .connect(login_credentials)
        .await
        .unwrap();

    let payload = phx_join::PhxJoin {
        config: phx_join::JoinConfig {
            broadcast: phx_join::BroadcastConfig {
                self_item: false,
                ack: false,
            },
            presence: phx_join::PresenceConfig { key: String::new() },
            postgres_changes: vec![phx_join::PostgrsChanges {
                event: phx_join::PostgresChangetEvent::All,
                schema: "public".to_owned(),
                table: args.table,
                filter: args.filter,
            }],
        },
        access_token: None,
    };
    client.subscribe_to_changes(payload).await.unwrap();
    tracing::info!("Polling realtime connection");
    while let Some(msg) = realtime.next().await {
        match msg {
            Ok(msg) => {
                use rp_supabase_realtime::message::ProtocolPayload::*;
                match msg.payload {
                    PostgresChanges(postgres_changes_payload) => {
                        let changes = postgres_changes_payload
                            .data
                            .parse_record::<simd_json::OwnedValue>()
                            .unwrap()
                            .parse_old_record::<simd_json::OwnedValue>()
                            .unwrap();

                        tracing::info!(?changes, "Received database change");
                    }
                    other_msg => {
                        tracing::debug!(?other_msg, "Received protocol message");
                    }
                }
            }
            Err(err) => {
                tracing::warn!(?err, "Realtime error");
            }
        }
    }
    tracing::error!("Realtime connection exited");
}
```
