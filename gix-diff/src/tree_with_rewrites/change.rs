use crate::blob::{DiffLineStats, ResourceKind};
use crate::tree;
use bstr::BString;
use bstr::{BStr, ByteSlice};

/// Represents any possible change in order to turn one tree into another, which references data owned by its producer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChangeRef<'a> {
    /// An entry was added, like the addition of a file or directory.
    Addition {
        /// The location of the file or directory.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        location: &'a BStr,
        /// The mode of the added entry.
        entry_mode: gix_object::tree::EntryMode,
        /// Identifies a relationship between this instance and another one,
        /// making it easy to reconstruct the top-level of directory changes.
        relation: Option<tree::visit::Relation>,
        /// The object id of the added entry.
        id: gix_hash::ObjectId,
    },
    /// An entry was deleted, like the deletion of a file or directory.
    Deletion {
        /// The location of the file or directory.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        /// are tracked.
        location: &'a BStr,
        /// The mode of the deleted entry.
        entry_mode: gix_object::tree::EntryMode,
        /// Identifies a relationship between this instance and another one,
        /// making it easy to reconstruct the top-level of directory changes.
        relation: Option<tree::visit::Relation>,
        /// The object id of the deleted entry.
        id: gix_hash::ObjectId,
    },
    /// An entry was modified, e.g. changing the contents of a file adjusts its object id and turning
    /// a file into a symbolic link adjusts its mode.
    Modification {
        /// The location of the file or directory.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        /// are tracked.
        location: &'a BStr,
        /// The mode of the entry before the modification.
        previous_entry_mode: gix_object::tree::EntryMode,
        /// The object id of the entry before the modification.
        previous_id: gix_hash::ObjectId,

        /// The mode of the entry after the modification.
        entry_mode: gix_object::tree::EntryMode,
        /// The object id after the modification.
        id: gix_hash::ObjectId,
    },
    /// Entries are considered rewritten if they are not trees and they, according to some understanding of identity, were renamed
    /// or copied.
    /// In case of renames, this means they originally appeared as [`Deletion`](ChangeRef::Deletion) signalling their source as well as an
    /// [`Addition`](ChangeRef::Addition) acting as destination.
    ///
    /// In case of copies, the `copy` flag is true and typically represents a perfect copy of a source was made.
    ///
    /// This variant can only be encountered if [rewrite tracking](super::Options::rewrites) is enabled.
    ///
    /// Note that mode changes may have occurred as well, i.e. changes from executable to non-executable or vice-versa.
    Rewrite {
        /// The location of the source of the rename or copy operation.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        /// are tracked.
        source_location: &'a BStr,
        /// The mode of the entry before the rename.
        source_entry_mode: gix_object::tree::EntryMode,
        /// Identifies a relationship between the source and another source,
        /// making it easy to reconstruct the top-level of directory changes.
        source_relation: Option<tree::visit::Relation>,
        /// The object id of the entry before the rename.
        ///
        /// Note that this is the same as `id` if we require the [similarity to be 100%](super::Rewrites::percentage), but may
        /// be different otherwise.
        source_id: gix_hash::ObjectId,
        /// Information about the diff we performed to detect similarity and match the `source_id` with the current state at `id`.
        /// It's `None` if `source_id` is equal to `id`, as identity made an actual diff computation unnecessary.
        diff: Option<DiffLineStats>,
        /// The mode of the entry after the rename.
        /// It could differ but still be considered a rename as we are concerned only about content.
        entry_mode: gix_object::tree::EntryMode,
        /// The object id after the rename.
        id: gix_hash::ObjectId,
        /// The location after the rename or copy operation.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        location: &'a BStr,
        /// Identifies a relationship between this destination and another destination,
        /// making it easy to reconstruct the top-level of directory changes.
        relation: Option<tree::visit::Relation>,
        /// If true, this rewrite is created by copy, and `source_id` is pointing to its source. Otherwise, it's a rename, and `source_id`
        /// points to a deleted object, as renames are tracked as deletions and additions of the same or similar content.
        copy: bool,
    },
}

