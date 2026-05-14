use std::borrow::Cow;

use bstr::BStr;
use gix_config::{
    Boolean, Color, File, Integer, color,
    file::{Metadata, init},
    integer, path,
};

use crate::file::cow_str;

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
        assert!(!config.boolean("core.bool-explicit").expect("exists")?);
        assert!(!config.boolean("core.bool-explicit").expect("exists")?);

        assert!(
            config.value::<Boolean>("core.bool-implicit").is_err(),
            "this cannot work like in git as the original value isn't there for us"
        );
        assert!(
            config.boolean("core.bool-implicit").expect("present")?,
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
            Some(cow_str("")),
            "mere presence with equal sign is always the empty implicit string"
        );
        assert!(config.path("core.empty-equals").is_some(), "this is an empty path…");
        assert_eq!(
            config.string("core.empty-explicit"),
            Some(cow_str("")),
            "and so is an explicit empty string"
        );
        assert!(
            config.path("core.empty-explicit").is_some(),
            "and so is an explicit empty path"
        );
        assert_eq!(
            config.strings("core.bool-implicit").expect("present"),
            &[cow_str("")],
            "unset values show up as empty within a string array"
        );
        assert_eq!(config.strings("core.bool-implicit").expect("present"), &[cow_str("")]);

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
            let string = config.value::<Cow<'_, BStr>>("core.other")?;
            assert_eq!(string, cow_str("hello world"));
            assert!(
                matches!(string, Cow::Borrowed(_)),
                "no copy is made, we reference the `file` itself"
            );
        }

        assert_eq!(config.string("core.other-quoted").unwrap(), cow_str("hello world"));

        {
            let strings = config.strings("core.other-quoted").unwrap();
            assert_eq!(strings, vec![cow_str("hello"), cow_str("hello world")]);
            assert!(matches!(strings[0], Cow::Borrowed(_)));
            assert!(matches!(strings[1], Cow::Borrowed(_)));
        }

        {
            let cow = config.string("core.other").expect("present");
            assert_eq!(cow.as_ref(), "hello world");
            assert!(matches!(cow, Cow::Borrowed(_)));
        }
        assert_eq!(
            config.string("core.other-quoted").expect("present").as_ref(),
            "hello world"
        );

        {
            let actual = config.value::<gix_config::Path>("core.location")?;
            assert_eq!(&*actual, "~/tmp", "no interpolation occurs when querying a path");

            let home = std::env::current_dir()?;
            let expected = home.join("tmp");
            assert!(matches!(actual.value, Cow::Borrowed(_)));
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
        file.boolean("core.bool-implicit").expect("present")?,
        "correct handling of booleans is implemented specifically"
    );

    assert!(
        !file.value::<Boolean>("core.bool-explicit")?.0,
        "explicit values always work"
    );

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
fn single_section() {
    let config = File::try_from("[core]\na=b\nc").unwrap();
    let first_value = config.string("core.a").unwrap();
    assert_eq!(first_value, cow_str("b"));

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
    assert_eq!(value, cow_str("git@github.com:GitoxideLabs/gitoxide.git"));
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

    config.set_raw_value_by("present", Some("subsection".into()), "key", "value")?;
    assert!(config.section("present", Some("subsection".into())).is_ok());

    config.set_raw_value_by("new", Some("subsection".into()), "key", "value")?;
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
    assert_eq!(config.raw_value("alias.save").unwrap().as_ref(), expected);
    assert_eq!(config.string("alias.save").unwrap().as_ref(), expected);
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
        config.raw_value("core.escape-sequence").unwrap().as_ref(),
        expected,
        "raw_value is normalized…"
    );
    assert_eq!(
        config.string("core.escape-sequence").unwrap().as_ref(),
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
    assert_eq!(config.raw_value("alias.save").unwrap().as_ref(), expected);
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
    assert_eq!(config.raw_value("alias.save").unwrap().as_ref(), expected);
}

#[test]
fn overrides_with_implicit_booleans_work_in_single_section() {
    let config = r#"
        [a]
            b = false
            b
        "#;
    let config = File::try_from(config).unwrap();
    assert_eq!(config.boolean("a.b"), Some(Ok(true)), "empty implicit booleans ");
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
    assert_eq!(config.boolean("a.b"), Some(Ok(true)), "empty implicit booleans ");
}
