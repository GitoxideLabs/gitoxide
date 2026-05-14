use std::borrow::Cow;

use bstr::BStr;

use crate::{AsKey, File, file::Metadata, value};

/// Comfortable API for accessing values
impl File<'_> {
    /// Like [`string_by()`](File::string_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn string(&self, key: impl AsKey) -> Option<Cow<'_, BStr>> {
        self.string_filter(key, |_| true)
    }

    /// Like [`value()`](File::value()), but returning `None` if the string wasn't found.
    ///
    /// As strings perform no conversions, this will never fail.
    pub fn string_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
    ) -> Option<Cow<'_, BStr>> {
        self.string_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref(), |_| true)
    }

    /// Like [`string_filter_by()`](File::string_filter_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn string_filter(&self, key: impl AsKey, filter: impl FnMut(&Metadata) -> bool) -> Option<Cow<'_, BStr>> {
        let key = key.try_as_key()?;
        self.raw_value_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
            .ok()
    }

    /// Like [`string()`](File::string()), but the section containing the returned value must pass `filter` as well.
    pub fn string_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<Cow<'_, BStr>> {
        self.raw_value_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref(), filter)
            .ok()
    }

    /// Like [`path_by()`](File::path_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn path(&self, key: impl AsKey) -> Option<crate::Path<'_>> {
        self.path_filter(key, |_| true)
    }

    /// Like [`value()`](File::value()), but returning `None` if the path wasn't found.
    ///
    /// Note that this path is not vetted and should only point to resources which can't be used
    /// to pose a security risk. Prefer using [`path_filter()`](File::path_filter()) instead.
    ///
    /// As paths perform no conversions, this will never fail.
    pub fn path_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
    ) -> Option<crate::Path<'_>> {
        self.path_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref(), |_| true)
    }

    /// Like [`path_filter_by()`](File::path_filter_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn path_filter(&self, key: impl AsKey, filter: impl FnMut(&Metadata) -> bool) -> Option<crate::Path<'_>> {
        let key = key.try_as_key()?;
        self.path_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Like [`path()`](File::path()), but the section containing the returned value must pass `filter` as well.
    ///
    /// This should be the preferred way of accessing paths as those from untrusted
    /// locations can be
    ///
    /// As paths perform no conversions, this will never fail.
    pub fn path_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<crate::Path<'_>> {
        self.raw_value_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref(), filter)
            .ok()
            .map(crate::Path::from)
    }

    /// Like [`boolean_by()`](File::boolean_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn boolean(&self, key: impl AsKey) -> Option<Result<bool, value::Error>> {
        self.boolean_filter(key, |_| true)
    }

    /// Like [`value()`](File::value()), but returning `None` if the boolean value wasn't found.
    pub fn boolean_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
    ) -> Option<Result<bool, value::Error>> {
        self.boolean_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref(), |_| true)
    }

    /// Like [`boolean_filter_by()`](File::boolean_filter_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn boolean_filter(
        &self,
        key: impl AsKey,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<Result<bool, value::Error>> {
        let key = key.try_as_key()?;
        self.boolean_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Like [`boolean_by()`](File::boolean_by()), but the section containing the returned value must pass `filter` as well.
    pub fn boolean_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<Result<bool, value::Error>> {
        let section_name = section_name.as_ref();
        let section_ids = self
            .section_ids_by_name_and_subname(section_name, subsection_name)
            .ok()?;
        let key = value_name.as_ref();
        for section_id in section_ids.rev() {
            let section = self.sections.get(&section_id).expect("known section id");
            if !filter(section.meta()) {
                continue;
            }
            match section.value_implicit(key) {
                Some(Some(v)) => return Some(crate::Boolean::try_from(v).map(Into::into)),
                Some(None) => return Some(Ok(true)),
                None => continue,
            }
        }
        None
    }

    /// Like [`integer_by()`](File::integer_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn integer(&self, key: impl AsKey) -> Option<Result<i64, value::Error>> {
        self.integer_filter(key, |_| true)
    }

    /// Like [`value()`](File::value()), but returning an `Option` if the integer wasn't found.
    pub fn integer_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
    ) -> Option<Result<i64, value::Error>> {
        self.integer_filter_by(section_name, subsection_name, value_name, |_| true)
    }

    /// Like [`integer_filter_by()`](File::integer_filter_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn integer_filter(
        &self,
        key: impl AsKey,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<Result<i64, value::Error>> {
        let key = key.try_as_key()?;
        self.integer_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Like [`integer_by()`](File::integer_by()), but the section containing the returned value must pass `filter` as well.
    pub fn integer_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<Result<i64, value::Error>> {
        let int = self
            .raw_value_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref(), filter)
            .ok()?;
        Some(crate::Integer::try_from(int.as_ref()).and_then(|b| {
            b.to_decimal()
                .ok_or_else(|| value::Error::new("Integer overflow", int.into_owned()))
        }))
    }

    /// Like [`strings_by()`](File::strings_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn strings(&self, key: impl AsKey) -> Option<Vec<Cow<'_, BStr>>> {
        let key = key.try_as_key()?;
        self.strings_by(key.section_name, key.subsection_name, key.value_name)
    }

    /// Similar to [`values_by(…)`](File::values_by()) but returning strings if at least one of them was found.
    pub fn strings_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
    ) -> Option<Vec<Cow<'_, BStr>>> {
        self.raw_values_by(section_name.as_ref(), subsection_name, value_name.as_ref())
            .ok()
    }

    /// Like [`strings_filter_by()`](File::strings_filter_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn strings_filter(&self, key: impl AsKey, filter: impl FnMut(&Metadata) -> bool) -> Option<Vec<Cow<'_, BStr>>> {
        let key = key.try_as_key()?;
        self.strings_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Similar to [`strings_by(…)`](File::strings_by()), but all values are in sections that passed `filter`.
    pub fn strings_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<Vec<Cow<'_, BStr>>> {
        self.raw_values_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref(), filter)
            .ok()
    }

    /// Like [`integers()`](File::integers()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn integers(&self, key: impl AsKey) -> Option<Result<Vec<i64>, value::Error>> {
        self.integers_filter(key, |_| true)
    }

    /// Similar to [`values_by(…)`](File::values_by()) but returning integers if at least one of them was found
    /// and if none of them overflows.
    pub fn integers_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
    ) -> Option<Result<Vec<i64>, value::Error>> {
        self.integers_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref(), |_| true)
    }

    /// Like [`integers_filter_by()`](File::integers_filter_by()), but suitable for statically known `key`s like `remote.origin.url`.
    pub fn integers_filter(
        &self,
        key: impl AsKey,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<Result<Vec<i64>, value::Error>> {
        let key = key.try_as_key()?;
        self.integers_filter_by(key.section_name, key.subsection_name, key.value_name, filter)
    }

    /// Similar to [`integers_by(…)`](File::integers_by()) but all integers are in sections that passed `filter`
    /// and that are not overflowing.
    pub fn integers_filter_by(
        &self,
        section_name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        value_name: impl AsRef<str>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<Result<Vec<i64>, value::Error>> {
        self.raw_values_filter_by(section_name.as_ref(), subsection_name, value_name.as_ref(), filter)
            .ok()
            .map(|values| {
                values
                    .into_iter()
                    .map(|v| {
                        crate::Integer::try_from(v.as_ref()).and_then(|int| {
                            int.to_decimal()
                                .ok_or_else(|| value::Error::new("Integer overflow", v.into_owned()))
                        })
                    })
                    .collect()
            })
    }
}
