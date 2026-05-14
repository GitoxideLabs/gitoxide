use std::{
    io::Read,
    path::{Path, PathBuf},
};

use bstr::{BStr, BString, ByteSlice, ByteVec};

use crate::{pattern::Case, search::Pattern};

/// A list of patterns which optionally know where they were loaded from and what their base is.
///
/// Knowing their base which is relative to a source directory, it will ignore all path to match against
/// that don't also start with said base.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Default)]
pub struct List<T: Pattern> {
    /// Patterns and their associated data in the order they were loaded in or specified,
    /// the line number in its source file or its sequence number (_`(pattern, value, line_number)`_).
    ///
    /// During matching, this order is reversed.
    pub patterns: Vec<Mapping<T::Value>>,

    /// The path from which the patterns were read, or `None` if the patterns
    /// don't originate in a file on disk.
    pub source: Option<PathBuf>,

    /// The parent directory of source, or `None` if the patterns are _global_ to match against the repository root.
    /// It's processed to contain slashes only and to end with a trailing slash, and is relative to the repository root.
    pub base: Option<BString>,
}

/// An association of a pattern with its value, along with a sequence number providing a sort order in relation to its peers.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub struct Mapping<T> {
    /// The pattern itself, like `/target/*`
    pub pattern: crate::Pattern,
    /// The value associated with the pattern.
    pub value: T,
    /// Typically the line number in the file the pattern was parsed from.
    pub sequence_number: usize,
}

fn read_in_full_ignore_missing(path: &Path, follow_symlinks: bool, buf: &mut Vec<u8>) -> std::io::Result<bool> {
    buf.clear();
    let file = if follow_symlinks {
        std::fs::File::open(path)
    } else {
        gix_features::fs::open_options_no_follow().read(true).open(path)
    };
    Ok(match file {
        Ok(mut file) => {
            if let Err(err) = file.read_to_end(buf) {
                if io_err_is_dir(&err) {
                    false
                } else {
                    return Err(err);
                }
            } else {
                true
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound || io_err_is_dir(&err) => false,
        Err(err) => return Err(err),
    })
}

fn io_err_is_dir(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::IsADirectory | std::io::ErrorKind::NotADirectory
    ) || {
        #[cfg(windows)]
        {
            err.kind() == std::io::ErrorKind::PermissionDenied
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

/// Instantiation
impl<T> List<T>
where
    T: Pattern,
{
    /// `source_file` is the location of the `bytes` which represents a list of patterns, one pattern per line.
    /// If `root` is `Some(…)` it's used to see `source_file` as relative to itself, if `source_file` is absolute.
    /// If source is relative and should be treated as base, set `root` to `Some("")`.
    /// `parse` is a way to parse bytes to pattern.
    pub fn from_bytes(bytes: &[u8], source_file: PathBuf, root: Option<&Path>, parse: T) -> Self {
        let patterns = parse.bytes_to_patterns(bytes, source_file.as_path());
        let base = root
            .and_then(|root| source_file.parent().expect("file").strip_prefix(root).ok())
            .and_then(|base| {
                (!base.as_os_str().is_empty()).then(|| {
                    let mut base: BString =
                        gix_path::to_unix_separators_on_windows(gix_path::into_bstr(base)).into_owned();

                    base.push_byte(b'/');
                    base
                })
            });
        List {
            patterns,
            source: Some(source_file),
            base,
        }
    }

    /// Create a pattern list from the `source` file, which may be located underneath `root`, while optionally
    /// following symlinks with `follow_symlinks`, providing `buf` to temporarily store the data contained in the file.
    /// `parse` is a way to parse bytes to pattern.
    pub fn from_file(
        source: impl Into<PathBuf>,
        root: Option<&Path>,
        follow_symlinks: bool,
        buf: &mut Vec<u8>,
        parse: T,
    ) -> std::io::Result<Option<Self>> {
        let source = source.into();
        Ok(read_in_full_ignore_missing(&source, follow_symlinks, buf)?
            .then(|| Self::from_bytes(buf, source, root, parse)))
    }
}

/// Utilities
impl<T> List<T>
where
    T: Pattern,
{
    /// If this list is anchored to a base path, return `relative_path` as being relative to our base and return
    /// an updated `basename_pos` as well if it was set.
    /// `case` is respected for the comparison.
    ///
    /// This is useful to turn repository-relative paths into paths relative to a particular search base.
    pub fn strip_base_handle_recompute_basename_pos<'a>(
        &self,
        relative_path: &'a BStr,
        basename_pos: Option<usize>,
        case: Case,
    ) -> Option<(&'a BStr, Option<usize>)> {
        match self.base.as_deref() {
            Some(base) => strip_base_handle_recompute_basename_pos(base.as_bstr(), relative_path, basename_pos, case)?,
            None => (relative_path, basename_pos),
        }
        .into()
    }
}

///  Return`relative_path` as being relative to `base` along with an updated `basename_pos` if it was set.
/// `case` is respected for the comparison.
///
/// This is useful to turn repository-relative paths into paths relative to a particular search base.
pub fn strip_base_handle_recompute_basename_pos<'a>(
    base: &BStr,
    relative_path: &'a BStr,
    basename_pos: Option<usize>,
    case: Case,
) -> Option<(&'a BStr, Option<usize>)> {
    Some((
        match case {
            Case::Sensitive => relative_path.strip_prefix(base.as_bytes())?.as_bstr(),
            Case::Fold => {
                let rela_dir = relative_path.get(..base.len())?;
                if !rela_dir.eq_ignore_ascii_case(base) {
                    return None;
                }
                &relative_path[base.len()..]
            }
        },
        basename_pos.and_then(|pos| {
            let pos = pos - base.len();
            (pos != 0).then_some(pos)
        }),
    ))
}
