use std::{borrow::Cow, collections::HashMap, ops::DerefMut};

use bstr::{BStr, BString, ByteVec};

use crate::{
    file::{
        self, Section, SectionId,
        mutable::{Whitespace, escape_value},
    },
    lookup,
    parse::{Event, section},
    value::{normalize_bstr, normalize_bstring},
};

/// Internal data structure for [`MutableMultiValue`]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct EntryData {
    pub(crate) section_id: SectionId,
    pub(crate) offset_index: usize,
}

/// An intermediate representation of a mutable multivar obtained from a [`File`][crate::File].
#[derive(PartialEq, Eq, Debug)]
pub struct MultiValueMut<'borrow, 'lookup, 'event> {
    pub(crate) section: &'borrow mut HashMap<SectionId, Section<'event>>,
    pub(crate) key: section::ValueName<'lookup>,
    /// Each entry data struct provides sufficient information to index into
    /// [`Self::offsets`]. This layer of indirection is used for users to index
    /// into the offsets rather than leaking the internal data structures.
    pub(crate) indices_and_sizes: Vec<EntryData>,
    /// Each offset represents the size of a event slice and whether or not the
    /// event slice is significant or not. This is used to index into the
    /// actual section.
    pub(crate) offsets: HashMap<SectionId, Vec<usize>>,
}

