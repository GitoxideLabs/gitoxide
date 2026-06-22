use bstr::{BStr, ByteSlice};
use gix_features::threading::OwnShared;

use crate::{
    File,
    file::{self, Metadata, SectionBodyIdsLut, SectionId, SectionMut, rename_section, write::ends_with_newline},
    lookup,
    parse::{Event, FrontMatterEvents, Span, section},
};

/// Convert subsection inputs for creating new sections.
pub trait IntoSubsection {
    /// Convert into an optional owned subsection name.
    fn into_subsection(self) -> Option<bstr::BString>;
}

impl IntoSubsection for Option<bstr::BString> {
    fn into_subsection(self) -> Option<bstr::BString> {
        self
    }
}

impl IntoSubsection for bstr::BString {
    fn into_subsection(self) -> Option<bstr::BString> {
        Some(self)
    }
}

/// Mutating low-level access methods.
impl File {
    /// Returns the last mutable section with a given `name` and optional `subsection_name`, _if it exists_.
    pub fn section_mut<'a>(
        &'a mut self,
        name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
    ) -> Result<SectionMut<'a>, lookup::existing::Error> {
        self.section_mut_inner(name.as_ref(), subsection_name)
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
    pub fn section_mut_by_key<'a, 'b>(
        &'a mut self,
        key: impl Into<&'b BStr>,
    ) -> Result<SectionMut<'a>, lookup::existing::Error> {
        let key = section::unvalidated::Key::parse(key).ok_or(lookup::existing::Error::KeyMissing)?;
        self.section_mut(key.section_name, key.subsection_name)
    }

    /// Return the mutable section identified by `id`, or `None` if it didn't exist.
    ///
    /// Note that `id` is stable across deletions and insertions.
    pub fn section_mut_by_id(&mut self, id: SectionId) -> Option<SectionMut<'_>> {
        let nl = self.detect_newline_style_smallvec();
        self.section_mut_from_id(id, nl)
    }

    /// Returns the last mutable section with a given `name` and optional `subsection_name`, _if it exists_, or create a new section.
    pub fn section_mut_or_create_new<'a>(
        &'a mut self,
        name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
    ) -> Result<SectionMut<'a>, section::header::Error> {
        self.section_mut_or_create_new_filter(name, subsection_name, |_| true)
    }

    /// Returns an mutable section with a given `name` and optional `subsection_name`, _if it exists_ **and** passes `filter`, or create
    /// a new section.
    pub fn section_mut_or_create_new_filter<'a>(
        &'a mut self,
        name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<SectionMut<'a>, section::header::Error> {
        self.section_mut_or_create_new_filter_inner(name.as_ref(), subsection_name, filter)
    }

    fn section_mut_or_create_new_filter_inner<'a>(
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
            None => self.new_section(name, subsection_name.map(bstr::BString::from)),
        }
    }

    /// Returns the last found mutable section with a given `name` and optional `subsection_name`, that matches `filter`, _if it exists_.
    ///
    /// If there are sections matching `section_name` and `subsection_name` but the `filter` rejects all of them, `Ok(None)`
    /// is returned.
    pub fn section_mut_filter<'a>(
        &'a mut self,
        name: impl AsRef<str>,
        subsection_name: Option<&BStr>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<file::SectionMut<'a>>, lookup::existing::Error> {
        self.section_mut_filter_inner(name.as_ref(), subsection_name, filter)
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
    pub fn section_mut_filter_by_key<'a, 'b>(
        &'a mut self,
        key: impl Into<&'b BStr>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<Option<file::SectionMut<'a>>, lookup::existing::Error> {
        let key = section::unvalidated::Key::parse(key).ok_or(lookup::existing::Error::KeyMissing)?;
        self.section_mut_filter(key.section_name, key.subsection_name, filter)
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
    /// let section = git_config.new_section("hello", bstr::BString::from("world"))?;
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
    /// let mut section = git_config.new_section("hello", bstr::BString::from("world"))?;
    /// section.push(section::ValueName::try_from("a")?, Some("b".into()));
    /// let nl = section.newline().to_owned();
    /// assert_eq!(git_config.to_string(), format!("[hello \"world\"]{nl}\ta = b{nl}"));
    /// let _section = git_config.new_section("core", None);
    /// assert_eq!(git_config.to_string(), format!("[hello \"world\"]{nl}\ta = b{nl}[core]{nl}"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new_section(
        &mut self,
        name: impl AsRef<str>,
        subsection: impl IntoSubsection,
    ) -> Result<SectionMut<'_>, section::header::Error> {
        self.new_section_inner(name.as_ref(), subsection.into_subsection())
    }

    fn new_section_inner(
        &mut self,
        name: &str,
        subsection: Option<bstr::BString>,
    ) -> Result<SectionMut<'_>, section::header::Error> {
        let id = self.push_section_internal(file::SectionData::new(name, subsection, OwnShared::clone(&self.meta))?);
        let nl = self.detect_newline_style_smallvec();
        let mut section = self.section_mut_from_id(id, nl).expect("each id yields a section");
        section.push_newline();
        Ok(section)
    }

    /// Removes the section with `name` and `subsection_name`, returning true if there was a matching section.
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
    /// assert!(git_config.remove_section("hello", Some("world".into())));
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
    /// assert!(git_config.remove_section("hello", Some("world".into())));
    /// assert_eq!(git_config.to_string(), "[hello \"world\"]\n    some-value = 4\n");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn remove_section<'a>(&mut self, name: impl AsRef<str>, subsection_name: impl Into<Option<&'a BStr>>) -> bool {
        let id = self
            .section_ids_by_name_and_subname(name.as_ref(), subsection_name.into())
            .ok()
            .and_then(|mut ids| ids.next_back());
        let Some(id) = id else {
            return false;
        };
        self.remove_section_by_id(id)
    }

    /// Remove the section identified by `id` if it exists and return true, or return false if no such section was present.
    ///
    /// Note that section ids are unambiguous even in the face of removals and additions of sections.
    pub fn remove_section_by_id(&mut self, id: SectionId) -> bool {
        self.section_order
            .iter()
            .position(|v| *v == id)
            .and_then(|pos| self.section_order.remove(pos));
        let Some(section) = self.sections.remove(&id) else {
            return false;
        };
        let lookup_name = section::Name(section.header.name.0.to_bstring_in(&self.event_backing).into());
        let lut = self
            .section_lookup_tree
            .get_mut(&lookup_name)
            .expect("lookup cache still has name to be deleted");
        // NOTE: this leaves empty lists in the data structure which our code now has to deal with.
        for entry in lut {
            match section
                .header
                .subsection_name
                .as_ref()
                .map(|v| v.to_bstring_in(&self.event_backing))
            {
                Some(subsection_name) => {
                    if let SectionBodyIdsLut::NonTerminal(map) = entry {
                        if let Some(ids) = map.get_mut(subsection_name.as_bstr()) {
                            ids.remove(ids.iter().position(|v| *v == id).expect("present"));
                            break;
                        }
                    }
                }
                None => {
                    if let SectionBodyIdsLut::Terminal(ids) = entry {
                        ids.remove(ids.iter().position(|v| *v == id).expect("present"));
                        break;
                    }
                }
            }
        }
        true
    }

    /// Removes the section with `name` and `subsection_name` that passed `filter`, returning true if at least one section matched the `filter`.
    /// If multiple sections have the same name, then the last one is returned. Note that
    /// later sections with the same name have precedent over earlier ones.
    pub fn remove_section_filter<'a>(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl Into<Option<&'a BStr>>,
        filter: impl FnMut(&Metadata) -> bool,
    ) -> bool {
        self.remove_section_filter_inner(name.as_ref(), subsection_name.into(), filter)
    }

    fn remove_section_filter_inner(
        &mut self,
        name: &str,
        subsection_name: Option<&BStr>,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> bool {
        let id = self
            .section_ids_by_name_and_subname(name, subsection_name)
            .ok()
            .into_iter()
            .flatten()
            .rev()
            .find(|id| self.sections.get(id).is_some_and(|section| filter(&section.meta)));
        let Some(id) = id else {
            return false;
        };
        self.remove_section_by_id(id)
    }

    /// Adds the provided `section` to the config, returning a mutable reference to it for immediate editing.
    /// Note that its meta-data will remain as is.
    pub fn push_section(&mut self, section: file::Section<'_>) -> SectionMut<'_> {
        let section = file::SectionData::from_view(section, &mut self.event_backing);
        let id = self.push_section_internal(section);
        let nl = self.detect_newline_style_smallvec();
        self.section_mut_from_id(id, nl).expect("each id yields a section")
    }

    /// Renames the section with `name` and `subsection_name`, modifying the last matching section
    /// to use `new_name` and `new_subsection_name`.
    pub fn rename_section<'a>(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl Into<Option<&'a BStr>>,
        new_name: impl AsRef<str>,
        new_subsection_name: impl Into<Option<bstr::BString>>,
    ) -> Result<(), rename_section::Error> {
        let id = self
            .section_ids_by_name_and_subname(name.as_ref(), subsection_name.into())?
            .next_back()
            .expect("list of sections were empty, which violates invariant");
        let section = self.sections.get_mut(&id).expect("known section-id");
        let mut header = section::Header::new(new_name, new_subsection_name)?;
        header.intern(&mut self.event_backing);
        section.header = header;
        Ok(())
    }

    /// Renames the section with `name` and `subsection_name`, modifying the last matching section
    /// that also passes `filter` to use `new_name` and `new_subsection_name`.
    ///
    /// Note that the otherwise unused [`lookup::existing::Error::KeyMissing`] variant is used to indicate
    /// that the `filter` rejected all candidates, leading to no section being renamed after all.
    pub fn rename_section_filter<'a>(
        &mut self,
        name: impl AsRef<str>,
        subsection_name: impl Into<Option<&'a BStr>>,
        new_name: impl AsRef<str>,
        new_subsection_name: impl Into<Option<bstr::BString>>,
        mut filter: impl FnMut(&Metadata) -> bool,
    ) -> Result<(), rename_section::Error> {
        let id = self
            .section_ids_by_name_and_subname(name.as_ref(), subsection_name.into())?
            .rev()
            .find(|id| filter(&self.sections.get(id).expect("each id has a section").meta))
            .ok_or(rename_section::Error::Lookup(lookup::existing::Error::KeyMissing))?;
        let section = self.sections.get_mut(&id).expect("known section-id");
        let mut header = section::Header::new(new_name, new_subsection_name)?;
        header.intern(&mut self.event_backing);
        section.header = header;
        Ok(())
    }

    /// Append another File to the end of ourselves, without losing any information.
    pub fn append(&mut self, other: Self) -> &mut Self {
        self.append_or_insert(other, None)
    }

    /// Append another File to the end of ourselves, without losing any information.
    pub(crate) fn append_or_insert(&mut self, mut other: Self, mut insert_after: Option<SectionId>) -> &mut Self {
        let nl = self.detect_newline_style_smallvec();
        let backing_offset = self.event_backing.len();
        other.rebase_events(backing_offset);
        self.event_backing.extend_from_slice(&other.event_backing);

        fn extend_and_assure_newline(
            lhs: &mut FrontMatterEvents,
            rhs: FrontMatterEvents,
            backing: &mut Vec<u8>,
            nl: &impl AsRef<[u8]>,
        ) {
            if !ends_with_newline(lhs.as_ref(), backing, nl, true)
                && !rhs
                    .first()
                    .is_none_or(|e| e.to_bstr_lossy_in(backing).starts_with(nl.as_ref()))
            {
                lhs.push(Event::Newline(Span::append(backing, nl.as_ref())));
            }
            lhs.extend(rhs);
        }
        #[allow(clippy::unnecessary_lazy_evaluations)]
        let our_last_section_before_append =
            insert_after.or_else(|| (self.section_id_counter != 0).then(|| SectionId(self.section_id_counter - 1)));

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
            return self;
        }

        match our_last_section_before_append {
            Some(last_id) => extend_and_assure_newline(
                self.frontmatter_post_section.entry(last_id).or_default(),
                other.frontmatter_events,
                &mut self.event_backing,
                &nl,
            ),
            None => extend_and_assure_newline(
                &mut self.frontmatter_events,
                other.frontmatter_events,
                &mut self.event_backing,
                &nl,
            ),
        }
        self
    }

    fn rebase_events(&mut self, offset: usize) {
        for event in &mut self.frontmatter_events {
            event.rebase(offset);
        }
        for events in self.frontmatter_post_section.values_mut() {
            for event in events {
                event.rebase(offset);
            }
        }
        for section in self.sections.values_mut() {
            section.header.rebase(offset);
            for event in &mut section.body.0 {
                event.rebase(offset);
            }
        }
    }

    pub(crate) fn section_mut_from_id(
        &mut self,
        id: SectionId,
        newline: smallvec::SmallVec<[u8; 2]>,
    ) -> Option<SectionMut<'_>> {
        let section = self.sections.get_mut(&id)?;
        Some(section.to_mut(&mut self.event_backing, newline))
    }
}
