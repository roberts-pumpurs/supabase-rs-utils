use reqwest::header::InvalidHeaderValue;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Reqwest error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Url parse error {0}")]
    UrlParse(#[from] url::ParseError),
    #[error("JSON error {0}")]
    Json(#[from] simd_json::Error),
    #[error("Invalid header value {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}
