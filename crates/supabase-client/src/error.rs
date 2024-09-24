use reqwest::header::InvalidHeaderValue;
use serde::{Deserialize, Serialize};
use supabase_auth::{RefreshStreamError, SignInError};

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
    #[error("Supabase Sign in Error {0}")]
    SignInError(#[from] SignInError),
    #[error("JWT refresh stream crashed")]
    JwtStreamCrash,
    #[error("Recoverable JWT Refresh stream error {0}")]
    RefreshStreamError(#[from] RefreshStreamError),
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
