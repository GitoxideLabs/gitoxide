use gix_mailmap::Entry;
use gix_testtools::fixture_bytes;

#[test]
fn line_numbers_are_counted_correctly_in_errors() {
    let input = fixture_bytes("invalid.txt");
    let mut actual = gix_mailmap::parse(&input).collect::<Vec<_>>().into_iter();
    assert_eq!(actual.len(), 2);

    let err = actual.next().expect("two items left").unwrap_err();
    insta::assert_debug_snapshot!(err, "malformed mailmap lines retain their original input and one-based line number", @r#"3: Missing closing bracket '>' in email, "input"="<missing closing brace""#);

    let err = actual.next().expect("one item left").unwrap_err();
    insta::assert_debug_snapshot!(err, "malformed mailmap lines retain their original input and one-based line number", @r#"Line 5 does not contain an email, "input"="just a name""#);
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
fn trailing_content_after_a_complete_mapping_is_ignored() {
    assert_eq!(
        line("Canonical Name <canonical@example.com> <mapped@example.com> # secondary <ignored@example.com>"),
        Entry::change_name_and_email_by_email("Canonical Name", "canonical@example.com", "mapped@example.com"),
        "only the first two names and emails are used to build the mapping"
    );
    assert_eq!(
        line("Canonical Name <canonical@example.com> <mapped@example.com> <ignored@example.com>"),
        Entry::change_name_and_email_by_email("Canonical Name", "canonical@example.com", "mapped@example.com"),
        "a third email does not invalidate the line"
    );
    assert_eq!(
        line("Canonical Name <mapped@example.com> Ignored Trailing Name"),
        Entry::change_name_by_email("Canonical Name", "mapped@example.com"),
        "a trailing name without an email is not a mapping source and is dropped"
    );
}

#[test]
fn malformed_second_emails_are_ignored() {
    assert_eq!(
        line("Canonical Name <mapped@example.com> Ignored <broken"),
        Entry::change_name_by_email("Canonical Name", "mapped@example.com"),
        "a malformed second identity is ignored"
    );
}

#[test]
fn the_second_email_may_be_empty() {
    assert_eq!(
        line("Canonical Name <canonical@example.com> <>"),
        Entry::change_name_and_email_by_email("Canonical Name", "canonical@example.com", ""),
    );
}

#[test]
fn error_if_there_is_just_a_name() {
    let err = try_line("just a name").unwrap_err();
    insta::assert_debug_snapshot!(err, "a name without an email reports the incomplete identity on line one", @r#"Line 1 does not contain an email, "input"="just a name""#);
}

#[test]
fn error_if_there_is_just_an_email() {
    let err = try_line("<email>").unwrap_err();
    insta::assert_debug_snapshot!(err, "an email needs a name or email to map to", @r#"1: Emails without a name or email to map to are invalid, "input"="<email>""#);

    let err = try_line("   \t  <email>").unwrap_err();
    insta::assert_debug_snapshot!(err, "an email needs a name or email to map to", @r#"1: Emails without a name or email to map to are invalid, "input"="<email>""#);
}

#[test]
fn error_if_email_is_empty() {
    let err = try_line("hello <").unwrap_err();
    insta::assert_debug_snapshot!(err, "an incomplete or empty email identifies the malformed input on line one", @r#"1: Missing closing bracket '>' in email, "input"="hello <""#);

    let err = try_line("hello < \t").unwrap_err();
    insta::assert_debug_snapshot!(err, "an incomplete or empty email identifies the malformed input on line one", @r#"1: Missing closing bracket '>' in email, "input"="hello <""#);

    let err = try_line("hello < \t\r >").unwrap_err();
    insta::assert_debug_snapshot!(err, "an incomplete or empty email identifies the malformed input on line one", @r#"1: Email must not be empty, "input"="hello < \t\r >""#);
}

fn line(input: &str) -> Entry<'_> {
    try_line(input).unwrap()
}

fn try_line(input: &str) -> gix_error::Result<Entry<'_>> {
    let mut lines = gix_mailmap::parse(input.as_bytes());
    let res = lines.next().expect("single line");
    assert!(lines.next().is_none(), "only one line provided");
    res
}
