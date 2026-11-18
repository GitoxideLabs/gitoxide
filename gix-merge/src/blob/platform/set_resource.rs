use bstr::BStr;
use gix_error::Result;
use gix_error::{ResultExt, bail, message};

use crate::blob::{Platform, ResourceKind, platform::Resource};

/// Preparation
impl Platform {
    /// Compare two regular-file versions after normalization, without running a merge driver.
    pub(crate) fn unchanged_after_renormalization(
        &mut self,
        rela_path: &BStr,
        (base_blob_id, base_mode): (gix_hash::ObjectId, gix_object::tree::EntryMode),
        (side_blob_id, side_mode): (gix_hash::ObjectId, gix_object::tree::EntryMode),
        side: ResourceKind,
        objects: &impl gix_object::FindObjectOrHeader,
    ) -> Result<bool> {
        if self.filter_mode != crate::blob::pipeline::Mode::Renormalize
            || !base_mode.is_blob()
            || base_mode.kind() != side_mode.kind()
        {
            return Ok(false);
        }
        self.set_resource(
            Some(base_blob_id),
            base_mode.kind(),
            rela_path,
            ResourceKind::CommonAncestorOrBase,
            objects,
        )?;
        self.set_resource(Some(side_blob_id), side_mode.kind(), rela_path, side, objects)?;
        let base = self
            .ancestor
            .as_ref()
            .expect("the base resource was configured successfully");
        let side = match side {
            ResourceKind::CurrentOrOurs => &self.current,
            ResourceKind::OtherOrTheirs => &self.other,
            ResourceKind::CommonAncestorOrBase => unreachable!("the side must differ from the base"),
        }
        .as_ref()
        .expect("the side resource was configured successfully");
        Ok(base.data == Some(crate::blob::pipeline::Data::Buffer)
            && side.data == Some(crate::blob::pipeline::Data::Buffer)
            && base.buffer == side.buffer)
    }

    /// Store enough information about a resource to eventually use it in a merge, where…
    ///
    /// * `id` is the hash of the resource. If it is `None`, it should either
    ///   be a resource in the worktree, or it's considered a non-existing, deleted object.
    ///   If an `id` is known, as the hash of the object as (would) be stored in `git`, then it should be provided
    ///   for completeness. Note that it's not expected to be in `objects` if `rela_path` is set and a worktree-root
    ///   is available for `kind`.
    ///   Null IDs in `Some` are treated as `None`.
    /// * `mode` is the kind of object (only blobs and executables are allowed)
    /// * `rela_path` is the relative path as seen from the (work)tree root.
    /// * `kind` identifies the side of the merge this resource will be used for.
    /// * `objects` provides access to the object database in case the resource can't be read from a worktree.
    ///
    /// If setup fails, the resource of `kind` is unset. Other resources remain available, but preparing a merge
    /// requires successfully setting the failed resource again.
    pub fn set_resource(
        &mut self,
        id: Option<gix_hash::ObjectId>,
        mode: gix_object::tree::EntryKind,
        rela_path: &BStr,
        kind: ResourceKind,
        objects: &impl gix_object::FindObjectOrHeader,
    ) -> Result {
        let storage = match kind {
            ResourceKind::OtherOrTheirs => &mut self.other,
            ResourceKind::CommonAncestorOrBase => &mut self.ancestor,
            ResourceKind::CurrentOrOurs => &mut self.current,
        };
        let mut buffer = storage.take().map_or_else(Vec::new, |resource| resource.buffer);
        if !matches!(
            mode,
            gix_object::tree::EntryKind::Blob | gix_object::tree::EntryKind::BlobExecutable
        ) {
            bail!("Can only diff blobs, not {mode:?}".validation());
        }
        let id = id.filter(|id| !id.is_null());
        let entry = self
            .attr_stack
            .at_entry(rela_path, None, objects)
            .or_raise(|| message!("Failed to obtain attributes for {kind:?} resource at '{rela_path}'"))?;

        let out = self
            .filter
            .convert_to_mergeable(
                id.as_deref(),
                mode,
                rela_path,
                kind,
                &mut |_, out| {
                    let _ = entry.matching_attributes(out);
                },
                objects,
                self.filter_mode,
                &mut buffer,
            )
            .or_raise(|| message("Failed to convert resource to mergeable form"))?;

        *storage = Some(Resource {
            id,
            rela_path: rela_path.to_owned(),
            data: out,
            mode,
            buffer,
        });
        Ok(())
    }
}