/// Represents any possible change in order to turn one tree into another, with fully-owned data.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// An entry was added, like the addition of a file or directory.
    Addition {
        /// The location of the file or directory.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        location: BString,
        /// Identifies a relationship between this instance and another one,
        /// making it easy to reconstruct the top-level of directory changes.
        relation: Option<tree::visit::Relation>,
        /// The mode of the added entry.
        entry_mode: gix_object::tree::EntryMode,
        /// The object id of the added entry.
        id: gix_hash::ObjectId,
    },
    /// An entry was deleted, like the deletion of a file or directory.
    Deletion {
        /// The location of the file or directory.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        location: BString,
        /// Identifies a relationship between this instance and another one,
        /// making it easy to reconstruct the top-level of directory changes.
        relation: Option<tree::visit::Relation>,
        /// The mode of the deleted entry.
        entry_mode: gix_object::tree::EntryMode,
        /// The object id of the deleted entry.
        id: gix_hash::ObjectId,
    },
    /// An entry was modified, e.g. changing the contents of a file adjusts its object id and turning
    /// a file into a symbolic link adjusts its mode.
    Modification {
        /// The location of the file or directory.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        location: BString,
        /// The mode of the entry before the modification.
        previous_entry_mode: gix_object::tree::EntryMode,
        /// The object id of the entry before the modification.
        previous_id: gix_hash::ObjectId,

        /// The mode of the entry after the modification.
        entry_mode: gix_object::tree::EntryMode,
        /// The object id after the modification.
        id: gix_hash::ObjectId,
    },
    /// Entries are considered rewritten if they are not trees and they, according to some understanding of identity, were renamed
    /// or copied.
    /// In case of renames, this means they originally appeared as [`Deletion`](ChangeRef::Deletion) signalling their source as well as an
    /// [`Addition`](ChangeRef::Addition) acting as destination.
    ///
    /// In case of copies, the `copy` flag is true and typically represents a perfect copy of a source was made.
    ///
    /// This variant can only be encountered if [rewrite tracking](super::Options::rewrites) is enabled.
    ///
    /// Note that mode changes may have occurred as well, i.e. changes from executable to non-executable or vice-versa.
    Rewrite {
        /// The location of the source of the rename operation.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        source_location: BString,
        /// The mode of the entry before the rename.
        source_entry_mode: gix_object::tree::EntryMode,
        /// Identifies a relationship between the source and another source,
        /// making it easy to reconstruct the top-level of directory changes.
        source_relation: Option<tree::visit::Relation>,
        /// The object id of the entry before the rename.
        ///
        /// Note that this is the same as `id` if we require the [similarity to be 100%](super::Rewrites::percentage), but may
        /// be different otherwise.
        source_id: gix_hash::ObjectId,
        /// Information about the diff we performed to detect similarity and match the `source_id` with the current state at `id`.
        /// It's `None` if `source_id` is equal to `id`, as identity made an actual diff computation unnecessary.
        diff: Option<DiffLineStats>,
        /// The mode of the entry after the rename.
        /// It could differ but still be considered a rename as we are concerned only about content.
        entry_mode: gix_object::tree::EntryMode,
        /// The object id after the rename.
        id: gix_hash::ObjectId,
        /// The location after the rename or copy operation.
        ///
        /// It may be empty if [file names](super::Options::location) is `None`.
        location: BString,
        /// Identifies a relationship between this destination and another destination,
        /// making it easy to reconstruct the top-level of directory changes.
        relation: Option<tree::visit::Relation>,
        /// If true, this rewrite is created by copy, and `source_id` is pointing to its source. Otherwise, it's a rename, and `source_id`
        /// points to a deleted object, as renames are tracked as deletions and additions of the same or similar content.
        copy: bool,
    },
}

