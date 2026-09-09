mod section {
    use crate::parse::{Comment, Event, Events, SectionRef, section};

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn size_of_events() {
        assert_eq!(
            std::mem::size_of::<SectionRef>(),
            40,
            "this value should only ever decrease"
        );
        assert_eq!(std::mem::size_of::<Events>(), 512);
        assert_eq!(std::mem::size_of::<crate::parse::Span>(), 8);
        assert_eq!(std::mem::size_of::<Event>(), 56);
        assert_eq!(std::mem::size_of::<Comment>(), 12);
        assert_eq!(std::mem::size_of::<section::Name>(), 24);
        assert_eq!(std::mem::size_of::<section::ValueName>(), 24);
    }

    mod header {
        mod unvalidated {
            use crate::parse::section::unvalidated::KeyRef;

            #[test]
            fn section_name_only() {
                assert_eq!(
                    KeyRef::parse("core").unwrap(),
                    KeyRef {
                        section_name: "core",
                        subsection_name: None
                    }
                );
            }

            #[test]
            fn section_name_and_subsection() {
                assert_eq!(
                    KeyRef::parse("core.bare").unwrap(),
                    KeyRef {
                        section_name: "core",
                        subsection_name: Some("bare".into())
                    }
                );
            }

            #[test]
            fn section_name_and_subsection_with_separators() {
                assert_eq!(
                    KeyRef::parse("remote.https:///home/user.git").unwrap(),
                    KeyRef {
                        section_name: "remote",
                        subsection_name: Some("https:///home/user.git".into())
                    }
                );
            }
        }
    }
}

mod span {
    use crate::parse::Span;

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn reports_range_and_rebase_overflow() {
        assert!(Span::range(u32::MAX as usize + 1, 0).is_err());
        assert!(Span::range(0, u32::MAX as usize + 1).is_err());

        let mut span = Span::range(u32::MAX as usize, 0).expect("the maximum offset is representable");
        assert_eq!(
            span.rebase(1).expect_err("rebasing beyond u32 must fail").message,
            format!(
                "configuration data exceeds the supported span size of {} bytes",
                u32::MAX
            )
        );
    }
}

mod event {
    mod write_to {
        use crate::parse::{EventRef, Events};

        fn header(
            name: &'static str,
            subsection: impl Into<Option<(&'static str, &'static str)>>,
        ) -> EventRef<'static> {
            let (separator, subsection_name) =
                subsection.into().map_or((None, None), |(separator, subsection_name)| {
                    (Some(separator.into()), Some(subsection_name.into()))
                });
            EventRef::SectionHeader {
                name: name.into(),
                separator,
                subsection_name,
            }
        }

        fn write_event(event: EventRef<'_>) -> Vec<u8> {
            let mut out = Vec::new();
            event.write_to(&mut out).expect("writing to memory succeeds");
            out
        }

        fn write_events(input: &str) -> Vec<u8> {
            let events = Events::from_str(input).unwrap();
            let mut out = Vec::new();
            for event in events.iter() {
                event.write_to(&mut out).unwrap();
            }
            out
        }

