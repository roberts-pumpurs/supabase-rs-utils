use std::fmt::Display;

use reqwest::header::InvalidHeaderValue;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum SupabaseClientError {
    #[error("Reqwest error {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Url Parse Error {0}")]
    UlrParseError(#[from] url::ParseError),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[error("PostgREST Error {0:?}")]
    PostgRestError(PostgrestError),
    #[error("Serde JSON error {0}")]
    SerdeJsonError(#[from] simd_json::Error),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PostgrestError {
    pub message: String,
    pub code: String,
    pub details: Option<String>,
    pub hint: Option<String>,
}

impl From<PostgrestError> for SupabaseClientError {
    fn from(value: PostgrestError) -> Self {
        SupabaseClientError::PostgRestError(value)
    }
}
