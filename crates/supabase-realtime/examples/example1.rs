use std::borrow::Cow;
use std::time::Duration;

use clap::Parser;
use futures::StreamExt;
use supabase_auth::redact::Secret;
use supabase_realtime::message::{phx_join, PhoenixMessage, ProtocolMessage};
use tokio_stream::wrappers::ReceiverStream;
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
                .add_directive(format!("supabase_auth=info").parse().unwrap())
                .add_directive(format!("supabase_realtime=trace").parse().unwrap())
                .add_directive(format!("example1=info").parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    let supabase_auth = SupabaseAuth::new(
        args.supabase_api_url.clone(),
        args.annon_key,
        5,
        Duration::from_secs(3),
    );
    let token_refresh = supabase_auth
        .sign_in(supabase_auth::TokenBody {
            email: args.email,
            password: args.pass,
        })
        .unwrap();
    let (tx, rx) = tokio::sync::mpsc::channel(5);
    let mut realtime = supabase_realtime::realtime::RealtimeConnection::new(args.supabase_api_url)
        .connect(token_refresh, ReceiverStream::new(rx))
        .await
        .unwrap();

    let message_to_send = ProtocolMessage::PhxJoin(PhoenixMessage {
        topic: "realtime:table-db-changes".to_string(),
        payload: phx_join::PhxJoin {
            config: phx_join::JoinConfig {
                broadcast: phx_join::BroadcastConfig {
                    self_item: false,
                    ack: false,
                },
                presence: phx_join::PresenceConfig {
                    key: "".to_string(),
                },
                postgres_changes: vec![phx_join::PostgrsChanges {
                    event: phx_join::PostgresChangetEvent::All,
                    schema: "public".to_string(),
                    table: args.table,
                    filter: args.filter,
                }],
            },
            access_token: None,
        },
        ref_field: Some("1".to_string()),
        join_ref: Some("1".to_string()),
    });
    tx.send(message_to_send).await.unwrap();
    tracing::debug!("pooling realtime connection");
    while let Some(msg) = realtime.next().await {
        tracing::info!(?msg, "reading protocol message");
    }
    tracing::error!("realtime connection exited");
}
