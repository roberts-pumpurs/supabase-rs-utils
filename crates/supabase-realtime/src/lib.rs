#![cfg_attr(doc, doc = include_str!("../README.md"))]
extern crate alloc;

mod connection;
pub mod error;
pub mod message;
pub mod realtime;

pub use {futures, rp_supabase_auth, url};
