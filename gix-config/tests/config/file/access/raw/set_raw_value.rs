fn file(input: &str) -> gix_config::File {
    input.parse().unwrap()
}

fn assert_set_value(value: &str) {
    let mut file = file("[a]\nk=c\nk=d");
    file.set_raw_value_by("a", None, "k", value).unwrap();
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
fn non_existing_values_cannot_be_set() -> crate::Result {
    let mut file = gix_config::File::default();
    file.set_raw_value_by("new", None, "key", "value")?;
    file.set_raw_value_by("new", Some("subsection".into()), "key", "subsection-value")?;

    assert_eq!(file.string("new.key").expect("present"), "value");
    assert_eq!(
        file.string_by("new", Some("subsection".into()), "key")
            .expect("present"),
        "subsection-value"
    );
    Ok(())
}

#[test]
fn accepts_short_lived_keys() -> crate::Result {
    let mut file = gix_config::File::default();
    let key = String::from("new.key");

    file.set_raw_value(key.as_str(), "value")?;

    assert_eq!(file.string("new.key").expect("present"), "value");
    Ok(())
}
