use core::time::Duration;

use examples::get_supabase_credentials;
use rp_supabase_auth::auth_client::{new_authenticated_stream, requests};
use rp_supabase_auth::futures::StreamExt as _;
use rp_supabase_auth::jwt_stream::SupabaseAuthConfig;
use rp_supabase_auth::types::LoginCredentials;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .from_env()?
                .add_directive("rp_supabase_auth=debug".to_owned().parse()?)
                .add_directive("auth_example=debug".to_owned().parse()?),
        )
        .init();

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

    let mut auth_client_stream = new_authenticated_stream(config, login_credentials)?;
    while let Some(item) = auth_client_stream.next().await {
        tracing::debug!(?item, "new client?");
        let Ok(Ok(client)) = item else {
            continue;
        };
        let result = client
            .build_request(&requests::UserGetRequest)?
            .execute()
            .await?
            .json()
            .await?;
        tracing::info!(data =? result, "user info");
    }
    tracing::error!("realtime connection exited");

    eyre::bail!("unexpected exit")
}
