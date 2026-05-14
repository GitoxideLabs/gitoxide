mod section {
    use std::borrow::Cow;

    use bstr::BStr;

    use crate::parse::{Comment, Event, Events, Section, section, section::Header};

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn size_of_events() {
        assert_eq!(
            std::mem::size_of::<Section<'_>>(),
            96,
            "this value should only ever decrease"
        );
        assert_eq!(std::mem::size_of::<Events<'_>>(), 616);
        assert_eq!(std::mem::size_of::<Event<'_>>(), 72);
        assert_eq!(std::mem::size_of::<Header<'_>>(), 72);
        assert_eq!(std::mem::size_of::<Comment<'_>>(), 32);
        assert_eq!(std::mem::size_of::<Option<Cow<'_, BStr>>>(), 24);
        assert_eq!(std::mem::size_of::<section::Name<'_>>(), 24);
        assert_eq!(std::mem::size_of::<section::ValueName<'_>>(), 24);
    }

    mod header {
        mod unvalidated {
            use crate::parse::section::unvalidated::Key;

            #[test]
            fn section_name_only() {
                assert_eq!(
                    Key::parse("core").unwrap(),
                    Key {
                        section_name: "core",
                        subsection_name: None
                    }
                );
            }

            #[test]
            fn section_name_and_subsection() {
                assert_eq!(
                    Key::parse("core.bare").unwrap(),
                    Key {
                        section_name: "core",
                        subsection_name: Some("bare".into())
                    }
                );
            }

            #[test]
            fn section_name_and_subsection_with_separators() {
                assert_eq!(
                    Key::parse("remote.https:///home/user.git").unwrap(),
                    Key {
                        section_name: "remote",
                        subsection_name: Some("https:///home/user.git".into())
                    }
                );
            }
        }

        mod write_to {
            use std::borrow::Cow;

            use crate::parse::section;

            fn header(name: &str, subsection: impl Into<Option<(&'static str, &'static str)>>) -> section::Header<'_> {
                let name = section::Name(Cow::Borrowed(name.into()));
                if let Some((separator, subsection_name)) = subsection.into() {
                    section::Header {
                        name,
                        separator: Some(Cow::Borrowed(separator.into())),
                        subsection_name: Some(Cow::Borrowed(subsection_name.into())),
                    }
                } else {
                    section::Header {
                        name,
                        separator: None,
                        subsection_name: None,
                    }
                }
            }

            #[test]
            fn legacy_subsection_format_does_not_use_escapes() {
                let invalid = header("invalid", Some((".", r#"\ ""#)));
                assert_eq!(
                    invalid.to_bstring(),
                    r#"[invalid.\ "]"#,
                    "no escaping happens for legacy subsections"
                );
                assert!(invalid.is_legacy());
            }

            #[test]
            fn subsections_escape_two_characters_only() {
                let invalid = header("invalid", Some((" ", "\\ \"\npost newline")));
                assert_eq!(
                    invalid.to_bstring(),
                    "[invalid \"\\\\ \\\"\npost newline\"]",
                    "newlines are actually invalid in subsection, but they are possible due to unvalidated instance creation"
                );
                assert!(!invalid.is_legacy());
            }

            #[test]
            fn empty_section_name_with_quoted_subsection() {
                let header = header("", Some((" ", "core")));
                let mut out = Vec::new();
                header.write_to(&mut out).unwrap();
                assert_eq!(
                    out, br#"[ "core"]"#,
                    "Git accepts this as an empty section name with `core` as subsection, and we keep it"
                );
                assert!(!header.is_legacy());
            }

            #[test]
            fn nul_byte_in_quoted_subsection() {
                let header = header("hello", Some((" ", "hello\0")));
                let mut out = Vec::new();
                header.write_to(&mut out).unwrap();
                assert_eq!(
                    out, b"[hello \"hello\0\"]",
                    "Git accepts NUL bytes in quoted subsection names, and we preserve them"
                );
                assert!(!header.is_legacy());
            }
        }
    }
}

mod event {
    mod write_to {
        use crate::parse::Events;

        fn write_events(input: &str) -> Vec<u8> {
            let events = Events::from_str(input).unwrap().into_vec();
            let mut out = Vec::new();
            for event in &events {
                event.write_to(&mut out).unwrap();
            }
            out
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

    use std::borrow::Cow;

    use crate::parse::{Comment, Event, section};

    pub fn section_header(
        name: &str,
        subsection: impl Into<Option<(&'static str, &'static str)>>,
    ) -> section::Header<'_> {
        let name = section::Name::try_from(name).unwrap();
        if let Some((separator, subsection_name)) = subsection.into() {
            section::Header {
                name,
                separator: Some(Cow::Borrowed(separator.into())),
                subsection_name: Some(Cow::Borrowed(subsection_name.into())),
            }
        } else {
            section::Header {
                name,
                separator: None,
                subsection_name: None,
            }
        }
    }

    pub(crate) fn name_event(name: &'static str) -> Event<'static> {
        Event::SectionValueName(section::ValueName(Cow::Borrowed(name.into())))
    }

    pub(crate) fn value_event(value: &'static str) -> Event<'static> {
        Event::Value(Cow::Borrowed(value.into()))
    }

    pub(crate) fn value_not_done_event(value: &'static str) -> Event<'static> {
        Event::ValueNotDone(Cow::Borrowed(value.into()))
    }

    pub(crate) fn value_done_event(value: &'static str) -> Event<'static> {
        Event::ValueDone(Cow::Borrowed(value.into()))
    }

    pub(crate) fn newline_event() -> Event<'static> {
        newline_custom_event("\n")
    }

    pub(crate) fn newline_custom_event(value: &'static str) -> Event<'static> {
        Event::Newline(Cow::Borrowed(value.into()))
    }

    pub(crate) fn whitespace_event(value: &'static str) -> Event<'static> {
        Event::Whitespace(Cow::Borrowed(value.into()))
    }

    pub(crate) fn comment_event(tag: char, msg: &'static str) -> Event<'static> {
        Event::Comment(comment(tag, msg))
    }

    pub(crate) fn comment(comment_tag: char, comment: &'static str) -> Comment<'static> {
        Comment {
            tag: comment_tag as u8,
            text: Cow::Borrowed(comment.into()),
        }
    }

    pub(crate) const fn fully_consumed<T>(t: T) -> (&'static [u8], T) {
        (&[], t)
    }
}
