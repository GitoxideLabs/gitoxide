use bstr::{BStr, BString};

use crate::blob::platform::Resource;
use crate::blob::{pipeline, Platform, ResourceKind};

/// The error returned by [Platform::set_resource](Platform::set_resource).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Can only diff blobs, not {mode:?}")]
    InvalidMode { mode: gix_object::tree::EntryKind },
    #[error("Failed to read {kind:?} worktree data from '{rela_path}'")]
    Io {
        rela_path: BString,
        kind: ResourceKind,
        source: std::io::Error,
    },
    #[error("Failed to obtain attributes for {kind:?} resource at '{rela_path}'")]
    Attributes {
        rela_path: BString,
        kind: ResourceKind,
        source: std::io::Error,
    },
    #[error(transparent)]
    ConvertToMergeable(#[from] pipeline::convert_to_mergeable::Error),
}

/// Preparation
impl Platform {
    /// Store enough information about a resource to eventually use it in a merge, where…
    ///
    /// * `id` is the hash of the resource. If it [is null](gix_hash::ObjectId::is_null()), it should either
    ///   be a resource in the worktree, or it's considered a non-existing, deleted object.
    ///   If an `id` is known, as the hash of the object as (would) be stored in `git`, then it should be provided
    ///   for completeness. Note that it's not expected to be in `objects` if `rela_path` is set and a worktree-root
    ///   is available for `kind`.
    /// * `mode` is the kind of object (only blobs and executables are allowed)
    /// * `rela_path` is the relative path as seen from the (work)tree root.
    /// * `kind` identifies the side of the merge this resource will be used for.
    /// * `objects` provides access to the object database in case the resource can't be read from a worktree.
    pub fn set_resource(
        &mut self,
        id: gix_hash::ObjectId,
        mode: gix_object::tree::EntryKind,
        rela_path: &BStr,
        kind: ResourceKind,
        objects: &impl gix_object::FindObjectOrHeader,
    ) -> Result<(), Error> {
        if !matches!(
            mode,
            gix_object::tree::EntryKind::Blob | gix_object::tree::EntryKind::BlobExecutable
        ) {
            return Err(Error::InvalidMode { mode });
        }
        let entry = self
            .attr_stack
            .at_entry(rela_path, None, objects)
            .map_err(|err| Error::Attributes {
                source: err,
                kind,
                rela_path: rela_path.to_owned(),
            })?;

        let storage = match kind {
            ResourceKind::OtherOrTheirs => &mut self.other,
            ResourceKind::CommonAncestorOrBase => &mut self.ancestor,
            ResourceKind::CurrentOrOurs => &mut self.current,
        };

        let mut buf_storage = Vec::new();
        let out = self.filter.convert_to_mergeable(
            &id,
            mode,
            rela_path,
            kind,
            &mut |_, out| {
                let _ = entry.matching_attributes(out);
            },
            objects,
            self.filter_mode,
            storage.as_mut().map_or(&mut buf_storage, |s| &mut s.buffer),
        )?;

        match storage {
            None => {
                *storage = Some(Resource {
                    id,
                    rela_path: rela_path.to_owned(),
                    data: out,
                    mode,
                    buffer: buf_storage,
                });
            }
            Some(storage) => {
                storage.id = id;
                storage.rela_path = rela_path.to_owned();
                storage.data = out;
                storage.mode = mode;
            }
        }
        Ok(())
    }
}
