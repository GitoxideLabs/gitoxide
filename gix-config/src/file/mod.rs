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
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Lookup(crate::lookup::existing::Error),
        Section(crate::parse::section::header::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Lookup(err) => std::fmt::Display::fmt(err, f),
                Error::Section(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Lookup(err) => err.source(),
                Error::Section(err) => err.source(),
            }
        }
    }

    impl From<crate::lookup::existing::Error> for Error {
        fn from(err: crate::lookup::existing::Error) -> Self {
            Error::Lookup(err)
        }
    }

    impl From<crate::parse::section::header::Error> for Error {
        fn from(err: crate::parse::section::header::Error) -> Self {
            Error::Section(err)
        }
    }
}

///
pub mod set_raw_value {
    /// The error returned by [`File::set_raw_value(…)`][crate::File::set_raw_value()].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Lookup(crate::lookup::existing::Error),
        Header(crate::parse::section::header::Error),
        ValueName(crate::parse::section::value_name::Error),
        Span(crate::parse::span::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Lookup(err) => std::fmt::Display::fmt(err, f),
                Error::Header(err) => std::fmt::Display::fmt(err, f),
                Error::ValueName(err) => std::fmt::Display::fmt(err, f),
                Error::Span(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Lookup(err) => err.source(),
                Error::Header(err) => err.source(),
                Error::ValueName(err) => err.source(),
                Error::Span(err) => err.source(),
            }
        }
    }

    impl From<crate::lookup::existing::Error> for Error {
        fn from(err: crate::lookup::existing::Error) -> Self {
            Error::Lookup(err)
        }
    }

    impl From<crate::parse::section::header::Error> for Error {
        fn from(err: crate::parse::section::header::Error) -> Self {
            Error::Header(err)
        }
    }

    impl From<crate::parse::section::value_name::Error> for Error {
        fn from(err: crate::parse::section::value_name::Error) -> Self {
            Error::ValueName(err)
        }
    }

    impl From<crate::parse::span::Error> for Error {
        fn from(err: crate::parse::span::Error) -> Self {
            Error::Span(err)
        }
    }
}

/// Convert ergonomic subsection inputs into an optional owned name.
pub trait IntoBStringOpt {
    /// Convert into an optional owned subsection name.
    fn into_bstring_opt(self) -> Option<bstr::BString>;
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

#[derive(Clone, Debug)]
pub(crate) struct SectionData {
    pub(crate) header: crate::parse::section::HeaderData,
    pub(crate) body: section::BodyData,
    pub(crate) meta: OwnShared<Metadata>,
    pub(crate) id: SectionId,
}

/// A fully owned, self-contained configuration section.
///
/// Use [`Section::to_ref()`] for read-only access and [`Section::to_mut()`] for mutation. This type is returned when
/// removing sections from a [`File`][crate::File] and can be inserted again with [`File::push_section()`](crate::File::push_section()).
#[derive(Clone, Debug)]
pub struct Section {
    backing: Vec<u8>,
    data: SectionData,
}

/// A section in a git-config file, like `[core]` or `[remote "origin"]`, along with all of its keys.
///
/// This is a view into data owned by its [`File`][crate::File].
#[derive(Copy, Clone, Debug)]
pub struct SectionRef<'a> {
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

/// All section ids referred to by a section name, in file order within each list.
#[derive(Default, PartialEq, Eq, Clone, Debug)]
pub(crate) struct SectionLookup {
    without_subsection: Vec<SectionId>,
    by_subsection: HashMap<BString, Vec<SectionId>>,
}
#[cfg(test)]
mod tests;
mod write;
