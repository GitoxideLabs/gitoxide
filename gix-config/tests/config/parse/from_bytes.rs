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

    assert_eq!(
        Events::from_str(config)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>(),
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

    assert_eq!(
        Events::from_bytes(bytes, None),
        Events::from_bytes(bytes_with_gb18030_bom.as_bytes(), None)
    );
    assert_eq!(
        Events::from_bytes_owned(bytes, None),
        Events::from_bytes_owned(bytes_with_gb18030_bom.as_bytes(), None)
    );
}
