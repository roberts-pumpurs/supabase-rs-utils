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
    PostgerstResponse, ResponseError, SUPABASE_KEY, SupabaseClientError, anonymous_client,
    new_authenticated,
};
pub use postgrest;
#[cfg(feature = "client")]
pub use {rp_postgrest_error, rp_supabase_auth};
