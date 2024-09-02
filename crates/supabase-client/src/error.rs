use reqwest::header::InvalidHeaderValue;

#[derive(thiserror::Error, Debug)]
pub enum SupabaseClientError {
    #[error("Reqwest error {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Url Parse Error {0}")]
    UlrParseError(#[from] url::ParseError),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}
