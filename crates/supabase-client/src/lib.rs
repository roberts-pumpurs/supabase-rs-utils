mod client;
pub mod error;
pub use client::{
    query_builder, AuthenticatedSupabaseClient, PostgRestQuery, SupabaseClient, SupabaseClientExt,
};
