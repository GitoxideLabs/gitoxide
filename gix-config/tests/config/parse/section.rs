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
    use gix_error::ExnMessageResult;

    fn serialized(name: &str, subsection: impl IntoBStringOpt) -> ExnMessageResult<bstr::BString> {
        let mut config = gix_config::File::default();
        let section = config.new_section(name, subsection.into_bstring_opt())?;
        Ok(section.header().to_bstring())
    }

    mod write_to {
        use crate::Result;
        use crate::parse::section::header::serialized;

        #[test]
        fn subsection_backslashes_and_quotes_are_escaped() -> Result {
            assert_eq!(serialized("core", r"a\b")?, r#"[core "a\\b"]"#);
            assert_eq!(serialized("core", r#"a:"b""#)?, r#"[core "a:\"b\""]"#);
            Ok(())
        }

        #[test]
        fn everything_is_allowed() -> Result {
            assert_eq!(serialized("core", "a/b \t\t a\\b")?, "[core \"a/b \t\t a\\\\b\"]");
            Ok(())
        }
    }
    mod new {
        use crate::parse::section::header::serialized;

        #[test]
        fn names_must_be_mostly_ascii() {
            let mut message_diagnostics = Vec::new();
            for name in ["🤗", "x.y", "x y", "x\ny"] {
                message_diagnostics.push(gix_testtools::redact_debug_snapshot(
                    &(serialized(name, None).expect_err("name must be rejected")),
                    &[],
                ));
            }
            insta::assert_debug_snapshot!(message_diagnostics, "names must be mostly ascii", @r#"
            [
                section names can only be ascii, '-', "input"="🤗",
                section names can only be ascii, '-', "input"="x.y",
                section names can only be ascii, '-', "input"="x y",
                section names can only be ascii, '-', "input"="x\ny",
            ]
            "#);
        }

        #[test]
        fn subsections_with_newlines_and_null_bytes_are_rejected() {
            let mut message_diagnostics = Vec::new();
            for subsection in ["a\nb", "a\0b"] {
                message_diagnostics.push(gix_testtools::redact_debug_snapshot(
                    &(serialized("a", subsection).expect_err("subsection must be rejected")),
                    &[],
                ));
            }
            insta::assert_debug_snapshot!(message_diagnostics, "subsections with newlines and null bytes are rejected", @r#"
            [
                sub-section names must not contain newlines or null bytes, "input"="a\nb",
                sub-section names must not contain newlines or null bytes, "input"="a\0b",
            ]
            "#);
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
