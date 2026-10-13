use crate::TestResult;
use gix_config::File;

use crate::file::bstring;

#[test]
fn single_value_is_identical_to_single_value_query() -> TestResult {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert_eq!(vec![config.raw_value("core.a")?], config.raw_values("core.a")?);
    Ok(())
}

#[test]
fn multi_value_in_section() -> TestResult {
    let config = File::try_from("[core]\na=b\na=c")?;
    assert_eq!(config.raw_values("core.a")?, vec![bstring("b"), bstring("c")]);
    Ok(())
}

#[test]
fn multi_value_across_sections() -> TestResult {
    let config = File::try_from(
        "[core]\n\
         a=b\n\
         a=c\n\
         [core]a=d",
    )?;
    assert_eq!(
        config.raw_values("core.a")?,
        vec![bstring("b"), bstring("c"), bstring("d")]
    );
    Ok(())
}

#[test]
fn values_with_sections_identify_each_values_section_in_file_order() -> TestResult {
    let config = File::try_from(
        "[core]\n\
         a=b\n\
         a=c\n\
         [core]a=d",
    )?;
    let section_ids: Vec<_> = config.sections().map(|section| section.id()).collect();

    let values = config.raw_values_with_sections("core.a")?;
    let actual: Vec<_> = values
        .into_iter()
        .map(|(value, section)| (value, section.id()))
        .collect();
    assert_eq!(
        actual,
        [
            (bstring("b"), section_ids[0]),
            (bstring("c"), section_ids[0]),
            (bstring("d"), section_ids[1]),
        ]
    );

    let by = config.raw_values_with_sections_by("core", None, "a")?;
    assert_eq!(by.len(), 3, "the explicit-component variant has identical semantics");
    Ok(())
}

#[test]
fn values_with_sections_filter_returns_values_from_accepted_sections() -> TestResult {
    let config = File::try_from(
        "[core]\n\
         a=b\n\
         a=c\n\
         [core]a=d",
    )?;
    let second_section_id = config.sections().nth(1).expect("second section").id();

    let mut reject_first_section = true;
    let values =
        config.raw_values_with_sections_filter("core.a", |_meta| !std::mem::take(&mut reject_first_section))?;
    assert_eq!(
        values
            .into_iter()
            .map(|(value, section)| (value, section.id()))
            .collect::<Vec<_>>(),
        [(bstring("d"), second_section_id)],
        "only values from sections accepted by the filter are returned"
    );

    let values = config.raw_values_with_sections_filter_by("core", None, "a", |_| true)?;
    assert_eq!(
        values.len(),
        3,
        "the component variant applies the same lookup semantics"
    );
    Ok(())
}

