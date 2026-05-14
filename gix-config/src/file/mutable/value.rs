use std::borrow::Cow;

use bstr::BStr;

use crate::{
    file,
    file::{Index, Size, mutable::section::SectionMut},
    lookup,
    parse::section,
};

/// An intermediate representation of a mutable value obtained from a [`File`][crate::File].
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ValueMut<'borrow, 'lookup, 'event> {
    pub(crate) section: SectionMut<'borrow, 'event>,
    pub(crate) key: section::ValueName<'lookup>,
    pub(crate) index: Index,
    pub(crate) size: Size,
}

impl<'borrow, 'event> ValueMut<'borrow, '_, 'event> {
    /// Returns the actual value. This is computed each time this is called
    /// requiring an allocation for multi-line values.
    pub fn get(&self) -> Result<Cow<'_, BStr>, lookup::existing::Error> {
        self.section.get(&self.key, self.index, self.index + self.size)
    }

    /// Update the value to the provided one. This modifies the value such that
    /// the Value event(s) are replaced with a single new event containing the
    /// new value.
    pub fn set_string(&mut self, input: impl AsRef<str>) {
        self.set(input.as_ref());
    }

    /// Update the value to the provided one. This modifies the value such that
    /// the Value event(s) are replaced with a single new event containing the
    /// new value.
    pub fn set<'a>(&mut self, input: impl Into<&'a BStr>) {
        if self.size.0 > 0 {
            self.section.delete(self.index, self.index + self.size);
        }
        self.size = self.section.set_internal(self.index, self.key.to_owned(), input.into());
    }

    /// Removes the value. Does nothing when called multiple times in
    /// succession.
    pub fn delete(&mut self) {
        if self.size.0 > 0 {
            self.section.delete(self.index, self.index + self.size);
            self.size = Size(0);
        }
    }

    /// Return the section containing the value.
    pub fn section(&self) -> &file::Section<'event> {
        &self.section
    }

    /// Convert this value into its owning mutable section.
    pub fn into_section_mut(self) -> file::SectionMut<'borrow, 'event> {
        self.section
    }
}
