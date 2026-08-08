use bstr::{BStr, BString, ByteSlice};
use smallvec::SmallVec;

use crate::{
    file,
    file::{IntoBStringOpt, Metadata, Section, SectionData, SectionMut, SectionRef},
    parse,
    parse::{Event, section},
};

pub(crate) mod body;
pub(crate) use body::BodyData;
pub use body::{BodyRef, BodyRefIter};
use gix_features::threading::OwnShared;

use crate::file::{SectionId, write::platform_newline};

/// Errors related to changing values in a section.
pub mod value {
    /// The error returned when adding or changing a value in a section.
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        ValueName(crate::parse::section::value_name::Error),
        Span(crate::parse::span::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::ValueName(err) => std::fmt::Display::fmt(err, f),
                Error::Span(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::ValueName(err) => err.source(),
                Error::Span(err) => err.source(),
            }
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

impl std::ops::Deref for SectionData {
    type Target = BodyData;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

/// A view of a section header whose bytes are owned by the containing [`File`][crate::File].
#[derive(Copy, Clone, Debug)]
pub struct HeaderRef<'a> {
    pub(crate) header: &'a section::HeaderData,
    pub(crate) backing: &'a [u8],
}

impl<'a> HeaderRef<'a> {
    /// Return true if this is a header like `[legacy.subsection]`, or false otherwise.
    pub fn is_legacy(&self) -> bool {
        self.header
            .separator
            .as_ref()
            .is_some_and(|separator| separator.as_slice_in(self.backing) == b".")
    }

    /// Return the subsection name, if present, i.e. "origin" in `[remote "origin"]`.
    pub fn subsection_name(&self) -> Option<&'a BStr> {
        self.header
            .subsection_name
            .as_ref()
            .map(|subsection_name| subsection_name.value_in(self.backing))
    }

    /// Return the name of the header, like "remote" in `[remote "origin"]`.
    pub fn name(&self) -> &'a BStr {
        self.header.name.as_bstr_in(self.backing)
    }

    /// Serialize this header view into a `BString` for convenience.
    #[must_use]
    pub fn to_bstring(&self) -> BString {
        let mut buf = Vec::new();
        self.header
            .write_to_in(self.backing, &mut buf)
            .expect("io error impossible");
        buf.into()
    }
}

/// Instantiation and conversion
impl<'file> SectionRef<'file> {
    pub(crate) fn from_data(data: &'file SectionData, backing: &'file [u8]) -> Self {
        SectionRef { data, backing }
    }
}

impl Section {
    /// Create an owned section with an empty body.
    pub fn new(
        name: impl AsRef<str>,
        subsection: impl IntoBStringOpt,
        meta: impl Into<OwnShared<file::Metadata>>,
    ) -> Result<Self, parse::section::header::Error> {
        let mut backing = Vec::new();
        let data = SectionData::new(name, subsection.into_bstring_opt(), meta, &mut backing)?;
        Ok(Section { backing, data })
    }

    /// Return a read-only view of this section.
    pub fn to_ref(&self) -> SectionRef<'_> {
        SectionRef::from_data(&self.data, &self.backing)
    }

    /// Return a mutable view of this section.
    pub fn to_mut(&mut self) -> SectionMut<'_> {
        let newline = self
            .data
            .body
            .detect_newline_style_in(&self.backing)
            .unwrap_or_else(|| platform_newline())
            .as_bytes()
            .into();
        SectionMut::new(&mut self.data, &mut self.backing, None, newline)
    }

    pub(crate) fn from_data(data: &SectionData, source: &[u8]) -> Self {
        let mut backing = Vec::new();
        let data = data
            .copy_to_backing_in(source, &mut backing)
            .expect("copying into an empty buffer cannot exceed the source buffer's span limit");
        Section { backing, data }
    }

    pub(crate) fn into_data(self, target: &mut Vec<u8>) -> Result<SectionData, parse::span::Error> {
        self.data.copy_to_backing_in(&self.backing, target)
    }
}

impl SectionData {
    /// Create a new section with the given `name` and optional, `subsection`, `meta`-data and an empty body.
    pub(crate) fn new(
        name: impl AsRef<str>,
        subsection: impl Into<Option<BString>>,
        meta: impl Into<OwnShared<file::Metadata>>,
        backing: &mut Vec<u8>,
    ) -> Result<Self, parse::section::header::Error> {
        Ok(SectionData {
            header: parse::section::HeaderData::new_in(name, subsection, backing)?,
            body: Default::default(),
            meta: meta.into(),
            id: SectionId::default(),
        })
    }

