use std::borrow::Cow;

use gix_config::parse::{Event, section};

pub fn header_event(name: &'static str, subsection: impl Into<Option<&'static str>>) -> Event<'static> {
    Event::SectionHeader(section::Header::new(name, subsection.into().map(|s| Cow::Borrowed(s.into()))).unwrap())
}

mod header {
    use std::borrow::Cow;

    use bstr::BStr;

    fn cow_section(name: &str) -> Option<Cow<'_, BStr>> {
        Some(Cow::Borrowed(name.into()))
    }
    mod write_to {
        use gix_config::parse::section;

        use crate::parse::section::header::cow_section;

        #[test]
        fn subsection_backslashes_and_quotes_are_escaped() -> crate::Result {
            assert_eq!(
                section::Header::new("core", cow_section(r"a\b"))?.to_bstring(),
                r#"[core "a\\b"]"#
            );
            assert_eq!(
                section::Header::new("core", cow_section(r#"a:"b""#))?.to_bstring(),
                r#"[core "a:\"b\""]"#
            );
            Ok(())
        }

        #[test]
        fn everything_is_allowed() -> crate::Result {
            assert_eq!(
                section::Header::new("core", cow_section("a/b \t\t a\\b"))?.to_bstring(),
                "[core \"a/b \t\t a\\\\b\"]"
            );
            Ok(())
        }
    }
    mod new {
        use gix_config::parse::section;

        use crate::parse::section::header::cow_section;

        #[test]
        fn names_must_be_mostly_ascii() {
            for name in ["🤗", "x.y", "x y", "x\ny"] {
                assert_eq!(
                    section::Header::new(name, None),
                    Err(section::header::Error::InvalidName)
                );
            }
        }

        #[test]
        fn subsections_with_newlines_and_null_bytes_are_rejected() {
            assert_eq!(
                section::Header::new("a", cow_section("a\nb")),
                Err(section::header::Error::InvalidSubSection)
            );
            assert_eq!(
                section::Header::new("a", cow_section("a\0b")),
                Err(section::header::Error::InvalidSubSection)
            );
        }
    }
}
mod name {
    use gix_config::parse::section::Name;

    #[test]
    fn alphanum_and_dash_are_valid() {
        assert!(Name::try_from("1a").is_ok());
        assert!(Name::try_from("Hello-World").is_ok());
    }

    #[test]
    fn rejects_invalid_format() {
        assert!(Name::try_from("").is_err());
        assert!(Name::try_from("a.2").is_err());
        assert!(Name::try_from("\"").is_err());
        assert!(Name::try_from("##").is_err());
    }
}

mod key {
    use std::cmp::Ordering;

    use gix_config::parse::section::ValueName;

    fn key(k: &str) -> ValueName<'_> {
        ValueName::try_from(k).unwrap()
    }

    #[test]
    fn rejects_invalid_format() {
        assert!(ValueName::try_from("").is_err());
        assert!(ValueName::try_from("1a").is_err());
        assert!(ValueName::try_from("a.2").is_err());
        assert!(ValueName::try_from("##").is_err());
        assert!(ValueName::try_from("\"").is_err());
    }

    #[test]
    fn case_insensitive_eq() {
        assert_eq!(key("aB-c"), key("Ab-C"));
    }

    #[test]
    fn case_insensitive_ord() {
        assert_eq!(key("a").cmp(&key("a")), Ordering::Equal);
        assert_eq!(key("aBc").cmp(&key("AbC")), Ordering::Equal);
    }

    #[test]
    fn case_insensitive_hash() {
        fn calculate_hash<T: std::hash::Hash>(t: T) -> u64 {
            use std::hash::Hasher;
            let mut s = std::collections::hash_map::DefaultHasher::new();
            t.hash(&mut s);
            s.finish()
        }
        assert_eq!(calculate_hash(key("aBc")), calculate_hash(key("AbC")));
    }
}
