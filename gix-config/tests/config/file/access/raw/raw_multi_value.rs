use gix_config::File;

use crate::file::bstring;

#[test]
fn single_value_is_identical_to_single_value_query() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    assert_eq!(vec![config.raw_value("core.a")?], config.raw_values("core.a")?);
    Ok(())
}

#[test]
fn multi_value_in_section() -> crate::Result {
    let config = File::try_from("[core]\na=b\na=c")?;
    assert_eq!(config.raw_values("core.a")?, vec![bstring("b"), bstring("c")]);
    Ok(())
}

#[test]
fn multi_value_across_sections() -> crate::Result {
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
fn values_with_sections_identify_each_values_section_in_file_order() -> crate::Result {
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
fn values_with_sections_filter_returns_values_from_accepted_sections() -> crate::Result {
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
fn section_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    let err = config.raw_values("foo.a").unwrap_err();
    assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
    assert_eq!(err.to_string(), "The requested section does not exist");
    Ok(())
}

#[test]
fn subsection_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    let err = config.raw_values("core.a.a").unwrap_err();
    assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
    assert_eq!(err.to_string(), "The requested subsection does not exist");
    Ok(())
}

#[test]
fn key_not_found() -> crate::Result {
    let config = File::try_from("[core]\na=b\nc=d")?;
    let err = config.raw_values("core.aaaaaa").unwrap_err();
    assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
    assert_eq!(err.to_string(), "The key does not exist in the requested section");
    Ok(())
}

#[test]
fn subsection_must_be_respected() -> crate::Result {
    let config = File::try_from("[core]a=b\n[core.a]a=c")?;
    assert_eq!(config.raw_values("core.a")?, vec![bstring("b")]);
    assert_eq!(config.raw_values("core.a.a")?, vec![bstring("c")]);
    Ok(())
}

#[test]
fn non_relevant_subsection_is_ignored() -> crate::Result {
    let config = File::try_from("[core]\na=b\na=c\n[core]a=d\n[core]g=g")?;
    assert_eq!(
        config.raw_values("core.a")?,
        vec![bstring("b"), bstring("c"), bstring("d")]
    );
    Ok(())
}
