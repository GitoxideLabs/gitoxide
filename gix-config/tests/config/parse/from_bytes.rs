use super::*;

#[test]
fn fuzz() {
    assert!(
        Events::from_str("[]A=\\\\\r\\\n\n").is_err(),
        "empty sections are not allowed, and it won't crash either"
    );
    assert!(
        Events::from_str(include_str!(
            "../../fixtures/clusterfuzz-testcase-minimized-gix-config-parse-6431708583690240"
        ))
        .is_err(),
        "works without hanging - these 400kb take 10s in debug mode right now, but just as long in release mode. With nom all tests ran in below 1s in debug mode"
    );
}

#[test]
fn filters_receive_event_refs_for_content_access() -> gix_testtools::TestResult {
    fn reject_drop_values(event: EventRef<'_>) -> bool {
        !matches!(event, EventRef::Value(value) if value == b"drop".as_slice())
    }
    let events = Events::from_bytes(b"[core]\nkeep = keep\ndrop = drop\n", Some(reject_drop_values))?;

    assert!(
        !events
            .iter()
            .any(|event| matches!(event, EventRef::Value(value) if value == b"drop".as_slice())),
        "the filter can reject events by inspecting their value bytes"
    );
    Ok(())
}

#[test]
fn legacy_header_spelling_is_preserved() -> gix_testtools::TestResult {
    let input = r#"[Remote.Origin]
	url = location
"#;
    let events = Events::from_str(input)?;
    let expected_header = EventRef::SectionHeader {
        name: "Remote".into(),
        separator: Some(".".into()),
        subsection_name: Some("Origin".into()),
    };
    assert_eq!(
        events.iter().next(),
        Some(expected_header),
        "syntactic event views retain the original legacy spelling"
    );
    assert_eq!(
        events.sections().next().expect("one section").header(),
        expected_header,
        "section header views and the full event stream expose the same spelling"
    );
    let mut out = Vec::new();
    for event in events.iter() {
        event.write_to(&mut out)?;
    }
    assert_eq!(out, input.as_bytes(), "writing event views preserves legacy headers");
    assert_eq!(
        gix_config::format::normalize(input.as_bytes(), Default::default())?,
        input,
        "whitespace formatting leaves legacy header spelling untouched"
    );
    Ok(())
}

#[test]
#[rustfmt::skip]
fn complex() {
    let config = r#"[user]
        email = code@eddie.sh
        name = Foo Bar
[core]
        autocrlf = input
[push]
        default = simple
[commit]
        gpgsign = true
[gpg]
        program = gpg
[url "ssh://git@github.com/"]
        insteadOf = "github://"
[url "ssh://git@git.eddie.sh/edward/"]
        insteadOf = "gitea://"
[pull]
        ff = only
[init]
        defaultBranch = master"#;

    let events = Events::from_str(config).unwrap();
    assert_eq!(
        events.iter().collect::<Vec<_>>(),
        vec![
            section::header_event("user", None),
            newline(),

            whitespace("        "),
            name("email"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("code@eddie.sh"),
            newline(),

            whitespace("        "),
            name("name"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("Foo Bar"),
            newline(),

            section::header_event("core", None),
            newline(),

            whitespace("        "),
            name("autocrlf"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("input"),
            newline(),

            section::header_event("push", None),
            newline(),

            whitespace("        "),
            name("default"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("simple"),
            newline(),

            section::header_event("commit", None),
            newline(),

            whitespace("        "),
            name("gpgsign"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("true"),
            newline(),

            section::header_event("gpg", None),
            newline(),

            whitespace("        "),
            name("program"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("gpg"),
            newline(),

            section::header_event("url", "ssh://git@github.com/"),
            newline(),

            whitespace("        "),
            name("insteadOf"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("\"github://\""),
            newline(),

            section::header_event("url",  "ssh://git@git.eddie.sh/edward/"),
            newline(),

            whitespace("        "),
            name("insteadOf"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("\"gitea://\""),
            newline(),

            section::header_event("pull", None),
            newline(),

            whitespace("        "),
            name("ff"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("only"),
            newline(),

            section::header_event("init", None),
            newline(),

            whitespace("        "),
            name("defaultBranch"),
            whitespace(" "),
            separator(),
            whitespace(" "),
            value("master"),
        ]
    );
}

#[test]
fn skips_bom() {
    let bytes = b"
    [core]
        a = 1
";
    let bytes_with_gb18030_bom = "\u{feff}
    [core]
        a = 1
";

    fn render_events(events: &Events) -> Vec<u8> {
        let mut out = Vec::new();
        for event in events.iter() {
            event.write_to(&mut out).expect("in-memory writes cannot fail");
        }
        out
    }

    assert_eq!(
        Events::from_bytes(bytes, None).map(|events| render_events(&events)),
        Events::from_bytes(bytes_with_gb18030_bom.as_bytes(), None).map(|events| render_events(&events))
    );
    assert_eq!(
        Events::from_bytes(bytes, None).map(|events| render_events(&events)),
        Events::from_bytes(bytes_with_gb18030_bom.as_bytes(), None).map(|events| render_events(&events))
    );
}
