use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use bstr::{BStr, ByteSlice};
use gix_glob::search::{Pattern, pattern};

use crate::Search;

/// Describes a matching pattern within a search for ignored paths.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Match<'a> {
    /// The glob pattern itself, like `/target/*`.
    pub pattern: &'a gix_glob::Pattern,
    /// The path to the source from which the pattern was loaded, or `None` if it was specified by other means.
    pub source: Option<&'a Path>,
    /// The kind of pattern this match represents.
    pub kind: crate::Kind,
    /// The line at which the pattern was found in its `source` file, or the occurrence in which it was provided.
    pub sequence_number: usize,
}

/// An implementation of the [`Pattern`] trait for ignore-patterns.
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct Ignore {
    /// If `support_precious` is `true`, we will parse `$` prefixed entries as precious.
    /// This is backward-incompatible as files that actually start with `$` like `$houdini`
    /// will then not be ignored anymore, instead it ignores `houdini`.
    pub support_precious: bool,
}

impl Pattern for Ignore {
    type Value = crate::Kind;

    fn bytes_to_patterns(&self, bytes: &[u8], _source: &std::path::Path) -> Vec<pattern::Mapping<Self::Value>> {
        crate::parse(bytes, self.support_precious)
            .map(|(pattern, line_number, kind)| pattern::Mapping {
                pattern,
                value: kind,
                sequence_number: line_number,
            })
            .collect()
    }
}

/// Instantiation of a search for ignore patterns.
impl Search {
    /// Given `git_dir`, a `.git` repository, load static ignore patterns from `info/exclude`
    /// and from `excludes_file` if it is provided.
    /// Note that it's not considered an error if the provided `excludes_file` does not exist.
    /// `parse` is a way to parse bytes to ignore patterns.
    pub fn from_git_dir(
        git_dir: &Path,
        excludes_file: Option<PathBuf>,
        buf: &mut Vec<u8>,
        parse: Ignore,
    ) -> std::io::Result<Self> {
        let mut group = Self::default();

        let follow_symlinks = true;
        // order matters! More important ones first.
        group.patterns.extend(
            excludes_file
                .and_then(|file| {
                    pattern::List::<Ignore>::from_file(file, None, follow_symlinks, buf, parse).transpose()
                })
                .transpose()?,
        );
        group.patterns.extend(pattern::List::<Ignore>::from_file(
            git_dir.join("info").join("exclude"),
            None,
            follow_symlinks,
            buf,
            parse,
        )?);
        Ok(group)
    }

    /// Parse a list of ignore patterns, using slashes as path separators.
    /// `parse` is a way to parse bytes to ignore patterns.
    pub fn from_overrides(patterns: impl IntoIterator<Item = impl Into<OsString>>, parse: Ignore) -> Self {
        Self::from_overrides_inner(&mut patterns.into_iter().map(Into::into), parse)
    }

    fn from_overrides_inner(patterns: &mut dyn Iterator<Item = OsString>, parse: Ignore) -> Self {
        Search {
            patterns: vec![pattern::List {
                patterns: patterns
                    .enumerate()
                    .filter_map(|(seq_id, pattern)| {
                        let pattern = gix_path::try_into_bstr(PathBuf::from(pattern)).ok()?;
                        crate::parse(pattern.as_ref(), parse.support_precious)
                            .next()
                            .map(|(p, _seq_id, kind)| pattern::Mapping {
                                pattern: p,
                                value: kind,
                                sequence_number: seq_id + 1,
                            })
                    })
                    .collect(),
                source: None,
                base: None,
            }],
        }
    }
}

/// Mutation
impl Search {
    /// Add patterns as parsed from `bytes`, providing their `source` path and possibly their `root` path, the path they
    /// are relative to. This also means that `source` is contained within `root` if `root` is provided.
    /// Use `parse` to control how ignore patterns are parsed.
    pub fn add_patterns_buffer(
        &mut self,
        bytes: &[u8],
        source: impl Into<PathBuf>,
        root: Option<&Path>,
        parse: Ignore,
    ) {
        self.patterns
            .push(pattern::List::from_bytes(bytes, source.into(), root, parse));
    }
}

/// Return a match if a pattern matches `relative_path`, providing a pre-computed `basename_pos` which is the
/// starting position of the basename of `relative_path`. `is_dir` is true if `relative_path` is a directory.
/// `case` specifies whether cases should be folded during matching or not.
pub fn pattern_matching_relative_path<'a>(
    list: &'a gix_glob::search::pattern::List<Ignore>,
    relative_path: &BStr,
    basename_pos: Option<usize>,
    is_dir: Option<bool>,
    case: gix_glob::pattern::Case,
) -> Option<Match<'a>> {
    let (relative_path, basename_start_pos) =
        list.strip_base_handle_recompute_basename_pos(relative_path, basename_pos, case)?;
    list.patterns.iter().rev().find_map(
        |pattern::Mapping {
             pattern,
             value: kind,
             sequence_number,
         }| {
            pattern
                .matches_repo_relative_path(
                    relative_path,
                    basename_start_pos,
                    is_dir,
                    case,
                    gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL,
                )
                .then_some(Match {
                    pattern,
                    kind: *kind,
                    source: list.source.as_deref(),
                    sequence_number: *sequence_number,
                })
        },
    )
}

/// Like [`pattern_matching_relative_path()`], but returns an index to the pattern
/// that matched `relative_path`, instead of the match itself.
pub fn pattern_idx_matching_relative_path(
    list: &gix_glob::search::pattern::List<Ignore>,
    relative_path: &BStr,
    basename_pos: Option<usize>,
    is_dir: Option<bool>,
    case: gix_glob::pattern::Case,
) -> Option<usize> {
    let (relative_path, basename_start_pos) =
        list.strip_base_handle_recompute_basename_pos(relative_path, basename_pos, case)?;
    list.patterns.iter().enumerate().rev().find_map(|(idx, pm)| {
        pm.pattern
            .matches_repo_relative_path(
                relative_path,
                basename_start_pos,
                is_dir,
                case,
                gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL,
            )
            .then_some(idx)
    })
}

/// Matching of ignore patterns.
impl Search {
    /// Match `relative_path` and return the first match if found.
    /// `is_dir` is true if `relative_path` is a directory.
    /// `case` specifies whether cases should be folded during matching or not.
    pub fn pattern_matching_relative_path(
        &self,
        relative_path: &BStr,
        is_dir: Option<bool>,
        case: gix_glob::pattern::Case,
    ) -> Option<Match<'_>> {
        let basename_pos = relative_path.rfind(b"/").map(|p| p + 1);
        self.patterns
            .iter()
            .rev()
            .find_map(|pl| pattern_matching_relative_path(pl, relative_path, basename_pos, is_dir, case))
    }
}
