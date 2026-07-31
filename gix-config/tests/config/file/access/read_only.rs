use bstr::BString;
use gix_config::{
    Boolean, Color, File, Integer, color,
    file::{Metadata, init},
    integer, path,
};

use crate::file::bstring;

#[test]
fn parsed_section_header_legacy_check_uses_backing_buffer() -> crate::Result {
    let config = File::try_from(
        "[remote.origin]\n\turl = https://example.com\n[remote \"upstream\"]\n\turl = https://example.com\n",
    )?;
    let sections = config.sections().collect::<Vec<_>>();

    assert!(sections[0].header().is_legacy());
    assert!(!sections[1].header().is_legacy());

    Ok(())
}

/// Asserts we can cast into all variants of our type
#[test]
fn get_value_for_all_provided_values() -> crate::Result {
    let config = r#"
        [core]
            other-quoted = "hello"
        [core]
            bool-explicit = false
            bool-implicit
            integer-no-prefix = 10
            integer-prefix = 10g
            color = brightgreen red \
            bold
            other = hello world
            other-quoted = "hello world"
            location = ~/tmp
            location-quoted = "~/quoted"
            empty-implicit
            empty-equals = 
            empty-explicit = ""
    "#;
    for lossy in [false, true] {
        let config = File::from_bytes_no_includes(
            config.as_bytes(),
            Metadata::api(),
            init::Options {
                lossy,
                ..Default::default()
            },
        )?;

        assert!(!config.value::<Boolean>("core.bool-explicit")?.0);
        assert!(!config.boolean("core.bool-explicit")?.expect("exists"));
        assert!(!config.boolean("core.bool-explicit")?.expect("exists"));

        assert!(
            config.value::<Boolean>("core.bool-implicit").is_err(),
            "this cannot work like in git as the original value isn't there for us"
        );
        assert!(
            config.boolean("core.bool-implicit")?.expect("present"),
            "implicit booleans resolve to being true"
        );
        assert_eq!(
            config.string("core.bool-implicit"),
            None,
            "unset values are not present"
        );
        assert_eq!(
            config.string("core.empty-implicit"),
            None,
            "mere presence is at most a boolean"
        );
        assert_eq!(
            config.path("core.empty-implicit"),
            None,
            "mere presence is at most a boolean"
        );
        assert_eq!(
            config.string("core.empty-equals"),
            Some(bstring("")),
            "mere presence with equal sign is always the empty implicit string"
        );
        assert!(config.path("core.empty-equals").is_some(), "this is an empty path…");
        assert_eq!(
            config.string("core.empty-explicit"),
            Some(bstring("")),
            "and so is an explicit empty string"
        );
        assert!(
            config.path("core.empty-explicit").is_some(),
            "and so is an explicit empty path"
        );
        assert_eq!(
            config.strings("core.bool-implicit").expect("present"),
            &[bstring("")],
            "unset values show up as empty within a string array"
        );
        assert_eq!(config.strings("core.bool-implicit").expect("present"), &[bstring("")]);

        assert_eq!(config.string("doesn't.exist"), None);

        assert_eq!(
            config.value::<Integer>("core.integer-no-prefix")?,
            Integer {
                value: 10,
                suffix: None
            }
        );

        assert_eq!(
            config.value::<Integer>("core.integer-no-prefix")?,
            Integer {
                value: 10,
                suffix: None
            }
        );

        assert_eq!(
            config.value::<Integer>("core.integer-prefix")?,
            Integer {
                value: 10,
                suffix: Some(integer::Suffix::Gibi),
            }
        );

        assert_eq!(
            config.value::<Color>("core.color")?,
            Color {
                foreground: Some(color::Name::BrightGreen),
                background: Some(color::Name::Red),
                attributes: color::Attribute::BOLD
            }
        );

        {
            let string = config.value::<BString>("core.other")?;
            assert_eq!(string, bstring("hello world"));
        }

        assert_eq!(config.string("core.other-quoted").unwrap(), bstring("hello world"));

        {
            let strings = config.strings("core.other-quoted").unwrap();
            assert_eq!(strings, vec![bstring("hello"), bstring("hello world")]);
        }

        {
            let value = config.string("core.other").expect("present");
            assert_eq!(value, "hello world");
        }
        assert_eq!(config.string("core.other-quoted").expect("present"), "hello world");

        {
            let actual = config.value::<gix_config::Path>("core.location")?;
            assert_eq!(&*actual, "~/tmp", "no interpolation occurs when querying a path");

            let home = std::env::current_dir()?;
            let expected = home.join("tmp");
            assert_eq!(
                actual
                    .interpolate(path::interpolate::Context {
                        home_dir: home.as_path().into(),
                        ..Default::default()
                    })
                    .unwrap(),
                expected
            );
        }

        let actual = config.path("core.location").expect("present");
        assert_eq!(&*actual, "~/tmp");
        let actual = config.path("core.location").expect("present");
        assert_eq!(&*actual, "~/tmp");

        let actual = config.path("core.location-quoted").expect("present");
        assert_eq!(&*actual, "~/quoted");

        let actual = config.value::<gix_config::Path>("core.location-quoted")?;
        assert_eq!(&*actual, "~/quoted", "but the path is unquoted");
    }

    Ok(())
}

