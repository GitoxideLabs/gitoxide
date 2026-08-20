use std::io;

use gix_error::ResultExt;

use crate::Kind;

/// Describe the capability to write git objects into an object store.
pub trait Write {
    /// Write objects using the intrinsic kind of [`hash`](gix_hash::Kind) into the database,
    /// returning id to reference it in subsequent reads.
    fn write(&self, object: &dyn WriteTo) -> Result<gix_hash::ObjectId, crate::write::Error> {
        let mut buf = Vec::with_capacity(2048);
        object.write_to(&mut buf).or_erased()?;
        self.write_stream(object.kind(), buf.len() as u64, &mut buf.as_slice())
    }
    /// As [`write`](Write::write), but takes an [`object` kind](Kind) along with its encoded bytes.
    fn write_buf(&self, object: crate::Kind, mut from: &[u8]) -> Result<gix_hash::ObjectId, crate::write::Error> {
        self.write_stream(object, from.len() as u64, &mut from)
    }
    /// As [`write_buf`](Write::write_buf), but the object `id` has already been computed by the caller.
    ///
    /// Implementations may trust the given `id` and avoid computing it again. Callers must make sure `id` matches
    /// the provided `object` and `from` bytes.
    fn write_buf_with_known_id(
        &self,
        object: crate::Kind,
        from: &[u8],
        id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, crate::write::Error>;
    /// As [`write`](Write::write), but takes an input stream.
    /// This is commonly used for writing blobs directly without reading them to memory first.
    fn write_stream(
        &self,
        kind: crate::Kind,
        size: u64,
        from: &mut dyn io::Read,
    ) -> Result<gix_hash::ObjectId, crate::write::Error>;
    /// As [`write_stream`](Write::write_stream), but the object `id` has already been computed by the caller.
    ///
    /// Implementations may trust the given `id` and avoid computing it again. Callers must make sure `id` matches
    /// the provided `kind`, `size` and stream contents.
    fn write_stream_with_known_id(
        &self,
        kind: crate::Kind,
        size: u64,
        from: &mut dyn io::Read,
        id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, crate::write::Error>;
}

/// Writing of objects to a `Write` implementation
pub trait WriteTo {
    /// Write a representation of this instance to `out`.
    fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()>;

    /// Returns the type of this object.
    fn kind(&self) -> Kind;

    /// Returns the size of this object's representation (the amount
    /// of data which would be written by [`write_to`](Self::write_to)).
    ///
    /// [`size`](Self::size)'s value has no bearing on the validity of
    /// the object, as such it's possible for [`size`](Self::size) to
    /// return a sensible value but [`write_to`](Self::write_to) to
    /// fail because the object was not actually valid in some way.
    fn size(&self) -> u64;

    /// Returns a loose object header based on the object's data
    fn loose_header(&self) -> smallvec::SmallVec<[u8; 28]> {
        crate::encode::loose_header(self.kind(), self.size())
    }
}

mod _impls;

mod find;
pub use find::*;
