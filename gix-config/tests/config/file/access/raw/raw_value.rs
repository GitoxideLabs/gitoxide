use gix_config::{File, lookup};

#[test]
fn single_section() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert_eq!(config.raw_value("core.a")?.as_ref(), "b");
    assert_eq!(config.raw_value_by("core", None, "c")?.as_ref(), "d");
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
    assert_eq!(config.raw_value("core.a")?.as_ref(), "d");
    Ok(())
}

#[test]
fn last_one_wins_respected_across_section() -> crate::Result {
    let config = File::try_from("[core]\na=b\n[core]\na=d")?;
    assert_eq!(config.raw_value("core.a")?.as_ref(), "d");
    Ok(())
}

#[test]
fn section_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert!(matches!(
        config.raw_value("foo.a"),
        Err(lookup::existing::Error::SectionMissing)
    ));
    Ok(())
}

#[test]
fn subsection_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert!(matches!(
        config.raw_value("core.a.a"),
        Err(lookup::existing::Error::SubSectionMissing)
    ));
    Ok(())
}

#[test]
fn key_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert!(matches!(
        config.raw_value("core.aaaaaa"),
        Err(lookup::existing::Error::KeyMissing)
    ));
    Ok(())
}

#[test]
fn subsection_must_be_respected() -> crate::Result {
    let config = File::try_from("[core]a=b\n[core.a]a=c")?;
    assert_eq!(config.raw_value("core.a")?.as_ref(), "b");
    assert_eq!(config.raw_value("core.a.a")?.as_ref(), "c");
    Ok(())
}
