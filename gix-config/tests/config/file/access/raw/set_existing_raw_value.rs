fn file(input: &str) -> gix_config::File {
    input.parse().unwrap()
}

fn assert_set_value(value: &str) {
    let mut file = file("[a]k=b\n[a]\nk=c\nk=d");
    file.set_existing_raw_value_by("a", None, "k", value).unwrap();
    assert_eq!(file.raw_value("a.k").unwrap(), value);

    let file: gix_config::File = file.to_string().parse().unwrap();
    assert_eq!(
        file.raw_value("a.k").unwrap(),
        value,
        "{:?} didn't have expected value {:?}",
        file.to_string(),
        value
    );
}

#[test]
fn single_line() {
    assert_set_value("hello world");
}

#[test]
fn global_property_uses_empty_section_name() -> crate::Result {
    let mut file = file("a=b\n[core]\na=c");
    let err = file.set_existing_raw_value_by("", None, "a", "d").unwrap_err();
    assert_eq!(
        err.to_string(),
        "The requested section does not exist",
        "cannot set global values"
    );
    Ok(())
}

#[test]
fn starts_with_whitespace() {
    assert_set_value("\ta");
    assert_set_value(" a");
}

#[test]
fn ends_with_whitespace() {
    assert_set_value("a\t");
    assert_set_value("a ");
}

#[test]
fn quotes_and_backslashes() {
    assert_set_value(r#""hello"\"there"\\\b\x"#);
}

#[test]
fn multi_line() {
    assert_set_value("a\nb   \n\t   c");
}

#[test]
fn comment_included() {
    assert_set_value(";hello ");
    assert_set_value(" # hello");
}

#[test]
fn non_existing_values_cannot_be_set() {
    let mut file = gix_config::File::default();
    assert!(
        file.set_existing_raw_value_by("new", None, "key", "value").is_err(),
        "new values are not ever created"
    );
}
