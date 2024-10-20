use core::time::Duration;

use clap::Parser;
use supabase_auth::futures::StreamExt as _;
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

    /// The supabase table to subscribe to
    #[arg(short, long)]
    table: String,

    /// The filter to apply on the table
    /// e.g. "id=eq.83a19c16-fcd8-45d0-9710-d7b06ce6f329"
    #[arg(short, long)]
    filter: Option<String>,
}

#[tokio::main]
async fn main() {
    use supabase_auth::SupabaseAuth;

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

    let config = supabase_auth::SupabaseAuthConfig {
        api_key: args.annon_key,
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(3),
        url: args.supabase_api_url.clone(),
    };
    let supabase_auth = SupabaseAuth::new(config);
    let mut token_refresh = supabase_auth
        .sign_in(supabase_auth::LoginCredentials {
            email: args.email,
            password: args.pass,
        })
        .unwrap();

    while let Some(msg) = token_refresh.next().await {
        tracing::debug!(?msg, "reading protocol message");
    }
    tracing::error!("realtime connection exited");
}