    /// Returns a mutable version of this section for adjustment of values.
    pub(crate) fn to_mut<'a>(
        &'a mut self,
        backing: &'a mut Vec<u8>,
        lookup: file::mutable::section::LookupMut<'a>,
        newline: SmallVec<[u8; 2]>,
    ) -> SectionMut<'a> {
        SectionMut::new(self, backing, Some(lookup), newline)
    }

    pub(crate) fn meta(&self) -> &Metadata {
        &self.meta
    }

    pub(crate) fn copy_to_backing_in(&self, source: &[u8], target: &mut Vec<u8>) -> Result<Self, parse::span::Error> {
        Ok(SectionData {
            header: self.header.copy_to_backing_in(source, target)?,
            body: self.body.copy_to_backing_in(source, target)?,
            meta: OwnShared::clone(&self.meta),
            id: self.id,
        })
    }
}

/// Access
impl<'file> SectionRef<'file> {
    /// Copy this view into a self-contained owned section.
    #[must_use]
    pub fn to_owned(self) -> Section {
        Section::from_data(self.data, self.backing)
    }

    /// Return our header.
    pub fn header(&self) -> HeaderRef<'file> {
        HeaderRef {
            header: &self.data.header,
            backing: self.backing,
        }
    }

    /// Return the unique `id` of the section, for use with the `*_by_id()` family of methods
    /// in [`gix_config::File`][crate::File].
    pub fn id(&self) -> SectionId {
        self.data.id
    }

    /// Return our body, containing all value names and values.
    pub fn body(&self) -> BodyRef<'file> {
        BodyRef {
            body: &self.data.body,
            backing: self.backing,
        }
    }

    pub(crate) fn body_data(&self) -> &'file BodyData {
        &self.data.body
    }

    /// Serialize this type into a `BString` for convenience.
    ///
    /// Note that `to_string()` can also be used, but might not be lossless.
    #[must_use]
    pub fn to_bstring(&self) -> BString {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("io error impossible");
        buf.into()
    }

    /// Stream ourselves to the given `out`, in order to reproduce this section mostly losslessly
    /// as it was parsed.
    pub fn write_to(&self, mut out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let nl = self
            .body_data()
            .detect_newline_style_in(self.backing)
            .unwrap_or_else(|| platform_newline());
        self.write_to_with_newline(&mut out, nl)
    }

    pub(crate) fn write_to_with_newline(&self, mut out: &mut dyn std::io::Write, nl: &BStr) -> std::io::Result<()> {
        self.data.header.write_to_in(self.backing, &mut *out)?;

        if self.body_data().0.is_empty() {
            return Ok(());
        }

        if !self
            .body_data()
            .as_ref()
            .iter()
            .take_while(|e| !matches!(e, Event::SectionValueName(_)))
            .any(|e| e.to_bstr_lossy_in(self.backing).contains_str(nl))
        {
            out.write_all(nl)?;
        }

        let mut saw_newline_after_value = true;
        let mut in_key_value_pair = false;
        for (idx, event) in self.body_data().as_ref().iter().enumerate() {
            match event {
                Event::SectionValueName(_) => {
                    if !saw_newline_after_value {
                        out.write_all(nl)?;
                    }
                    saw_newline_after_value = false;
                    in_key_value_pair = true;
                }
                Event::Newline(_) if !in_key_value_pair => {
                    saw_newline_after_value = true;
                }
                Event::Value(_) | Event::ValueDone(_) => {
                    in_key_value_pair = false;
                }
                _ => {}
            }
            event.write_to_in(self.backing, &mut out)?;
            if let Event::ValueNotDone(_) = event {
                if self
                    .body_data()
                    .0
                    .get(idx + 1)
                    .filter(|e| matches!(e, Event::Newline(_)))
                    .is_none()
                {
                    out.write_all(nl)?;
                }
            }
        }
        Ok(())
    }

    /// Return additional information about this sections origin.
    pub fn meta(&self) -> &'file Metadata {
        &self.data.meta
    }

    /// Retrieves the last matching value in this section with the given value name, if present.
    #[must_use]
    pub fn value(&self, value_name: impl AsRef<str>) -> Option<BString> {
        self.data
            .body
            .value_implicit_in(self.backing, value_name.as_ref())
            .flatten()
    }

    /// Retrieves the last matching value in this section, including implicit values.
    #[must_use]
    pub fn value_implicit(&self, value_name: &str) -> Option<Option<BString>> {
        self.data.body.value_implicit_in(self.backing, value_name)
    }

    /// Retrieves all values that have the provided value name.
    #[must_use]
    pub fn values(&self, value_name: &str) -> Vec<BString> {
        self.data.body.values_in(self.backing, value_name)
    }

    /// Returns an iterator visiting all value names in order.
    pub fn value_names(&self) -> impl Iterator<Item = String> + '_ {
        self.data.body.as_ref().iter().filter_map(move |e| match e {
            Event::SectionValueName(k) => Some(
                k.as_bstr_in(self.backing)
                    .to_str()
                    .expect("parsed value names are ASCII")
                    .to_owned(),
            ),
            _ => None,
        })
    }

    /// Returns true if the section contains the provided value name.
    #[must_use]
    pub fn contains_value_name(&self, value_name: &str) -> bool {
        self.data.body.contains_value_name_in(self.backing, value_name)
    }
}
