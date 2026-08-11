use bstr::BString;

use crate::{
    file,
    file::{Index, Size, mutable::section::SectionMut},
    lookup,
    parse::section,
};

/// An intermediate representation of a mutable value obtained from a [`File`][crate::File].
#[derive(Debug)]
pub struct ValueMut<'borrow> {
    pub(crate) section: SectionMut<'borrow>,
    pub(crate) key: section::ValueName,
    pub(crate) index: Index,
    pub(crate) size: Size,
}

impl<'borrow> ValueMut<'borrow> {
    /// Returns the actual value. This is computed each time this is called
    /// requiring an allocation for multi-line values.
    pub fn get(&self) -> Result<BString, lookup::existing::Error> {
        self.section.get(&self.key, self.index, self.index + self.size)
    }

    /// Update the value to the provided one. This modifies the value such that
    /// the Value event(s) are replaced with a single new event containing the
    /// new value.
    pub fn set_string(&mut self, input: impl AsRef<str>) -> Result<(), crate::parse::span::Error> {
        self.set(input.as_ref())
    }

    /// Update the value to the provided one. This modifies the value such that
    /// the Value event(s) are replaced with a single new event containing the
    /// new value.
    pub fn set(&mut self, input: impl crate::AsBStr) -> Result<(), crate::parse::span::Error> {
        let new_size = self
            .section
            .set_internal(self.index, self.key.to_owned(), input.as_bstr())?;
        if self.size.0 > 0 {
            self.section
                .delete_replaced_value(self.index + new_size, self.index + new_size + self.size);
        }
        self.size = new_size;
        Ok(())
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
    pub fn section(&self) -> file::SectionRef<'_> {
        self.section.section()
    }

    /// Convert this value into its owning mutable section.
    pub fn into_section_mut(self) -> file::SectionMut<'borrow> {
        self.section
    }
}