/// Lifecycle
impl ChangeRef<'_> {
    /// Copy this instance into a fully-owned version
    pub fn into_owned(self) -> Change {
        match self {
            ChangeRef::Addition {
                location,
                entry_mode,
                id,
                relation,
            } => Change::Addition {
                location: location.to_owned(),
                entry_mode,
                id,
                relation,
            },
            ChangeRef::Deletion {
                location,
                entry_mode,
                id,
                relation,
            } => Change::Deletion {
                location: location.to_owned(),
                entry_mode,
                id,
                relation,
            },
            ChangeRef::Modification {
                location,
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
            } => Change::Modification {
                location: location.to_owned(),
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
            },
            ChangeRef::Rewrite {
                source_location,
                source_relation,
                source_entry_mode,
                source_id,
                diff,
                entry_mode,
                id,
                location,
                relation,
                copy,
            } => Change::Rewrite {
                source_location: source_location.to_owned(),
                source_relation,
                source_entry_mode,
                source_id,
                diff,
                entry_mode,
                id,
                location: location.to_owned(),
                relation,
                copy,
            },
        }
    }
}

/// Lifecycle
impl Change {
    /// Return an attached version of this instance that uses `old_repo` for previous values and `new_repo` for current values.
    pub fn to_ref(&self) -> ChangeRef<'_> {
        match self {
            Change::Addition {
                location,
                relation,
                entry_mode,
                id,
            } => ChangeRef::Addition {
                location: location.as_bstr(),
                entry_mode: *entry_mode,
                id: *id,
                relation: *relation,
            },
            Change::Deletion {
                location,
                relation,
                entry_mode,
                id,
            } => ChangeRef::Deletion {
                location: location.as_bstr(),
                entry_mode: *entry_mode,
                id: *id,
                relation: *relation,
            },
            Change::Modification {
                location,
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
            } => ChangeRef::Modification {
                location: location.as_bstr(),
                previous_entry_mode: *previous_entry_mode,
                previous_id: *previous_id,
                entry_mode: *entry_mode,
                id: *id,
            },
            Change::Rewrite {
                source_location,
                source_relation,
                source_entry_mode,
                source_id,
                diff,
                entry_mode,
                id,
                location,
                relation,
                copy,
            } => ChangeRef::Rewrite {
                source_location: source_location.as_ref(),
                source_relation: *source_relation,
                source_entry_mode: *source_entry_mode,
                source_id: *source_id,
                diff: *diff,
                entry_mode: *entry_mode,
                id: *id,
                location: location.as_bstr(),
                relation: *relation,
                copy: *copy,
            },
        }
    }
}

impl crate::blob::Platform {
    /// Set ourselves up to produces blob-diffs from `change`, so this platform can be used to produce diffs easily.
    /// `objects` are used to fetch object data as needed.
    ///
    /// ### Warning about Memory Consumption
    ///
    /// This instance only grows, so one should call [`crate::blob::Platform::clear_resource_cache`] occasionally.
    pub fn set_resource_by_change(
        &mut self,
        change: ChangeRef<'_>,
        objects: &impl gix_object::FindObjectOrHeader,
    ) -> Result<&mut Self, crate::blob::platform::set_resource::Error> {
        match change {
            ChangeRef::Addition {
                location,
                relation: _,
                entry_mode,
                id,
            } => {
                self.set_resource(
                    id.kind().null(),
                    entry_mode.kind(),
                    location,
                    ResourceKind::OldOrSource,
                    objects,
                )?;
                self.set_resource(id, entry_mode.kind(), location, ResourceKind::NewOrDestination, objects)?;
            }
            ChangeRef::Deletion {
                location,
                relation: _,
                entry_mode,
                id,
            } => {
                self.set_resource(id, entry_mode.kind(), location, ResourceKind::OldOrSource, objects)?;
                self.set_resource(
                    id.kind().null(),
                    entry_mode.kind(),
                    location,
                    ResourceKind::NewOrDestination,
                    objects,
                )?;
            }
            ChangeRef::Modification {
                location,
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
            } => {
                self.set_resource(
                    previous_id,
                    previous_entry_mode.kind(),
                    location,
                    ResourceKind::OldOrSource,
                    objects,
                )?;
                self.set_resource(id, entry_mode.kind(), location, ResourceKind::NewOrDestination, objects)?;
            }
            ChangeRef::Rewrite {
                source_location,
                source_relation: _,
                source_entry_mode,
                source_id,
                entry_mode,
                id,
                location,
                relation: _,
                diff: _,
                copy: _,
            } => {
                self.set_resource(
                    source_id,
                    source_entry_mode.kind(),
                    source_location,
                    ResourceKind::OldOrSource,
                    objects,
                )?;
                self.set_resource(id, entry_mode.kind(), location, ResourceKind::NewOrDestination, objects)?;
            }
        }
        Ok(self)
    }
}

