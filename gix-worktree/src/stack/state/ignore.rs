use std::path::Path;

use bstr::{BStr, ByteSlice};
use gix_glob::pattern::Case;
use gix_object::FindExt;

use crate::{
    PathIdMapping,
    stack::state::{Ignore, IgnoreMatchGroup},
};

/// Specify how to parse ignore patterns.
pub use gix_ignore::search::Ignore as ParseIgnore;

/// Decide where to read `.gitignore` files from.
#[derive(Default, Debug, Clone, Copy)]
pub enum Source {
    /// Retrieve ignore files from id mappings, see
    /// [State::id_mappings_from_index()][crate::stack::State::id_mappings_from_index()].
    ///
    /// These mappings are typically produced from an index.
    /// If a tree should be the source, build an attribute list from a tree instead, or convert a tree to an index.
    ///
    /// Use this when no worktree checkout is available, like in bare repositories or when accessing blobs from other parts
    /// of the history which aren't checked out.
    IdMapping,
    /// Read from the worktree and if not present, read them from the id mappings *if* these don't have the skip-worktree bit set.
    #[default]
    WorktreeThenIdMappingIfNotSkipped,
}

impl Source {
    /// Returns non-worktree variants of `self` if `is_bare` is true.
    pub fn adjust_for_bare(self, is_bare: bool) -> Self {
        if is_bare { Source::IdMapping } else { self }
    }
}

/// Various aggregate numbers related [`Ignore`].
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

impl Ignore {
    /// Configure gitignore file matching by providing the immutable groups being `overrides` and `globals`, while letting the directory
    /// stack be dynamic.
    ///
    /// The `exclude_file_name_for_directories` is an optional override for the filename to use when checking per-directory
    /// ignore files within the repository, defaults to`.gitignore`.
    ///
    /// `parse` controls how to parse ignore files.
    pub fn new(
        overrides: IgnoreMatchGroup,
        globals: IgnoreMatchGroup,
        exclude_file_name_for_directories: Option<&BStr>,
        source: Source,
        parse: gix_ignore::search::Ignore,
    ) -> Self {
        Ignore {
            overrides,
            globals,
            stack: Default::default(),
            matched_directory_patterns_stack: Vec::with_capacity(6),
            exclude_file_name_for_directories: exclude_file_name_for_directories
                .map_or_else(|| ".gitignore".into(), ToOwned::to_owned),
            source,
            parse,
        }
    }
}

impl Ignore {
    pub(crate) fn pop_directory(&mut self) {
        self.matched_directory_patterns_stack.pop().expect("something to pop");
        self.stack.patterns.pop().expect("something to pop");
    }
    /// The match groups from lowest priority to highest.
    pub(crate) fn match_groups(&self) -> [&IgnoreMatchGroup; 3] {
        [&self.globals, &self.stack, &self.overrides]
    }

    pub(crate) fn matching_exclude_pattern(
        &self,
        relative_path: &BStr,
        is_dir: Option<bool>,
        case: Case,
    ) -> Option<gix_ignore::search::Match<'_>> {
        let groups = self.match_groups();
        let mut dir_match = None;
        if let Some((source, mapping)) = self
            .matched_directory_patterns_stack
            .iter()
            .rev()
            .filter_map(|v| *v)
            .map(|(gidx, plidx, pidx)| {
                let list = &groups[gidx].patterns[plidx];
                (list.source.as_deref(), &list.patterns[pidx])
            })
            .next()
        {
            let match_ = gix_ignore::search::Match {
                pattern: &mapping.pattern,
                sequence_number: mapping.sequence_number,
                kind: mapping.value,
                source,
            };
            if mapping.pattern.is_negative() {
                dir_match = Some(match_);
            } else {
                // Note that returning here is wrong if this pattern _was_ preceded by a negative pattern that
                // didn't match the directory, but would match now.
                // Git does it similarly so we do too even though it's incorrect.
                // To fix this, one would probably keep track of whether there was a preceding negative pattern, and
                // if so we check the path in full and only use the dir match if there was no match, similar to the negative
                // case above whose fix fortunately won't change the overall result.
                return match_.into();
            }
        }
        groups
            .iter()
            .rev()
            .find_map(|group| group.pattern_matching_relative_path(relative_path, is_dir, case))
            .or(dir_match)
    }

    /// Like `matching_exclude_pattern()` but without checking if the current directory is excluded.
    /// It returns a triple-index into our data structure from which a match can be reconstructed.
    pub(crate) fn matching_exclude_pattern_no_dir(
        &self,
        relative_path: &BStr,
        is_dir: Option<bool>,
        case: Case,
    ) -> Option<(usize, usize, usize)> {
        let groups = self.match_groups();
        groups.iter().enumerate().rev().find_map(|(gidx, group)| {
            let basename_pos = relative_path.rfind(b"/").map(|p| p + 1);
            group
                .patterns
                .iter()
                .enumerate()
                .rev()
                .find_map(|(plidx, pl)| {
                    gix_ignore::search::pattern_idx_matching_relative_path(
                        pl,
                        relative_path,
                        basename_pos,
                        is_dir,
                        case,
                    )
                    .map(|idx| (plidx, idx))
                })
                .map(|(plidx, pidx)| (gidx, plidx, pidx))
        })
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
        case: Case,
        stats: &mut Statistics,
    ) -> std::io::Result<()> {
        self.matched_directory_patterns_stack
            .push(self.matching_exclude_pattern_no_dir(rela_dir, Some(true), case));

        let ignore_path_relative = gix_path::join_bstr_unix_pathsep(rela_dir, ".gitignore");
        let ignore_file_in_index = id_mappings.binary_search_by(|t| t.0.as_bstr().cmp(ignore_path_relative.as_ref()));
        match self.source {
            Source::IdMapping => {
                match ignore_file_in_index {
                    Ok(idx) => {
                        let ignore_blob = objects
                            .find_blob(&id_mappings[idx].1, buf)
                            .map_err(std::io::Error::other)?;
                        let ignore_path = gix_path::from_bstring(ignore_path_relative.into_owned());
                        self.stack
                            .add_patterns_buffer(ignore_blob.data, ignore_path, Some(Path::new("")), self.parse);
                        stats.patterns_buffers += 1;
                    }
                    Err(_) => {
                        // Need one stack level per component so push and pop matches.
                        self.stack.patterns.push(Default::default());
                    }
                }
            }
            Source::WorktreeThenIdMappingIfNotSkipped => {
                let follow_symlinks = ignore_file_in_index.is_err();
                let added = gix_glob::search::add_patterns_file(
                    &mut self.stack.patterns,
                    dir.join(".gitignore"),
                    follow_symlinks,
                    Some(root),
                    buf,
                    self.parse,
                )?;
                stats.pattern_files += usize::from(added);
                stats.tried_pattern_files += 1;
                if !added {
                    match ignore_file_in_index {
                        Ok(idx) => {
                            let ignore_blob = objects
                                .find_blob(&id_mappings[idx].1, buf)
                                .map_err(std::io::Error::other)?;
                            let ignore_path = gix_path::from_bstring(ignore_path_relative.into_owned());
                            self.stack.add_patterns_buffer(
                                ignore_blob.data,
                                ignore_path,
                                Some(Path::new("")),
                                self.parse,
                            );
                            stats.patterns_buffers += 1;
                        }
                        Err(_) => {
                            // Need one stack level per component so push and pop matches.
                            self.stack.patterns.push(Default::default());
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
