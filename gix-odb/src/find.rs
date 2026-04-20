use std::io::{self, Cursor, Read};

/// A streaming view over an object's decoded bytes.
pub struct Stream {
    kind: gix_object::Kind,
    size: u64,
    inner: StreamInner,
}

enum StreamInner {
    InMemory(Cursor<Vec<u8>>),
    File(std::fs::File),
    Loose(crate::store_impls::loose::find::StreamReader),
}

impl Stream {
    /// Return the kind of the object yielded by this stream.
    pub fn kind(&self) -> gix_object::Kind {
        self.kind
    }

    /// Return the decoded object size in bytes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Return an empty blob stream.
    pub fn empty_blob() -> Self {
        Self::from_bytes(gix_object::Kind::Blob, Vec::new())
    }

    pub(crate) fn from_bytes(kind: gix_object::Kind, data: Vec<u8>) -> Self {
        Self {
            kind,
            size: data.len() as u64,
            inner: StreamInner::InMemory(Cursor::new(data)),
        }
    }

    pub(crate) fn from_file(kind: gix_object::Kind, size: u64, file: std::fs::File) -> Self {
        Self {
            kind,
            size,
            inner: StreamInner::File(file),
        }
    }

    pub(crate) fn from_loose(
        kind: gix_object::Kind,
        size: u64,
        reader: crate::store_impls::loose::find::StreamReader,
    ) -> Self {
        Self {
            kind,
            size,
            inner: StreamInner::Loose(reader),
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.inner {
            StreamInner::InMemory(cursor) => cursor.read(buf),
            StreamInner::File(file) => file.read(buf),
            StreamInner::Loose(reader) => reader.read(buf),
        }
    }
}

/// An object header informing about object properties, without it being fully decoded in the process.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Header {
    /// The object was not packed, but is currently located in the loose object portion of the database.
    ///
    /// As packs are searched first, this means that in this very moment, the object whose header we retrieved is unique
    /// in the object database.
    Loose {
        /// The kind of the object.
        kind: gix_object::Kind,
        /// The size of the object's data in bytes.
        size: u64,
    },
    /// The object was present in a pack.
    ///
    /// Note that this does not imply it is unique in the database, as it might be present in more than one pack and even
    /// as loose object.
    Packed(gix_pack::data::decode::header::Outcome),
}

mod header {
    use super::Header;

    impl Header {
        /// Return the object kind of the object we represent.
        pub fn kind(&self) -> gix_object::Kind {
            match self {
                Header::Packed(out) => out.kind,
                Header::Loose { kind, .. } => *kind,
            }
        }
        /// Return the size of the object in bytes.
        pub fn size(&self) -> u64 {
            match self {
                Header::Packed(out) => out.object_size,
                Header::Loose { size, .. } => *size,
            }
        }
        /// Return the amount of deltas decoded to obtain this header, if the object was packed.
        pub fn num_deltas(&self) -> Option<u32> {
            match self {
                Header::Packed(out) => out.num_deltas.into(),
                Header::Loose { .. } => None,
            }
        }
    }

    impl From<gix_pack::data::decode::header::Outcome> for Header {
        fn from(packed_header: gix_pack::data::decode::header::Outcome) -> Self {
            Header::Packed(packed_header)
        }
    }

    impl From<(u64, gix_object::Kind)> for Header {
        fn from((object_size, kind): (u64, gix_object::Kind)) -> Self {
            Header::Loose {
                kind,
                size: object_size,
            }
        }
    }
}