        #[test]
        fn legacy_subsection_format_does_not_use_escapes() {
            assert_eq!(
                write_event(header("invalid", Some((".", r#"\ ""#)))),
                br#"[invalid.\ "]"#,
                "no escaping happens for legacy subsections"
            );
        }

        #[test]
        fn subsections_escape_two_characters_only() {
            assert_eq!(
                write_event(header("invalid", Some((" ", "\\ \"\npost newline")))),
                b"[invalid \"\\\\ \\\"\npost newline\"]",
                "newlines are invalid in validated subsections, but EventRef can represent them"
            );
        }

        #[test]
        fn empty_section_name_with_quoted_subsection() {
            assert_eq!(
                write_event(header("", Some((" ", "core")))),
                br#"[ "core"]"#,
                "Git accepts an empty section name with `core` as subsection"
            );
        }

        #[test]
        fn nul_byte_in_quoted_subsection() {
            assert_eq!(
                write_event(header("hello", Some((" ", "hello\0")))),
                b"[hello \"hello\0\"]",
                "Git accepts NUL bytes in quoted subsection names"
            );
        }

        #[test]
        fn key_value_before_first_section() {
            let input = "a = b\n";
            assert_eq!(
                write_events(input),
                input.as_bytes(),
                "Git accepts key/value pairs before the first section, and we preserve them"
            );
        }

        #[test]
        fn value_with_trailing_backslash_at_eof() {
            let input = "[core]\na=hello\\";
            assert_eq!(
                write_events(input),
                input.as_bytes(),
                "Git accepts EOF as a line continuation terminator, and we preserve the original trailing backslash"
            );
        }
    }
}

pub(crate) mod util {
    //! This module is only included for tests, and contains common unit test helper
    //! functions.
    //!
    //! Parsed events store spans into a separate backing buffer, so their fields cannot be inspected
    //! or compared meaningfully without the backing buffer. The `Owned*` types resolve those spans into
    //! owned byte strings, allowing tests to compare event contents independently of span coordinates
    //! and to retain expected or actual values after a temporary backing buffer is dropped.

    use bstr::BString;

    use crate::parse::{Event, section};

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct OwnedHeader {
        pub(crate) name: BString,
        pub(crate) separator: Option<BString>,
        pub(crate) subsection_name: Option<BString>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) enum OwnedEvent {
        Comment(OwnedComment),
        SectionHeader(OwnedHeader),
        SectionValueName(BString),
        Value(BString),
        Newline(BString),
        ValueNotDone(BString),
        ValueDone(BString),
        Whitespace(BString),
        KeyValueSeparator,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct OwnedComment {
        pub(crate) tag: u8,
        pub(crate) text: BString,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct OwnedSection {
        pub(crate) header: OwnedHeader,
        pub(crate) events: Vec<OwnedEvent>,
    }

    pub(crate) fn own_event(event: &Event, backing: &[u8]) -> OwnedEvent {
        match event {
            Event::Comment(comment) => OwnedEvent::Comment(OwnedComment {
                tag: comment.tag,
                text: comment.text.to_bstring_in(backing),
            }),
            Event::SectionHeader(header) => OwnedEvent::SectionHeader(own_header(header, backing)),
            Event::SectionValueName(name) => OwnedEvent::SectionValueName(name.to_bstring_in(backing)),
            Event::Value(value) => OwnedEvent::Value(value.to_bstring_in(backing)),
            Event::Newline(value) => OwnedEvent::Newline(value.to_bstring_in(backing)),
            Event::ValueNotDone(value) => OwnedEvent::ValueNotDone(value.to_bstring_in(backing)),
            Event::ValueDone(value) => OwnedEvent::ValueDone(value.to_bstring_in(backing)),
            Event::Whitespace(value) => OwnedEvent::Whitespace(value.to_bstring_in(backing)),
            Event::KeyValueSeparator => OwnedEvent::KeyValueSeparator,
        }
    }

    pub(crate) fn own_header(header: &section::HeaderData, backing: &[u8]) -> OwnedHeader {
        OwnedHeader {
            name: header.name.to_bstring_in(backing),
            separator: header.separator.map(|separator| separator.to_bstring_in(backing)),
            subsection_name: header
                .subsection_name
                .as_ref()
                .map(|subsection_name| subsection_name.value_in(backing).to_owned()),
        }
    }

    pub fn section_header(name: &str, subsection: impl Into<Option<(&'static str, &'static str)>>) -> OwnedHeader {
        if let Some((separator, subsection_name)) = subsection.into() {
            OwnedHeader {
                name: name.into(),
                separator: Some(separator.into()),
                subsection_name: Some(subsection_name.into()),
            }
        } else {
            OwnedHeader {
                name: name.into(),
                separator: None,
                subsection_name: None,
            }
        }
    }

    pub(crate) fn name_event(name: &'static str) -> OwnedEvent {
        OwnedEvent::SectionValueName(name.into())
    }

    pub(crate) fn value_event(value: &'static str) -> OwnedEvent {
        OwnedEvent::Value(value.into())
    }

    pub(crate) fn value_not_done_event(value: &'static str) -> OwnedEvent {
        OwnedEvent::ValueNotDone(value.into())
    }

    pub(crate) fn value_done_event(value: &'static str) -> OwnedEvent {
        OwnedEvent::ValueDone(value.into())
    }

    pub(crate) fn newline_event() -> OwnedEvent {
        newline_custom_event("\n")
    }

    pub(crate) fn newline_custom_event(value: &'static str) -> OwnedEvent {
        OwnedEvent::Newline(value.into())
    }

    pub(crate) fn whitespace_event(value: &'static str) -> OwnedEvent {
        OwnedEvent::Whitespace(value.into())
    }

    pub(crate) fn comment_event(tag: char, msg: &'static str) -> OwnedEvent {
        OwnedEvent::Comment(comment(tag, msg))
    }

    pub(crate) fn comment(comment_tag: char, comment: &'static str) -> OwnedComment {
        OwnedComment {
            tag: comment_tag as u8,
            text: comment.into(),
        }
    }

    pub(crate) const fn fully_consumed<T>(t: T) -> (&'static [u8], T) {
        (&[], t)
    }
}
