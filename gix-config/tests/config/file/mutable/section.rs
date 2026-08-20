#[test]
fn section_mut_must_exist_as_section_is_not_created_automatically() {
    let mut config = multi_value_section();
    assert!(config.section_mut("foo", None).is_err());
}

#[test]
fn section_mut_or_create_new_is_infallible() -> crate::Result {
    let mut config = multi_value_section();
    let section = config.section_mut_or_create_new("name", "subsection")?;
    assert_eq!(section.header().name(), "name");
    assert_eq!(section.header().subsection_name().expect("set"), "subsection");
    Ok(())
}

#[test]
fn section_mut_or_create_new_filter_may_reject_existing_sections() -> crate::Result {
    let mut config = multi_value_section();
    let section = config.section_mut_or_create_new_filter("a", None, |_| false)?;
    assert_eq!(section.header().name(), "a");
    assert_eq!(section.header().subsection_name(), None);
    assert_eq!(section.to_bstring(), "[a]\n");
    assert_eq!(
        section.meta(),
        &gix_config::file::Metadata::api(),
        "new sections are of source 'API'"
    );
    Ok(())
}

#[test]
fn section_mut_by_id() {
    let mut config = multi_value_section();
    let id = config.sections_and_ids().next().expect("at least one").1;
    let section = config.section_mut_by_id(id).expect("present");
    assert_eq!(section.header().name(), "a");
    assert_eq!(section.header().subsection_name(), None);
}

mod rename {
    use bstr::ByteSlice;

    #[test]
    fn detached_sections_can_be_renamed() -> crate::Result {
        let mut section = gix_config::file::Section::new("remote", "origin", gix_config::file::Metadata::default())?;
        section.to_mut().rename("branch", "main")?;

        let section = section.to_ref();
        assert_eq!(section.header().name(), "branch");
        assert_eq!(section.header().subsection_name(), Some("main".into()));
        Ok(())
    }

    #[test]
    fn attached_sections_are_renamed_unambiguously_and_update_lookups() -> crate::Result {
        let mut file = gix_config::File::try_from(
            "[target \"same\"] key = first\n\
             [source \"old\"] key = selected\n\
             [target \"same\"] key = middle\n\
             [source \"old\"] key = last\n",
        )?;
        let selected_id = file
            .sections_and_ids_by_name("source")
            .expect("source sections exist")
            .next()
            .expect("at least one source section")
            .1;

        file.section_mut_by_id(selected_id)
            .expect("selected section exists")
            .rename("target", "same")?;

        insta::assert_snapshot!(file.to_string(), "only the selected section is renamed", @r#"
        [target "same"]
         key = first
        [target "same"]
         key = selected
        [target "same"]
         key = middle
        [source "old"]
         key = last
        "#);

        assert_eq!(
            file.section("source", Some("old".as_bytes().as_bstr()))?.value("key"),
            Some("last".into()),
            "the other source section remains available"
        );
        Ok(())
    }

    #[test]
    fn invalid_names_leave_attached_sections_unchanged() -> crate::Result {
        let mut file = gix_config::File::try_from("[core] key = value\n")?;
        assert!(file.section_mut("core", None)?.rename("not_valid", None).is_err());
        assert_eq!(
            file.section("core", None)?.value("key"),
            Some("value".into()),
            "no change was performed"
        );
        assert!(
            file.section("not-valid", None).is_err(),
            "the valid version of the name is also not present"
        );
        Ok(())
    }
}

mod remove {
    use super::multi_value_section;

    #[test]
    fn all() -> crate::Result {
        let mut config = multi_value_section();
        let mut section = config.section_mut("a", None)?;

        assert_eq!(section.num_values(), 5);
        assert_eq!(section.value_names().count(), 5);

        let prev_values = vec!["v", "", "", "", "a        b        c"];
        let mut num_values = section.num_values();
        for (key, expected_prev_value) in ('a'..='e').zip(prev_values) {
            let prev_value = section.remove(&key.to_string());
            num_values -= 1;
            assert_eq!(prev_value.expect("present"), expected_prev_value);
            assert_eq!(section.num_values(), num_values);
        }

        assert!(!section.is_void(), "everything is still there");
        assert_eq!(config.to_string(), "\n        [a]\n");
        Ok(())
    }
}

mod pop {
    use super::multi_value_section;

