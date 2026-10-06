use crate::{Error, ResponseMetadata};
use serde::de::DeserializeOwned;

/// Decodes a checked success response, retaining authoritative metadata on failure.
///
/// # Errors
/// Returns body-read or JSON/shape decode failures with the response metadata.
pub async fn decode<T: DeserializeOwned>(
    mut response: reqwest::Response,
) -> Result<(T, ResponseMetadata), Error> {
    let metadata = ResponseMetadata::take_from_response(&mut response);
    if response.status() == reqwest::StatusCode::NO_CONTENT {
        return match T::deserialize(serde::de::value::UnitDeserializer::<serde_json::Error>::new())
        {
            Ok(data) => Ok((data, metadata)),
            Err(source) => Err(Error::ResponseDecode {
                metadata: Box::new(metadata),
                source,
            }),
        };
    }
    let body = match response.bytes().await {
        Ok(body) => body,
        Err(source) => {
            return Err(Error::ResponseBody {
                metadata: Box::new(metadata),
                source,
            });
        }
    };
    match serde_json::from_slice(&body) {
        Ok(data) => Ok((data, metadata)),
        Err(source) => Err(Error::ResponseDecode {
            metadata: Box::new(metadata),
            source,
        }),
    }
}
