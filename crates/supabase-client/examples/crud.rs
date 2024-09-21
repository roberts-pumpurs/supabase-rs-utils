use std::borrow::Cow;

use clap::Parser;
use supabase_auth::redact::{self, Secret};
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
                .add_directive(format!("supabase-client=trace").parse().unwrap())
                .add_directive(format!("get=debug").parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    let client =
        supabase_client::SupabaseClient::new(args.supabase_api_url, args.annon_key).unwrap();
    get_data(&client).await;
    let client = client
        .sign_in_with_password(supabase_auth::TokenBody {
            email: Cow::Owned(args.email),
            password: redact::Secret::new(Cow::Owned(args.pass)),
        })
        .await
        .unwrap();
    get_data(&client).await;
}

async fn get_data<T: supabase_client::SupabaseClientExt>(client: &T) {
    // client.execute(query);
    // todo
}
