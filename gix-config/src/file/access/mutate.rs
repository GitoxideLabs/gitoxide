use bstr::BStr;
use gix_features::threading::OwnShared;

use crate::{
    AsBStrOpt, File,
    file::{self, IntoBStringOpt, Metadata, SectionId, SectionMut, rename_section, write::ends_with_newline},
    lookup,
    parse::{Event, FrontMatterEvents, Span, section},
};

impl IntoBStringOpt for Option<bstr::BString> {
    fn into_bstring_opt(self) -> Option<bstr::BString> {
        self
    }
}

impl IntoBStringOpt for bstr::BString {
    fn into_bstring_opt(self) -> Option<bstr::BString> {
        Some(self)
    }
}

impl IntoBStringOpt for String {
    fn into_bstring_opt(self) -> Option<bstr::BString> {
        Some(self.into())
    }
}

impl IntoBStringOpt for Vec<u8> {
    fn into_bstring_opt(self) -> Option<bstr::BString> {
        Some(self.into())
    }
}

impl<const N: usize> IntoBStringOpt for [u8; N] {
    fn into_bstring_opt(self) -> Option<bstr::BString> {
        Some(self.to_vec().into())
    }
}

impl<T: crate::AsBStr + ?Sized> IntoBStringOpt for &T {
    fn into_bstring_opt(self) -> Option<bstr::BString> {
        Some(self.as_bstr().to_owned())
    }
}

