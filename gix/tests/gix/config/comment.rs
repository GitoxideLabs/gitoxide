use gix::{bstr::ByteSlice, config::comment_prefix};
use gix_testtools::TestResult;

#[test]
fn prefixes_follow_configuration_order() -> TestResult {
    // Git reference: environment.c at 6de20f6092dcf9bdb1c8efe03db4b70c82b423dd.
    for (config, expected) in [
        ("", "#"),
        ("[core]\ncommentChar = \";\"\n", ";"),
        ("[core]\ncommentChar = //\n", "//"),
        ("[core]\ncommentChar = //\ncommentString = >\n", ">"),
        ("[core]\ncommentString = >\ncommentChar = //\n", "//"),
        ("[core]\ncommentChar = >\n[CORE]\ncommentString = //\n", "//"),
        ("[core]\ncommentChar = auto\n", "#"),
        ("[core]\ncommentString = AUTO\n", "#"),
        ("[core \"subsection\"]\ncommentChar = >\n", "#"),
    ] {
        let config = gix_config::File::try_from(config)?;
        assert_eq!(comment_prefix(&config)?, expected, "the last applicable alias wins");
    }
    let config = gix_config::File::try_from(b"[core]\ncommentChar = \xff\n".as_bstr())?;
    assert_eq!(
        comment_prefix(&config)?.as_slice(),
        b"\xff",
        "prefixes preserve non-UTF-8 bytes"
    );
    Ok(())
}

#[test]
fn invalid_prefixes_have_configuration_diagnostics() -> TestResult {
    for (config, key, value, diagnostic) in [
        (
            "[core]\ncommentChar =\n",
            "core.commentchar",
            b"".as_slice(),
            "must have at least one character",
        ),
        (
            "[core]\ncommentChar = \"\\n\"\n",
            "core.commentchar",
            b"\n",
            "cannot contain newline",
        ),
        (
            "[core]\ncommentString =\n",
            "core.commentstring",
            b"",
            "must have at least one character",
        ),
        (
            "[core]\ncommentChar =\ncommentChar = //\n",
            "core.commentchar",
            b"",
            "must have at least one character",
        ),
    ] {
        let config = gix_config::File::try_from(config)?;
        let error = comment_prefix(&config).expect_err("invalid prefixes must be rejected");
        assert!(
            error.to_string().contains(diagnostic),
            "the error describes the invalid setting"
        );
        super::key::assert_config_error(&error, key, Some(value.as_bstr().into()), None);
    }
    let config = gix_config::File::try_from("[core]\ncommentChar\n")?;
    assert!(
        comment_prefix(&config).is_err(),
        "an implicit boolean is not a comment prefix"
    );
    Ok(())
}
