use std::path::{Path, PathBuf};

use bstr::{BStr, ByteSlice};
use gix_glob::search::{Pattern, pattern};

use super::Attributes;
use crate::{
    Search,
    search::{Assignments, MetadataCollection, Outcome, TrackedAssignment, Value},
};

/// Instantiation and initialization.
impl Search {
    /// Create a search instance preloaded with *built-ins* followed by attribute `files` from various global locations.
    ///
    /// See [`Source`][crate::Source] for a way to obtain these paths.
    ///
    /// Note that parsing is lenient and errors are logged.
    ///
    /// * `buf` is used to read `files` from disk which will be ignored if they do not exist.
    /// * `collection` will be updated with information necessary to perform lookups later.
    pub fn new_globals(
        files: impl IntoIterator<Item = impl Into<PathBuf>>,
        buf: &mut Vec<u8>,
        collection: &mut MetadataCollection,
    ) -> std::io::Result<Self> {
        let mut group = Self::default();
        group.add_patterns_buffer(
            b"[attr]binary -diff -merge -text",
            "[builtin]".into(),
            None,
            collection,
            true, /* allow macros */
        );

        for path in files.into_iter() {
            group.add_patterns_file(path.into(), true, None, buf, collection, true /* allow macros */)?;
        }
        Ok(group)
    }
}

/// Mutation
impl Search {
    /// Add the given file at `source` to our patterns if it exists, otherwise do nothing.
    /// Update `collection` with newly added attribute names.
    /// If a `root` is provided, it's not considered a global file anymore.
    /// If `allow_macros` is `true`, macros will be processed like normal, otherwise they will be skipped entirely.
    /// Returns `true` if the file was added, or `false` if it didn't exist.
    pub fn add_patterns_file(
        &mut self,
        source: PathBuf,
        follow_symlinks: bool,
        root: Option<&Path>,
        buf: &mut Vec<u8>,
        collection: &mut MetadataCollection,
        allow_macros: bool,
    ) -> std::io::Result<bool> {
        // TODO: should `Pattern` trait use an instance as first argument to carry this information
        //       (so no `retain` later, it's slower than skipping)
        let was_added =
            gix_glob::search::add_patterns_file(&mut self.patterns, source, follow_symlinks, root, buf, Attributes)?;
        if was_added {
            let last = self.patterns.last_mut().expect("just added");
            if !allow_macros {
                last.patterns
                    .retain(|p| !matches!(p.value, Value::MacroAssignments { .. }));
            }
            collection.update_from_list(last);
        }
        Ok(was_added)
    }
    /// Add patterns as parsed from `bytes`, providing their `source` path and possibly their `root` path, the path they
    /// are relative to. This also means that `source` is contained within `root` if `root` is provided.
    /// If `allow_macros` is `true`, macros will be processed like normal, otherwise they will be skipped entirely.
    pub fn add_patterns_buffer(
        &mut self,
        bytes: &[u8],
        source: PathBuf,
        root: Option<&Path>,
        collection: &mut MetadataCollection,
        allow_macros: bool,
    ) {
        self.patterns
            .push(pattern::List::from_bytes(bytes, source, root, Attributes));
        let last = self.patterns.last_mut().expect("just added");
        if !allow_macros {
            last.patterns
                .retain(|p| !matches!(p.value, Value::MacroAssignments { .. }));
        }
        collection.update_from_list(last);
    }

    /// Pop the last attribute patterns list from our queue.
    pub fn pop_pattern_list(&mut self) -> Option<gix_glob::search::pattern::List<Attributes>> {
        self.patterns.pop()
    }
}

/// Access and matching
impl Search {
    /// Match `relative_path`, a path relative to the repository, while respective `case`-sensitivity and write them to `out`
    /// Return `true` if at least one pattern matched.
    pub fn pattern_matching_relative_path(
        &self,
        relative_path: &BStr,
        case: gix_glob::pattern::Case,
        is_dir: Option<bool>,
        out: &mut Outcome,
    ) -> bool {
        let basename_pos = relative_path.rfind(b"/").map(|p| p + 1);
        let mut has_match = false;
        self.patterns.iter().rev().any(|pl| {
            has_match |= pattern_matching_relative_path(pl, relative_path, basename_pos, case, is_dir, out);
            out.is_done()
        });
        has_match
    }

