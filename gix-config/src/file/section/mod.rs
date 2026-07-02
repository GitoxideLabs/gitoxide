use bstr::{BStr, BString, ByteSlice};
use smallvec::SmallVec;

use crate::{
    file,
    file::{Metadata, Section, SectionData, SectionMut},
    parse,
    parse::{Event, section},
};

pub(crate) mod body;
pub use body::{Body, BodyIter, BodyRef, BodyRefIter};
use gix_features::threading::OwnShared;

use crate::file::{
    SectionId,
    write::{extract_newline, platform_newline},
};

impl std::ops::Deref for SectionData {
    type Target = Body;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

/// A view of a section header whose bytes are owned by the containing [`File`][crate::File].
#[derive(Copy, Clone, Debug)]
pub struct Header<'a> {
    pub(crate) header: &'a section::Header,
    pub(crate) backing: &'a [u8],
}

impl<'a> Header<'a> {
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
            .map(|subsection_name| subsection_name.as_bstr_in(self.backing))
    }

    /// Return the name of the header, like "remote" in `[remote "origin"]`.
    pub fn name(&self) -> &'a BStr {
        self.header.name.0.as_bstr_in(self.backing)
    }

    /// Convert this view into an owned parser header.
    #[must_use]
    pub fn to_owned(&self) -> section::Header {
        self.header.to_owned_in(self.backing)
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
impl<'file> Section<'file> {
    pub(crate) fn from_data(data: &'file SectionData, backing: &'file [u8]) -> Self {
        Section { data, backing }
    }
}

impl SectionData {
    /// Create a new section with the given `name` and optional, `subsection`, `meta`-data and an empty body.
    pub(crate) fn new(
        name: impl AsRef<str>,
        subsection: impl Into<Option<BString>>,
        meta: impl Into<OwnShared<file::Metadata>>,
    ) -> Result<Self, parse::section::header::Error> {
        Ok(SectionData {
            header: parse::section::Header::new(name, subsection)?,
            body: Default::default(),
            meta: meta.into(),
            id: SectionId::default(),
        })
    }

    pub(crate) fn from_view(section: Section<'_>, backing: &mut Vec<u8>) -> Self {
        SectionData {
            header: section.data.header.copy_to_backing_in(section.backing, backing),
            body: section.data.body.copy_to_backing_in(section.backing, backing),
            meta: OwnShared::clone(&section.data.meta),
            id: section.data.id,
        }
    }

    /// Returns a mutable version of this section for adjustment of values.
    pub(crate) fn to_mut<'a>(&'a mut self, backing: &'a mut Vec<u8>, newline: SmallVec<[u8; 2]>) -> SectionMut<'a> {
        SectionMut::new(self, backing, newline)
    }

    pub(crate) fn meta(&self) -> &Metadata {
        &self.meta
    }

    pub(crate) fn intern(&mut self, backing: &mut Vec<u8>) {
        self.header.intern(backing);
        self.body.intern(backing);
    }
}

/// Access
impl<'file> Section<'file> {
    /// Return our header.
    pub fn header(&self) -> Header<'file> {
        Header {
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

    pub(crate) fn body_data(&self) -> &'file Body {
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
        self.data.header.write_to_in(self.backing, &mut *out)?;

        if self.body_data().0.is_empty() {
            return Ok(());
        }

        let nl = self
            .body_data()
            .as_ref()
            .iter()
            .find_map(|event| extract_newline(event, self.backing))
            .unwrap_or_else(|| platform_newline());

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
    pub fn value_names(&self) -> impl Iterator<Item = section::ValueName> + '_ {
        self.data.body.as_ref().iter().filter_map(move |e| match e {
            Event::SectionValueName(k) => Some(section::ValueName(k.0.to_bstring_in(self.backing).into())),
            _ => None,
        })
    }

    /// Returns true if the section contains the provided value name.
    #[must_use]
    pub fn contains_value_name(&self, value_name: &str) -> bool {
        self.data.body.contains_value_name_in(self.backing, value_name)
    }
}