#[test]
fn get_value_looks_up_all_sections_before_failing() -> crate::Result {
    let config = r#"
        [core]
            bool-explicit = false
            bool-implicit = false
        [core]
            bool-implicit
    "#;

    let file = File::try_from(config)?;

    // Checks that we check the last entry first still
    assert!(
        !file.value::<Boolean>("core.bool-implicit")?.0,
        "implicit bool is invisible to `value` and boolean is the only value we want. Would have to special case it."
    );
    assert!(
        file.boolean("core.bool-implicit")?.expect("present"),
        "correct handling of booleans is implemented specifically"
    );

    assert!(
        !file.value::<Boolean>("core.bool-explicit")?.0,
        "explicit values always work"
    );

    Ok(())
}

#[test]
fn interpreted_values_can_be_returned_with_their_sections() -> crate::Result {
    let file = File::try_from(
        "[core]\n\
         a=1\n\
         a=2\n\
         [core]\n\
         a=3\n",
    )?;
    let section_ids: Vec<_> = file.sections().map(|section| section.id()).collect();

    let (value, section) = file.value_with_section::<Integer>("core.a")?;
    assert_eq!(value.value, 3);
    assert_eq!(section.id(), section_ids[1]);

    let values = file.values_with_sections::<Integer>("core.a")?;
    let actual: Vec<_> = values
        .into_iter()
        .map(|(value, section)| (value.value, section.id()))
        .collect();
    assert_eq!(actual, [(1, section_ids[0]), (2, section_ids[0]), (3, section_ids[1])]);

    let (value, section) = file.value_with_section_by::<Integer>("core", None, "a")?;
    assert_eq!((value.value, section.id()), (3, section_ids[1]));
    assert_eq!(file.values_with_sections_by::<Integer>("core", None, "a")?.len(), 3);
    Ok(())
}

#[test]
fn section_names_are_case_insensitive() -> crate::Result {
    let config = "[core] a=true";
    let file = File::try_from(config)?;
    assert_eq!(
        file.value::<Boolean>("core.a").unwrap(),
        file.value::<Boolean>("CORE.a").unwrap()
    );

    Ok(())
}

#[test]
fn value_names_are_case_insensitive() -> crate::Result {
    let config = "[core]
        a = true
        A = false";
    let file = File::try_from(config)?;
    assert_eq!(file.values::<Boolean>("core.a")?.len(), 2);
    assert_eq!(
        file.value::<Boolean>("core.a").unwrap(),
        file.value::<Boolean>("core.A").unwrap()
    );

    Ok(())
}

#[test]
fn section_value_access_is_case_insensitive() -> crate::Result {
    let file = File::try_from("[core]\nMixedCase = one\nMIXEDCASE = two")?;
    let section = file.section("core", None)?;

    assert_eq!(section.values("mixedcase"), vec![bstring("one"), bstring("two")]);
    assert_eq!(section.value("mIxEdCaSe"), Some(bstring("two")));
    assert!(section.contains_value_name("mixedcase"));
    Ok(())
}