    /// Return the amount of pattern lists contained in this instance.
    pub fn num_pattern_lists(&self) -> usize {
        self.patterns.len()
    }
}

impl Pattern for Attributes {
    type Value = Value;

    fn bytes_to_patterns(&self, bytes: &[u8], _source: &std::path::Path) -> Vec<pattern::Mapping<Self::Value>> {
        fn into_owned_assignments<'a>(
            attrs: impl Iterator<Item = Result<crate::AssignmentRef<'a>, crate::name::Error>>,
        ) -> Option<Assignments> {
            let res = attrs
                .map(|res| {
                    res.map(|a| TrackedAssignment {
                        id: Default::default(),
                        inner: a.to_owned(),
                    })
                })
                .collect::<Result<Assignments, _>>();
            match res {
                Ok(res) => Some(res),
                Err(_err) => {
                    gix_trace::warn!("{}", _err);
                    None
                }
            }
        }

        crate::parse(bytes)
            .filter_map(|res| match res {
                Ok(pattern) => Some(pattern),
                Err(_err) => {
                    gix_trace::warn!("{}: {}", _source.display(), _err);
                    None
                }
            })
            .filter_map(|(pattern_kind, assignments, line_number)| {
                let (pattern, value) = match pattern_kind {
                    crate::parse::Kind::Macro(macro_name) => (
                        gix_glob::Pattern {
                            text: macro_name.as_str().into(),
                            mode: macro_mode(),
                            first_wildcard_pos: None,
                        },
                        Value::MacroAssignments {
                            id: Default::default(),
                            assignments: into_owned_assignments(assignments)?,
                        },
                    ),
                    crate::parse::Kind::Pattern(p) => (
                        (!p.is_negative()).then_some(p)?,
                        Value::Assignments(into_owned_assignments(assignments)?),
                    ),
                };
                pattern::Mapping {
                    pattern,
                    value,
                    sequence_number: line_number,
                }
                .into()
            })
            .collect()
    }
}

impl Attributes {
    fn may_use_glob_pattern(pattern: &gix_glob::Pattern) -> bool {
        pattern.mode != macro_mode()
    }
}

fn macro_mode() -> gix_glob::pattern::Mode {
    gix_glob::pattern::Mode::all()
}

/// Append all matches of patterns matching `relative_path` to `out`,
/// providing a pre-computed `basename_pos` which is the starting position of the basename of `relative_path`.
/// `case` specifies whether cases should be folded during matching or not.
/// `is_dir` is true if `relative_path` is a directory.
/// Return `true` if at least one pattern matched.
#[allow(unused_variables)]
fn pattern_matching_relative_path(
    list: &gix_glob::search::pattern::List<Attributes>,
    relative_path: &BStr,
    basename_pos: Option<usize>,
    case: gix_glob::pattern::Case,
    is_dir: Option<bool>,
    out: &mut Outcome,
) -> bool {
    let (relative_path, basename_start_pos) =
        match list.strip_base_handle_recompute_basename_pos(relative_path, basename_pos, case) {
            Some(r) => r,
            None => return false,
        };
    let cur_len = out.remaining();
    'outer: for pattern::Mapping {
        pattern,
        value,
        sequence_number,
    } in list
        .patterns
        .iter()
        .rev()
        .filter(|pm| Attributes::may_use_glob_pattern(&pm.pattern))
    {
        let value: &Value = value;
        let attrs = match value {
            Value::MacroAssignments { .. } => {
                unreachable!("we can't match on macros as they have no pattern")
            }
            Value::Assignments(attrs) => attrs,
        };
        if out.has_unspecified_attributes(attrs.iter().map(|attr| attr.id))
            && pattern.matches_repo_relative_path(
                relative_path,
                basename_start_pos,
                is_dir,
                case,
                gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL,
            )
        {
            let all_filled = out.fill_attributes(attrs.iter(), pattern, list.source.as_ref(), *sequence_number);
            if all_filled {
                break 'outer;
            }
        }
    }
    cur_len != out.remaining()
}