    #[test]
    fn all() -> crate::Result {
        let mut config = multi_value_section();
        let mut section = config.section_mut_by_key("a")?;

        assert_eq!(section.num_values(), 5);
        assert_eq!(section.value_names().count(), 5);

        for key in b'a'..=b'e' {
            assert!(section.contains_value_name(std::str::from_utf8(&[key])?));
        }
        let mut num_values = section.num_values();
        for _ in 0..section.num_values() {
            section.pop();
            num_values -= 1;
            assert_eq!(section.num_values(), num_values);
        }
        assert!(!section.is_void(), "there still is some whitespace");
        assert_eq!(config.to_string(), "\n        [a]\n");
        Ok(())
    }
}

mod set {
    use super::multi_value_section;

    #[test]
    fn various_escapes_onto_various_kinds_of_values() -> crate::Result {
        let mut config = multi_value_section();
        let mut section = config.section_mut("a", None)?;
        let values = vec!["", " a", "b\t", "; comment", "a\n\tc  d\\ \"x\""];
        let prev_values = vec!["v", "", "", "", "a        b        c"];
        assert_eq!(section.num_values(), values.len());

        for (key, (new_value, expected_prev_value)) in (b'a'..=b'e').zip(values.into_iter().zip(prev_values)) {
            let key = std::str::from_utf8(std::slice::from_ref(&key))?.to_owned();
            let prev_value = section.set(&key, new_value)?;
            assert_eq!(prev_value.expect("prev value set"), expected_prev_value);
        }

        assert_eq!(
            config.to_string(),
            "\n        [a]\n            a = \n            b = \" a\"\n            c=\"b\\t\"\n            d\"; comment\"\n            e =a\\n\\tc  d\\\\ \\\"x\\\"\n"
        );
        assert_eq!(
            config.section_mut("a", None)?.set("new-one", "value")?,
            None,
            "new values don't replace an existing one"
        );
        Ok(())
    }
}

mod value_name_validation {
    #[test]
    fn mutations_validate_names_and_leave_the_section_unchanged_on_error() -> crate::Result {
        let mut config = gix_config::File::default();
        let mut section = config.new_section("core", None)?;

        let err: gix_error::ValidationError = section.push("not.valid", Some("value".into())).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Valid value names consist of alphanumeric characters or dashes, starting with an alphabetic character.: \"not.valid\""
        );
        let err: gix_error::ValidationError = section
            .push_with_comment("1invalid", Some("value".into()), "comment")
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Valid value names consist of alphanumeric characters or dashes, starting with an alphabetic character.: \"1invalid\""
        );
        let err: gix_error::ValidationError = section.set("also invalid", "value").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Valid value names consist of alphanumeric characters or dashes, starting with an alphabetic character.: \"also invalid\""
        );
        assert_eq!(section.num_values(), 0, "validation happens before mutation");
        Ok(())
    }

    #[test]
    fn names_returned_by_public_apis_are_strings() -> crate::Result {
        let mut config = super::multi_value_section();
        let mut section = config.section_mut("a", None)?;
        let names: Vec<String> = section.value_names().collect();
        assert_eq!(names, ["a", "b", "c", "d", "e"]);

        let (name, _) = section.pop().expect("at least one value");
        let _: String = name;
        Ok(())
    }
}

mod push {
    use crate::file::bstring;

    #[test]
    fn none_as_value_omits_the_key_value_separator() -> crate::Result {
        let mut file = gix_config::File::default();
        let mut section = file.section_mut_or_create_new("a", "sub")?;
        section.push("key", None)?;
        let expected = format!("[a \"sub\"]{nl}\tkey{nl}", nl = section.newline());
        assert_eq!(section.value("key"), None, "single value counts as None");
        assert_eq!(
            section.values("key"),
            &[bstring("")],
            "multi-value counts as empty value"
        );
        assert_eq!(file.to_bstring(), expected);
        Ok(())
    }

