use core::time::Duration;

use clap::Parser;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::LoginCredentials;
use rp_supabase_auth::url;
use rp_supabase_realtime::futures::StreamExt as _;
use rp_supabase_realtime::message::broadcast::Broadcast;
use rp_supabase_realtime::message::phx_join;
use rp_supabase_realtime::realtime;
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
                .add_directive("rp_supabase_auth=info".to_owned().parse().unwrap())
                .add_directive("rp_supabase_realtime=info".to_owned().parse().unwrap())
                .add_directive("examples=info".to_owned().parse().unwrap())
                .add_directive("broadcast_example=info".to_owned().parse().unwrap()),
        )
        .init();
    color_eyre::install().unwrap();

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
    let (mut realtime, mut client) = realtime::RealtimeConnection::new(config, "af")
        .connect(login_credentials)
        .await
        .unwrap();

    let payload = phx_join::PhxJoin {
        config: phx_join::JoinConfig {
            broadcast: phx_join::BroadcastConfig {
                self_item: true,
                ack: true,
            },
            presence: phx_join::PresenceConfig { key: String::new() },
            postgres_changes: vec![],
        },
        access_token: None,
    };
    client.subscribe_to_changes(payload).await.unwrap();
    client
        .broadcast(Broadcast {
            r#type: "broadcast".to_string(),
            event: "update".to_string(),
            payload: simd_json::json!({"aaa": "bbbb"}),
        })
        .await
        .unwrap();
    tracing::info!("pooling realtime connection");
    while let Some(msg) = realtime.next().await {
        tracing::info!(?msg, "message");
    }
    tracing::error!("realtime connection exited");
}
