#![cfg_attr(doc, doc = include_str!("../README.md"))]

extern crate alloc;

mod code;
mod error;

pub use code::{Authentication, ErrorCode, ErrorKind, PostgresErrorCode, PostgrestErrorCode};
pub use error::{DecodeError, ErrorResponse, PostgrestError};
