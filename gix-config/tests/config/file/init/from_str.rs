use crate::Result;

#[test]
fn empty_yields_default_file() -> Result {
    let a: gix_config::File = "".parse()?;
    assert_eq!(a, gix_config::File::default());
    assert_eq!(a.to_string(), "");
    Ok(())
}

#[test]
fn whitespace_without_section_contains_front_matter() -> Result {
    let input = "    \t";
    let a: gix_config::File = input.parse()?;
    assert_eq!(a.to_string(), input);
    Ok(())
}
