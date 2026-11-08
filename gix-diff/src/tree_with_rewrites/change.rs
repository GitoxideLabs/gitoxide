use bstr::{BStr, BString, ByteSlice};
use gix_error::ExnResult;

use crate::{blob::ResourceKind, rewrites::Similarity, tree};

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
    /// Entries are considered rewritten if they, according to some understanding of identity, were renamed or copied.
    /// Directory renames are inferred from completed file renames into added or existing directories.
    /// In case of renames, this means they originally appeared as [`Deletion`](ChangeRef::Deletion) signalling their source as well as an
    /// [`Addition`](ChangeRef::Addition) acting as destination, or a [`Modification`](ChangeRef::Modification) for an existing directory.
    /// Existing destination directories retain their modification event alongside the inferred rename.
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
        /// The byte-span score of an inexact match, including binary files.
        /// `None` denotes an exact identity match or an inferred directory rename.
        /// Line statistics can be computed separately with the blob diff APIs.
        similarity: Option<Similarity>,
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
    /// Entries are considered rewritten if they, according to some understanding of identity, were renamed or copied.
    /// Directory renames are inferred from completed file renames into added or existing directories.
    /// In case of renames, this means they originally appeared as [`Deletion`](ChangeRef::Deletion) signalling their source as well as an
    /// [`Addition`](ChangeRef::Addition) acting as destination, or a [`Modification`](ChangeRef::Modification) for an existing directory.
    /// Existing destination directories retain their modification event alongside the inferred rename.
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
        /// The byte-span score of an inexact match, including binary files.
        /// `None` denotes an exact identity match or an inferred directory rename.
        /// Line statistics can be computed separately with the blob diff APIs.
        similarity: Option<Similarity>,
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
                similarity,
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
                similarity,
                entry_mode,
                id,
                location: location.to_owned(),
                relation,
                copy,
            },
        }
    }
}

impl<'a> ChangeRef<'a> {
    pub(super) fn from_tree_change(change: &tree::visit::Change, location: &'a BStr) -> Self {
        match *change {
            tree::visit::Change::Addition {
                entry_mode,
                oid,
                relation,
            } => Self::Addition {
                location,
                relation,
                entry_mode,
                id: oid,
            },
            tree::visit::Change::Deletion {
                entry_mode,
                oid,
                relation,
            } => Self::Deletion {
                entry_mode,
                location,
                relation,
                id: oid,
            },
            tree::visit::Change::Modification {
                previous_entry_mode,
                previous_oid,
                entry_mode,
                oid,
            } => Self::Modification {
                location,
                previous_entry_mode,
                entry_mode,
                previous_id: previous_oid,
                id: oid,
            },
        }
    }

    pub(super) fn with_location(mut self, location: Option<tree::recorder::Location>) -> Self {
        let project = |path: &'a BStr| match location {
            None => BStr::new(b""),
            Some(tree::recorder::Location::FileName) => path.rsplit_str(b"/").next().unwrap_or_default().as_bstr(),
            Some(tree::recorder::Location::Path) => path,
        };
        if let Self::Rewrite { source_location, .. } = &mut self {
            *source_location = project(source_location);
        }
        match &mut self {
            Self::Addition { location, .. }
            | Self::Deletion { location, .. }
            | Self::Modification { location, .. }
            | Self::Rewrite { location, .. } => *location = project(location),
        }
        self
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
                similarity,
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
                similarity: *similarity,
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
    ) -> ExnResult<&mut Self, crate::blob::platform::set_resource::Error> {
        match change {
            ChangeRef::Addition {
                location,
                relation: _,
                entry_mode,
                id,
            } => {
                self.set_resource(None, entry_mode.kind(), location, ResourceKind::OldOrSource, objects)?;
                self.set_resource(
                    Some(id),
                    entry_mode.kind(),
                    location,
                    ResourceKind::NewOrDestination,
                    objects,
                )?;
            }
            ChangeRef::Deletion {
                location,
                relation: _,
                entry_mode,
                id,
            } => {
                self.set_resource(
                    Some(id),
                    entry_mode.kind(),
                    location,
                    ResourceKind::OldOrSource,
                    objects,
                )?;
                self.set_resource(
                    None,
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
                    Some(previous_id),
                    previous_entry_mode.kind(),
                    location,
                    ResourceKind::OldOrSource,
                    objects,
                )?;
                self.set_resource(
                    Some(id),
                    entry_mode.kind(),
                    location,
                    ResourceKind::NewOrDestination,
                    objects,
                )?;
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
                similarity: _,
                copy: _,
            } => {
                self.set_resource(
                    Some(source_id),
                    source_entry_mode.kind(),
                    source_location,
                    ResourceKind::OldOrSource,
                    objects,
                )?;
                self.set_resource(
                    Some(id),
                    entry_mode.kind(),
                    location,
                    ResourceKind::NewOrDestination,
                    objects,
                )?;
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

    /// Return the current mode of this instance, along with its object id.
    pub fn entry_mode_and_id(&self) -> (gix_object::tree::EntryMode, &gix_hash::oid) {
        match self {
            ChangeRef::Addition { entry_mode, id, .. }
            | ChangeRef::Deletion { entry_mode, id, .. }
            | ChangeRef::Modification { entry_mode, id, .. }
            | ChangeRef::Rewrite { entry_mode, id, .. } => (*entry_mode, id),
        }
    }

    /// Return the *previous* mode and id of the resource where possible, i.e. the source of a rename or copy, or a modification.
    pub fn source_entry_mode_and_id(&self) -> (gix_object::tree::EntryMode, &gix_hash::oid) {
        match self {
            ChangeRef::Addition { entry_mode, id, .. }
            | ChangeRef::Deletion { entry_mode, id, .. }
            | ChangeRef::Modification {
                previous_entry_mode: entry_mode,
                previous_id: id,
                ..
            }
            | ChangeRef::Rewrite {
                source_entry_mode: entry_mode,
                source_id: id,
                ..
            } => (*entry_mode, id),
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

    /// Return the current mode of this instance, along with its object id.
    pub fn entry_mode_and_id(&self) -> (gix_object::tree::EntryMode, &gix_hash::oid) {
        match self {
            Change::Addition { entry_mode, id, .. }
            | Change::Deletion { entry_mode, id, .. }
            | Change::Modification { entry_mode, id, .. }
            | Change::Rewrite { entry_mode, id, .. } => (*entry_mode, id),
        }
    }

    /// Return the *previous* mode and id of the resource where possible, i.e. the source of a rename or copy, or a modification.
    pub fn source_entry_mode_and_id(&self) -> (gix_object::tree::EntryMode, &gix_hash::oid) {
        match self {
            Change::Addition { entry_mode, id, .. }
            | Change::Deletion { entry_mode, id, .. }
            | Change::Modification {
                previous_entry_mode: entry_mode,
                previous_id: id,
                ..
            }
            | Change::Rewrite {
                source_entry_mode: entry_mode,
                source_id: id,
                ..
            } => (*entry_mode, id),
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
