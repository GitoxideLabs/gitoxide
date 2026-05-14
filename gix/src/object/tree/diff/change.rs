use super::ChangeDetached;
use crate::{
    Repository,
    bstr::{BStr, ByteSlice},
    ext::ObjectIdExt,
    object::tree::diff::Change,
};

impl Change<'_, '_, '_> {
    /// Produce a platform for performing a line-diff no matter whether the underlying [Change] is an addition, modification,
    /// deletion or rewrite.
    /// Use `resource_cache` to store the diffable data and possibly reuse previously stored data, usually obtained with
    /// [Repository::diff_resource_cache()].
    /// Afterward the platform, which holds on to `resource_cache`, can be used to perform ready-made operations on the
    /// pre-set resources.
    ///
    /// ### Warning about Memory Consumption
    ///
    /// `resource_cache` only grows, so one should call [`gix_diff::blob::Platform::clear_resource_cache`] occasionally.
    pub fn diff<'b>(
        &self,
        resource_cache: &'b mut gix_diff::blob::Platform,
    ) -> Result<crate::object::blob::diff::Platform<'b>, crate::object::blob::diff::init::Error> {
        resource_cache.set_resource_by_change((*self).into(), &self.id().repo.objects)?;
        Ok(crate::object::blob::diff::Platform { resource_cache })
    }
}

impl<'a> From<Change<'a, '_, '_>> for gix_diff::tree_with_rewrites::ChangeRef<'a> {
    fn from(value: Change<'a, '_, '_>) -> Self {
        use gix_diff::tree_with_rewrites::ChangeRef;
        match value {
            Change::Addition {
                location,
                entry_mode,
                relation,
                id,
            } => ChangeRef::Addition {
                location,
                entry_mode,
                relation,
                id: id.detach(),
            },
            Change::Deletion {
                location,
                entry_mode,
                relation,
                id,
            } => ChangeRef::Deletion {
                location,
                entry_mode,
                relation,
                id: id.detach(),
            },
            Change::Modification {
                location,
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
            } => ChangeRef::Modification {
                location,
                previous_entry_mode,
                previous_id: previous_id.detach(),
                entry_mode,
                id: id.detach(),
            },
            Change::Rewrite {
                source_location,
                source_relation,
                source_entry_mode,
                source_id,
                diff,
                entry_mode,
                location,
                id,
                relation,
                copy,
            } => ChangeRef::Rewrite {
                source_location,
                source_entry_mode,
                source_relation,
                source_id: source_id.detach(),
                diff,
                entry_mode,
                id: id.detach(),
                location,
                relation,
                copy,
            },
        }
    }
}

impl<'a, 'old, 'new> Change<'a, 'old, 'new> {
    /// Convert `change` into this instance type, attaching the `old_repo` and `new_repo` to each side respectively.
    /// Note that both repos are typically the same.
    pub fn from_change_ref(
        change: gix_diff::tree_with_rewrites::ChangeRef<'a>,
        old_repo: &'old Repository,
        new_repo: &'new Repository,
    ) -> Self {
        use gix_diff::tree_with_rewrites::ChangeRef;
        match change {
            ChangeRef::Addition {
                location,
                entry_mode,
                relation,
                id,
            } => Change::Addition {
                location,
                entry_mode,
                relation,
                id: id.attach(new_repo),
            },
            ChangeRef::Deletion {
                location,
                entry_mode,
                relation,
                id,
            } => Change::Deletion {
                location,
                entry_mode,
                relation,
                id: id.attach(old_repo),
            },
            ChangeRef::Modification {
                location,
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
            } => Change::Modification {
                location,
                previous_entry_mode,
                entry_mode,
                previous_id: previous_id.attach(old_repo),
                id: id.attach(new_repo),
            },
            ChangeRef::Rewrite {
                source_location,
                source_entry_mode,
                source_relation,
                source_id,
                diff,
                entry_mode,
                id,
                location,
                relation,
                copy,
            } => Change::Rewrite {
                source_location,
                source_relation,
                source_entry_mode,
                source_id: source_id.attach(old_repo),
                diff,
                entry_mode,
                location,
                id: id.attach(new_repo),
                relation,
                copy,
            },
        }
    }
}

