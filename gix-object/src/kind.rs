use crate::Kind;

/// The Error used in [`Kind::from_bytes()`].
pub type Error = gix_error::ValidationError;

/// Initialization
impl Kind {
    /// Parse a `Kind` from its serialized loose git objects.
    pub fn from_bytes(s: &[u8]) -> Result<Kind, Error> {
        Ok(match s {
            b"tree" => Kind::Tree,
            b"blob" => Kind::Blob,
            b"commit" => Kind::Commit,
            b"tag" => Kind::Tag,
            _ => return Err(Error::new_with_input("Unknown object kind", s)),
        })
    }
}

/// Access
impl Kind {
    /// Return the name of `self` for use in serialized loose git objects.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Kind::Tree => b"tree",
            Kind::Commit => b"commit",
            Kind::Blob => b"blob",
            Kind::Tag => b"tag",
        }
    }

    /// Returns `true` if this instance is representing a commit.
    pub fn is_commit(&self) -> bool {
        matches!(self, Kind::Commit)
    }

    /// Returns `true` if this instance is representing a tree.
    pub fn is_tree(&self) -> bool {
        matches!(self, Kind::Tree)
    }

    /// Returns `true` if this instance is representing a tag.
    pub fn is_tag(&self) -> bool {
        matches!(self, Kind::Tag)
    }

    /// Returns `true` if this instance is representing a blob.
    pub fn is_blob(&self) -> bool {
        matches!(self, Kind::Blob)
    }
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::str::from_utf8(self.as_bytes()).expect("Converting Kind name to utf8"))
    }
}
