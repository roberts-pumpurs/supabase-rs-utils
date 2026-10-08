use crate::error::{PathError, StorageError};

/// Validates a bucket id: one non-empty segment that is not `.` or `..`.
pub fn bucket(id: &str) -> Result<&str, StorageError> {
    check(id, core::iter::once(id))?;
    Ok(id)
}

/// Validates an object path and returns its `/`-separated segments.
pub fn object(path: &str) -> Result<core::str::Split<'_, char>, StorageError> {
    check(path, path.split('/'))?;
    Ok(path.split('/'))
}

fn check<'a>(input: &str, mut segments: impl Iterator<Item = &'a str>) -> Result<(), StorageError> {
    let reason = if input.is_empty() {
        Some(PathError::Empty)
    } else {
        segments.find_map(|segment| match segment {
            "" => Some(PathError::EmptySegment),
            "." | ".." => Some(PathError::DotSegment),
            _ => None,
        })
    };
    reason.map_or(Ok(()), |reason| {
        Err(StorageError::InvalidPath {
            path: input.to_owned(),
            reason,
        })
    })
}
