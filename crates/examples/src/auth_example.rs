use core::time::Duration;

use clap::Parser;
use supabase_auth::futures::StreamExt as _;
use supabase_auth::jwt_stream::SupabaseAuthConfig;
use supabase_auth::types::LoginCredentials;
use supabase_auth::url;
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
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .from_env()
                .unwrap()
                .add_directive("supabase_auth=debug".to_owned().parse().unwrap())
                .add_directive("auth_example=debug".to_owned().parse().unwrap()),
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

    let mut auth_client_stream =
        supabase_auth::auth_client::new_authenticated_stream(config, login_credentials).unwrap();
    while let Some(item) = auth_client_stream.next().await {
        tracing::debug!(?item, "new client?");
        let Ok(Ok(client)) = item else {
            continue;
        };
        let result = client
            .build_request(&supabase_auth::auth_client::requests::UserGetRequest)
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
