use std::{cmp::Ordering, collections::HashMap};

use bstr::BStr;

use crate::{
    File,
    file::{self, SectionBodyIdsLut, SectionId},
    lookup,
    parse::section,
};

/// Private helper functions
impl<'event> File<'event> {
    /// Adds a new section to the config file, returning the section id of the newly added section.
    pub(crate) fn push_section_internal(&mut self, mut section: file::Section<'event>) -> SectionId {
        let new_section_id = SectionId(self.section_id_counter);
        section.id = new_section_id;
        self.sections.insert(new_section_id, section);
        let header = &self.sections[&new_section_id].header;
        let lookup = self.section_lookup_tree.entry(header.name.clone()).or_default();

        let mut found_node = false;
        if let Some(subsection_name) = header.subsection_name.clone() {
            for node in lookup.iter_mut() {
                if let SectionBodyIdsLut::NonTerminal(subsections) = node {
                    found_node = true;
                    subsections
                        .entry(subsection_name.clone())
                        .or_default()
                        .push(new_section_id);
                    break;
                }
            }
            if !found_node {
                let mut map = HashMap::new();
                map.insert(subsection_name, vec![new_section_id]);
                lookup.push(SectionBodyIdsLut::NonTerminal(map));
            }
        } else {
            for node in lookup.iter_mut() {
                if let SectionBodyIdsLut::Terminal(vec) = node {
                    found_node = true;
                    vec.push(new_section_id);
                    break;
                }
            }
            if !found_node {
                lookup.push(SectionBodyIdsLut::Terminal(vec![new_section_id]));
            }
        }
        self.section_order.push_back(new_section_id);
        self.section_id_counter += 1;
        new_section_id
    }

    /// Inserts `section` after the section that comes `before` it, and maintains correct ordering in all of our lookup structures.
    pub(crate) fn insert_section_after(&mut self, mut section: file::Section<'event>, before: SectionId) -> SectionId {
        let lookup_section_order = {
            let section_order = &self.section_order;
            move |section_id| {
                section_order
                    .iter()
                    .enumerate()
                    .find_map(|(idx, id)| (*id == section_id).then_some(idx))
                    .expect("before-section exists")
            }
        };

        let before_order = lookup_section_order(before);
        let new_section_id = SectionId(self.section_id_counter);
        section.id = new_section_id;
        self.sections.insert(new_section_id, section);
        let header = &self.sections[&new_section_id].header;
        let lookup = self.section_lookup_tree.entry(header.name.clone()).or_default();

        let mut found_node = false;
        if let Some(subsection_name) = header.subsection_name.clone() {
            for node in lookup.iter_mut() {
                if let SectionBodyIdsLut::NonTerminal(subsections) = node {
                    found_node = true;
                    let sections_with_name_and_subsection_name =
                        subsections.entry(subsection_name.clone()).or_default();
                    let insert_pos = find_insert_pos_by_order(
                        sections_with_name_and_subsection_name,
                        before_order,
                        lookup_section_order,
                    );
                    sections_with_name_and_subsection_name.insert(insert_pos, new_section_id);
                    break;
                }
            }
            if !found_node {
                let mut map = HashMap::new();
                map.insert(subsection_name, vec![new_section_id]);
                lookup.push(SectionBodyIdsLut::NonTerminal(map));
            }
        } else {
            for node in lookup.iter_mut() {
                if let SectionBodyIdsLut::Terminal(sections_with_name) = node {
                    found_node = true;
                    let insert_pos = find_insert_pos_by_order(sections_with_name, before_order, lookup_section_order);
                    sections_with_name.insert(insert_pos, new_section_id);
                    break;
                }
            }
            if !found_node {
                lookup.push(SectionBodyIdsLut::Terminal(vec![new_section_id]));
            }
        }

        self.section_order.insert(before_order + 1, new_section_id);
        self.section_id_counter += 1;
        new_section_id
    }

    /// Returns the mapping between section and subsection name to section ids.
    pub(crate) fn section_ids_by_name_and_subname<'a>(
        &'a self,
        section_name: &'a str,
        subsection_name: Option<&BStr>,
    ) -> Result<impl ExactSizeIterator<Item = SectionId> + DoubleEndedIterator + 'a, lookup::existing::Error> {
        let section_name = section::Name::from_str_unchecked(section_name);
        let section_ids = self
            .section_lookup_tree
            .get(&section_name)
            .ok_or(lookup::existing::Error::SectionMissing)?;
        let mut maybe_ids = None;
        if let Some(subsection_name) = subsection_name {
            for node in section_ids {
                if let SectionBodyIdsLut::NonTerminal(subsection_lookup) = node {
                    maybe_ids = subsection_lookup.get(subsection_name).map(|v| v.iter().copied());
                    break;
                }
            }
        } else {
            for node in section_ids {
                if let SectionBodyIdsLut::Terminal(subsection_lookup) = node {
                    maybe_ids = Some(subsection_lookup.iter().copied());
                    break;
                }
            }
        }
        maybe_ids.ok_or(lookup::existing::Error::SubSectionMissing)
    }

    pub(crate) fn section_ids_by_name<'a>(
        &'a self,
        section_name: &'a str,
    ) -> Result<impl Iterator<Item = SectionId> + 'a, lookup::existing::Error> {
        let section_name = section::Name::from_str_unchecked(section_name);
        match self.section_lookup_tree.get(&section_name) {
            Some(lookup) => {
                let mut lut = Vec::with_capacity(self.section_order.len());
                for node in lookup {
                    match node {
                        SectionBodyIdsLut::Terminal(v) => lut.extend(v.iter().copied()),
                        SectionBodyIdsLut::NonTerminal(v) => lut.extend(v.values().flatten().copied()),
                    }
                }

                Ok(self.section_order.iter().filter(move |a| lut.contains(a)).copied())
            }
            None => Err(lookup::existing::Error::SectionMissing),
        }
    }
}

fn find_insert_pos_by_order(
    sections_with_name: &[SectionId],
    before_order: usize,
    lookup_section_order: impl Fn(SectionId) -> usize,
) -> usize {
    let mut insert_pos = sections_with_name.len(); // push back by default
    for (idx, candidate_id) in sections_with_name.iter().enumerate() {
        let candidate_order = lookup_section_order(*candidate_id);
        match candidate_order.cmp(&before_order) {
            Ordering::Less => {}
            Ordering::Equal => {
                insert_pos = idx + 1; // insert right after this one
                break;
            }
            Ordering::Greater => {
                insert_pos = idx; // insert before this one
                break;
            }
        }
    }
    insert_pos
}
