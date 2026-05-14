use std::path::{Path, PathBuf};

use bstr::{BStr, ByteSlice};
use gix_glob::pattern::Case;
use gix_object::FindExt;

use crate::{
    PathIdMapping, Stack,
    stack::state::{AttributeMatchGroup, Attributes},
};

/// Various aggregate numbers related [`Attributes`].
#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Statistics {
    /// Amount of patterns buffers read from the index.
    pub patterns_buffers: usize,
    /// Amount of pattern files read from disk.
    pub pattern_files: usize,
    /// Amount of pattern files we tried to find on disk.
    pub tried_pattern_files: usize,
}

/// Decide where to read `.gitattributes` files from.
///
/// To Retrieve attribute files from id mappings, see
/// [State::id_mappings_from_index()][crate::stack::State::id_mappings_from_index()].
///
/// These mappings are typically produced from an index.
/// If a tree should be the source, build an attribute list from a tree instead, or convert a tree to an index.
///
#[derive(Default, Debug, Clone, Copy)]
pub enum Source {
    /// Use this when no worktree checkout is available, like in bare repositories, during clones, or when accessing blobs from
    /// other parts of the history which aren't checked out.
    #[default]
    IdMapping,
    /// Read from an id mappings and if not present, read from the worktree.
    ///
    /// This us typically used when *checking out* files.
    IdMappingThenWorktree,
    /// Read from the worktree and if not present, read them from the id mappings.
    ///
    /// This is typically used when *checking in* files, and it's possible for sparse worktrees not to have a `.gitattribute` file
    /// checked out even though it's available in the index.
    WorktreeThenIdMapping,
}

impl Source {
    /// Returns non-worktree variants of `self` if `is_bare` is true.
    pub fn adjust_for_bare(self, is_bare: bool) -> Self {
        if is_bare { Source::IdMapping } else { self }
    }
}

/// Initialization
impl Attributes {
    /// Create a new instance from an attribute match group that represents `globals`. It can more easily be created with
    /// [`AttributeMatchGroup::new_globals()`].
    ///
    /// * `globals` contribute first and consist of all globally available, static files.
    /// * `info_attributes` is a path that should refer to `.git/info/attributes`, and it's not an error if the file doesn't exist.
    /// * `case` is used to control case-sensitivity during matching.
    /// * `source` specifies from where the directory-based attribute files should be loaded from.
    pub fn new(
        globals: AttributeMatchGroup,
        info_attributes: Option<PathBuf>,
        source: Source,
        collection: gix_attributes::search::MetadataCollection,
    ) -> Self {
        Attributes {
            globals,
            stack: Default::default(),
            info_attributes,
            source,
            collection,
        }
    }
}

