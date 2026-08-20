use std::collections::HashMap;

use bstr::{BStr, BString, ByteSlice};

use crate::{
    File,
    file::{self, SectionId, SectionLookup},
    lookup,
    parse::section,
};

/// Private helper functions
impl File {
    /// Adds a new section to the config file, returning the section id of the newly added section.
    pub(crate) fn push_section_internal(&mut self, mut section: file::SectionData) -> SectionId {
        let new_section_id = self.allocate_section_id();
        section.id = new_section_id;

        let lookup_name = section::Name(section.header.name.to_bstring_in(&self.backing));
        let subsection_name = section
            .header
            .subsection_name
            .as_ref()
            .map(|name| name.value_in(&self.backing).to_owned());
        self.sections.insert(new_section_id, section);
        let lookup = self.section_lookup_tree.entry(lookup_name).or_default();
        match subsection_name {
            Some(name) => lookup.by_subsection.entry(name).or_default().push(new_section_id),
            None => lookup.without_subsection.push(new_section_id),
        }
        self.section_order.push(new_section_id);
        new_section_id
    }

    /// Inserts `section` after the section that comes `before` it, and maintains correct ordering in all of our lookup structures.
    pub(crate) fn insert_section_after(&mut self, mut section: file::SectionData, before: SectionId) -> SectionId {
        let before_order = self.section_order_position(before);
        let new_section_id = self.allocate_section_id();
        section.id = new_section_id;
        let lookup_name = section::Name(section.header.name.to_bstring_in(&self.backing));
        let subsection_name = section
            .header
            .subsection_name
            .as_ref()
            .map(|name| name.value_in(&self.backing).to_owned());
        self.sections.insert(new_section_id, section);
        self.section_order.insert(before_order + 1, new_section_id);

        let order = self.section_order_position(new_section_id);
        let earlier_sections = &self.section_order[..order];
        let lookup = self.section_lookup_tree.entry(lookup_name).or_default();
        let ids = match subsection_name {
            Some(name) => lookup.by_subsection.entry(name).or_default(),
            None => &mut lookup.without_subsection,
        };
        insert_id_in_order(ids, earlier_sections, new_section_id);
        new_section_id
    }

    /// Returns the mapping between section and subsection name to section ids.
    pub(crate) fn section_ids_by_name_and_subname<'a>(
        &'a self,
        section_name: &'a str,
        subsection_name: Option<&BStr>,
    ) -> Result<impl ExactSizeIterator<Item = SectionId> + DoubleEndedIterator + 'a, lookup::existing::Error> {
        let section_name = section::Name::from_str_unchecked(section_name);
        let lookup = self
            .section_lookup_tree
            .get(&section_name)
            .ok_or_else(lookup::existing::section_missing)?;
        match subsection_name {
            Some(name) => lookup.by_subsection.get(name),
            None => (!lookup.without_subsection.is_empty()).then_some(&lookup.without_subsection),
        }
        .ok_or_else(lookup::existing::subsection_missing)
        .map(|ids| ids.iter().copied())
    }

    pub(crate) fn section_ids_by_name<'a>(
        &'a self,
        section_name: &str,
    ) -> Result<impl Iterator<Item = SectionId> + 'a + use<'a>, lookup::existing::Error> {
        let lookup_name = section::Name::from_str_unchecked(section_name);
        let lookup = self
            .section_lookup_tree
            .get(&lookup_name)
            .ok_or_else(lookup::existing::section_missing)?;
        let mut ids = Vec::with_capacity(self.section_order.len());
        ids.extend_from_slice(&lookup.without_subsection);
        ids.extend(lookup.by_subsection.values().flatten().copied());
        Ok(self.section_order.iter().filter(move |id| ids.contains(id)).copied())
    }

    fn allocate_section_id(&mut self) -> SectionId {
        let id = SectionId(self.next_section_id);
        self.next_section_id += 1;
        id
    }

    pub(crate) fn section_order_position(&self, id: SectionId) -> usize {
        self.section_order
            .iter()
            .position(|candidate| *candidate == id)
            .expect("each section-id is present in section order")
    }
}

pub(crate) fn remove_section_id_from_lookup(
    lookup_tree: &mut HashMap<section::Name, SectionLookup>,
    section_name: &section::Name,
    subsection_name: Option<&BStr>,
    section_id: SectionId,
) {
    let lookup = lookup_tree
        .get_mut(section_name)
        .expect("lookup cache contains the section to be changed");
    match subsection_name {
        Some(name) => {
            let ids = lookup
                .by_subsection
                .get_mut(name)
                .expect("lookup cache contains the subsection to be changed");
            remove_id(ids, section_id);
            if ids.is_empty() {
                lookup.by_subsection.remove(name);
            }
        }
        None => remove_id(&mut lookup.without_subsection, section_id),
    }
    if lookup.without_subsection.is_empty() && lookup.by_subsection.is_empty() {
        lookup_tree.remove(section_name);
    }
}

pub(crate) fn set_section_header(
    section_data: &mut file::SectionData,
    backing: &[u8],
    lookup_tree: &mut HashMap<section::Name, SectionLookup>,
    section_order: &[SectionId],
    header: section::HeaderData,
) {
    let old_name = section::Name(section_data.header.name.to_bstring_in(backing));
    let old_subsection_name = section_data
        .header
        .subsection_name
        .as_ref()
        .map(|name| name.value_in(backing).to_owned());
    let new_name = section::Name(header.name.to_bstring_in(backing));
    let new_subsection_name = header
        .subsection_name
        .as_ref()
        .map(|name| name.value_in(backing).to_owned());

    if old_name != new_name || old_subsection_name != new_subsection_name {
        remove_section_id_from_lookup(
            lookup_tree,
            &old_name,
            old_subsection_name.as_ref().map(|name| name.as_bstr()),
            section_data.id,
        );
        insert_section_id_into_lookup(
            lookup_tree,
            section_order,
            new_name,
            new_subsection_name,
            section_data.id,
        );
    }
    section_data.header = header;
}

pub(crate) fn insert_section_id_into_lookup(
    lookup_tree: &mut HashMap<section::Name, SectionLookup>,
    section_order: &[SectionId],
    section_name: section::Name,
    subsection_name: Option<BString>,
    section_id: SectionId,
) {
    let order = section_order
        .iter()
        .position(|id| *id == section_id)
        .expect("section-id is present in section order");
    let lookup = lookup_tree.entry(section_name).or_default();
    let earlier_sections = &section_order[..order];
    let ids = match subsection_name {
        Some(name) => lookup.by_subsection.entry(name).or_default(),
        None => &mut lookup.without_subsection,
    };
    insert_id_in_order(ids, earlier_sections, section_id);
}

fn insert_id_in_order(ids: &mut Vec<SectionId>, earlier_sections: &[SectionId], section_id: SectionId) {
    let position = ids.partition_point(|id| earlier_sections.contains(id));
    ids.insert(position, section_id);
}

fn remove_id(ids: &mut Vec<SectionId>, section_id: SectionId) {
    let position = ids
        .iter()
        .position(|candidate| *candidate == section_id)
        .expect("lookup cache contains the section-id to be changed");
    ids.remove(position);
}
