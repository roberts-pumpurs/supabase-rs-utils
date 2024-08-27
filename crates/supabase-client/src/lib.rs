use std::borrow::{Borrow, Cow};
use std::ops::Deref;
use std::sync::Arc;

pub use error::SupabaseClientError;
use reqwest::{header, Upgraded};
use supabase_auth::futures::StreamExt;
use supabase_auth::{redact, AuthResponse, RefreshStream, TokenBody, SUPABASE_KEY};
use tokio::sync::{mpsc, RwLock};
use tokio::task::{JoinHandle, JoinSet};
mod error;

pub struct SupabaseClient {
    supabase_url: url::Url,
    client: reqwest::Client,
    annon_key: String,
}

pub struct AuthenticatedSupabaseClient {
    supabase_url: url::Url,
    annon_key_box: Box<String>,
    token_refresh: JoinHandle<()>,
    client: Arc<RwLock<reqwest::Client>>,
}

impl SupabaseClient {
    pub fn new(supabase_url: url::Url, annon_key: String) -> Result<Self, SupabaseClientError> {
        let client = construct_client(&annon_key, &annon_key)?;
        Ok(Self {
            supabase_url,
            client,
            annon_key,
        })
    }

    pub async fn sign_in_with_password(
        self,
        token_body: TokenBody<'static>,
    ) -> Result<AuthenticatedSupabaseClient, SupabaseClientError> {
        let auth =
            supabase_auth::SupabaseAuth::new(self.supabase_url.clone(), self.annon_key.clone());
        let mut auth = auth.sign_in(token_body).unwrap();
        let auth_resp = auth.next().await.unwrap().unwrap();
        let client = construct_client(&self.annon_key, &auth_resp.access_token).unwrap();
        let client = Arc::new(RwLock::new(client));

        let annon_key_box = Box::new(self.annon_key);
        let annon_key: &'static str = Box::leak(annon_key_box.clone());

        let handle = tokio::spawn({
            let client = Arc::clone(&client);

            async move {
                let mut update_handle = tokio::task::spawn(async {});
                while let Some(Ok(auth_resp)) = auth.next().await {
                    // cancel the previous update if necessary
                    if !update_handle.is_finished() {
                        update_handle.abort();
                    }
                    // update the shared token
                    update_handle = tokio::spawn({
                        let client = Arc::clone(&client);
                        async move {
                            let mut w = client.write().await;
                            *w = construct_client(&annon_key, &auth_resp.access_token).unwrap();
                            drop(w);
                        }
                    });
                }
            }
        });
        Ok(AuthenticatedSupabaseClient {
            supabase_url: self.supabase_url,
            client,
            annon_key_box,
            token_refresh: handle,
        })
    }
}

impl Drop for AuthenticatedSupabaseClient {
    fn drop(&mut self) {
        self.token_refresh.abort();
    }
}

trait SupabaseClientExt {
    async fn client(&mut self) -> reqwest::Client;
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

fn construct_client(
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

pub struct QueryBuilderWithTable<'a, 'b> {
    client: &'a SupabaseClient,
    table_name: &'b str,
}

pub struct QueryBuilderWithSelect<'a, 'b> {
    client: &'a SupabaseClient,
    table_name: &'b str,
}
