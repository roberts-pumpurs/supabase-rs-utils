mod authenticated;
mod unauthenticated;

use std::future::Future;

pub use authenticated::AuthenticatedSupabaseClient;
use reqwest::header;
use supabase_auth::SUPABASE_KEY;
pub use unauthenticated::SupabaseClient;

use crate::SupabaseClientError;

pub trait SupabaseClientExt {
    fn client(&mut self) -> impl Future<Output = reqwest::Client> + Send;
}

impl SupabaseClientExt for AuthenticatedSupabaseClient {
    async fn client(&mut self) -> reqwest::Client {
        let client = self.client.read().await;
        client.clone()
    }
}

impl SupabaseClientExt for SupabaseClient {
    async fn client(&mut self) -> reqwest::Client {
        self.client.clone()
    }
}

pub(crate) fn construct_client(
    api_key: &str,
    bearer_token: &str,
) -> Result<reqwest::Client, SupabaseClientError> {
    let builder = reqwest::Client::builder();
    let mut headers = header::HeaderMap::new();
    headers.insert(SUPABASE_KEY, header::HeaderValue::from_str(api_key)?);
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(format!("Bearer {bearer_token}").as_str())?,
    );
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    let client = builder.default_headers(headers).build()?;
    Ok(client)
}
