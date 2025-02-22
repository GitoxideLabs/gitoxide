use bstr::{BString, ByteSlice};
use gix_glob::pattern::Case;

use crate::{stack::State, PathIdMapping};

#[cfg(feature = "attributes")]
type AttributeMatchGroup = gix_attributes::Search;
type IgnoreMatchGroup = gix_ignore::Search;

/// State related to attributes associated with files in the repository.
#[derive(Default, Clone)]
#[cfg(feature = "attributes")]
pub struct Attributes {
    /// Attribute patterns which aren't tied to the repository root, hence are global, they contribute first.
    globals: AttributeMatchGroup,
    /// Attribute patterns that match the currently set directory (in the stack).
    ///
    /// Note that the root-level file is always loaded, if present, followed by, the `$GIT_DIR/info/attributes`, if present, based
    /// on the location of the `info_attributes` file.
    stack: AttributeMatchGroup,
    /// The first time we push the root, we have to load additional information from this file if it exists along with the root attributes
    /// file if possible, and keep them there throughout.
    info_attributes: Option<std::path::PathBuf>,
    /// A lookup table to accelerate searches.
    collection: gix_attributes::search::MetadataCollection,
    /// Where to read `.gitattributes` data from.
    source: attributes::Source,
}

/// State related to the exclusion of files, supporting static overrides and globals, along with a stack of dynamically read
/// ignore files from disk or from the index each time the directory changes.
#[derive(Default, Clone)]
#[allow(unused)]
pub struct Ignore {
    /// Ignore patterns passed as overrides to everything else, typically passed on the command-line and the first patterns to
    /// be consulted.
    overrides: IgnoreMatchGroup,
    /// Ignore patterns that match the currently set director (in the stack), which is pushed and popped as needed.
    stack: IgnoreMatchGroup,
    /// Ignore patterns which aren't tied to the repository root, hence are global. They are consulted last.
    globals: IgnoreMatchGroup,
    /// A matching stack of pattern indices which is empty if we have just been initialized to indicate that the
    /// currently set directory had a pattern matched. Note that this one could be negated.
    /// (index into match groups, index into list of pattern lists, index into pattern list)
    matched_directory_patterns_stack: Vec<Option<(usize, usize, usize)>>,
    ///  The name of the file to look for in directories.
    pub(crate) exclude_file_name_for_directories: BString,
    /// Where to read ignore files from
    source: ignore::Source,
}

///
#[cfg(feature = "attributes")]
pub mod attributes;
///
pub mod ignore;

/// Initialization
impl State {
    /// Configure a state to be suitable for checking out files, which only needs access to attribute files read from the index.
    #[cfg(feature = "attributes")]
    pub fn for_checkout(
        unlink_on_collision: bool,
        validate: gix_validate::path::component::Options,
        attributes: Attributes,
    ) -> Self {
        State::CreateDirectoryAndAttributesStack {
            unlink_on_collision,
            validate,
            attributes,
        }
    }

    /// Configure a state for adding files, with support for ignore files and attribute files.
    #[cfg(feature = "attributes")]
    pub fn for_add(attributes: Attributes, ignore: Ignore) -> Self {
        State::AttributesAndIgnoreStack { attributes, ignore }
    }
}

/// Utilities
impl State {
    /// Returns a vec of tuples of relative index paths along with the best usable blob OID for
    /// either *ignore* or *attribute* files or both. This allows files to be accessed directly from
    /// the object database without the need for a worktree checkout.
    ///
    /// Note that this method…
    /// - ignores entries which aren't blobs.
    /// - ignores ignore entries which are not skip-worktree.
    /// - within merges, picks 'our' stage both for *ignore* and *attribute* files.
    ///
    /// * `index` is where we look for suitable files by path in order to obtain their blob hash.
    /// * `paths` is the indices storage backend for paths.
    /// * `case` determines if the search for files should be case-sensitive or not.
    pub fn id_mappings_from_index(
        &self,
        index: &gix_index::State,
        paths: &gix_index::PathStorageRef,
        case: Case,
    ) -> Vec<PathIdMapping> {
        let a1_backing;
        #[cfg(feature = "attributes")]
        let a2_backing;
        let names = match self {
            State::IgnoreStack(ignore) => {
                a1_backing = [(
                    ignore.exclude_file_name_for_directories.as_bytes().as_bstr(),
                    Some(ignore.source),
                )];
                a1_backing.as_ref()
            }
            #[cfg(feature = "attributes")]
            State::AttributesAndIgnoreStack { ignore, .. } => {
                a2_backing = [
                    (
                        ignore.exclude_file_name_for_directories.as_bytes().as_bstr(),
                        Some(ignore.source),
                    ),
                    (".gitattributes".into(), None),
                ];
                a2_backing.as_ref()
            }
            #[cfg(feature = "attributes")]
            State::CreateDirectoryAndAttributesStack { .. } | State::AttributesStack(_) => {
                a1_backing = [(".gitattributes".into(), None)];
                a1_backing.as_ref()
            }
        };

        index
            .entries()
            .iter()
            .filter_map(move |entry| {
                let path = entry.path_in(paths);

                // Stage 0 means there is no merge going on, stage 2 means it's 'our' side of the merge, but then
                // there won't be a stage 0.
                if entry.mode == gix_index::entry::Mode::FILE && (entry.stage_raw() == 0 || entry.stage_raw() == 2) {
                    let basename = path.rfind_byte(b'/').map_or(path, |pos| path[pos + 1..].as_bstr());
                    let ignore_source = names.iter().find_map(|t| {
                        match case {
                            Case::Sensitive => basename == t.0,
                            Case::Fold => basename.eq_ignore_ascii_case(t.0),
                        }
                        .then_some(t.1)
                    })?;
                    if let Some(source) = ignore_source {
                        match source {
                            ignore::Source::IdMapping => {}
                            ignore::Source::WorktreeThenIdMappingIfNotSkipped => {
                                // See https://github.com/git/git/blob/master/dir.c#L912:L912
                                if !entry.flags.contains(gix_index::entry::Flags::SKIP_WORKTREE) {
                                    return None;
                                }
                            }
                        }
                    }
                    Some((path.to_owned(), entry.id))
                } else {
                    None
                }
            })
            .collect()
    }

    pub(crate) fn ignore_or_panic(&self) -> &Ignore {
        match self {
            State::IgnoreStack(v) => v,
            #[cfg(feature = "attributes")]
            State::AttributesAndIgnoreStack { ignore, .. } => ignore,
            #[cfg(feature = "attributes")]
            State::AttributesStack(_) | State::CreateDirectoryAndAttributesStack { .. } => {
                unreachable!("BUG: must not try to check excludes without it being setup")
            }
        }
    }

    #[cfg(feature = "attributes")]
    pub(crate) fn attributes_or_panic(&self) -> &Attributes {
        match self {
            State::AttributesStack(attributes)
            | State::AttributesAndIgnoreStack { attributes, .. }
            | State::CreateDirectoryAndAttributesStack { attributes, .. } => attributes,
            State::IgnoreStack(_) => {
                unreachable!("BUG: must not try to check excludes without it being setup")
            }
        }
    }
}
