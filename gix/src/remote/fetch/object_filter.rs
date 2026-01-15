use std::{fmt, num::ParseIntError, str::FromStr};

/// Describe object filters to apply when requesting data from a remote.
///
/// This type mirrors [`git clone --filter`](https://git-scm.com/docs/git-clone#Documentation/git-clone.txt---filterltfilter-specgt)
/// and is currently limited to blob filters. Additional variants may be added in the future.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectFilter {
    /// Exclude all blobs from the initial clone, downloading them lazily as needed.
    BlobNone,
    /// Limit blobs included in the clone to those smaller or equal to `limit` bytes.
    BlobLimit(u64),
}

impl ObjectFilter {
    /// Render this filter into the argument string expected by remote servers.
    pub fn to_argument_string(&self) -> String {
        match self {
            ObjectFilter::BlobNone => "blob:none".into(),
            ObjectFilter::BlobLimit(limit) => format!("blob:limit={limit}"),
        }
    }
}

impl fmt::Display for ObjectFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_argument_string())
    }
}

/// Errors emitted when parsing [`ObjectFilter`] values from command-line input.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// The provided filter specification was empty.
    #[error("filter specification must not be empty")]
    Empty,
    /// The provided filter specification is not supported yet.
    #[error("unsupported filter specification '{0}'")]
    Unsupported(String),
    /// The provided blob size limit could not be parsed as an integer.
    #[error("invalid blob size limit '{value}'")]
    InvalidBlobLimit {
        /// The string that failed to parse.
        value: String,
        /// The underlying parse error.
        source: ParseIntError,
    },
}

impl FromStr for ObjectFilter {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let spec = input.trim();
        if spec.is_empty() {
            return Err(ParseError::Empty);
        }
        if spec.eq_ignore_ascii_case("blob:none") {
            return Ok(ObjectFilter::BlobNone);
        }
        if let Some(limit_str) = spec.strip_prefix("blob:limit=") {
            let limit = limit_str
                .parse::<u64>()
                .map_err(|source| ParseError::InvalidBlobLimit {
                    value: limit_str.to_owned(),
                    source,
                })?;
            return Ok(ObjectFilter::BlobLimit(limit));
        }
        Err(ParseError::Unsupported(spec.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectFilter;

    #[test]
    fn parse_blob_none() {
        assert_eq!("blob:none".parse::<ObjectFilter>().ok(), Some(ObjectFilter::BlobNone));
    }

    #[test]
    fn parse_blob_limit() {
        assert_eq!(
            "blob:limit=42".parse::<ObjectFilter>().ok(),
            Some(ObjectFilter::BlobLimit(42))
        );
    }

    #[test]
    fn parse_invalid_limit() {
        let err = "blob:limit=foo".parse::<ObjectFilter>().unwrap_err();
        assert!(matches!(err, super::ParseError::InvalidBlobLimit { .. }));
    }

    #[test]
    fn parse_unsupported() {
        let err = "tree:0".parse::<ObjectFilter>().unwrap_err();
        assert!(matches!(err, super::ParseError::Unsupported(_)));
    }

    #[test]
    fn displays_git_equivalent_filter_strings() {
        assert_eq!(ObjectFilter::BlobNone.to_argument_string(), "blob:none");
        assert_eq!(ObjectFilter::BlobNone.to_string(), "blob:none");
        assert_eq!(ObjectFilter::BlobLimit(7).to_argument_string(), "blob:limit=7");
        assert_eq!(ObjectFilter::BlobLimit(7).to_string(), "blob:limit=7");
    }
}
