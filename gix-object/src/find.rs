use gix_error::ResultExt;

/// The error type returned by the [`Find`](crate::Find) trait.
pub type Error = gix_error::Exn;
///
pub mod existing {
    /// The error returned by the [`find(…)`][crate::FindExt::find()] trait methods.
    pub type Error = gix_error::Exn;
}

///
pub mod existing_object {
    /// The error returned by the various [`find_*()`][crate::FindExt::find_commit()] trait methods.
    pub type Error = gix_error::Exn;
}

///
pub mod existing_iter {
    /// The error returned by the various [`find_*_iter()`][crate::FindExt::find_commit_iter()] trait methods.
    pub type Error = gix_error::Exn;
}

/// An implementation of object access traits that stores nothing and finds nothing.
/// Use [`Never::panic_on_access()`] to panic on object access instead.
#[derive(Debug, Copy, Clone)]
pub struct Never;

impl Never {
    /// Return an implementation that panics whenever an object access trait method is called.
    /// Useful for asserting that an operation does not access the object database.
    pub const fn panic_on_access() -> PanicAlways {
        PanicAlways
    }
}

/// An implementation of object access traits that panics on every call.
/// Obtain it with [`Never::panic_on_access()`].
#[derive(Debug, Copy, Clone)]
pub struct PanicAlways;

impl super::FindHeader for PanicAlways {
    fn try_header(&self, _id: &gix_hash::oid) -> Result<Option<crate::Header>, Error> {
        panic!("object header lookups are forbidden");
    }
}

impl super::Find for PanicAlways {
    fn try_find<'a>(&self, _id: &gix_hash::oid, _buffer: &'a mut Vec<u8>) -> Result<Option<crate::Data<'a>>, Error> {
        panic!("object lookups are forbidden");
    }
}

impl super::Exists for PanicAlways {
    fn exists(&self, _id: &gix_hash::oid) -> bool {
        panic!("object existence checks are forbidden");
    }
}

impl super::Write for PanicAlways {
    fn write(&self, _object: &dyn crate::WriteTo) -> Result<gix_hash::ObjectId, crate::write::Error> {
        panic!("object writes are forbidden");
    }

    fn write_buf_with_known_id(
        &self,
        _object: crate::Kind,
        _from: &[u8],
        _id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, crate::write::Error> {
        panic!("object writes are forbidden");
    }

    fn write_stream(
        &self,
        _kind: crate::Kind,
        _size: u64,
        _from: &mut dyn std::io::Read,
    ) -> Result<gix_hash::ObjectId, crate::write::Error> {
        panic!("object writes are forbidden");
    }

    fn write_stream_with_known_id(
        &self,
        _kind: crate::Kind,
        _size: u64,
        _from: &mut dyn std::io::Read,
        _id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, crate::write::Error> {
        panic!("object writes are forbidden");
    }
}

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
        crate::compute_hash(gix_hash::Kind::default(), object, from).or_erased()
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
        .or_erased()
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
            from.read_exact(&mut buf[..bytes]).or_erased()?;
            size -= bytes as u64;
        }
        Ok(id)
    }
}
