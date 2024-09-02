pub use error::SupabaseClientError;
mod client;
mod error;
pub use client::{AuthenticatedSupabaseClient, SupabaseClient, SupabaseClientExt};
