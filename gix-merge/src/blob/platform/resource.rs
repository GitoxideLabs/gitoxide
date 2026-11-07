use crate::blob::{
    pipeline,
    platform::{Resource, ResourceRef},
};

impl<'a> ResourceRef<'a> {
    pub(super) fn new(cache: &'a Resource) -> Self {
        ResourceRef {
            data: cache.data.map_or(Data::Missing, |data| match data {
                pipeline::Data::Buffer => Data::Buffer(&cache.buffer),
                pipeline::Data::TooLarge { size } => Data::TooLarge { size },
            }),
            rela_path: cache.rela_path.as_ref(),
            id: cache.id.as_deref(),
        }
    }
}

/// The data of a mergeable resource, as it could be determined and computed previously.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Data<'a> {
    /// The object is missing, either because it didn't exist in the working tree or because its `id` was `None`
    /// and no worktree root was configured. A supplied ID does not override absence from the worktree.
    /// Text merges interpret this as an empty buffer; binary identity checks retain the distinction.
    Missing,
    /// The data as processed and ready for merging, i.e. suitable for storage in Git.
    /// An empty buffer represents a present resource with empty content, unlike [`Data::Missing`].
    Buffer(&'a [u8]),
    /// The file or blob is above the big-file threshold and cannot be processed.
    ///
    /// In this state, the file cannot be merged.
    TooLarge {
        /// The size of the object prior to performing any filtering or as it was found on disk.
        ///
        /// Note that technically, the size isn't always representative of the same 'state' of the
        /// content, as once it can be the size of the blob in Git, and once it's the size of file
        /// in the worktree.
        size: u64,
    },
}

impl<'a> Data<'a> {
    /// Return the bytes of a present [`Data::Buffer`], including an empty one.
    /// Return `None` for [`Data::Missing`] and [`Data::TooLarge`]; inspect the variant to distinguish them.
    pub fn as_slice(&self) -> Option<&'a [u8]> {
        match self {
            Data::Buffer(d) => Some(d),
            Data::Missing | Data::TooLarge { .. } => None,
        }
    }
}