    #[test]
    fn whitespace_is_derived_from_whitespace_before_first_value() -> crate::Result {
        for (input, expected_pre_key, expected_sep) in [
            ("[a]\n\t\tb=c", Some("\t\t".into()), (None, None)),
            ("[a]\nb= c", None, (None, Some(" "))),
            ("[a]", Some("\t".into()), (Some(" "), Some(" "))),
            ("[a] b", Some(" ".into()), (None, None)),
            ("[a]\tb = ", Some("\t".into()), (Some(" "), Some(" "))),
            ("[a]\t\tb =c", Some("\t\t".into()), (Some(" "), None)),
            (
                "[a]\n\t\t  \n    \t    b =  c",
                Some("    \t    ".into()),
                (Some(" "), Some("  ")),
            ),
        ] {
            let mut config: gix_config::File = input.parse()?;
            let section = config.section_mut("a", None)?;
            assert_eq!(
                section.leading_whitespace(),
                expected_pre_key,
                "{input:?} should find {expected_pre_key:?} as leading whitespace"
            );

            let (pre_sep, post_sep) = expected_sep;
            assert_eq!(
                section.separator_whitespace(),
                (pre_sep.map(Into::into), post_sep.map(Into::into)),
                "{input:?} should find {expected_sep:?} as sep whitespace"
            );
        }
        Ok(())
    }

    #[test]
    fn values_are_escaped() {
        for (value, expected) in [
            ("a b", "$head\tk = a b$nl"),
            (" a b", "$head\tk = \" a b\"$nl"),
            ("a b\t", "$head\tk = \"a b\\t\"$nl"),
            (";c", "$head\tk = \";c\"$nl"),
            ("#c", "$head\tk = \"#c\"$nl"),
            ("a\nb\n\tc", "$head\tk = a\\nb\\n\\tc$nl"),
        ] {
            let mut config = gix_config::File::default();
            let mut section = config.new_section("a", None).unwrap();
            section.set_implicit_newline(false);
            section
                .push("k", Some(value.into()))
                .expect("the fixture fits into the backing buffer");
            let expected = expected
                .replace("$head", &format!("[a]{nl}", nl = section.newline()))
                .replace("$nl", &section.newline().to_string());
            assert_eq!(config.to_bstring(), expected);
        }
    }
}

mod push_with_comment {
    #[test]
    fn various_comments_and_escaping() {
        for (comment, expected) in [
            ("", "$head\tk = v #$nl"),
            ("this is v!", "$head\tk = v # this is v!$nl"),
            (" no double space", "$head\tk = v # no double space$nl"),
            ("\tno double whitespace", "$head\tk = v #\tno double whitespace$nl"),
            (
                "one\ntwo\nnewlines are replaced with space",
                "$head\tk = v # one two newlines are replaced with space$nl",
            ),
            (
                "a\rb\r\nlinefeeds aren't special",
                "$head\tk = v # a\rb\r linefeeds aren't special$nl",
            ),
        ] {
            let mut config = gix_config::File::default();
            let mut section = config.new_section("a", None).unwrap();
            section.set_implicit_newline(false);
            section
                .push_with_comment("k", Some("v".into()), comment)
                .expect("the fixture fits into the backing buffer");
            let expected = expected
                .replace("$head", &format!("[a]{nl}", nl = section.newline()))
                .replace("$nl", &section.newline().to_string());
            assert_eq!(config.to_bstring(), expected);
        }
    }
}

mod set_leading_whitespace {
    #[test]
    fn any_whitespace_is_ok() -> crate::Result {
        let mut config = gix_config::File::default();
        let mut section = config.new_section("core", None)?;

        let nl = section.newline().to_owned();
        section.set_leading_whitespace(format!("{nl}\t"));
        section.push("a", Some("v".into()))?;

        assert_eq!(config.to_string(), format!("[core]{nl}{nl}\ta = v{nl}"));
        Ok(())
    }

    #[test]
    #[should_panic]
    fn panics_if_non_whitespace_is_used() {
        let mut config = gix_config::File::default();
        let mut section = config.new_section("core", None).unwrap();
        section.set_leading_whitespace("foo");
    }
}

fn multi_value_section() -> gix_config::File {
    r"
        [a]
            a = v
            b = 
            c=
            d
            e =a \
       b \
       c"
    .parse()
    .unwrap()
}