impl<'a> ChangeRef<'a> {
    /// Return the relation this instance may have to other changes.
    pub fn relation(&self) -> Option<tree::visit::Relation> {
        match self {
            ChangeRef::Addition { relation, .. }
            | ChangeRef::Deletion { relation, .. }
            | ChangeRef::Rewrite { relation, .. } => *relation,
            ChangeRef::Modification { .. } => None,
        }
    }

    /// Return the current mode of this instance.
    pub fn entry_mode(&self) -> gix_object::tree::EntryMode {
        match self {
            ChangeRef::Addition { entry_mode, .. }
            | ChangeRef::Deletion { entry_mode, .. }
            | ChangeRef::Modification { entry_mode, .. }
            | ChangeRef::Rewrite { entry_mode, .. } => *entry_mode,
        }
    }

    /// Return the *current* location of the resource, i.e. the destination of a rename or copy, or the
    /// location at which an addition, deletion or modification took place.
    pub fn location(&self) -> &'a BStr {
        match self {
            ChangeRef::Addition { location, .. }
            | ChangeRef::Deletion { location, .. }
            | ChangeRef::Modification { location, .. }
            | ChangeRef::Rewrite { location, .. } => location,
        }
    }

    /// Return the *previous* location of the resource where possible, i.e. the source of a rename or copy, or the
    /// location at which an addition, deletion or modification took place.
    pub fn source_location(&self) -> &BStr {
        match self {
            ChangeRef::Addition { location, .. }
            | ChangeRef::Deletion { location, .. }
            | ChangeRef::Modification { location, .. } => location,
            ChangeRef::Rewrite { source_location, .. } => source_location,
        }
    }
}

impl Change {
    /// Return the relation this instance may have to other changes.
    pub fn relation(&self) -> Option<tree::visit::Relation> {
        match self {
            Change::Addition { relation, .. }
            | Change::Deletion { relation, .. }
            | Change::Rewrite { relation, .. } => *relation,
            Change::Modification { .. } => None,
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

    /// Return the *current* location of the resource, i.e. the destination of a rename or copy, or the
    /// location at which an addition, deletion or modification took place.
    pub fn location(&self) -> &BStr {
        match self {
            Change::Addition { location, .. }
            | Change::Deletion { location, .. }
            | Change::Modification { location, .. }
            | Change::Rewrite { location, .. } => location.as_bstr(),
        }
    }

    /// Return the *previous* location of the resource where possible, i.e. the source of a rename or copy, or the
    /// location at which an addition, deletion or modification took place.
    pub fn source_location(&self) -> &BStr {
        match self {
            Change::Addition { location, .. }
            | Change::Deletion { location, .. }
            | Change::Modification { location, .. } => location.as_bstr(),
            Change::Rewrite { source_location, .. } => source_location.as_bstr(),
        }
    }
}
