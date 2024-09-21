pub use error::SupabaseClientError;
mod client;
mod error;
pub use client::{
    query_builder, AuthenticatedSupabaseClient, PostgRestQuery, SupabaseClient, SupabaseClientExt,
};