impl Attributes {
    pub(crate) fn pop_directory(&mut self) {
        self.stack.pop_pattern_list().expect("something to pop");
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_directory(
        &mut self,
        root: &Path,
        dir: &Path,
        rela_dir: &BStr,
        buf: &mut Vec<u8>,
        id_mappings: &[PathIdMapping],
        objects: &dyn gix_object::Find,
        stats: &mut Statistics,
    ) -> std::io::Result<()> {
        let attr_path_relative = gix_path::join_bstr_unix_pathsep(rela_dir, ".gitattributes");
        let attr_file_in_index = id_mappings.binary_search_by(|t| t.0.as_bstr().cmp(attr_path_relative.as_ref()));
        // Git does not follow symbolic links as per documentation.
        let no_follow_symlinks = false;
        let read_macros_as_dir_is_root = root == dir;

        let mut added = false;
        match self.source {
            Source::IdMapping | Source::IdMappingThenWorktree => {
                if let Ok(idx) = attr_file_in_index {
                    let blob = objects
                        .find_blob(&id_mappings[idx].1, buf)
                        .map_err(std::io::Error::other)?;
                    let attr_path = gix_path::from_bstring(attr_path_relative.into_owned());
                    self.stack.add_patterns_buffer(
                        blob.data,
                        attr_path,
                        Some(Path::new("")),
                        &mut self.collection,
                        read_macros_as_dir_is_root,
                    );
                    added = true;
                    stats.patterns_buffers += 1;
                }
                if !added && matches!(self.source, Source::IdMappingThenWorktree) {
                    added = self.stack.add_patterns_file(
                        dir.join(".gitattributes"),
                        no_follow_symlinks,
                        Some(root),
                        buf,
                        &mut self.collection,
                        read_macros_as_dir_is_root,
                    )?;
                    stats.pattern_files += usize::from(added);
                    stats.tried_pattern_files += 1;
                }
            }
            Source::WorktreeThenIdMapping => {
                added = self.stack.add_patterns_file(
                    dir.join(".gitattributes"),
                    no_follow_symlinks,
                    Some(root),
                    buf,
                    &mut self.collection,
                    read_macros_as_dir_is_root,
                )?;
                stats.pattern_files += usize::from(added);
                stats.tried_pattern_files += 1;
                if let Some(idx) = attr_file_in_index.ok().filter(|_| !added) {
                    let blob = objects
                        .find_blob(&id_mappings[idx].1, buf)
                        .map_err(std::io::Error::other)?;
                    let attr_path = gix_path::from_bstring(attr_path_relative.into_owned());
                    self.stack.add_patterns_buffer(
                        blob.data,
                        attr_path,
                        Some(Path::new("")),
                        &mut self.collection,
                        read_macros_as_dir_is_root,
                    );
                    added = true;
                    stats.patterns_buffers += 1;
                }
            }
        }

        // Need one stack level per component so push and pop matches, but only if this isn't the root level which is never popped.
        if !added && self.info_attributes.is_none() {
            self.stack
                .add_patterns_buffer(&[], "<empty dummy>".into(), None, &mut self.collection, true);
        }

        // When reading the root, always the first call, we can try to also read the `.git/info/attributes` file which is
        // by nature never popped, and follows the root, as global.
        if let Some(info_attr) = self.info_attributes.take() {
            let added = self.stack.add_patterns_file(
                info_attr,
                true,
                None,
                buf,
                &mut self.collection,
                true, /* read macros */
            )?;
            stats.pattern_files += usize::from(added);
            stats.tried_pattern_files += 1;
        }

        Ok(())
    }

    pub(crate) fn matching_attributes(
        &self,
        relative_path: &BStr,
        case: Case,
        is_dir: Option<bool>,
        out: &mut gix_attributes::search::Outcome,
    ) -> bool {
        // assure `out` is ready to deal with possibly changed collections (append-only)
        out.initialize(&self.collection);

        let groups = [&self.globals, &self.stack];
        let mut has_match = false;
        groups.iter().rev().any(|group| {
            has_match |= group.pattern_matching_relative_path(relative_path, case, is_dir, out);
            out.is_done()
        });
        has_match
    }
}

/// Attribute matching specific methods
impl Stack {
    /// Creates a new container to store match outcomes for all attribute matches.
    ///
    /// ### Panics
    ///
    /// If attributes aren't configured.
    pub fn attribute_matches(&self) -> gix_attributes::search::Outcome {
        let mut out = gix_attributes::search::Outcome::default();
        out.initialize(&self.state.attributes_or_panic().collection);
        out
    }

    /// Creates a new container to store match outcomes for the given attributes.
    ///
    /// ### Panics
    ///
    /// If attributes aren't configured.
    pub fn selected_attribute_matches<'a>(
        &self,
        given: impl IntoIterator<Item = impl Into<&'a str>>,
    ) -> gix_attributes::search::Outcome {
        let mut out = gix_attributes::search::Outcome::default();
        out.initialize_with_selection(
            &self.state.attributes_or_panic().collection,
            given.into_iter().map(Into::into),
        );
        out
    }

    /// Return the metadata collection that enables initializing attribute match outcomes as done in
    /// [`attribute_matches()`][Stack::attribute_matches()] or [`selected_attribute_matches()`][Stack::selected_attribute_matches()]
    ///
    /// ### Panics
    ///
    /// If attributes aren't configured.
    pub fn attributes_collection(&self) -> &gix_attributes::search::MetadataCollection {
        &self.state.attributes_or_panic().collection
    }
}
