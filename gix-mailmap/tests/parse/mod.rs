use gix_mailmap::{parse, Entry};
use gix_testtools::fixture_bytes;

#[test]
fn line_numbers_are_counted_correctly_in_errors() {
    let input = fixture_bytes("invalid.txt");
    let mut actual = gix_mailmap::parse(&input).collect::<Vec<_>>().into_iter();
    assert_eq!(actual.len(), 2);

    let err = actual.next().expect("two items left").unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("3:"), "expected line 3, got: {err_str}");

    let err = actual.next().expect("one item left").unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("Line 5"), "expected line 5, got: {err_str}");
}

#[test]
fn a_typical_mailmap() {
    let input = fixture_bytes("typical.txt");
    let actual = gix_mailmap::parse(&input).map(Result::unwrap).collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            Entry::change_name_by_email("Joe R. Developer", "joe@example.com"),
            Entry::change_name_and_email_by_name_and_email(
                "Joe R. Developer",
                "joe@example.com",
                "Joe",
                "bugs@example.com"
            ),
            Entry::change_name_and_email_by_email("Jane Doe", "jane@example.com", "jane@laptop.(none)"),
            Entry::change_name_and_email_by_email("Jane Doe", "jane@example.com", "jane@desktop.(none)"),
            Entry::change_name_and_email_by_name_and_email("Jane Doe", "jane@example.com", "Jane", "bugs@example.com"),
            Entry::change_email_by_name_and_email("jane@example.com", "Jane", "Jane@ipad.(none)"),
        ]
    );
}

#[test]
fn empty_lines_and_comments_are_ignored() {
    assert!(gix_mailmap::parse(b"# comment").next().is_none());
    assert!(gix_mailmap::parse(b"\n\r\n\t\t   \n").next().is_none());
    assert_eq!(
        line(" # this is a name <email>"),
        Entry::change_name_by_email("# this is a name", "email"),
        "whitespace before hashes counts as name though"
    );
}

#[test]
fn windows_and_unix_line_endings_are_supported() {
    let actual = gix_mailmap::parse(b"a <a@example.com>\n<b-new><b-old>\r\nc <c@example.com>")
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            Entry::change_name_by_email("a", "a@example.com"),
            Entry::change_email_by_email("b-new", "b-old"),
            Entry::change_name_by_email("c", "c@example.com")
        ]
    );
}

#[test]
fn valid_entries() {
    assert_eq!(
        line(" \t proper name   <commit-email>"),
        Entry::change_name_by_email("proper name", "commit-email")
    );
    assert_eq!(
        line("  <proper email>   <commit-email>  \t "),
        Entry::change_email_by_email("proper email", "commit-email")
    );
    assert_eq!(
        line("  proper name \t  <proper email> \t <commit-email>"),
        Entry::change_name_and_email_by_email("proper name", "proper email", "commit-email")
    );
    assert_eq!(
        line("<proper-email> commit name <commit-email>"),
        Entry::change_email_by_name_and_email("proper-email", "commit name", "commit-email")
    );
}

#[test]
fn error_if_there_is_just_a_name() {
    let err = try_line("just a name").unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("Line 1"), "expected line 1, got: {err_str}");
}

#[test]
fn error_if_there_is_just_an_email() {
    let err = try_line("<email>").unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("1:"), "expected line 1, got: {err_str}");

    let err = try_line("   \t  <email>").unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("1:"), "expected line 1, got: {err_str}");
}

#[test]
fn error_if_email_is_empty() {
    let err = try_line("hello <").unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("1:"), "expected line 1, got: {err_str}");

    let err = try_line("hello < \t").unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("1:"), "expected line 1, got: {err_str}");

    let err = try_line("hello < \t\r >").unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("1:"), "expected line 1, got: {err_str}");
}

fn line(input: &str) -> Entry<'_> {
    try_line(input).unwrap()
}

fn try_line(input: &str) -> Result<Entry<'_>, parse::Error> {
    let mut lines = gix_mailmap::parse(input.as_bytes());
    let res = lines.next().expect("single line");
    assert!(lines.next().is_none(), "only one line provided");
    res
}
