use core::time::Duration;

use clap::Parser;
use supabase_auth::url;
use supabase_realtime::futures::StreamExt as _;
use supabase_realtime::message::phx_join;
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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .from_env()
                .unwrap()
                .add_directive("supabase_auth=info".to_owned().parse().unwrap())
                .add_directive("supabase_realtime=info".to_owned().parse().unwrap())
                .add_directive("examples=info".to_owned().parse().unwrap())
                .add_directive("realtime_example=info".to_owned().parse().unwrap()),
        )
        .init();
    color_eyre::install().unwrap();

    let args = Args::parse();

    let config = supabase_auth::SupabaseAuthConfig {
        api_key: args.annon_key,
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(3),
        url: args.supabase_api_url.clone(),
    };
    let login_info = supabase_auth::LoginCredentials {
        email: args.email,
        password: args.pass,
    };
    let (mut realtime, mut client) = supabase_realtime::realtime::RealtimeConnection::new(config)
        .connect(login_info)
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
    tracing::info!("pooling realtime connection");
    while let Some(msg) = realtime.next().await {
        match msg {
            Ok(msg) => {
                use supabase_realtime::message::ProtocolPayload::*;
                match msg.payload {
                    PostgresChanges(postgres_changes_payload) => {
                        let changes = postgres_changes_payload
                            .data
                            .parse_record::<simd_json::OwnedValue>()
                            .unwrap()
                            .parse_old_record::<simd_json::OwnedValue>()
                            .unwrap();

                        tracing::info!(?changes, "reading protocol message");
                    }
                    msg => {
                        tracing::debug!(?msg, "reading protocol message");
                    }
                }
            }
            Err(err) => {
                tracing::warn!(?err, "realtime error");
            }
        }
    }
    tracing::error!("realtime connection exited");
}
