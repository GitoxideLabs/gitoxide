use gix_config::File;

#[test]
fn single_section() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert_eq!(config.raw_value("core.a")?, "b");
    assert_eq!(config.raw_value_by("core", None, "c")?, "d");
    Ok(())
}

#[test]
fn global_property_uses_empty_section_name() -> crate::Result {
    let config = File::try_from("a=b\n[core]\na=c")?;
    assert_eq!(
        config.raw_value_by("", None, "a").unwrap_err().to_string(),
        "The requested section does not exist",
        "these are not readable because the supporting this adds a lot of complexity"
    );
    Ok(())
}

#[test]
fn last_one_wins_respected_in_section() -> crate::Result {
    let config = File::try_from("[core]\na=b\na=d")?;
    assert_eq!(config.raw_value("core.a")?, "d");
    Ok(())
}

#[test]
fn last_one_wins_respected_across_section() -> crate::Result {
    let config = File::try_from("[core]\na=b\n[core]\na=d")?;
    assert_eq!(config.raw_value("core.a")?, "d");
    Ok(())
}

#[test]
fn value_with_section_identifies_the_section_containing_the_resolved_value() -> crate::Result {
    let config = File::try_from(
        "[core]\n\
         a=first\n\
         [core]\n\
         a\n",
    )?;
    let first_section_id = config.sections().next().expect("first section").id();

    let (value, section) = config.raw_value_with_section("core.a")?;
    assert_eq!(value, "first", "implicit values are skipped during resolution");
    assert_eq!(section.id(), first_section_id, "the returned section owns the value");

    let (value, section) = config.raw_value_with_section_by("core", None, "a")?;
    assert_eq!(value, "first");
    assert_eq!(section.id(), first_section_id);
    Ok(())
}

#[test]
fn value_with_section_filter_identifies_the_section_containing_the_resolved_value() -> crate::Result {
    let config = File::try_from(
        "[core]\n\
         a=first\n\
         [core]\n\
         a=second\n",
    )?;
    let first_section_id = config.sections().next().expect("first section").id();

    let mut reject_last_section = true;
    let (value, section) =
        config.raw_value_with_section_filter("core.a", |_meta| !std::mem::take(&mut reject_last_section))?;
    assert_eq!(value, "first", "the last value in an accepted section wins");
    assert_eq!(
        section.id(),
        first_section_id,
        "the returned section is the one accepted by the filter"
    );
    Ok(())
}

#[test]
fn mutable_value_filters_have_key_and_component_variants() -> crate::Result {
    let mut config = File::try_from(
        "[core]\n\
         a=first\n\
         [core]\n\
         a=second\n",
    )?;

    let mut reject_last_section = true;
    config
        .raw_value_mut_filter("core.a", |_meta| !std::mem::take(&mut reject_last_section))?
        .set("changed")?;
    assert_eq!(
        config.raw_values("core.a")?,
        ["changed", "second"],
        "the key variant mutates the value in the accepted section"
    );

    config
        .raw_value_mut_filter_by(String::from("core"), None, String::from("a"), |_| true)?
        .set("last")?;
    assert_eq!(
        config.raw_values("core.a")?,
        ["changed", "last"],
        "the component variant accepts owned string components"
    );

    insta::assert_snapshot!(config.to_string(), "both values changed", @"
    [core]
    a=changed
    [core]
    a=last
    ");
    Ok(())
}

#[test]
fn section_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    let err = config.raw_value("foo.a").unwrap_err();
    assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
    assert_eq!(err.to_string(), "The requested section does not exist");
    Ok(())
}

#[test]
fn subsection_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    let err = config.raw_value("core.a.a").unwrap_err();
    assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
    assert_eq!(err.to_string(), "The requested subsection does not exist");
    Ok(())
}

#[test]
fn key_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    let err = config.raw_value("core.aaaaaa").unwrap_err();
    assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
    assert_eq!(err.to_string(), "The key does not exist in the requested section");
    Ok(())
}

#[test]
fn invalid_value_names_are_reported_by_mutable_lookups() -> crate::Result {
    let mut config = File::try_from("[core]\na=b")?;
    let err = config.raw_value_mut_by("core", None, "1invalid").unwrap_err();
    assert!(err.downcast_any_ref::<gix_error::ValidationError>().is_some());
    assert_eq!(
        err.to_string(),
        "Valid value names consist of alphanumeric characters or dashes, starting with an alphabetic character.: \"1invalid\""
    );
    let err = config.raw_values_mut_by("core", None, "contains.dot").unwrap_err();
    assert!(err.downcast_any_ref::<gix_error::ValidationError>().is_some());
    assert_eq!(
        err.to_string(),
        "Valid value names consist of alphanumeric characters or dashes, starting with an alphabetic character.: \"contains.dot\""
    );
    Ok(())
}

#[test]
fn subsection_must_be_respected() -> crate::Result {
    let config = File::try_from("[core]a=b\n[core.a]a=c")?;
    assert_eq!(config.raw_value("core.a")?, "b");
    assert_eq!(config.raw_value("core.a.a")?, "c");
    Ok(())
}
