use std::sync::{Arc, RwLock};

use supabase_auth::User;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct AuthenticatedSupabaseClient {
    pub(crate) token_refresh: JoinHandle<()>,
    pub(crate) supabase_url: url::Url,
    pub(crate) client: Arc<RwLock<reqwest::Client>>,
    pub user: User,
}

impl Drop for AuthenticatedSupabaseClient {
    fn drop(&mut self) {
        self.token_refresh.abort();
    }
}