#[test]
fn single_section() {
    let config = File::try_from("[core]\na=b\nc").unwrap();
    let first_value = config.string("core.a").unwrap();
    assert_eq!(first_value, bstring("b"));

    assert!(
        config.raw_value("core.c").is_err(),
        "value is considered false as it is without '=', so it's like not present"
    );

    assert!(
        config.boolean("core.c").expect("present").unwrap(),
        "asking for a boolean is true true, as per git rules"
    );
}

#[test]
fn sections_by_name() -> crate::Result {
    let config = r#"
    [core]
        repositoryformatversion = 0
        filemode = true
        bare = false
        logallrefupdates = true
    [remote "origin"]
        url = git@github.com:GitoxideLabs/gitoxide.git
        fetch = +refs/heads/*:refs/remotes/origin/*
    "#;

    let config = File::try_from(config)?;
    let value = config.string_by("remote", Some("origin".into()), "url").unwrap();
    assert_eq!(value, bstring("git@github.com:GitoxideLabs/gitoxide.git"));
    Ok(())
}

#[test]
fn sections_by_name_ignores_subsections_and_preserves_file_order() -> crate::Result {
    let config = File::try_from(
        "[remote] marker=plain\n\
         [other] marker=unrelated\n\
         [REMOTE \"origin\"] marker=origin\n\
         [remote \"upstream\"] marker=upstream\n\
         [remote] marker=last\n",
    )?;

    let markers: Vec<_> = config
        .sections_by_name("Remote")
        .expect("remote sections exist case-insensitively")
        .map(|section| section.value("marker").expect("each matching section has a marker"))
        .collect();
    assert_eq!(
        markers,
        [
            bstring("plain"),
            bstring("origin"),
            bstring("upstream"),
            bstring("last")
        ],
        "plain and subsection sections are returned in file order"
    );
    Ok(())
}

#[test]
fn unknown_section() -> crate::Result {
    let config = File::default();
    assert!(matches!(
        config.section("missing", None).unwrap_err(),
        gix_config::lookup::existing::Error::SectionMissing
    ));

    let config = r#"
    [present]
        key = false
    "#;
    let mut config = File::try_from(config)?;
    assert!(matches!(
        config.section("present", Some("subsection".into())).unwrap_err(),
        gix_config::lookup::existing::Error::SubSectionMissing
    ));

    config.set_raw_value_by("present", "subsection", "key", "value")?;
    assert!(config.section("present", Some("subsection".into())).is_ok());

    config.set_raw_value_by("new", "subsection", "key", "value")?;
    assert!(config.section("new", Some("subsection".into())).is_ok());

    for id in config.sections_and_ids().map(|(_, id)| id).collect::<Vec<_>>() {
        assert!(config.remove_section_by_id(id).is_some());
    }
    assert!(matches!(
        config.section("present", None).unwrap_err(),
        gix_config::lookup::existing::Error::SectionMissing
    ));

    Ok(())
}

#[test]
fn multi_line_value_plain() {
    let config = r#"
[alias]
   save = !git status \
        && git add -A \
        && git commit -m \"$1\" \
        && git push -f \
        && git log -1 \
        && :            # comment
    "#;

    let config = File::try_from(config).unwrap();

    let expected = r#"!git status         && git add -A         && git commit -m "$1"         && git push -f         && git log -1         && :"#;
    assert_eq!(config.raw_value("alias.save").unwrap(), expected);
    assert_eq!(config.string("alias.save").unwrap(), expected);
}

#[test]
fn complex_quoted_values() {
    let config = r#"
    [core]
            escape-sequence = "hi\nho\n\tthere\bi\\\" \""
"#;
    let config = File::try_from(config).unwrap();
    let expected = "hi\nho\n\ttheri\\\" \"";
    assert_eq!(
        config.raw_value("core.escape-sequence").unwrap(),
        expected,
        "raw_value is normalized…"
    );
    assert_eq!(
        config.string("core.escape-sequence").unwrap(),
        expected,
        "…and so is the comfort API"
    );
}

#[test]
fn multi_line_value_outer_quotes_unescaped_inner_quotes() {
    let config = r#"
[alias]
   save = "!f() { \
           git status; \
           git add -A; \
           git commit -m "$1"; \
           git push -f; \
           git log -1;  \
        }; \
        f;  \
        unset f"
"#;
    let config = File::try_from(config).unwrap();
    let expected = r#"!f() {            git status;            git add -A;            git commit -m $1;            git push -f;            git log -1;          };         f;          unset f"#;
    assert_eq!(config.raw_value("alias.save").unwrap(), expected);
}

#[test]
fn multi_line_value_outer_quotes_escaped_inner_quotes() {
    let config = r#"
[alias]
   save = "!f() { \
           git status; \
           git add -A; \
           git commit -m \"$1\"; \
           git push -f; \
           git log -1;  \
        }; \
        f;  \
        unset f"
"#;
    let config = File::try_from(config).unwrap();
    let expected = r#"!f() {            git status;            git add -A;            git commit -m "$1";            git push -f;            git log -1;          };         f;          unset f"#;
    assert_eq!(config.raw_value("alias.save").unwrap(), expected);
}

#[test]
fn multi_line_value_with_empty_continuation_line() {
    for config in [
        "[core]\n\tk = abc\\\n\n",
        "[core]\n\tk = abc\\\n; comment\n",
        "[core]\n\tk = abc\\\n",
        "[core]\n\tk = abc\\",
        "[core]\n\tk = abc\\\n\n[other]\n",
        "[core]\r\n\tk = abc\\\r\n",
    ] {
        let file = File::try_from(config).unwrap();
        assert_eq!(
            file.raw_values("core.k").unwrap(),
            vec![bstring("abc")],
            "Git reports only `abc` for {config:?}, as a continuation line that is empty ends the value"
        );
    }

    let config = "[core]\n\tk = abc\\\n\tk = def\n";
    let file = File::try_from(config).unwrap();
    assert_eq!(
        file.raw_value("core.k").unwrap(),
        bstring("abc\tk = def"),
        "Git reports one value, `abc\tk = def`, for {config:?}, as the next line continues the first value"
    );
}

#[test]
fn overrides_with_implicit_booleans_work_in_single_section() {
    let config = r#"
        [a]
            b = false
            b
        "#;
    let config = File::try_from(config).unwrap();
    assert_eq!(config.boolean("a.b"), Ok(Some(true)), "empty implicit booleans ");
}

#[test]
fn implicit_booleans_may_be_followed_by_whitespace() -> crate::Result {
    for config in [
        "[a]\n\tb \n",
        "[a]\n\tb\t\n",
        "[a]\n\tb  \n",
        "[a]\n\tb \t \n",
        "[a]\n\tb ",
        "[a]\n\tb \r\n",
        "[a]\n\tb\n",
    ] {
        let file = File::try_from(config)?;
        assert_eq!(
            file.boolean("a.b"),
            Ok(Some(true)),
            "Git sees no separator in {config:?}, so the value is an implicit boolean and thus true"
        );
        assert_eq!(
            file.string("a.b"),
            None,
            "implicit booleans have no value of their own, no matter the whitespace that follows them"
        );
    }

    for config in ["[a]\n\tb =\n", "[a]\n\tb = \n", "[a]\n\tb=\"\"\n", "[a]\n\tb ="] {
        let file = File::try_from(config)?;
        assert_eq!(
            file.boolean("a.b"),
            Ok(Some(false)),
            "a separator in {config:?} makes the value explicitly empty, and an empty value is false"
        );
        assert_eq!(
            file.string("a.b"),
            Some(bstring("")),
            "an explicitly empty value is the empty string"
        );
    }

    Ok(())
}

#[test]
fn overrides_with_implicit_booleans_work_across_sections() {
    let config = r#"
        [a]
            b = false
        [a]
            b
        "#;
    let config = File::try_from(config).unwrap();
    assert_eq!(config.boolean("a.b"), Ok(Some(true)), "empty implicit booleans ");
}
