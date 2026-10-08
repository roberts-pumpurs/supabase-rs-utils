#![cfg_attr(doc, doc = include_str!("../README.md"))]

#[cfg(feature = "rest")]
pub use rp_postgrest;
#[cfg(feature = "auth")]
pub use rp_supabase_auth;
#[cfg(feature = "typed")]
pub use rp_supabase_client;
#[cfg(feature = "functions")]
pub use rp_supabase_functions;
#[cfg(feature = "realtime")]
pub use rp_supabase_realtime;
#[cfg(feature = "storage")]
pub use rp_supabase_storage;
pub use url::Url;

/// Header that carries the project API key.
#[cfg(feature = "rest")]
const API_KEY_HEADER: &str = "apikey";

/// Maximum sign-in and token refresh retries in [`Client::realtime_config`].
#[cfg(feature = "realtime")]
pub const REALTIME_MAX_RECONNECT_ATTEMPTS: u8 = 5;

/// Delay between sign-in and token refresh retries in [`Client::realtime_config`].
#[cfg(feature = "realtime")]
pub const REALTIME_RECONNECT_INTERVAL: core::time::Duration = core::time::Duration::from_secs(3);

/// Errors from building a [`Client`].
#[derive(Debug, thiserror::Error)]
#[expect(
    clippy::error_impl_error,
    reason = "the crate exposes one error type named `Error`"
)]
#[non_exhaustive]
pub enum Error {
    /// The project URL does not parse.
    #[error("invalid project URL: {0}")]
    UrlParse(#[from] url::ParseError),
    /// The project URL parses but is not a usable project base URL.
    #[error("invalid project URL: {0}")]
    InvalidProjectUrl(&'static str),
    /// The REST client rejected its configuration.
    #[cfg(feature = "rest")]
    #[error(transparent)]
    Rest(#[from] rp_postgrest::Error),
    /// The auth client rejected its configuration.
    #[cfg(feature = "auth")]
    #[error(transparent)]
    Auth(#[from] rp_supabase_auth::error::AuthError),
    /// The storage client rejected its configuration.
    #[cfg(feature = "storage")]
    #[error(transparent)]
    Storage(#[from] rp_supabase_storage::StorageError),
    /// The functions client rejected its configuration.
    #[cfg(feature = "functions")]
    #[error(transparent)]
    Functions(#[from] rp_supabase_functions::FunctionsError),
    /// The default HTTP client cannot be built.
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

/// Client for one Supabase project.
///
/// It holds one shared `reqwest::Client` and one sub-client per enabled feature.
/// Cloning is cheap: the connection pool is shared.
#[derive(Clone)]
pub struct Client {
    project_url: Url,
    api_key: String,
    http: reqwest::Client,
    #[cfg(feature = "rest")]
    rest: rp_postgrest::Postgrest,
    #[cfg(feature = "auth")]
    auth: rp_supabase_auth::auth_client::ApiClient,
    #[cfg(feature = "storage")]
    storage: rp_supabase_storage::StorageClient,
    #[cfg(feature = "functions")]
    functions: rp_supabase_functions::FunctionsClient,
}

impl core::fmt::Debug for Client {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Client")
            .field("project_url", &self.project_url.as_str())
            .field("api_key", &"[redacted]")
            .finish_non_exhaustive()
    }
}

impl Client {
    /// Creates a client for the project at `project_url`, for example `https://abc.supabase.co/`.
    ///
    /// # Errors
    /// Returns [`Error::UrlParse`] or [`Error::InvalidProjectUrl`] for a bad URL, and a
    /// sub-client error when a credential is not a valid header value.
    pub fn new(project_url: &str, api_key: &str) -> Result<Self, Error> {
        Self::new_with_client(project_url, api_key, reqwest::Client::builder().build()?)
    }

    /// Creates a client that reuses the connection pool and policies of `http`.
    ///
    /// # Errors
    /// Same as [`Client::new`].
    pub fn new_with_client(
        project_url: &str,
        api_key: &str,
        http: reqwest::Client,
    ) -> Result<Self, Error> {
        let project_url = parse_project_url(project_url)?;
        Ok(Self {
            #[cfg(feature = "rest")]
            rest: rest_client(&project_url, api_key, None, http.clone())?,
            #[cfg(feature = "auth")]
            auth: rp_supabase_auth::auth_client::ApiClient::new_unauthenticated_with_client(
                &project_url,
                api_key,
                http.clone(),
            )?,
            #[cfg(feature = "storage")]
            storage: rp_supabase_storage::StorageClient::new_with_client(
                &project_url,
                api_key,
                http.clone(),
            )?,
            #[cfg(feature = "functions")]
            functions: rp_supabase_functions::FunctionsClient::new_with_client(
                &project_url,
                api_key,
                http.clone(),
            )?,
            api_key: api_key.to_owned(),
            http,
            project_url,
        })
    }

    /// Returns a client that acts as the user who owns `token` (a JWT from sign-in).
    ///
    /// REST, auth, storage, and functions send `Authorization: Bearer <token>`.
    /// The `apikey` header keeps the project key. The connection pool is shared.
    ///
    /// # Errors
    /// Returns a sub-client error when `token` is not a valid header value.
    #[cfg_attr(
        not(any(
            feature = "rest",
            feature = "auth",
            feature = "storage",
            feature = "functions"
        )),
        expect(
            unused_variables,
            clippy::unnecessary_wraps,
            reason = "no enabled service uses a token"
        )
    )]
    pub fn with_access_token(&self, token: &str) -> Result<Self, Error> {
        Ok(Self {
            project_url: self.project_url.clone(),
            api_key: self.api_key.clone(),
            http: self.http.clone(),
            #[cfg(feature = "rest")]
            rest: rest_client(
                &self.project_url,
                &self.api_key,
                Some(token),
                self.http.clone(),
            )?,
            #[cfg(feature = "auth")]
            auth: rp_supabase_auth::auth_client::ApiClient::new_authenticated_with_client(
                &self.project_url,
                &self.api_key,
                token,
                self.http.clone(),
            )?,
            #[cfg(feature = "storage")]
            storage: self.storage.with_access_token(token)?,
            #[cfg(feature = "functions")]
            functions: self.functions.with_access_token(token)?,
        })
    }

    /// Project base URL.
    #[must_use]
    pub const fn project_url(&self) -> &Url {
        &self.project_url
    }

    /// REST client for `{project}/rest/v1/`.
    ///
    /// The typed runtime of `rp-supabase-client` takes this same `Postgrest`.
    #[cfg(feature = "rest")]
    #[must_use]
    pub const fn rest(&self) -> &rp_postgrest::Postgrest {
        &self.rest
    }

    /// Starts a query on `table`. Shortcut for `self.rest().from(table)`.
    #[cfg(feature = "rest")]
    pub fn from(&self, table: &str) -> rp_postgrest::Builder {
        self.rest.from(table)
    }

    /// Supabase Auth client for `{project}/auth/v1/`.
    ///
    /// After [`Client::with_access_token`] it is authenticated, so `get_user`,
    /// `update_user`, and `sign_out` work.
    #[cfg(feature = "auth")]
    #[must_use]
    pub const fn auth(&self) -> &rp_supabase_auth::auth_client::ApiClient {
        &self.auth
    }

    /// Storage client for `{project}/storage/v1/`.
    #[cfg(feature = "storage")]
    #[must_use]
    pub const fn storage(&self) -> &rp_supabase_storage::StorageClient {
        &self.storage
    }

    /// Edge Functions client for `{project}/functions/v1/`.
    #[cfg(feature = "functions")]
    #[must_use]
    pub const fn functions(&self) -> &rp_supabase_functions::FunctionsClient {
        &self.functions
    }

    /// Configuration for `RealtimeConnection::db_changes`, `presence`, and `broadcast`.
    ///
    /// Realtime signs in on its own, so it uses the project key, not a user token.
    /// Retry defaults for sign-in and token refresh: [`REALTIME_MAX_RECONNECT_ATTEMPTS`]
    /// attempts, [`REALTIME_RECONNECT_INTERVAL`] apart. The websocket itself does not reconnect.
    /// Change the returned fields to override the retry settings.
    #[cfg(feature = "realtime")]
    #[must_use]
    pub fn realtime_config(
        &self,
    ) -> rp_supabase_realtime::rp_supabase_auth::jwt_stream::SupabaseAuthConfig {
        rp_supabase_realtime::rp_supabase_auth::jwt_stream::SupabaseAuthConfig {
            api_key: self.api_key.clone(),
            max_reconnect_attempts: REALTIME_MAX_RECONNECT_ATTEMPTS,
            reconnect_interval: REALTIME_RECONNECT_INTERVAL,
            url: self.project_url.clone(),
        }
    }
}

fn parse_project_url(input: &str) -> Result<Url, Error> {
    let url = Url::parse(input)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(Error::InvalidProjectUrl("scheme must be http or https"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Error::InvalidProjectUrl("credentials are not allowed"));
    }
    if url.query().is_some() {
        return Err(Error::InvalidProjectUrl("query is not allowed"));
    }
    if url.fragment().is_some() {
        return Err(Error::InvalidProjectUrl("fragment is not allowed"));
    }
    Ok(url)
}

/// Builds the REST client the same way as `rp_supabase_client::anonymous_client_with_client`,
/// plus an optional user bearer.
#[cfg(feature = "rest")]
fn rest_client(
    project_url: &Url,
    api_key: &str,
    token: Option<&str>,
    http: reqwest::Client,
) -> Result<rp_postgrest::Postgrest, Error> {
    let base = project_url.join("rest/v1/")?;
    let client = rp_postgrest::Postgrest::new_with_client(base.as_str(), http)?
        .insert_header(API_KEY_HEADER, api_key)?;
    Ok(match token {
        Some(token) => client.auth(token)?,
        None => client,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_project_urls() {
        for input in [
            "not a url",
            "ftp://abc.supabase.co/",
            "https://user:pass@abc.supabase.co/",
            "https://abc.supabase.co/?x=1",
            "https://abc.supabase.co/#frag",
        ] {
            assert!(
                Client::new(input, "key").is_err(),
                "{input} must be rejected"
            );
        }
    }
}
