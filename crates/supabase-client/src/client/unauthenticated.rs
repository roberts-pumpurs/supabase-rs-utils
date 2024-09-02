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
    pub(crate) annon_key: String,
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

        let handle = tokio::spawn({
            let annon_key = Arc::new(self.annon_key);
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
                        let annon_key = Arc::clone(&annon_key);
                        async move {
                            let mut w = client.write().await;
                            if let Ok(construct_client) =
                                construct_client(&annon_key, &auth_resp.access_token)
                            {
                                *w = construct_client;
                            } else {
                                tracing::warn!("could not create a new client");
                            }
                            drop(w);
                        }
                    });
                }
            }
        });
        Ok(AuthenticatedSupabaseClient {
            supabase_url: self.supabase_url,
            client,
            token_refresh: handle,
        })
    }

    pub fn rest_url(&self) -> Result<url::Url, SupabaseClientError> {
        let url = self.supabase_url.join("/rest/v1/")?;
        Ok(url)
    }
}
