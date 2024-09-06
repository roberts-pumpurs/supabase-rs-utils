extern crate alloc;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub struct AuthenticatedSupabaseClient {
    pub(crate) token_refresh: JoinHandle<()>,
    pub(crate) supabase_url: url::Url,
    pub(crate) client: Arc<RwLock<reqwest::Client>>,
}

impl Drop for AuthenticatedSupabaseClient {
    fn drop(&mut self) {
        self.token_refresh.abort();
    }
}
