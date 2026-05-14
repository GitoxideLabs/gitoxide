use std::borrow::Cow;

use bstr::{BStr, ByteSlice};
use gix_features::threading::OwnShared;
use smallvec::SmallVec;

use crate::{
    AsKey, File,
    file::{
        self, Metadata, SectionId,
        write::{extract_newline, platform_newline},
    },
    lookup,
    parse::Event,
};

/// Read-only low-level access methods, as it requires generics for converting into
/// custom values defined in this crate like [`Integer`](crate::Integer) and
/// [`Color`](crate::Color).
impl<'event> File<'event> {
    /// Returns an interpreted value given a `key`.
    ///
    /// It's recommended to use one of the value types provide dby this crate
    /// as they implement the conversion, but this function is flexible and
    /// will accept any type that implements [`TryFrom<&BStr>`](TryFrom).
    ///
    /// Consider [`Self::values`] if you want to get all values of a multivar instead.
    ///
    /// If a `string` is desired, use the [`string()`](Self::string()) method instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gix_config::File;
    /// # use gix_config::{Integer, Boolean};
    /// # use std::borrow::Cow;
    /// # use std::convert::TryFrom;
    /// let config = r#"
    ///     [core]
    ///         a = 10k
    ///         c = false
    /// "#;
    /// let git_config = gix_config::File::try_from(config)?;
    /// // You can either use the turbofish to determine the type...
    /// let a_value = git_config.value::<Integer>("core.a")?;
    /// // ... or explicitly declare the type to avoid the turbofish
    /// let c_value: Boolean = git_config.value("core.c")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn value<'a, T: TryFrom<Cow<'a, BStr>>>(&'a self, key: impl AsKey) -> Result<T, lookup::Error<T::Error>> {
        let key = key.as_key();
        self.value_by(key.section_name, key.subsection_name, key.value_name)
    }

    /// Returns an interpreted value given a section, an optional subsection and
    /// value name.
    ///
    /// It's recommended to use one of the value types provide dby this crate
    /// as they implement the conversion, but this function is flexible and
    /// will accept any type that implements [`TryFrom<&BStr>`](std::convert::TryFrom).
    ///
    /// Consider [`Self::values`] if you want to get all values of a multivar instead.
    ///
    /// If a `string` is desired, use the [`string()`](Self::string()) method instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gix_config::File;
    /// # use gix_config::{Integer, Boolean};
    /// # use std::borrow::Cow;
    /// # use std::convert::TryFrom;
    /// let config = r#"
    ///     [core]
    ///         a = 10k
    ///         c = false
    /// "#;
    /// let git_config = gix_config::File::try_from(config)?;
    /// // You can either use the turbofish to determine the type...
    /// let a_value = git_config.value_by::<Integer>("core", None, "a")?;
    /// // ... or explicitly declare the type to avoid the turbofish
    /// let c_value: Boolean = git_config.value_by("core", None, "c")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn value_by<'a, T: TryFrom<Cow<'a, BStr>>>(
        &'a self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
    ) -> Result<T, lookup::Error<T::Error>> {
        T::try_from(self.raw_value_by(section_name, subsection_name, value_name)?)
            .map_err(lookup::Error::FailedConversion)
    }

    /// Like [`value()`](File::value()), but returning an `None` if the value wasn't found at `section[.subsection].value_name`
    pub fn try_value<'a, T: TryFrom<Cow<'a, BStr>>>(&'a self, key: impl AsKey) -> Option<Result<T, T::Error>> {
        let key = key.as_key();
        self.try_value_by(key.section_name, key.subsection_name, key.value_name)
    }

    /// Like [`value_by()`](File::value_by()), but returning an `None` if the value wasn't found at `section[.subsection].value_name`
    pub fn try_value_by<'a, T: TryFrom<Cow<'a, BStr>>>(
        &'a self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
    ) -> Option<Result<T, T::Error>> {
        self.raw_value_by(section_name, subsection_name, value_name)
            .ok()
            .map(T::try_from)
    }

    /// Returns all interpreted values given a section, an optional subsection
    /// and value name.
    ///
    /// It's recommended to use one of the value types provide dby this crate
    /// as they implement the conversion, but this function is flexible and
    /// will accept any type that implements [`TryFrom<&BStr>`](TryFrom).
    ///
    /// Consider [`Self::value`] if you want to get a single value
    /// (following last-one-wins resolution) instead.
    ///
    /// To access plain strings, use the [`strings()`](Self::strings()) method instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gix_config::File;
    /// # use gix_config::{Integer, Boolean};
    /// # use std::borrow::Cow;
    /// # use std::convert::TryFrom;
    /// # use bstr::ByteSlice;
    /// let config = r#"
    ///     [core]
    ///         a = true
    ///         c
    ///     [core]
    ///         a
    ///         a = false
    /// "#;
    /// let git_config = gix_config::File::try_from(config).unwrap();
    /// // You can either use the turbofish to determine the type...
    /// let a_value = git_config.values::<Boolean>("core.a")?;
    /// assert_eq!(
    ///     a_value,
    ///     vec![
    ///         Boolean(true),
    ///         Boolean(false),
    ///         Boolean(false),
    ///     ]
    /// );
    /// // ... or explicitly declare the type to avoid the turbofish
    /// let c_value: Vec<Boolean> = git_config.values("core.c").unwrap();
    /// assert_eq!(c_value, vec![Boolean(false)]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// [`value`]: crate::value
    /// [`TryFrom`]: std::convert::TryFrom
    pub fn values<'a, T: TryFrom<Cow<'a, BStr>>>(&'a self, key: impl AsKey) -> Result<Vec<T>, lookup::Error<T::Error>> {
        self.raw_values(key)?
            .into_iter()
            .map(T::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(lookup::Error::FailedConversion)
    }

    /// Returns all interpreted values given a section, an optional subsection
    /// and value name.
    ///
    /// It's recommended to use one of the value types provide dby this crate
    /// as they implement the conversion, but this function is flexible and
    /// will accept any type that implements [`TryFrom<&BStr>`](std::convert::TryFrom).
    ///
    /// Consider [`Self::value`] if you want to get a single value
    /// (following last-one-wins resolution) instead.
    ///
    /// To access plain strings, use the [`strings()`](Self::strings()) method instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use gix_config::File;
    /// # use gix_config::{Integer, Boolean};
    /// # use std::borrow::Cow;
    /// # use std::convert::TryFrom;
    /// # use bstr::ByteSlice;
    /// let config = r#"
    ///     [core]
    ///         a = true
    ///         c
    ///     [core]
    ///         a
    ///         a = false
    /// "#;
    /// let git_config = gix_config::File::try_from(config).unwrap();
    /// // You can either use the turbofish to determine the type...
    /// let a_value = git_config.values_by::<Boolean>("core", None, "a")?;
    /// assert_eq!(
    ///     a_value,
    ///     vec![
    ///         Boolean(true),
    ///         Boolean(false),
    ///         Boolean(false),
    ///     ]
    /// );
    /// // ... or explicitly declare the type to avoid the turbofish
    /// let c_value: Vec<Boolean> = git_config.values_by("core", None, "c").unwrap();
    /// assert_eq!(c_value, vec![Boolean(false)]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// [`value`]: crate::value
    /// [`TryFrom`]: std::convert::TryFrom
    pub fn values_by<'a, T: TryFrom<Cow<'a, BStr>>>(
        &'a self,
        section_name: &str,
        subsection_name: Option<&BStr>,
        value_name: &str,
    ) -> Result<Vec<T>, lookup::Error<T::Error>> {
        self.raw_values_by(section_name, subsection_name, value_name)?
            .into_iter()
            .map(T::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(lookup::Error::FailedConversion)
    }

    /// Returns the last found immutable section with a given `name` and optional `subsection_name`.
    pub fn section(
        &self,
        name: &str,
        subsection_name: Option<&BStr>,
    ) -> Result<&file::Section<'event>, lookup::existing::Error> {
        self.section_filter(name, subsection_name, |_| true)?
            .ok_or(lookup::existing::Error::SectionMissing)
    }

    /// Returns the last found immutable section with a given `section_key`, identifying the name and subsection name like `core`
    /// or `remote.origin`.
    pub fn section_by_key(&self, section_key: &BStr) -> Result<&file::Section<'event>, lookup::existing::Error> {
        let key =
            crate::parse::section::unvalidated::Key::parse(section_key).ok_or(lookup::existing::Error::KeyMissing)?;
        self.section(key.section_name, key.subsection_name)
    }

    /// Returns the last found immutable section with a given `name` and optional `subsection_name`, that matches `filter`.
    ///
    /// If there are sections matching `section_name` and `subsection_name` but the `filter` rejects all of them, `Ok(None)`
    /// is returned.
    pub fn section_filter<'a>(
        &'a self,
        name: &str,
        subsection_name: Option<&BStr>,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<&'a file::Section<'event>>, lookup::existing::Error> {
        Ok(self
            .section_ids_by_name_and_subname(name.as_ref(), subsection_name)?
            .rev()
            .find_map({
                let sections = &self.sections;
                move |id| {
                    let s = &sections[&id];
                    filter(s.meta()).then_some(s)
                }
            }))
    }

    /// Like [`section_filter()`](File::section_filter()), but identifies the section with `section_key` like `core` or `remote.origin`.
    pub fn section_filter_by_key<'a>(
        &'a self,
        section_key: &BStr,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<&'a file::Section<'event>>, lookup::existing::Error> {
        let key =
            crate::parse::section::unvalidated::Key::parse(section_key).ok_or(lookup::existing::Error::KeyMissing)?;
        self.section_filter(key.section_name, key.subsection_name, filter)
    }

    /// Gets all sections that match the provided `name`, ignoring any subsections.
    ///
    /// # Examples
    ///
    /// Provided the following config:
    ///
    /// ```text
    /// [core]
    ///     a = b
    /// [core ""]
    ///     c = d
    /// [core "apple"]
    ///     e = f
    /// ```
    ///
    /// Calling this method will yield all sections:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use gix_config::{Integer, Boolean};
    /// # use std::borrow::Cow;
    /// # use std::convert::TryFrom;
    /// let config = r#"
    ///     [core]
    ///         a = b
    ///     [core ""]
    ///         c = d
    ///     [core "apple"]
    ///         e = f
    /// "#;
    /// let git_config = gix_config::File::try_from(config)?;
    /// assert_eq!(git_config.sections_by_name("core").map_or(0, |s|s.count()), 3);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn sections_by_name<'a>(
        &'a self,
        name: &'a str,
    ) -> Option<impl Iterator<Item = &'a file::Section<'event>> + 'a> {
        self.section_ids_by_name(name).ok().map(move |ids| {
            ids.map(move |id| {
                self.sections
                    .get(&id)
                    .expect("section doesn't have id from from lookup")
            })
        })
    }

    /// Similar to [`sections_by_name()`](Self::sections_by_name()), but returns an identifier for this section as well to allow
    /// referring to it unambiguously even in the light of deletions.
    #[must_use]
    pub fn sections_and_ids_by_name<'a>(
        &'a self,
        name: &'a str,
    ) -> Option<impl Iterator<Item = (&'a file::Section<'event>, SectionId)> + 'a> {
        self.section_ids_by_name(name).ok().map(move |ids| {
            ids.map(move |id| {
                (
                    self.sections
                        .get(&id)
                        .expect("section doesn't have id from from lookup"),
                    id,
                )
            })
        })
    }

    /// Gets all sections that match the provided `name`, ignoring any subsections, and pass the `filter`.
    #[must_use]
    pub fn sections_by_name_and_filter<'a>(
        &'a self,
        name: &'a str,
        mut filter: impl FnMut(&Metadata) -> bool + 'a,
    ) -> Option<impl Iterator<Item = &'a file::Section<'event>> + 'a> {
        self.section_ids_by_name(name).ok().map(move |ids| {
            ids.filter_map(move |id| {
                let s = self
                    .sections
                    .get(&id)
                    .expect("section doesn't have id from from lookup");
                filter(s.meta()).then_some(s)
            })
        })
    }

    /// Returns the number of values in the config, no matter in which section.
    ///
    /// For example, a config with multiple empty sections will return 0.
    /// This ignores any comments.
    #[must_use]
    pub fn num_values(&self) -> usize {
        self.sections.values().map(|section| section.num_values()).sum()
    }

    /// Returns if there are no entries in the config. This will return true
    /// if there are only empty sections, with whitespace and comments not being considered
    /// void.
    #[must_use]
    pub fn is_void(&self) -> bool {
        self.sections.values().all(|s| s.body.is_void())
    }

    /// Return this file's metadata, typically set when it was first created to indicate its origins.
    ///
    /// It will be used in all newly created sections to identify them.
    /// Change it with [`File::set_meta()`].
    pub fn meta(&self) -> &Metadata {
        &self.meta
    }

    /// Change the origin of this instance to be the given `meta`data.
    ///
    /// This is useful to control what origin about-to-be-added sections receive.
    pub fn set_meta(&mut self, meta: impl Into<OwnShared<Metadata>>) -> &mut Self {
        self.meta = meta.into();
        self
    }

    /// Similar to [`meta()`](File::meta()), but with shared ownership.
    pub fn meta_owned(&self) -> OwnShared<Metadata> {
        OwnShared::clone(&self.meta)
    }

    /// Return an iterator over all sections, in order of occurrence in the file itself.
    pub fn sections(&self) -> impl Iterator<Item = &file::Section<'event>> + '_ {
        self.section_order.iter().map(move |id| &self.sections[id])
    }

    /// Return an iterator over all sections and their ids, in order of occurrence in the file itself.
    pub fn sections_and_ids(&self) -> impl Iterator<Item = (&file::Section<'event>, SectionId)> + '_ {
        self.section_order.iter().map(move |id| (&self.sections[id], *id))
    }

    /// Return an iterator over all section ids, in order of occurrence in the file itself.
    pub fn section_ids(&mut self) -> impl Iterator<Item = SectionId> + '_ {
        self.section_order.iter().copied()
    }

    /// Return an iterator over all sections along with non-section events that are placed right after them,
    /// in order of occurrence in the file itself.
    ///
    /// This allows to reproduce the look of sections perfectly when serializing them with
    /// [`write_to()`](file::Section::write_to()).
    pub fn sections_and_postmatter(&self) -> impl Iterator<Item = (&file::Section<'event>, Vec<&Event<'event>>)> {
        self.section_order.iter().map(move |id| {
            let s = &self.sections[id];
            let pm: Vec<_> = self
                .frontmatter_post_section
                .get(id)
                .map(|events| events.iter().collect())
                .unwrap_or_default();
            (s, pm)
        })
    }

    /// Return all events which are in front of the first of our sections, or `None` if there are none.
    pub fn frontmatter(&self) -> Option<impl Iterator<Item = &Event<'event>>> {
        (!self.frontmatter_events.is_empty()).then(|| self.frontmatter_events.iter())
    }

    /// Return the newline characters that have been detected in this config file or the default ones
    /// for the current platform.
    ///
    /// Note that the first found newline is the one we use in the assumption of consistency.
    pub fn detect_newline_style(&self) -> &BStr {
        self.frontmatter_events
            .iter()
            .find_map(extract_newline)
            .or_else(|| {
                self.sections()
                    .find_map(|s| s.body.as_ref().iter().find_map(extract_newline))
            })
            .unwrap_or_else(|| platform_newline())
    }

    pub(crate) fn detect_newline_style_smallvec(&self) -> SmallVec<[u8; 2]> {
        self.detect_newline_style().as_bytes().into()
    }
}
