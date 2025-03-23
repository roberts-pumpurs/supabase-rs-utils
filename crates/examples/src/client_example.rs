use core::time::Duration;

use clap::Parser;
use rp_supabase_auth::futures::StreamExt as _;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::LoginCredentials;
use rp_supabase_auth::url;
use rp_supabase_client::{PostgerstResponse, new_authenticated};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    supabase_api_url: url::Url,

    #[arg(short, long)]
    annon_key: String,

    #[arg(short, long)]
    email: String,

    #[arg(short, long)]
    pass: String,

    #[arg(short, long)]
    table: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .from_env()
                .unwrap()
                .add_directive("supabase_auth=info".to_owned().parse().unwrap())
                .add_directive("client_example=debug".to_owned().parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    let config = SupabaseAuthConfig {
        api_key: args.annon_key,
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(3),
        url: args.supabase_api_url.clone(),
    };
    let login_credentials = LoginCredentials::builder()
        .email(args.email)
        .password(args.pass)
        .build();
    let mut client_stream = new_authenticated(config, login_credentials).unwrap();

    while let Some(client) = client_stream.next().await {
        tracing::debug!(?client, "new client");

        let Ok((client, token_response)) = client else {
            continue;
        };
        tracing::info!(?token_response, "token response");
        let res = client
            .from(args.table.clone())
            .select("*")
            .build()
            .send()
            .await
            .map(PostgerstResponse::<simd_json::OwnedValue>::new)
            .unwrap()
            .json()
            .await;

        tracing::info!(?res, "postgrest response");
    }
    tracing::error!("realtime connection exited");
}