impl<'lookup, 'event> MultiValueMut<'_, 'lookup, 'event> {
    /// Returns the actual values.
    pub fn get(&self) -> Result<Vec<Cow<'_, BStr>>, lookup::existing::Error> {
        let mut expect_value = false;
        let mut values = Vec::new();
        let mut concatenated_value = BString::default();

        for EntryData {
            section_id,
            offset_index,
        } in &self.indices_and_sizes
        {
            let (offset, size) = MultiValueMut::index_and_size(&self.offsets, *section_id, *offset_index);
            for event in &self.section.get(section_id).expect("known section id").as_ref()[offset..offset + size] {
                match event {
                    Event::SectionValueName(section_key) if *section_key == self.key => expect_value = true,
                    Event::Value(v) if expect_value => {
                        expect_value = false;
                        values.push(normalize_bstr(v.as_ref()));
                    }
                    Event::ValueNotDone(v) if expect_value => concatenated_value.push_str(v.as_ref()),
                    Event::ValueDone(v) if expect_value => {
                        expect_value = false;
                        concatenated_value.push_str(v.as_ref());
                        values.push(normalize_bstring(std::mem::take(&mut concatenated_value)));
                    }
                    _ => (),
                }
            }
        }

        if values.is_empty() {
            return Err(lookup::existing::Error::KeyMissing);
        }

        Ok(values)
    }

    /// Returns the amount of values within this multivar.
    #[must_use]
    pub fn len(&self) -> usize {
        self.indices_and_sizes.len()
    }

    /// Returns true if the multivar does not have any values.
    /// This might occur if the value was deleted but wasn't yet set with a new value.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.indices_and_sizes.is_empty()
    }

    /// Sets the value at the given index.
    ///
    /// # Safety
    ///
    /// This will panic if the index is out of range.
    pub fn set_string_at(&mut self, index: usize, value: impl AsRef<str>) {
        self.set_at(index, value.as_ref());
    }

    /// Sets the value at the given index.
    ///
    /// # Safety
    ///
    /// This will panic if the index is out of range.
    pub fn set_at<'a>(&mut self, index: usize, value: impl Into<&'a BStr>) {
        let EntryData {
            section_id,
            offset_index,
        } = self.indices_and_sizes[index];
        MultiValueMut::set_value_inner(
            &self.key,
            &mut self.offsets,
            &mut self.section.get_mut(&section_id).expect("known section id").body,
            section_id,
            offset_index,
            value.into(),
        );
    }

    /// Sets all values to the provided ones. Note that this follows [`zip`]
    /// logic: if the number of values in the input is less than the number of
    /// values currently existing, then only the first `n` values are modified.
    /// If more values are provided than there currently are, then the
    /// remaining values are ignored.
    ///
    /// [`zip`]: std::iter::Iterator::zip
    pub fn set_values<'a, Iter, Item>(&mut self, values: Iter)
    where
        Iter: IntoIterator<Item = Item>,
        Item: Into<&'a BStr>,
    {
        for (
            EntryData {
                section_id,
                offset_index,
            },
            value,
        ) in self.indices_and_sizes.iter().zip(values)
        {
            Self::set_value_inner(
                &self.key,
                &mut self.offsets,
                &mut self.section.get_mut(section_id).expect("known section id").body,
                *section_id,
                *offset_index,
                value.into(),
            );
        }
    }

    /// Sets all values in this multivar to the provided one without owning the
    /// provided input.
    pub fn set_all<'a>(&mut self, input: impl Into<&'a BStr>) {
        let input = input.into();
        for EntryData {
            section_id,
            offset_index,
        } in &self.indices_and_sizes
        {
            Self::set_value_inner(
                &self.key,
                &mut self.offsets,
                &mut self.section.get_mut(section_id).expect("known section id").body,
                *section_id,
                *offset_index,
                input,
            );
        }
    }

    fn set_value_inner<'a: 'event>(
        value_name: &section::ValueName<'lookup>,
        offsets: &mut HashMap<SectionId, Vec<usize>>,
        section: &mut file::section::Body<'event>,
        section_id: SectionId,
        offset_index: usize,
        value: &BStr,
    ) {
        let (offset, size) = MultiValueMut::index_and_size(offsets, section_id, offset_index);
        let whitespace = Whitespace::from_body(section);
        let section = section.as_mut();
        section.drain(offset..offset + size);

        let key_sep_events = whitespace.key_value_separators();
        MultiValueMut::set_offset(offsets, section_id, offset_index, 2 + key_sep_events.len());
        section.insert(offset, Event::Value(escape_value(value).into()));
        section
            .splice(offset..offset, key_sep_events.into_iter().rev())
            .for_each(|_| {});
        section.insert(offset, Event::SectionValueName(value_name.to_owned()));
    }

    /// Removes the value at the given index. Does nothing when called multiple
    /// times in succession.
    ///
    /// # Safety
    ///
    /// This will panic if the index is out of range.
    pub fn delete(&mut self, index: usize) {
        let EntryData {
            section_id,
            offset_index,
        } = &self.indices_and_sizes[index];
        let (offset, size) = MultiValueMut::index_and_size(&self.offsets, *section_id, *offset_index);
        if size == 0 {
            return;
        }
        self.section
            .get_mut(section_id)
            .expect("known section id")
            .body
            .as_mut()
            .drain(offset..offset + size);

        Self::set_offset(&mut self.offsets, *section_id, *offset_index, 0);
        self.indices_and_sizes.remove(index);
    }

    /// Removes all values. Does nothing when called multiple times in
    /// succession.
    pub fn delete_all(&mut self) {
        for EntryData {
            section_id,
            offset_index,
        } in &self.indices_and_sizes
        {
            let (offset, size) = MultiValueMut::index_and_size(&self.offsets, *section_id, *offset_index);
            if size == 0 {
                continue;
            }
            self.section
                .get_mut(section_id)
                .expect("known section id")
                .body
                .as_mut()
                .drain(offset..offset + size);
            Self::set_offset(&mut self.offsets, *section_id, *offset_index, 0);
        }
        self.indices_and_sizes.clear();
    }

    fn index_and_size(
        offsets: &'lookup HashMap<SectionId, Vec<usize>>,
        section_id: SectionId,
        offset_index: usize,
    ) -> (usize, usize) {
        offsets
            .get(&section_id)
            .expect("known section id")
            .iter()
            .take(offset_index + 1)
            .fold((0, 0), |(total_ofs, ofs), size| (total_ofs + ofs, *size))
    }

    // This must be an associated function rather than a method to allow Rust
    // to split mutable borrows.
    fn set_offset(
        offsets: &mut HashMap<SectionId, Vec<usize>>,
        section_id: SectionId,
        offset_index: usize,
        value: usize,
    ) {
        *offsets
            .get_mut(&section_id)
            .expect("known section id")
            .get_mut(offset_index)
            .unwrap()
            .deref_mut() = value;
    }
}
