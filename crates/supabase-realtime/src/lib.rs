use std::task::{Poll, Waker};

use connection::WsSupabaseConnection;
use fastwebsockets::Frame;
use futures::{Stream, StreamExt};
use message::{PhoenixMessage, ProtocolMessage};
use pin_project::pin_project;
use supabase_auth::AuthResponse;

mod connection;
mod error;
pub mod message;
pub mod realtime;
