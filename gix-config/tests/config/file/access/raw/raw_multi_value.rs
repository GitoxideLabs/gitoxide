use gix_config::{File, lookup};

use crate::file::cow_str;

#[test]
fn single_value_is_identical_to_single_value_query() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert_eq!(vec![config.raw_value("core.a")?], config.raw_values("core.a")?);
    Ok(())
}

#[test]
fn multi_value_in_section() -> crate::Result {
    let config = File::try_from("[core]\na=b\na=c")?;
    assert_eq!(config.raw_values("core.a")?, vec![cow_str("b"), cow_str("c")]);
    Ok(())
}

#[test]
fn multi_value_across_sections() -> crate::Result {
    let config = File::try_from("[core]\na=b\na=c\n[core]a=d")?;
    assert_eq!(
        config.raw_values("core.a")?,
        vec![cow_str("b"), cow_str("c"), cow_str("d")]
    );
    Ok(())
}

#[test]
fn section_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert!(matches!(
        config.raw_values("foo.a"),
        Err(lookup::existing::Error::SectionMissing)
    ));
    Ok(())
}

#[test]
fn subsection_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert!(matches!(
        config.raw_values("core.a.a"),
        Err(lookup::existing::Error::SubSectionMissing)
    ));
    Ok(())
}

#[test]
fn key_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert!(matches!(
        config.raw_values("core.aaaaaa"),
        Err(lookup::existing::Error::KeyMissing)
    ));
    Ok(())
}

#[test]
fn subsection_must_be_respected() -> crate::Result {
    let config = File::try_from("[core]a=b\n[core.a]a=c")?;
    assert_eq!(config.raw_values("core.a")?, vec![cow_str("b")]);
    assert_eq!(config.raw_values("core.a.a")?, vec![cow_str("c")]);
    Ok(())
}

#[test]
fn non_relevant_subsection_is_ignored() -> crate::Result {
    let config = File::try_from("[core]\na=b\na=c\n[core]a=d\n[core]g=g")?;
    assert_eq!(
        config.raw_values("core.a")?,
        vec![cow_str("b"), cow_str("c"), cow_str("d")]
    );
    Ok(())
}
