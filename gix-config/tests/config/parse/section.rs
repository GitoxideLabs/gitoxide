use gix_config::parse::EventRef;

pub fn header_event(name: &'static str, subsection: impl Into<Option<&'static str>>) -> EventRef<'static> {
    let subsection_name = subsection.into();
    EventRef::SectionHeader {
        name: name.into(),
        separator: subsection_name.map(|_| " ".into()),
        subsection_name: subsection_name.map(Into::into),
    }
}

mod header {
    use gix_config::file::IntoBStringOpt;

    fn serialized(
        name: &str,
        subsection: impl IntoBStringOpt,
    ) -> Result<bstr::BString, gix_config::parse::section::header::Error> {
        let mut config = gix_config::File::default();
        let section = config.new_section(name, subsection.into_bstring_opt())?;
        Ok(section.header().to_bstring())
    }

    mod write_to {
        use crate::parse::section::header::serialized;

        #[test]
        fn subsection_backslashes_and_quotes_are_escaped() -> crate::Result {
            assert_eq!(serialized("core", r"a\b")?, r#"[core "a\\b"]"#);
            assert_eq!(serialized("core", r#"a:"b""#)?, r#"[core "a:\"b\""]"#);
            Ok(())
        }

        #[test]
        fn everything_is_allowed() -> crate::Result {
            assert_eq!(serialized("core", "a/b \t\t a\\b")?, "[core \"a/b \t\t a\\\\b\"]");
            Ok(())
        }
    }
    mod new {
        use crate::parse::section::header::serialized;

        #[test]
        fn names_must_be_mostly_ascii() {
            for name in ["🤗", "x.y", "x y", "x\ny"] {
                assert_eq!(
                    serialized(name, None).expect_err("name must be rejected").message,
                    "section names can only be ascii, '-'"
                );
            }
        }

        #[test]
        fn subsections_with_newlines_and_null_bytes_are_rejected() {
            for subsection in ["a\nb", "a\0b"] {
                assert_eq!(
                    serialized("a", subsection)
                        .expect_err("subsection must be rejected")
                        .message,
                    "sub-section names must not contain newlines or null bytes"
                );
            }
        }
    }
}
mod name {
    use gix_config::parse::section::Name;

    fn name(name: &str) -> Name {
        Name::try_from(name).expect("valid section name")
    }

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

    #[test]
    fn case_insensitive_eq() {
        assert_eq!(name("Co-Re"), name("cO-rE"));
    }
}
