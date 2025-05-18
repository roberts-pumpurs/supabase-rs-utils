use core::time::Duration;

use examples::get_supabase_credentials;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::LoginCredentials;
use rp_supabase_realtime::futures::StreamExt as _;
use rp_supabase_realtime::message::phx_join;
use rp_supabase_realtime::realtime::{self, DbUpdates};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .from_env()?
                .add_directive("rp_supabase_auth=info".to_owned().parse()?)
                .add_directive("rp_supabase_realtime=info".to_owned().parse()?)
                .add_directive("examples=info".to_owned().parse()?)
                .add_directive("db_updates_example=info".to_owned().parse()?),
        )
        .init();
    color_eyre::install()?;

    let credentials = get_supabase_credentials()?;

    let config = SupabaseAuthConfig {
        api_key: credentials.anon_key,
        max_reconnect_attempts: 5,
        reconnect_interval: Duration::from_secs(3),
        url: credentials.supabase_api_url,
    };
    let login_credentials = LoginCredentials::builder()
        .email(credentials.email)
        .password(credentials.password)
        .build();
    let (mut realtime, mut client) =
        realtime::RealtimeConnection::<DbUpdates>::channel_db_changes(config)
            .connect(login_credentials)
            .await?;

    client
        .subscribe_to_changes(vec![phx_join::PostgrsChanges {
            event: phx_join::PostgresChangetEvent::All,
            schema: "public".to_owned(),
            table: "messages".to_owned(),
            filter: None,
        }])
        .await?;
    tracing::info!("pooling realtime connection");
    while let Some(msg) = realtime.next().await {
        match msg {
            Ok(msg) => {
                use rp_supabase_realtime::message::ProtocolPayload::{
                    AccessToken, Broadcast, Heartbeat, PhxClose, PhxError, PhxJoin, PhxReply,
                    PostgresChanges, PresenceDiff, PresenceInner, PresenceState, System,
                };
                match msg.payload {
                    PostgresChanges(postgres_changes_payload) => {
                        let changes = postgres_changes_payload
                            .data
                            .parse_record::<simd_json::OwnedValue>()?
                            .parse_old_record::<simd_json::OwnedValue>()?;

                        tracing::info!(?changes, "reading protocol message");
                    }
                    msg @ (Heartbeat(_) | AccessToken(_) | PhxJoin(_) | PhxClose(_)
                    | PhxReply(_) | Broadcast(_) | PresenceInner(_) | PresenceState(_)
                    | PresenceDiff(_) | System(_) | PhxError(_)) => {
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

    eyre::bail!("should not have exited")
}
