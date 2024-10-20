use bstr::BStr;
use std::num::NonZeroU8;

/// The way the built-in [text driver](crate::blob::BuiltinDriver::Text) will express
/// merge conflicts in the resulting file.
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ConflictStyle {
    /// Only show the zealously minified conflicting lines of the local changes and the incoming (other) changes,
    /// hiding the base version entirely.
    ///
    /// ```
    /// line1-changed-by-both
    /// <<<<<<< local
    /// line2-to-be-changed-in-incoming
    /// =======
    /// line2-changed
    /// >>>>>>> incoming
    ///```
    #[default]
    Merge,
    /// Show non-minimized hunks of local changes, the base, and the incoming (other) changes.
    ///
    /// This mode does not hide any information.
    /// ```
    /// <<<<<<< local
    /// line1-changed-by-both
    /// line2-to-be-changed-in-incoming
    /// ||||||| 9a8d80c
    /// line1-to-be-changed-by-both
    /// line2-to-be-changed-in-incoming
    /// =======
    /// line1-changed-by-both
    /// line2-changed
    /// >>>>>>> incoming
    ///```
    Diff3,
    /// Like [`Diff3](Self::Diff3), but will show *minimized* hunks of local change and the incoming (other) changes,
    /// as well as non-minimized hunks of the base.
    ///
    /// ```
    /// line1-changed-by-both
    /// <<<<<<< local
    /// line2-to-be-changed-in-incoming
    /// ||||||| 9a8d80c
    /// line1-to-be-changed-by-both
    /// line2-to-be-changed-in-incoming
    /// =======
    /// line2-changed
    /// >>>>>>> incoming
    /// ```
    ZealousDiff3,
}

/// The set of labels to annotate conflict markers with.
///
/// That way it becomes clearer where the content of conflicts are originating from.
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq)]
pub struct Labels<'a> {
    pub ancestor: Option<&'a BStr>,
    pub current: Option<&'a BStr>,
    pub other: Option<&'a BStr>,
}

/// Options for the builtin [text driver](crate::blob::BuiltinDriver::Text).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Options {
    /// Determine of the diff will be performed.
    /// Defaults to [`imara_diff::Algorithm::Myers`].
    pub diff_algorithm: imara_diff::Algorithm,
    /// Decide what to do to automatically resolve conflicts, or to keep them.
    pub conflict: Conflict,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            conflict: Default::default(),
            diff_algorithm: imara_diff::Algorithm::Myers,
        }
    }
}

/// What to do to resolve a conflict.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Conflict {
    /// Keep the conflict by marking it in the source file.
    Keep {
        /// How to visualize conflicts in merged files.
        style: ConflictStyle,
        /// The amount of markers to draw, defaults to 7, i.e. `<<<<<<<`
        marker_size: NonZeroU8,
    },
    /// Chose our side to resolve a conflict.
    ResolveWithOurs,
    /// Chose their side to resolve a conflict.
    ResolveWithTheirs,
    /// Place our and their lines one after another, in any order
    ResolveWithUnion,
}

impl Conflict {
    /// The amount of conflict marker characters to print by default.
    // TODO: use NonZeroU8::new().unwrap() here once the MSRV supports it.
    pub const DEFAULT_MARKER_SIZE: u8 = 7;

    /// The amount of conflict markers to print if this instance contains them, or `None` otherwise
    pub fn marker_size(&self) -> Option<u8> {
        match self {
            Conflict::Keep { marker_size, .. } => Some(marker_size.get()),
            Conflict::ResolveWithOurs | Conflict::ResolveWithTheirs | Conflict::ResolveWithUnion => None,
        }
    }
}

impl Default for Conflict {
    fn default() -> Self {
        Conflict::Keep {
            style: Default::default(),
            marker_size: Conflict::DEFAULT_MARKER_SIZE.try_into().unwrap(),
        }
    }
}

pub(super) mod function;
mod utils;
