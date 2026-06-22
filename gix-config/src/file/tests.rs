use std::collections::HashMap;

use crate::{
    file::{self, SectionId},
    parse::section,
};

mod try_from {
    use std::collections::HashMap;

    use super::{bodies, headers};
    use crate::{
        File,
        file::{self, SectionBodyIdsLut, SectionId},
        parse::{
            Event, section,
            tests::util::{name_event, newline_event, section_header, value_event},
        },
    };

    #[test]
    fn empty() {
        let config = File::try_from("").unwrap();
        assert_eq!(config.section_id_counter, 0);
        assert!(config.section_lookup_tree.is_empty());
        assert!(config.sections.is_empty());
        assert!(config.section_order.is_empty());
    }

    #[test]
    fn single_section() {
        let mut config = File::try_from("[core]\na=b\nc=d").unwrap();
        let expected_separators = {
            let mut map = HashMap::new();
            map.insert(SectionId(0), section_header("core", None));
            map
        };
        assert_eq!(headers(&config), expected_separators);
        assert_eq!(config.section_id_counter, 1);
        let expected_lookup_tree = {
            let mut tree = HashMap::new();
            tree.insert(
                section::Name("core".into()),
                vec![SectionBodyIdsLut::Terminal(vec![SectionId(0)])],
            );
            tree
        };
        assert_eq!(config.section_lookup_tree, expected_lookup_tree);
        let expected_sections = {
            let mut sections = HashMap::new();
            sections.insert(
                SectionId(0),
                file::section::Body(vec![
                    newline_event(),
                    name_event("a"),
                    Event::KeyValueSeparator,
                    value_event("b"),
                    newline_event(),
                    name_event("c"),
                    Event::KeyValueSeparator,
                    value_event("d"),
                ]),
            );
            sections
        };
        assert_eq!(bodies(&config), expected_sections);
        assert_eq!(config.section_order.make_contiguous(), &[SectionId(0)]);
    }

    #[test]
    fn single_subsection() {
        let mut config = File::try_from("[core.sub]\na=b\nc=d").unwrap();
        let expected_separators = {
            let mut map = HashMap::new();
            map.insert(SectionId(0), section_header("core", (".", "sub")));
            map
        };
        assert_eq!(headers(&config), expected_separators);
        assert_eq!(config.section_id_counter, 1);
        let expected_lookup_tree = {
            let mut tree = HashMap::new();
            let mut inner_tree = HashMap::new();
            inner_tree.insert("sub".into(), vec![SectionId(0)]);
            tree.insert(
                section::Name("core".into()),
                vec![SectionBodyIdsLut::NonTerminal(inner_tree)],
            );
            tree
        };
        assert_eq!(config.section_lookup_tree, expected_lookup_tree);
        let expected_sections = {
            let mut sections = HashMap::new();
            sections.insert(
                SectionId(0),
                file::section::Body(vec![
                    newline_event(),
                    name_event("a"),
                    Event::KeyValueSeparator,
                    value_event("b"),
                    newline_event(),
                    name_event("c"),
                    Event::KeyValueSeparator,
                    value_event("d"),
                ]),
            );
            sections
        };
        assert_eq!(bodies(&config), expected_sections);
        assert_eq!(config.section_order.make_contiguous(), &[SectionId(0)]);
    }

    #[test]
    fn multiple_sections() {
        let mut config = File::try_from("[core]\na=b\nc=d\n[other]e=f").unwrap();
        let expected_separators = {
            let mut map = HashMap::new();
            map.insert(SectionId(0), section_header("core", None));
            map.insert(SectionId(1), section_header("other", None));
            map
        };
        assert_eq!(headers(&config), expected_separators);
        assert_eq!(config.section_id_counter, 2);
        let expected_lookup_tree = {
            let mut tree = HashMap::new();
            tree.insert(
                section::Name("core".into()),
                vec![SectionBodyIdsLut::Terminal(vec![SectionId(0)])],
            );
            tree.insert(
                section::Name("other".into()),
                vec![SectionBodyIdsLut::Terminal(vec![SectionId(1)])],
            );
            tree
        };
        assert_eq!(config.section_lookup_tree, expected_lookup_tree);
        let expected_sections = {
            let mut sections = HashMap::new();
            sections.insert(
                SectionId(0),
                file::section::Body(vec![
                    newline_event(),
                    name_event("a"),
                    Event::KeyValueSeparator,
                    value_event("b"),
                    newline_event(),
                    name_event("c"),
                    Event::KeyValueSeparator,
                    value_event("d"),
                    newline_event(),
                ]),
            );
            sections.insert(
                SectionId(1),
                file::section::Body(vec![name_event("e"), Event::KeyValueSeparator, value_event("f")]),
            );
            sections
        };
        assert_eq!(bodies(&config), expected_sections);
        assert_eq!(config.section_order.make_contiguous(), &[SectionId(0), SectionId(1)]);
    }

    #[test]
    fn multiple_duplicate_sections() {
        let mut config = File::try_from("[core]\na=b\nc=d\n[core]e=f").unwrap();
        let expected_separators = {
            let mut map = HashMap::new();
            map.insert(SectionId(0), section_header("core", None));
            map.insert(SectionId(1), section_header("core", None));
            map
        };
        assert_eq!(headers(&config), expected_separators);
        assert_eq!(config.section_id_counter, 2);
        let expected_lookup_tree = {
            let mut tree = HashMap::new();
            tree.insert(
                section::Name("core".into()),
                vec![SectionBodyIdsLut::Terminal(vec![SectionId(0), SectionId(1)])],
            );
            tree
        };
        assert_eq!(config.section_lookup_tree, expected_lookup_tree);
        let expected_sections = {
            let mut sections = HashMap::new();
            sections.insert(
                SectionId(0),
                file::section::Body(vec![
                    newline_event(),
                    name_event("a"),
                    Event::KeyValueSeparator,
                    value_event("b"),
                    newline_event(),
                    name_event("c"),
                    Event::KeyValueSeparator,
                    value_event("d"),
                    newline_event(),
                ]),
            );
            sections.insert(
                SectionId(1),
                file::section::Body(vec![name_event("e"), Event::KeyValueSeparator, value_event("f")]),
            );
            sections
        };
        assert_eq!(bodies(&config), expected_sections);
        assert_eq!(config.section_order.make_contiguous(), &[SectionId(0), SectionId(1)]);
    }
}

fn headers(config: &crate::File) -> HashMap<SectionId, section::Header> {
    config
        .sections
        .iter()
        .map(|(k, v)| (*k, v.header.to_owned_in(&config.event_backing)))
        .collect()
}

fn bodies(config: &crate::File) -> HashMap<SectionId, file::section::Body> {
    config
        .sections
        .iter()
        .map(|(k, v)| (*k, v.body.to_owned_in(&config.event_backing)))
        .collect()
}
