//! Supabase `PostgREST` client and the runtime for generated schema bindings.
//!
//! The default `client` feature provides authentication and response decoding.
//! Disable default features to depend on the generated-binding runtime ([`schema`]) alone.

extern crate alloc;

#[cfg(feature = "client")]
mod client;
pub mod schema;

#[cfg(feature = "client")]
pub use client::{
    SUPABASE_KEY, SupabaseClientError, anonymous_client, anonymous_client_with_client,
    new_authenticated, new_authenticated_with_client,
};
pub use rp_postgrest;
pub use rp_postgrest::{Error, Postgrest};
#[cfg(feature = "client")]
pub use {rp_postgrest_error, rp_supabase_auth};
