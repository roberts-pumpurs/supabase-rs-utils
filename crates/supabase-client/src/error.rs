use reqwest::header::InvalidHeaderValue;

#[derive(thiserror::Error, Debug)]
pub enum SupabaseClientError {
    #[error("Reqwest error")]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}
