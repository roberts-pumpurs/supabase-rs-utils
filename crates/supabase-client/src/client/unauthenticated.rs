use std::sync::Arc;

use supabase_auth::futures::StreamExt;
use supabase_auth::TokenBody;
use tokio::sync::RwLock;

use super::authenticated::AuthenticatedSupabaseClient;
use super::construct_client;
use crate::SupabaseClientError;

pub struct SupabaseClient {
    pub(crate) supabase_url: url::Url,
    pub(crate) client: reqwest::Client,
    pub(crate) anon_key: String,
}

impl SupabaseClient {
    pub fn new(supabase_url: url::Url, anon_key: String) -> Result<Self, SupabaseClientError> {
        let client = construct_client(&anon_key, &anon_key)?;
        Ok(Self {
            supabase_url,
            client,
            anon_key,
        })
    }

    pub async fn sign_in_with_password(
        self,
        token_body: TokenBody<'static>,
    ) -> Result<AuthenticatedSupabaseClient, SupabaseClientError> {
        let auth =
            supabase_auth::SupabaseAuth::new(self.supabase_url.clone(), self.anon_key.clone());
        let mut auth = auth.sign_in(token_body)?;
        let auth_resp = auth
            .next()
            .await
            .ok_or_else(|| SupabaseClientError::JwtStreamCrash)??;
        let client = construct_client(&self.anon_key, &auth_resp.access_token)?;
        let client = Arc::new(RwLock::new(client));

        let handle = tokio::spawn({
            let anon_key = self.anon_key.clone();
            let client = Arc::clone(&client);

            async move {
                while let Some(Ok(auth_resp)) = auth.next().await {
                    let mut w = client.write().await;
                    if let Ok(new_client) = construct_client(&anon_key, &auth_resp.access_token) {
                        *w = new_client;
                    } else {
                        tracing::warn!("could not create a new client");
                    }
                }
            }
        });

        Ok(AuthenticatedSupabaseClient {
            supabase_url: self.supabase_url,
            client,
            token_refresh: handle,
            user: auth_resp.user,
        })
    }
}
