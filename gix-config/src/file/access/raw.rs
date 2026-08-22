use std::collections::HashMap;

use bstr::{BStr, BString};
use gix_error::ResultExt;
use smallvec::ToSmallVec;

use crate::{
    AsBStrOpt, AsKey, File,
    file::{self, Index, Metadata, MultiValueMut, Size, ValueMut, mutable::multi_value::EntryData},
    lookup,
    parse::{Event, section},
};

/// # Raw value API
///
/// These functions are the raw value API, returning normalized byte strings.
impl File {
    /// Returns an uninterpreted value given a `key`.
    ///
    /// Consider [`Self::raw_values()`] if you want to get all values of
    /// a multivar instead.
    pub fn raw_value(&self, key: impl AsKey) -> Result<BString, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_value_filter_by(key.section_name, key.subsection_name, key.value_name, |_| true)
    }

    /// Returns an uninterpreted value given a section, an optional subsection
    /// and value name.
    ///
    /// Consider [`Self::raw_values()`] if you want to get all values of
    /// a multivar instead.
    pub fn raw_value_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
    ) -> Result<BString, lookup::existing::Error> {
        self.raw_value_filter_by(section_name, subsection_name, value_name, |_| true)
    }

    /// Returns an uninterpreted value and the section containing it given a `key`.
    ///
    /// Resolution is identical to [`raw_value()`][Self::raw_value()]: the last explicit value wins, even across
    /// multiple matching sections.
    pub fn raw_value_with_section(
        &self,
        key: impl AsKey,
    ) -> Result<(BString, file::SectionRef<'_>), lookup::existing::Error> {
        let key = key.as_key();
        self.raw_value_with_section_by(key.section_name, key.subsection_name, key.value_name)
    }

    /// Returns an uninterpreted value and the section containing it given its individual key components.
    ///
    /// Resolution is identical to [`raw_value_by()`][Self::raw_value_by()]: the last explicit value wins, even
    /// across multiple matching sections.
    pub fn raw_value_with_section_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
    ) -> Result<(BString, file::SectionRef<'_>), lookup::existing::Error> {
        self.raw_value_with_section_filter_by(section_name, subsection_name, value_name, |_| true)
    }

    /// Returns an uninterpreted value and the section containing it given a `key`, if the section passes `filter`.
    ///
    /// Resolution is identical to [`raw_value_filter()`][Self::raw_value_filter()]: the last explicit value in a
    /// matching section wins.
    pub fn raw_value_with_section_filter(
        &self,
        key: impl AsKey,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<(BString, file::SectionRef<'_>), lookup::existing::Error> {
        let key = key.as_key();
        self.raw_value_with_section_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Returns an uninterpreted value and the section containing it given its individual key components, if the
    /// section passes `filter`.
    pub fn raw_value_with_section_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<(BString, file::SectionRef<'_>), lookup::existing::Error> {
        self.raw_value_with_section_filter_inner(
            section_name.as_ref(),
            subsection_name.as_bstr_opt(),
            value_name.as_ref(),
            filter,
        )
    }

    /// Returns an uninterpreted value given a `key`, if it passes the `filter`.
    ///
    /// Consider [`Self::raw_values()`] if you want to get all values of
    /// a multivar instead.
    pub fn raw_value_filter(
        &self,
        key: impl AsKey,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<BString, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_value_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Returns an uninterpreted value given a section, an optional subsection
    /// and value name, if it passes the `filter`.
    ///
    /// Consider [`Self::raw_values()`] if you want to get all values of
    /// a multivar instead.
    pub fn raw_value_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<BString, lookup::existing::Error> {
        self.raw_value_filter_inner(
            section_name.as_ref(),
            subsection_name.as_bstr_opt(),
            value_name.as_ref(),
            filter,
        )
    }

    fn raw_value_filter_inner(
        &self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<BString, lookup::existing::Error> {
        self.raw_value_with_section_filter_inner(section_name, subsection_name, value_name, filter)
            .map(|(value, _section)| value)
    }

    fn raw_value_with_section_filter_inner(
        &self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<(BString, file::SectionRef<'_>), lookup::existing::Error> {
        let section_ids = self.section_ids_by_name_and_subname(section_name, subsection_name)?;
        for section_id in section_ids.rev() {
            let section = self.sections.get(&section_id).expect("known section id");
            if !filter(section.meta()) {
                continue;
            }
            if let Some(v) = section.body.value_implicit_in(&self.backing, value_name).flatten() {
                return Ok((v, file::SectionRef::from_data(section, &self.backing)));
            }
        }

        Err(lookup::existing::key_missing())
    }

    /// Returns a mutable reference to an uninterpreted value given a `key`.
    ///
    /// Consider [`Self::raw_values_mut`] if you want to get mutable
    /// references to all values of a multivar instead.
    pub fn raw_value_mut(&mut self, key: impl AsKey) -> Result<ValueMut<'_>, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_value_mut_filter_inner(key.section_name, key.subsection_name, key.value_name, |_| true)
    }

    /// Returns a mutable reference to an uninterpreted value given a section,
    /// an optional subsection and value name.
    ///
    /// Consider [`Self::raw_values_mut_by`] if you want to get mutable
    /// references to all values of a multivar instead.
    pub fn raw_value_mut_by(
        &mut self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
    ) -> Result<ValueMut<'_>, lookup::existing::Error> {
        self.raw_value_mut_filter_by(section_name, subsection_name, value_name, |_| true)
    }

    /// Returns a mutable reference to an uninterpreted value given a `key`, if its section passes `filter`.
    ///
    /// Consider [`Self::raw_values_mut_by`] if you want to get mutable
    /// references to all values of a multivar instead.
    pub fn raw_value_mut_filter(
        &mut self,
        key: impl AsKey,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<ValueMut<'_>, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_value_mut_filter_inner(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Returns a mutable reference to an uninterpreted value given a section, an optional subsection and value name,
    /// if its section passes `filter`.
    ///
    /// Consider [`Self::raw_values_mut_by`] if you want to get mutable references to all values of a multivar instead.
    pub fn raw_value_mut_filter_by(
        &mut self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<ValueMut<'_>, lookup::existing::Error> {
        self.raw_value_mut_filter_inner(
            section_name.as_ref(),
            subsection_name.as_bstr_opt(),
            value_name.as_ref(),
            filter,
        )
    }

    fn raw_value_mut_filter_inner(
        &mut self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<ValueMut<'_>, lookup::existing::Error> {
        let mut section_ids = self
            .section_ids_by_name_and_subname(section_name, subsection_name)?
            .rev();
        let key = section::ValueName::try_from(value_name).or_erased()?;

        while let Some(section_id) = section_ids.next() {
            let mut index = 0;
            let mut size = 0;
            let mut found_key = false;
            let section = self.sections.get(&section_id).expect("known section id");
            if !filter(section.meta()) {
                continue;
            }
            for (i, event) in section.as_ref().iter().enumerate() {
                match event {
                    Event::SectionValueName(event_key)
                        if event_key
                            .as_bstr_in(&self.backing)
                            .eq_ignore_ascii_case(key.0.as_slice()) =>
                    {
                        found_key = true;
                        index = i;
                        size = 1;
                    }
                    Event::Newline(_) | Event::Whitespace(_) | Event::ValueNotDone(_) if found_key => {
                        size += 1;
                    }
                    Event::ValueDone(_) | Event::Value(_) if found_key => {
                        found_key = false;
                        size += 1;
                    }
                    Event::KeyValueSeparator if found_key => {
                        size += 1;
                    }
                    _ => {}
                }
            }

            if size == 0 {
                continue;
            }

            drop(section_ids);
            let nl = self.detect_newline_style().to_smallvec();
            return Ok(ValueMut {
                section: self.section_mut_from_id(section_id, nl).expect("known section-id"),
                key,
                index: Index(index),
                size: Size(size),
            });
        }

        Err(lookup::existing::key_missing())
    }

    /// Returns all uninterpreted values given a `key`.
    ///
    /// The ordering means that the last of the returned values is the one that would be the
    /// value used in the single-value case.
    ///
    /// # Examples
    ///
    /// If you have the following config:
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// [core]
    ///     a = c
    ///     a = d
    /// ```
    ///
    /// Attempting to get all values of `a` yields the following:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// assert_eq!(
    ///     git_config.raw_values("core.a").unwrap(),
    ///     vec![
    ///         bstr::BString::from("b"),
    ///         bstr::BString::from("c"),
    ///         bstr::BString::from("d"),
    ///     ],
    /// );
    /// ```
    ///
    /// Consider [`Self::raw_value`] if you want to get the resolved single
    /// value for a given key, if your value does not support multi-valued values.
    pub fn raw_values(&self, key: impl AsKey) -> Result<Vec<BString>, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_values_by(key.section_name, key.subsection_name, key.value_name)
    }

    /// Returns all uninterpreted values given a section, an optional subsection
    /// and value name in order of occurrence.
    ///
    /// The ordering means that the last of the returned values is the one that would be the
    /// value used in the single-value case.
    ///
    /// # Examples
    ///
    /// If you have the following config:
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// [core]
    ///     a = c
    ///     a = d
    /// ```
    ///
    /// Attempting to get all values of `a` yields the following:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// assert_eq!(
    ///     git_config.raw_values_by("core", None, "a").unwrap(),
    ///     vec![
    ///         bstr::BString::from("b"),
    ///         bstr::BString::from("c"),
    ///         bstr::BString::from("d"),
    ///     ],
    /// );
    /// ```
    ///
    /// Consider [`Self::raw_value`] if you want to get the resolved single
    /// value for a given value name, if your value does not support multi-valued values.
    pub fn raw_values_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
    ) -> Result<Vec<BString>, lookup::existing::Error> {
        self.raw_values_filter_by(section_name, subsection_name, value_name, |_| true)
    }

    /// Returns all uninterpreted values and their containing sections given a `key`, in order of occurrence.
    pub fn raw_values_with_sections(
        &self,
        key: impl AsKey,
    ) -> Result<Vec<(BString, file::SectionRef<'_>)>, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_values_with_sections_by(key.section_name, key.subsection_name, key.value_name)
    }

    /// Returns all uninterpreted values and their containing sections given individual key components, in order of
    /// occurrence.
    pub fn raw_values_with_sections_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
    ) -> Result<Vec<(BString, file::SectionRef<'_>)>, lookup::existing::Error> {
        self.raw_values_with_sections_filter_by(section_name, subsection_name, value_name, |_| true)
    }

    /// Returns all uninterpreted values and their containing sections given a `key`, if their sections pass `filter`,
    /// in order of occurrence.
    pub fn raw_values_with_sections_filter(
        &self,
        key: impl AsKey,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Vec<(BString, file::SectionRef<'_>)>, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_values_with_sections_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Returns all uninterpreted values and their containing sections given individual key components, if their
    /// sections pass `filter`, in order of occurrence.
    pub fn raw_values_with_sections_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Vec<(BString, file::SectionRef<'_>)>, lookup::existing::Error> {
        self.raw_values_with_sections_filter_inner(
            section_name.as_ref(),
            subsection_name.as_bstr_opt(),
            value_name.as_ref(),
            filter,
        )
    }

    /// Returns all uninterpreted values given a `key`, if the value passes `filter`, in order of occurrence.
    ///
    /// The ordering means that the last of the returned values is the one that would be the
    /// value used in the single-value case.
    pub fn raw_values_filter(
        &self,
        key: impl AsKey,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Vec<BString>, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_values_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Returns all uninterpreted values given a section, an optional subsection
    /// and value name, if the value passes `filter`, in order of occurrence.
    ///
    /// The ordering means that the last of the returned values is the one that would be the
    /// value used in the single-value case.
    pub fn raw_values_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Vec<BString>, lookup::existing::Error> {
        self.raw_values_filter_inner(
            section_name.as_ref(),
            subsection_name.as_bstr_opt(),
            value_name.as_ref(),
            filter,
        )
    }

    fn raw_values_filter_inner(
        &self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Vec<BString>, lookup::existing::Error> {
        self.raw_values_with_sections_filter_inner(section_name, subsection_name, value_name, filter)
            .map(|values| values.into_iter().map(|(value, _section)| value).collect())
    }

    fn raw_values_with_sections_filter_inner(
        &self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Vec<(BString, file::SectionRef<'_>)>, lookup::existing::Error> {
        let mut values = Vec::new();
        let section_ids = self.section_ids_by_name_and_subname(section_name, subsection_name)?;
        for section_id in section_ids {
            let section = self.sections.get(&section_id).expect("known section id");
            if !filter(section.meta()) {
                continue;
            }
            let section_ref = file::SectionRef::from_data(section, &self.backing);
            values.extend(
                section
                    .body
                    .values_in(&self.backing, value_name)
                    .into_iter()
                    .map(|value| (value, section_ref)),
            );
        }

        if values.is_empty() {
            Err(lookup::existing::key_missing())
        } else {
            Ok(values)
        }
    }

    /// Returns mutable references to all uninterpreted values given a `key`.
    ///
    /// # Examples
    ///
    /// If you have the following config:
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// [core]
    ///     a = c
    ///     a = d
    /// ```
    ///
    /// Attempting to get all values of `a` yields the following:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// assert_eq!(
    ///     git_config.raw_values("core.a").expect("values exist"),
    ///     vec![
    ///         bstr::BString::from("b"),
    ///         bstr::BString::from("c"),
    ///         bstr::BString::from("d")
    ///     ]
    /// );
    ///
    /// git_config.raw_values_mut("core.a").expect("values exist").set_all("g");
    ///
    /// assert_eq!(
    ///     git_config.raw_values("core.a").expect("values exist"),
    ///     vec![
    ///         bstr::BString::from("g"),
    ///         bstr::BString::from("g"),
    ///         bstr::BString::from("g")
    ///     ],
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Consider [`Self::raw_value`] if you want to get the resolved single
    /// value for a given value name, if your value does not support multi-valued values.
    ///
    /// Note that this operation is relatively expensive, requiring a full
    /// traversal of the config.
    pub fn raw_values_mut(&mut self, key: impl AsKey) -> Result<MultiValueMut<'_>, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_values_mut_filter_inner(key.section_name, key.subsection_name, key.value_name, |_| true)
    }

    /// Returns mutable references to all uninterpreted values given a section,
    /// an optional subsection and value name.
    ///
    /// # Examples
    ///
    /// If you have the following config:
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// [core]
    ///     a = c
    ///     a = d
    /// ```
    ///
    /// Attempting to get all values of `a` yields the following:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// assert_eq!(
    ///     git_config.raw_values("core.a").expect("values exist"),
    ///     vec![
    ///         bstr::BString::from("b"),
    ///         bstr::BString::from("c"),
    ///         bstr::BString::from("d")
    ///     ]
    /// );
    ///
    /// git_config.raw_values_mut_by("core", None, "a").expect("values exist").set_all("g");
    ///
    /// assert_eq!(
    ///     git_config.raw_values("core.a").expect("values exist"),
    ///     vec![
    ///         bstr::BString::from("g"),
    ///         bstr::BString::from("g"),
    ///         bstr::BString::from("g")
    ///     ],
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Consider [`Self::raw_value`] if you want to get the resolved single
    /// value for a given value name, if your value does not support multi-valued values.
    ///
    /// Note that this operation is relatively expensive, requiring a full
    /// traversal of the config.
    pub fn raw_values_mut_by(
        &mut self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
    ) -> Result<MultiValueMut<'_>, lookup::existing::Error> {
        self.raw_values_mut_filter_by(section_name, subsection_name, value_name, |_| true)
    }

    /// Returns mutable references to all uninterpreted values given a `key`,
    /// if their sections pass `filter`.
    pub fn raw_values_mut_filter(
        &mut self,
        key: impl AsKey,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<MultiValueMut<'_>, lookup::existing::Error> {
        let key = key.as_key();
        self.raw_values_mut_filter_inner(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Returns mutable references to all uninterpreted values given a section,
    /// an optional subsection and value name, if their sections pass `filter`.
    pub fn raw_values_mut_filter_by(
        &mut self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<MultiValueMut<'_>, lookup::existing::Error> {
        self.raw_values_mut_filter_inner(
            section_name.as_ref(),
            subsection_name.as_bstr_opt(),
            value_name.as_ref(),
            filter,
        )
    }

    fn raw_values_mut_filter_inner(
        &mut self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<MultiValueMut<'_>, lookup::existing::Error> {
        let section_ids = self.section_ids_by_name_and_subname(section_name, subsection_name)?;
        let key = section::ValueName::try_from(value_name).or_erased()?;

        let mut offsets = HashMap::new();
        let mut entries = Vec::new();
        for section_id in section_ids.rev() {
            let mut last_boundary = 0;
            let mut expect_value = false;
            let mut offset_list = Vec::new();
            let mut offset_index = 0;
            let section = self.sections.get(&section_id).expect("known section-id");
            if !filter(section.meta()) {
                continue;
            }
            for (i, event) in section.as_ref().iter().enumerate() {
                match event {
                    Event::SectionValueName(event_key)
                        if event_key
                            .as_bstr_in(&self.backing)
                            .eq_ignore_ascii_case(key.0.as_slice()) =>
                    {
                        expect_value = true;
                        offset_list.push(i - last_boundary);
                        offset_index += 1;
                        last_boundary = i;
                    }
                    Event::Value(_) | Event::ValueDone(_) if expect_value => {
                        expect_value = false;
                        entries.push(EntryData {
                            section_id,
                            offset_index,
                        });
                        offset_list.push(i - last_boundary + 1);
                        offset_index += 1;
                        last_boundary = i + 1;
                    }
                    _ => (),
                }
            }
            offsets.insert(section_id, offset_list);
        }

        entries.sort();

        if entries.is_empty() {
            Err(lookup::existing::key_missing())
        } else {
            Ok(MultiValueMut {
                section: &mut self.sections,
                backing: &mut self.backing,
                key,
                indices_and_sizes: entries,
                offsets,
            })
        }
    }

    /// Sets a value in a given `key`.
    /// Note that the parts leading to the value name must exist for this method to work, i.e. the
    /// section and the subsection, if present.
    ///
    /// # Examples
    ///
    /// Given the config,
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// [core]
    ///     a = c
    ///     a = d
    /// ```
    ///
    /// Setting a new value to the key `core.a` will yield the following:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// git_config.set_existing_raw_value("core.a", "e").expect("value exists");
    /// assert_eq!(git_config.raw_value("core.a").expect("value exists"), "e");
    /// assert_eq!(
    ///     git_config.raw_values("core.a").expect("values exist"),
    ///     vec![
    ///         bstr::BString::from("b"),
    ///         bstr::BString::from("c"),
    ///         bstr::BString::from("e")
    ///     ],
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_existing_raw_value(
        &mut self,
        key: impl AsKey,
        new_value: impl crate::AsBStr,
    ) -> Result<(), crate::file::set_raw_value::Error> {
        let key = key.as_key();
        self.raw_value_mut_filter_inner(key.section_name, key.subsection_name, key.value_name, |_| true)?
            .set(new_value)
            .or_erased()?;
        Ok(())
    }

    /// Sets a value in a given `section_name`, optional `subsection_name`, and `value_name`.
    /// Note sections named `section_name` and `subsection_name` (if not `None`)
    /// must exist for this method to work.
    ///
    /// # Examples
    ///
    /// Given the config,
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// [core]
    ///     a = c
    ///     a = d
    /// ```
    ///
    /// Setting a new value to the key `core.a` will yield the following:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// git_config.set_existing_raw_value_by("core", None, "a", "e").expect("value exists");
    /// assert_eq!(git_config.raw_value("core.a").expect("value exists"), "e");
    /// assert_eq!(
    ///     git_config.raw_values("core.a").expect("values exist"),
    ///     vec![
    ///         bstr::BString::from("b"),
    ///         bstr::BString::from("c"),
    ///         bstr::BString::from("e")
    ///     ],
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_existing_raw_value_by(
        &mut self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        new_value: impl crate::AsBStr,
    ) -> Result<(), crate::file::set_raw_value::Error> {
        self.raw_value_mut_by(section_name, subsection_name, value_name)?
            .set(new_value)
            .or_erased()?;
        Ok(())
    }

    /// Sets a value in a given `key`.
    /// Creates the section if necessary and the value as well, or overwrites the last existing value otherwise.
    ///
    /// # Examples
    ///
    /// Given the config,
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// ```
    ///
    /// Setting a new value to the key `core.a` will yield the following:
    ///
    /// ```
    /// # use gix_config::File;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b").unwrap();
    /// let prev = git_config.set_raw_value(&"core.a", "e").expect("valid value");
    /// git_config.set_raw_value(&"core.b", "f").expect("valid value");
    /// assert_eq!(prev.expect("present"), "b");
    /// assert_eq!(git_config.raw_value("core.a").expect("value exists"), "e");
    /// assert_eq!(git_config.raw_value("core.b").expect("value exists"), "f");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_raw_value(
        &mut self,
        key: impl AsKey,
        new_value: impl crate::AsBStr,
    ) -> Result<Option<BString>, crate::file::set_raw_value::Error> {
        self.set_raw_value_filter(key, new_value, |_| true)
    }

    /// Sets a value in a given `section_name`, optional `subsection_name`, and `value_name`.
    /// Creates the section if necessary and the value as well, or overwrites the last existing value otherwise.
    ///
    /// # Examples
    ///
    /// Given the config,
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// ```
    ///
    /// Setting a new value to the key `core.a` will yield the following:
    ///
    /// ```
    /// # use gix_config::File;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b").unwrap();
    /// let prev = git_config.set_raw_value_by("core", None, "a", "e").expect("valid value");
    /// git_config.set_raw_value_by("core", None, "b", "f").expect("valid value");
    /// assert_eq!(prev.expect("present"), "b");
    /// assert_eq!(git_config.raw_value("core.a").expect("value exists"), "e");
    /// assert_eq!(git_config.raw_value("core.b").expect("value exists"), "f");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_raw_value_by(
        &mut self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        new_value: impl crate::AsBStr,
    ) -> Result<Option<BString>, crate::file::set_raw_value::Error> {
        self.set_raw_value_filter_by(section_name, subsection_name, value_name, new_value, |_| true)
    }

    /// Similar to [`set_raw_value()`](Self::set_raw_value()), but only sets existing values in sections matching
    /// `filter`, creating a new section otherwise.
    pub fn set_raw_value_filter(
        &mut self,
        key: impl AsKey,
        new_value: impl crate::AsBStr,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<BString>, crate::file::set_raw_value::Error> {
        let key = key.as_key();
        self.set_raw_value_filter_by_inner(key.section_name, key.subsection_name, key.value_name, new_value, filter)
    }

    /// Similar to [`set_raw_value_by()`](Self::set_raw_value_by()), but only sets existing values in sections matching
    /// `filter`, creating a new section otherwise.
    pub fn set_raw_value_filter_by(
        &mut self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        new_value: impl crate::AsBStr,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<BString>, crate::file::set_raw_value::Error> {
        self.set_raw_value_filter_by_inner(
            section_name.as_ref(),
            subsection_name.as_bstr_opt(),
            value_name.as_ref(),
            new_value,
            filter,
        )
    }

    fn set_raw_value_filter_by_inner(
        &mut self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
        new_value: impl crate::AsBStr,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<BString>, crate::file::set_raw_value::Error> {
        let key = section::ValueName::try_from(value_name).or_erased()?;
        let mut section = self
            .section_mut_or_create_new_filter_inner(section_name, subsection_name, filter)
            .or_erased()?;
        section.set_inner(key, new_value.as_bstr()).or_erased()
    }

    /// Sets a multivar in a given `key`.
    ///
    /// This internally zips together the new values and the existing values.
    /// As a result, if more new values are provided than the current amount of
    /// multivars, then the latter values are not applied. If there are less
    /// new values than old ones then the remaining old values are unmodified.
    ///
    /// **Note**: Mutation order is _not_ guaranteed and is non-deterministic.
    /// If you need finer control over which values of the multivar are set,
    /// consider using [`raw_values_mut()`](Self::raw_values_mut()), which will let you iterate
    /// and check over the values instead. This is best used as a convenience
    /// function for setting multivars whose values should be treated as an
    /// unordered set.
    ///
    /// # Examples
    ///
    /// Let us use the follow config for all examples:
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// [core]
    ///     a = c
    ///     a = d
    /// ```
    ///
    /// Setting an equal number of values:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// let new_values = vec![
    ///     "x",
    ///     "y",
    ///     "z",
    /// ];
    /// git_config.set_existing_raw_multi_value("core.a", new_values.into_iter()).expect("values exist");
    /// let fetched_config = git_config.raw_values("core.a").expect("values exist");
    /// assert!(fetched_config.iter().any(|v| v == "x"));
    /// assert!(fetched_config.iter().any(|v| v == "y"));
    /// assert!(fetched_config.iter().any(|v| v == "z"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Setting less than the number of present values sets the first ones found:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// let new_values = vec![
    ///     "x",
    ///     "y",
    /// ];
    /// git_config.set_existing_raw_multi_value("core.a", new_values.into_iter()).expect("values exist");
    /// let fetched_config = git_config.raw_values("core.a").expect("values exist");
    /// assert!(fetched_config.iter().any(|v| v == "x"));
    /// assert!(fetched_config.iter().any(|v| v == "y"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Setting more than the number of present values discards the rest:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// let new_values = vec![
    ///     "x",
    ///     "y",
    ///     "z",
    ///     "discarded",
    /// ];
    /// git_config.set_existing_raw_multi_value("core.a", new_values).expect("values exist");
    /// assert!(!git_config.raw_values("core.a").expect("values exist").iter().any(|v| v == "discarded"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_existing_raw_multi_value<Iter, Item>(
        &mut self,
        key: impl AsKey,
        new_values: Iter,
    ) -> Result<(), crate::file::set_raw_value::Error>
    where
        Iter: IntoIterator<Item = Item>,
        Item: crate::AsBStr,
    {
        let key = key.as_key();
        self.raw_values_mut_filter_inner(key.section_name, key.subsection_name, key.value_name, |_| true)?
            .set_values(new_values)
            .or_erased()?;
        Ok(())
    }

    /// Sets a multivar in a given section, optional subsection, and key value.
    ///
    /// This internally zips together the new values and the existing values.
    /// As a result, if more new values are provided than the current amount of
    /// multivars, then the latter values are not applied. If there are less
    /// new values than old ones then the remaining old values are unmodified.
    ///
    /// **Note**: Mutation order is _not_ guaranteed and is non-deterministic.
    /// If you need finer control over which values of the multivar are set,
    /// consider using [`raw_values_mut()`](Self::raw_values_mut()), which will let you iterate
    /// and check over the values instead. This is best used as a convenience
    /// function for setting multivars whose values should be treated as an
    /// unordered set.
    ///
    /// # Examples
    ///
    /// Let us use the follow config for all examples:
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// [core]
    ///     a = c
    ///     a = d
    /// ```
    ///
    /// Setting an equal number of values:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// let new_values = vec![
    ///     "x",
    ///     "y",
    ///     "z",
    /// ];
    /// git_config.set_existing_raw_multi_value_by("core", None, "a", new_values.into_iter()).expect("values exist");
    /// let fetched_config = git_config.raw_values("core.a").expect("values exist");
    /// assert!(fetched_config.iter().any(|v| v == "x"));
    /// assert!(fetched_config.iter().any(|v| v == "y"));
    /// assert!(fetched_config.iter().any(|v| v == "z"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Setting less than the number of present values sets the first ones found:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// let new_values = vec![
    ///     "x",
    ///     "y",
    /// ];
    /// git_config.set_existing_raw_multi_value_by("core", None, "a", new_values.into_iter()).expect("values exist");
    /// let fetched_config = git_config.raw_values("core.a").expect("values exist");
    /// assert!(fetched_config.iter().any(|v| v == "x"));
    /// assert!(fetched_config.iter().any(|v| v == "y"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Setting more than the number of present values discards the rest:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # let mut git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
    /// let new_values = vec![
    ///     "x",
    ///     "y",
    ///     "z",
    ///     "discarded",
    /// ];
    /// git_config.set_existing_raw_multi_value_by("core", None, "a", new_values).expect("values exist");
    /// assert!(!git_config.raw_values("core.a").expect("values exist").iter().any(|v| v == "discarded"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_existing_raw_multi_value_by<Iter, Item>(
        &mut self,
        section_name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        value_name: impl AsRef<str>,
        new_values: Iter,
    ) -> Result<(), crate::file::set_raw_value::Error>
    where
        Iter: IntoIterator<Item = Item>,
        Item: crate::AsBStr,
    {
        self.raw_values_mut_by(section_name, subsection_name, value_name)?
            .set_values(new_values)
            .or_erased()?;
        Ok(())
    }
}
