//! A high level wrapper around a single or multiple `git-config` file, for reading and mutation.
use std::{
    collections::HashMap,
    ops::{Add, AddAssign},
    path::PathBuf,
};

use bstr::BString;
use gix_features::threading::OwnShared;

mod mutable;
pub use mutable::{multi_value::MultiValueMut, section::SectionMut, value::ValueMut};

///
pub mod init;

mod access;
pub use access::IntoSubsection;
mod impls;
///
pub mod includes;
mod meta;
mod util;

///
pub mod section;

///
pub mod rename_section {
    /// The error returned by [`File::rename_section(…)`][crate::File::rename_section()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Lookup(#[from] crate::lookup::existing::Error),
        #[error(transparent)]
        Section(#[from] crate::parse::section::header::Error),
    }
}

///
pub mod set_raw_value {
    /// The error returned by [`File::set_raw_value(…)`][crate::File::set_raw_value()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Header(#[from] crate::parse::section::header::Error),
        #[error(transparent)]
        ValueName(#[from] crate::parse::section::value_name::Error),
    }
}

/// Additional information about a section.
#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct Metadata {
    /// The file path of the source, if known.
    pub path: Option<PathBuf>,
    /// Where the section is coming from.
    pub source: crate::Source,
    /// The levels of indirection of the file, with 0 being a section
    /// that was directly loaded, and 1 being an `include.path` of a
    /// level 0 file.
    pub level: u8,
    /// The trust-level for the section this meta-data is associated with.
    pub trust: gix_sec::Trust,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SectionData {
    pub(crate) header: crate::parse::section::Header,
    pub(crate) body: section::Body,
    pub(crate) meta: OwnShared<Metadata>,
    pub(crate) id: SectionId,
}

/// A section in a git-config file, like `[core]` or `[remote "origin"]`, along with all of its keys.
///
/// This is a view into data owned by its [`File`][crate::File].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Section<'a> {
    data: &'a SectionData,
    backing: &'a [u8],
}

/// A strongly typed index into some range.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Copy)]
pub(crate) struct Index(pub(crate) usize);

impl Add<Size> for Index {
    type Output = Self;

    fn add(self, rhs: Size) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

/// A strongly typed a size.
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Clone, Copy)]
pub(crate) struct Size(pub(crate) usize);

impl AddAssign<usize> for Size {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

/// The section ID is a monotonically increasing ID used to refer to section bodies.
/// This value does not imply any ordering between sections, as new sections
/// with higher section IDs may be in between lower ID sections after `File` mutation.
///
/// We need to use a section id because `git-config` permits sections with
/// identical names, making it ambiguous when used in maps, for instance.
///
/// This id guaranteed to be unique, but not guaranteed to be compact. In other
/// words, it's possible that a section may have an ID of 3 but the next section
/// has an ID of 5 as 4 was deleted.
#[derive(PartialEq, Eq, Hash, Copy, Clone, PartialOrd, Ord, Debug)]
pub struct SectionId(pub(crate) usize);

impl Default for SectionId {
    fn default() -> Self {
        SectionId(usize::MAX)
    }
}

/// All section body ids referred to by a section name.
///
/// Note that order in Vec matters as it represents the order
/// of section ids with the matched section and name, and is used for precedence
/// management.
#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) enum SectionBodyIdsLut {
    /// The list of section ids to use for obtaining the section body.
    Terminal(Vec<SectionId>),
    /// A hashmap from sub-section names to section ids.
    NonTerminal(HashMap<BString, Vec<SectionId>>),
}
#[cfg(test)]
mod tests;
mod write;