/// Lifecycle
impl Change<'_, '_, '_> {
    /// Detach the repository instance to obtain a fully-owned version
    pub fn detach(self) -> ChangeDetached {
        match self {
            Change::Addition {
                entry_mode,
                id,
                location,
                relation,
            } => ChangeDetached::Addition {
                entry_mode,
                id: id.detach(),
                location: location.to_owned(),
                relation,
            },
            Change::Deletion {
                entry_mode,
                id,
                location,
                relation,
            } => ChangeDetached::Deletion {
                entry_mode,
                id: id.detach(),
                location: location.to_owned(),
                relation,
            },
            Change::Modification {
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
                location,
            } => ChangeDetached::Modification {
                previous_entry_mode,
                previous_id: previous_id.detach(),
                entry_mode,
                id: id.detach(),
                location: location.to_owned(),
            },
            Change::Rewrite {
                source_location,
                source_relation,
                source_entry_mode,
                source_id,
                diff,
                entry_mode,
                id,
                relation,
                copy,
                location,
            } => ChangeDetached::Rewrite {
                source_location: source_location.to_owned(),
                source_entry_mode,
                source_relation,
                source_id: source_id.detach(),
                diff,
                entry_mode,
                id: id.detach(),
                copy,
                location: location.to_owned(),
                relation,
            },
        }
    }
}

impl crate::ext::TreeDiffChangeExt for gix_diff::tree_with_rewrites::Change {
    fn attach<'old, 'new>(&self, old_repo: &'old Repository, new_repo: &'new Repository) -> Change<'_, 'old, 'new> {
        match self {
            ChangeDetached::Addition {
                entry_mode,
                id,
                location,
                relation,
            } => Change::Addition {
                entry_mode: *entry_mode,
                id: id.attach(new_repo),
                location: location.as_bstr(),
                relation: *relation,
            },
            ChangeDetached::Deletion {
                entry_mode,
                id,
                location,
                relation,
            } => Change::Deletion {
                entry_mode: *entry_mode,
                id: id.attach(old_repo),
                location: location.as_bstr(),
                relation: *relation,
            },
            ChangeDetached::Modification {
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
                location,
            } => Change::Modification {
                previous_entry_mode: *previous_entry_mode,
                previous_id: previous_id.attach(old_repo),
                entry_mode: *entry_mode,
                id: id.attach(new_repo),
                location: location.as_bstr(),
            },
            ChangeDetached::Rewrite {
                source_location,
                source_relation,
                source_entry_mode,
                source_id,
                diff,
                entry_mode,
                id,
                copy,
                location,
                relation,
            } => Change::Rewrite {
                source_location: source_location.as_ref(),
                source_relation: *source_relation,
                source_entry_mode: *source_entry_mode,
                source_id: source_id.attach(old_repo),
                diff: *diff,
                entry_mode: *entry_mode,
                id: id.attach(new_repo),
                copy: *copy,
                relation: *relation,
                location: location.as_bstr(),
            },
        }
    }
}

impl Change<'_, '_, '_> {
    /// Return the current ID of the change.
    pub fn id(&self) -> crate::Id<'_> {
        match self {
            Change::Addition { id, .. }
            | Change::Deletion { id, .. }
            | Change::Modification { id, .. }
            | Change::Rewrite { id, .. } => *id,
        }
    }

    /// Return the location of this instance.
    pub fn location(&self) -> &BStr {
        match self {
            Change::Addition { location, .. }
            | Change::Deletion { location, .. }
            | Change::Modification { location, .. }
            | Change::Rewrite { location, .. } => location.as_bstr(),
        }
    }

    /// Return the current mode of this instance.
    pub fn entry_mode(&self) -> gix_object::tree::EntryMode {
        match self {
            Change::Addition { entry_mode, .. }
            | Change::Deletion { entry_mode, .. }
            | Change::Modification { entry_mode, .. }
            | Change::Rewrite { entry_mode, .. } => *entry_mode,
        }
    }
}
