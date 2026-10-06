use crate::ResponseMetadata;

/// The server strategy used to calculate a result total.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Count {
    /// Calculate an exact total.
    Exact,
    /// Use the database planner's estimate.
    Planned,
    /// Use an exact total for small results and a planner estimate for larger results.
    Estimated,
}
impl Count {
    pub(crate) const fn preference(self) -> &'static str {
        match self {
            Self::Exact => "count=exact",
            Self::Planned => "count=planned",
            Self::Estimated => "count=estimated",
        }
    }
}

/// A decoded representation and its server-reported total, independent of page size.
#[derive(Debug)]
pub struct Counted<T> {
    /// The requested representation.
    pub data: T,
    /// The total reported by the server, not the number of rows in this page.
    pub count: u64,
    /// Observed response status, headers and effective URL.
    pub metadata: ResponseMetadata,
}

/// A requested count could not be recovered from Content-Range.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum CountError {
    /// No Content-Range header was returned.
    #[error("the server did not return Content-Range for the requested count")]
    MissingContentRange,
    /// The server explicitly returned an unknown total (`*`).
    #[error("the server returned an unavailable Content-Range total")]
    UnavailableTotal,
    /// The header is malformed or its total is outside the u64 range.
    #[error("the server returned an invalid Content-Range")]
    InvalidContentRange,
}

fn decimal(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

/// Reads a unique Content-Range header and validates its server-reported total.
///
/// # Errors
/// Rejects missing, unavailable, malformed or overflowing totals.
pub fn response_count(headers: &reqwest::header::HeaderMap) -> Result<u64, CountError> {
    let mut headers = headers.get_all("content-range").iter();
    let header = headers.next().ok_or(CountError::MissingContentRange)?;
    if headers.next().is_some() {
        return Err(CountError::InvalidContentRange);
    }
    let value = header
        .to_str()
        .map_err(|_invalid_header| CountError::InvalidContentRange)?;
    let (range, total) = value
        .split_once('/')
        .ok_or(CountError::InvalidContentRange)?;
    if range != "*" {
        let (low, high) = range
            .split_once('-')
            .ok_or(CountError::InvalidContentRange)?;
        let low = decimal(low).ok_or(CountError::InvalidContentRange)?;
        let high = decimal(high).ok_or(CountError::InvalidContentRange)?;
        if low > high {
            return Err(CountError::InvalidContentRange);
        }
    }
    if total == "*" {
        return Err(CountError::UnavailableTotal);
    }
    decimal(total).ok_or(CountError::InvalidContentRange)
}