#[test]
fn implicit_values_with_sections_preserve_presence_order_and_metadata() -> TestResult {
    use gix_config::{Source, file::Metadata};

    let mut config = File::from_bytes_no_includes(
        b"[core]\nA \t # implicit\nother=untouched\na=\na=\"\"\n",
        Metadata::from(Source::User).at("global"),
        Default::default(),
    )?;
    config.append(File::from_bytes_no_includes(
        b"[core]\na=\"line\\n\"\\\n\"next\\t\"\n[core \"other\"]\na=ignored\n[core]\na\n",
        Metadata::from(Source::Local).at("local"),
        Default::default(),
    )?)?;
    let section_ids: Vec<_> = config.sections().map(|section| section.id()).collect();
    let values = config.raw_values_implicit_with_sections("core.a")?;
    assert_eq!(
        values.iter().map(|(value, _)| value.clone()).collect::<Vec<_>>(),
        [
            None,
            Some(bstring("")),
            Some(bstring("")),
            Some(bstring("line\nnext\t")),
            None
        ],
        "implicit entries remain distinct from explicit empty values, and continuations are normalized"
    );
    assert_eq!(
        values.iter().map(|(_, section)| section.id()).collect::<Vec<_>>(),
        [
            section_ids[0],
            section_ids[0],
            section_ids[0],
            section_ids[1],
            section_ids[3]
        ],
        "each occurrence retains its section, excluding unrelated subsections"
    );
    assert_eq!(
        values
            .iter()
            .map(|(_, section)| section.meta().source)
            .collect::<Vec<_>>(),
        [Source::User, Source::User, Source::User, Source::Local, Source::Local],
        "merged configuration preserves source precedence"
    );
    for (_, section) in &values {
        let expected = if section.meta().source == Source::User {
            "global"
        } else {
            "local"
        };
        assert_eq!(section.meta().path.as_deref(), Some(std::path::Path::new(expected)));
    }
    assert_eq!(
        config.raw_values("core.a")?,
        values
            .into_iter()
            .map(|(value, _)| value.unwrap_or_default())
            .collect::<Vec<_>>(),
        "existing raw lookups keep their empty-string representation of implicit values"
    );
    for key in ["missing.a", "core.absent", "core.missing.a"] {
        assert!(
            config
                .raw_values_implicit_with_sections(key)
                .expect_err("missing key")
                .is_not_found(),
            "missing sections, subsections and keys retain the existing lookup error class"
        );
    }
    Ok(())
}

#[test]
fn implicit_values_match_git_boolean_semantics() -> TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    let bytes = "[core]\nflag\nflag=\n";
    std::fs::write(dir.path().join("config"), bytes)?;
    let config = File::try_from(bytes)?;
    assert_eq!(
        config
            .raw_values_implicit_with_sections("core.flag")?
            .into_iter()
            .map(|(value, _)| value)
            .collect::<Vec<_>>(),
        [None, Some(bstring(""))],
        "an absent separator is different from an explicit empty value"
    );
    let git = gix_testtools::git_command(dir.path())
        .args([
            "config",
            "--file",
            "config",
            "--null",
            "--bool",
            "--get-all",
            "core.flag",
        ])
        .output()?;
    assert!(
        git.status.success(),
        "the isolated reference accepts both entries: {git:?}"
    );
    assert_eq!(
        git.stdout, b"true\0false\0",
        "Git distinguishes implicit true from explicit empty false"
    );
    Ok(())
}

#[test]
fn section_not_found() -> TestResult {
    let config = File::try_from("[core]\na=b\nc=d")?;
    let err = config.raw_values("foo.a").unwrap_err();
    assert!(err.is_not_found());
    insta::assert_debug_snapshot!(err, "section not found", @"The requested section does not exist");
    Ok(())
}

#[test]
fn subsection_not_found() -> TestResult {
    let config = File::try_from("[core]\na=b\nc=d")?;
    let err = config.raw_values("core.a.a").unwrap_err();
    assert!(err.is_not_found());
    insta::assert_debug_snapshot!(err, "subsection not found", @"The requested subsection does not exist");
    Ok(())
}

#[test]
fn key_not_found() -> TestResult {
    let config = File::try_from("[core]\na=b\nc=d")?;
    let err = config.raw_values("core.aaaaaa").unwrap_err();
    assert!(err.is_not_found());
    insta::assert_debug_snapshot!(err, "key not found", @"The key does not exist in the requested section");
    Ok(())
}

#[test]
fn subsection_must_be_respected() -> TestResult {
    let config = File::try_from("[core]a=b\n[core.a]a=c")?;
    assert_eq!(config.raw_values("core.a")?, vec![bstring("b")]);
    assert_eq!(config.raw_values("core.a.a")?, vec![bstring("c")]);
    Ok(())
}

#[test]
fn non_relevant_subsection_is_ignored() -> TestResult {
    let config = File::try_from("[core]\na=b\na=c\n[core]a=d\n[core]g=g")?;
    assert_eq!(
        config.raw_values("core.a")?,
        vec![bstring("b"), bstring("c"), bstring("d")]
    );
    Ok(())
}