/// Mutating low-level access methods.
impl File {
    /// Returns the last mutable section with a given `name` and optional `subsection_name`, _if it exists_.
    pub fn section_mut(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
    ) -> Result<SectionMut<'_>, lookup::existing::Error> {
        self.section_mut_inner(name.as_ref(), subsection_name.as_bstr_opt())
    }

    fn section_mut_inner<'a>(
        &'a mut self,
        name: &str,
        subsection_name: Option<&BStr>,
    ) -> Result<SectionMut<'a>, lookup::existing::Error> {
        let id = self
            .section_ids_by_name_and_subname(name, subsection_name)?
            .next_back()
            .expect("BUG: Section lookup vec was empty");
        let nl = self.detect_newline_style_smallvec();
        Ok(self
            .section_mut_from_id(id, nl)
            .expect("BUG: Section did not have id from lookup"))
    }

    /// Returns the last found mutable section with a given `key`, identifying the name and subsection name like `core` or `remote.origin`.
    pub fn section_mut_by_key(&mut self, key: impl crate::AsBStr) -> Result<SectionMut<'_>, lookup::existing::Error> {
        let key = section::unvalidated::KeyRef::parse(&key).ok_or_else(lookup::existing::key_missing)?;
        self.section_mut_inner(key.section_name, key.subsection_name)
    }

    /// Return the mutable section identified by `id`, or `None` if it didn't exist.
    ///
    /// Note that `id` is stable across deletions and insertions.
    pub fn section_mut_by_id(&mut self, id: SectionId) -> Option<SectionMut<'_>> {
        let nl = self.detect_newline_style_smallvec();
        self.section_mut_from_id(id, nl)
    }

    /// Returns the last mutable section with a given `name` and optional `subsection_name`, _if it exists_, or create a new section.
    pub fn section_mut_or_create_new(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
    ) -> Result<SectionMut<'_>, section::header::Error> {
        self.section_mut_or_create_new_inner(name.as_ref(), subsection_name.as_bstr_opt())
    }

    pub(crate) fn section_mut_or_create_new_inner<'a>(
        &'a mut self,
        name: &str,
        subsection_name: Option<&BStr>,
    ) -> Result<SectionMut<'a>, section::header::Error> {
        self.section_mut_or_create_new_filter_inner(name, subsection_name, |_| true)
    }

    /// Returns an mutable section with a given `name` and optional `subsection_name`, _if it exists_ **and** passes `filter`, or create
    /// a new section.
    pub fn section_mut_or_create_new_filter(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<SectionMut<'_>, section::header::Error> {
        self.section_mut_or_create_new_filter_inner(name.as_ref(), subsection_name.as_bstr_opt(), filter)
    }

    pub(crate) fn section_mut_or_create_new_filter_inner<'a>(
        &'a mut self,
        name: &str,
        subsection_name: Option<&BStr>,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<SectionMut<'a>, section::header::Error> {
        match self
            .section_ids_by_name_and_subname(name, subsection_name)
            .ok()
            .and_then(|it| {
                it.rev()
                    .find(|id| self.sections.get(id).is_some_and(|s| filter(&s.meta)))
            }) {
            Some(id) => {
                let nl = self.detect_newline_style_smallvec();
                Ok(self
                    .section_mut_from_id(id, nl)
                    .expect("BUG: Section did not have id from lookup"))
            }
            None => self.new_section_inner(name, subsection_name.map(bstr::BString::from)),
        }
    }

    /// Returns the last found mutable section with a given `name` and optional `subsection_name`, that matches `filter`, _if it exists_.
    ///
    /// If there are sections matching `section_name` and `subsection_name` but the `filter` rejects all of them, `Ok(None)`
    /// is returned.
    pub fn section_mut_filter(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<file::SectionMut<'_>>, lookup::existing::Error> {
        self.section_mut_filter_inner(name.as_ref(), subsection_name.as_bstr_opt(), filter)
    }

    fn section_mut_filter_inner<'a>(
        &'a mut self,
        name: &str,
        subsection_name: Option<&BStr>,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<file::SectionMut<'a>>, lookup::existing::Error> {
        let id = self
            .section_ids_by_name_and_subname(name, subsection_name)?
            .rev()
            .find(|id| {
                let s = &self.sections[id];
                filter(&s.meta)
            });
        let nl = self.detect_newline_style_smallvec();
        Ok(id.and_then(move |id| self.section_mut_from_id(id, nl)))
    }

    /// Like [`section_mut_filter()`][File::section_mut_filter()], but identifies the with a given `key`,
    /// like `core` or `remote.origin`.
    pub fn section_mut_filter_by_key(
        &mut self,
        key: impl crate::AsBStr,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<file::SectionMut<'_>>, lookup::existing::Error> {
        let key = section::unvalidated::KeyRef::parse(&key).ok_or_else(lookup::existing::key_missing)?;
        self.section_mut_filter_inner(key.section_name, key.subsection_name, filter)
    }

    /// Adds a new section. If a subsection name was provided, then
    /// the generated header will use the modern subsection syntax.
    /// Returns a reference to the new section for immediate editing.
    ///
    /// # Examples
    ///
    /// Creating a new empty section:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// let mut git_config = gix_config::File::default();
    /// let section = git_config.new_section("hello", "world")?;
    /// let nl = section.newline().to_owned();
    /// assert_eq!(git_config.to_string(), format!("[hello \"world\"]{nl}"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Creating a new empty section and adding values to it:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// # use bstr::ByteSlice;
    /// # use gix_config::parse::section;
    /// let mut git_config = gix_config::File::default();
    /// let mut section = git_config.new_section("hello", "world")?;
    /// section.push("a", Some("b".into()))?;
    /// let nl = section.newline().to_owned();
    /// assert_eq!(git_config.to_string(), format!("[hello \"world\"]{nl}\ta = b{nl}"));
    /// let _section = git_config.new_section("core", None);
    /// assert_eq!(git_config.to_string(), format!("[hello \"world\"]{nl}\ta = b{nl}[core]{nl}"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new_section(
        &mut self,
        name: impl AsRef<str>,
        subsection: impl IntoBStringOpt,
    ) -> Result<SectionMut<'_>, section::header::Error> {
        self.new_section_inner(name.as_ref(), subsection.into_bstring_opt())
    }

    fn new_section_inner(
        &mut self,
        name: &str,
        subsection: Option<bstr::BString>,
    ) -> Result<SectionMut<'_>, section::header::Error> {
        let section = file::SectionData::new(name, subsection, OwnShared::clone(&self.meta), &mut self.backing)?;
        let id = self.push_section_internal(section);
        let nl = self.detect_newline_style_smallvec();
        let mut section = self.section_mut_from_id(id, nl).expect("each id yields a section");
        section.push_newline()?;
        Ok(section)
    }

    /// Removes the section with `name` and `subsection_name`, returning it if there was a matching section.
    /// If multiple sections have the same name, then the last one is returned. Note that
    /// later sections with the same name have precedent over earlier ones.
    ///
    /// # Examples
    ///
    /// Creating and removing a section:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// let mut git_config = gix_config::File::try_from(
    /// r#"[hello "world"]
    ///     some-value = 4
    /// "#)?;
    ///
    /// let section = git_config.remove_section("hello", "world");
    /// assert!(section.is_some());
    /// assert_eq!(git_config.to_string(), "");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Precedence example for removing sections with the same name:
    ///
    /// ```
    /// # use gix_config::File;
    /// # use std::convert::TryFrom;
    /// let mut git_config = gix_config::File::try_from(
    /// r#"[hello "world"]
    ///     some-value = 4
    /// [hello "world"]
    ///     some-value = 5
    /// "#)?;
    ///
    /// let section = git_config.remove_section("hello", "world");
    /// assert!(section.is_some());
    /// assert_eq!(git_config.to_string(), "[hello \"world\"]\n    some-value = 4\n");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove_section(&mut self, name: impl AsRef<str>, subsection_name: impl AsBStrOpt) -> Option<file::Section> {
        let id = self
            .section_ids_by_name_and_subname(name.as_ref(), subsection_name.as_bstr_opt())
            .ok()
            .and_then(|mut ids| ids.next_back());
        let id = id?;
        self.remove_section_by_id(id)
    }

    /// Remove the section identified by `id` if it exists and return it, or return `None` if no such section was present.
    ///
    /// Note that section ids are unambiguous even in the face of removals and additions of sections.
    pub fn remove_section_by_id(&mut self, id: SectionId) -> Option<file::Section> {
        let section = self.sections.remove(&id)?;
        let position = self.section_order_position(id);
        self.section_order.remove(position);
        let lookup_name = section::Name(section.header.name.to_bstring_in(&self.backing));
        file::util::remove_section_id_from_lookup(
            &mut self.section_lookup_tree,
            &lookup_name,
            section
                .header
                .subsection_name
                .as_ref()
                .map(|name| name.value_in(&self.backing)),
            id,
        );
        Some(file::Section::from_data(&section, &self.backing))
    }

    /// Removes the section with `name` and `subsection_name` that passed `filter`, returning the removed section
    /// if at least one section matched the `filter`.
    /// If multiple sections have the same name, then the last one is returned. Note that
    /// later sections with the same name have precedent over earlier ones.
    pub fn remove_section_filter(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<file::Section> {
        self.remove_section_filter_inner(name.as_ref(), subsection_name.as_bstr_opt(), filter)
    }

    fn remove_section_filter_inner(
        &mut self,
        name: &str,
        subsection_name: Option<&BStr>,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Option<file::Section> {
        let id = self
            .section_ids_by_name_and_subname(name, subsection_name)
            .ok()
            .into_iter()
            .flatten()
            .rev()
            .find(|id| self.sections.get(id).is_some_and(|section| filter(&section.meta)));
        let id = id?;
        self.remove_section_by_id(id)
    }

    /// Adds the provided `section` to the config, returning a mutable reference to it for immediate editing.
    /// Note that its meta-data will remain as is.
    pub fn push_section(&mut self, section: file::Section) -> Result<SectionMut<'_>, crate::parse::span::Error> {
        let section = section.into_data(&mut self.backing)?;
        let id = self.push_section_internal(section);
        let nl = self.detect_newline_style_smallvec();
        Ok(self.section_mut_from_id(id, nl).expect("each id yields a section"))
    }

    /// Renames all sections with `name` and `subsection_name` to use `new_name` and `new_subsection_name`.
    ///
    /// Multiple sections may have the same name, and all matching sections are renamed. Existing sections with the target name
    /// are preserved.
    pub fn rename_section(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        new_name: impl AsRef<str>,
        new_subsection_name: impl IntoBStringOpt,
    ) -> Result<(), rename_section::Error> {
        self.rename_section_filter(name, subsection_name, new_name, new_subsection_name, |_| true)
    }

    /// Renames all sections with `name` and `subsection_name` that pass `filter` to use `new_name` and
    /// `new_subsection_name`.
    ///
    /// Existing sections with the target name are preserved.
    ///
    /// A not-found error indicates that the `filter` rejected all candidates, leading to no section being renamed.
    pub fn rename_section_filter(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl AsBStrOpt,
        new_name: impl AsRef<str>,
        new_subsection_name: impl IntoBStringOpt,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<(), rename_section::Error> {
        let ids: Vec<_> = self
            .section_ids_by_name_and_subname(name.as_ref(), subsection_name.as_bstr_opt())?
            .filter(|id| filter(&self.sections.get(id).expect("each id has a section").meta))
            .collect();
        if ids.is_empty() {
            return Err(lookup::existing::key_missing());
        }
        use gix_error::ResultExt;
        let header = section::HeaderData::new_in(new_name, new_subsection_name.into_bstring_opt(), &mut self.backing)
            .or_erased()?;
        for id in ids {
            file::util::set_section_header(
                self.sections
                    .get_mut(&id)
                    .expect("each id from the lookup has a section"),
                &self.backing,
                &mut self.section_lookup_tree,
                &self.section_order,
                header.clone(),
            );
        }
        Ok(())
    }

    /// Append another File to the end of ourselves, without losing any information.
    pub fn append(&mut self, other: Self) -> Result<&mut Self, crate::parse::span::Error> {
        self.append_or_insert(other, None)
    }

    /// Append another File to the end of ourselves, without losing any information.
    pub(crate) fn append_or_insert(
        &mut self,
        mut other: Self,
        mut insert_after: Option<SectionId>,
    ) -> Result<&mut Self, crate::parse::span::Error> {
        let nl = self.detect_newline_style_smallvec();
        let our_last_section_before_append =
            insert_after.or_else(|| (self.next_section_id != 0).then(|| SectionId(self.next_section_id - 1)));
        let needs_separator = if other.frontmatter_events.is_empty() {
            false
        } else {
            let lhs_ends_with_newline = match our_last_section_before_append {
                Some(id) => self
                    .frontmatter_post_section
                    .get(&id)
                    .is_none_or(|events| ends_with_newline(events, &self.backing, &nl, true)),
                None => ends_with_newline(self.frontmatter_events.as_ref(), &self.backing, &nl, true),
            };
            !lhs_ends_with_newline
                && !other
                    .frontmatter_events
                    .first()
                    .is_none_or(|event| event.to_bstr_lossy_in(&other.backing).starts_with(nl.as_ref()))
        };
        let separator = needs_separator
            .then(|| Span::append(&mut self.backing, nl.as_ref()).map(Event::Newline))
            .transpose()?;
        other.rebase_events(self.backing.len())?;
        self.backing.extend_from_slice(&other.backing);

        fn extend_with_separator(lhs: &mut FrontMatterEvents, separator: Option<Event>, rhs: FrontMatterEvents) {
            if let Some(separator) = separator {
                lhs.push(separator);
            }
            lhs.extend(rhs);
        }
        for id in std::mem::take(&mut other.section_order) {
            let section = other.sections.remove(&id).expect("present");

            let new_id = match insert_after {
                Some(id) => {
                    let new_id = self.insert_section_after(section, id);
                    insert_after = Some(new_id);
                    new_id
                }
                None => self.push_section_internal(section),
            };

            if let Some(post_matter) = other.frontmatter_post_section.remove(&id) {
                self.frontmatter_post_section.insert(new_id, post_matter);
            }
        }

        if other.frontmatter_events.is_empty() {
            return Ok(self);
        }

        match our_last_section_before_append {
            Some(last_id) => extend_with_separator(
                self.frontmatter_post_section.entry(last_id).or_default(),
                separator,
                other.frontmatter_events,
            ),
            None => extend_with_separator(&mut self.frontmatter_events, separator, other.frontmatter_events),
        }
        Ok(self)
    }

    fn rebase_events(&mut self, offset: usize) -> Result<(), crate::parse::span::Error> {
        for event in &mut self.frontmatter_events {
            event.rebase(offset)?;
        }
        for events in self.frontmatter_post_section.values_mut() {
            for event in events {
                event.rebase(offset)?;
            }
        }
        for section in self.sections.values_mut() {
            section.header.rebase(offset)?;
            for event in &mut section.body.0 {
                event.rebase(offset)?;
            }
        }
        Ok(())
    }

    pub(crate) fn section_mut_from_id(
        &mut self,
        id: SectionId,
        newline: smallvec::SmallVec<[u8; 2]>,
    ) -> Option<SectionMut<'_>> {
        let section = self.sections.get_mut(&id)?;
        let lookup = file::mutable::section::LookupMut {
            tree: &mut self.section_lookup_tree,
            order: &self.section_order,
        };
        Some(section.to_mut(&mut self.backing, lookup, newline))
    }
}
