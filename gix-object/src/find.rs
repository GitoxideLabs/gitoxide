/// The error type returned by the [`Find`](crate::Find) trait.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
///
pub mod existing {
    use gix_hash::ObjectId;

    /// The error returned by the [`find(…)`][crate::FindExt::find()] trait methods.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Find(crate::find::Error),
        NotFound { oid: ObjectId },
    }

    // TODO(review): these implementations hand-preserve `#[error(transparent)]` semantics for the
    //                `Find` variants of all three error types in this module: `Display` passes the
    //                formatter through and `source()` forwards to the inner error's source, exactly
    //                like the `thiserror`-generated code did — here through a `Box<dyn Error>`.
    //                The same pattern is used for `verify::Error`, `tree::editor::Error` and
    //                `LooseDecodeError` in this crate.
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Find(err) => std::fmt::Display::fmt(err, f),
                Error::NotFound { oid } => write!(f, "An object with id {oid} could not be found"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Find(err) => err.source(),
                Error::NotFound { .. } => None,
            }
        }
    }
}

///
pub mod existing_object {
    use gix_hash::ObjectId;

    /// The error returned by the various [`find_*()`][crate::FindExt::find_commit()] trait methods.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Find(crate::find::Error),
        Decode {
            oid: ObjectId,
            source: crate::decode::Error,
        },
        NotFound {
            oid: ObjectId,
        },
        ObjectKind {
            oid: ObjectId,
            actual: crate::Kind,
            expected: crate::Kind,
        },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Find(err) => std::fmt::Display::fmt(err, f),
                Error::Decode { oid, .. } => write!(f, "Could not decode object at {oid}"),
                Error::NotFound { oid } => write!(f, "An object with id {oid} could not be found"),
                Error::ObjectKind { oid, actual, expected } => {
                    write!(f, "Expected object of kind {expected} but got {actual} at {oid}")
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Find(err) => err.source(),
                Error::Decode { source, .. } => Some(source),
                Error::NotFound { .. } | Error::ObjectKind { .. } => None,
            }
        }
    }
}

///
pub mod existing_iter {
    use gix_hash::ObjectId;

    /// The error returned by the various [`find_*_iter()`][crate::FindExt::find_commit_iter()] trait methods.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Find(crate::find::Error),
        NotFound {
            oid: ObjectId,
        },
        ObjectKind {
            oid: ObjectId,
            actual: crate::Kind,
            expected: crate::Kind,
        },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Find(err) => std::fmt::Display::fmt(err, f),
                Error::NotFound { oid } => write!(f, "An object with id {oid} could not be found"),
                Error::ObjectKind { oid, actual, expected } => {
                    write!(f, "Expected object of kind {expected} but got {actual} at {oid}")
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Find(err) => err.source(),
                Error::NotFound { .. } | Error::ObjectKind { .. } => None,
            }
        }
    }
}

/// An implementation of object access traits that stores nothing and finds nothing.
#[derive(Debug, Copy, Clone)]
pub struct Never;

impl super::FindHeader for Never {
    fn try_header(&self, _id: &gix_hash::oid) -> Result<Option<crate::Header>, Error> {
        Ok(None)
    }
}

impl super::Find for Never {
    fn try_find<'a>(&self, _id: &gix_hash::oid, _buffer: &'a mut Vec<u8>) -> Result<Option<crate::Data<'a>>, Error> {
        Ok(None)
    }
}

impl super::Exists for Never {
    fn exists(&self, _id: &gix_hash::oid) -> bool {
        false
    }
}

impl super::Write for Never {
    fn write_buf(&self, object: crate::Kind, from: &[u8]) -> Result<gix_hash::ObjectId, crate::write::Error> {
        crate::compute_hash(gix_hash::Kind::default(), object, from).map_err(Into::into)
    }

    fn write_buf_with_known_id(
        &self,
        _object: crate::Kind,
        _from: &[u8],
        id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, crate::write::Error> {
        Ok(id)
    }

    fn write_stream(
        &self,
        kind: crate::Kind,
        size: u64,
        from: &mut dyn std::io::Read,
    ) -> Result<gix_hash::ObjectId, crate::write::Error> {
        crate::compute_stream_hash(
            gix_hash::Kind::default(),
            kind,
            from,
            size,
            &mut gix_features::progress::Discard,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .map_err(Into::into)
    }

    fn write_stream_with_known_id(
        &self,
        _kind: crate::Kind,
        mut size: u64,
        from: &mut dyn std::io::Read,
        id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, crate::write::Error> {
        let mut buf = [0u8; u16::MAX as usize];
        while size != 0 {
            let bytes = (size as usize).min(buf.len());
            from.read_exact(&mut buf[..bytes])?;
            size -= bytes as u64;
        }
        Ok(id)
    }
}
