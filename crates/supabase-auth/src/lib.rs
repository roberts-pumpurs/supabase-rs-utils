#![cfg_attr(doc, doc = include_str!("../README.md"))]
pub use {futures, redact, url};
pub const SUPABASE_KEY: &str = "apikey";

pub mod auth_client;
pub mod error;
pub mod jwt_stream;
pub mod types;
