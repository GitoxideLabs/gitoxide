use gix_config::{
    format::{self, Indent, Newline, Options},
    parse::Events,
};

#[test]
fn default_policy_is_git_compatible() {
    let input = concat!(
        "\r\n",                                 // first leading blank line
        " \r\n",                                // second leading line contains whitespace
        "   ; top comment\r\n",                 // root comment has excess indentation
        "  [core] ; section comment\r\n",       // section header has excess indentation
        "    # standalone\r\n",                 // section comment with excess indentation
        "    editor=vim   # value comment\r\n", // tight separator, trailing spaces, inline value comment
        " \r\n",                                // blank line contains spaces
        "\t\r\n",                               // blank line contains a tab
        "\t[user]\r\n",                         // another section has excess indentation
        " name=A",                              // missing trailing newline
    );
    assert_eq!(
        norm(input),
        concat!(
            "\r\n",                               // the default preserves the first leading blank line
            "\r\n",                               // the default preserves the second but removes its whitespace
            "; top comment\r\n",                  // root-comment indentation was removed
            "[core] ; section comment\r\n",       // section indentation was removed; inline spacing was normalized
            "\t# standalone\r\n",                 // excess indentation became the default tab
            "\teditor = vim # value comment\r\n", // indentation and separator spacing were normalized
            "\r\n",                               // spaces were removed from the first blank line
            "\r\n",                               // the tab was removed from the second blank line
            "[user]\r\n",                         // excess section indentation was removed
            "\tname = A\r\n",                     // indentation, separator, and final newline were added
        ),
        "all default formatting policies must be represented while respecting EOL"
    );
}

#[test]
fn meaning_is_preserved() {
    let simple = concat!(
        "[core]\n",       //
        "  editor=vim\n", //
    );
    let remote = concat!(
        "[remote \"origin\"]\n",               // quoted subsections must retain their meaning
        "\turl = https://example.com/x.git\n", // existing tab indentation must not affect meaning
    );
    let multiple_sections = concat!(
        "[a]\n",   //
        "x=1\n",   //
        "y = 2\n", //
        "[b]\n",   //
        "z=3\n",   //
    );
    let inline_comment = concat!(
        "[user]\n",                            //
        "\tname = A B   ; trailing comment\n", // the comment must not become value text
    );

    for input in [simple, remote, multiple_sections, inline_comment] {
        assert_eq!(
            semantic_triples(&norm(input)),
            semantic_triples(input),
            "formatting must not change meaning for: {input}"
        );
    }
}

#[test]
fn line_continuation_value_is_untouched() {
    let input = concat!(
        "[alias]\n",            //
        r#"save = "!f() { \"#,  // continuation starts
        "\n",                   //
        r#"    git status; \"#, // significant leading whitespace
        "\n",                   //
        r#"}; f""#,             // continuation ends
        "\n",                   //
    );
    let out = norm(input);
    assert_eq!(
        semantic_triples(&out),
        semantic_triples(input),
        "a continued line's significant leading whitespace must survive verbatim"
    );
}

#[test]
fn trailing_backslash_at_eof() {
    let input = concat!(
        "[core]\n",    //
        r#"a=hello\"#, // continuation at EOF
    );
    let out = norm(input);
    assert_eq!(
        semantic_triples(&out),
        semantic_triples(input),
        "a trailing backslash at EOF must parse and round-trip without corrupting the continuation"
    );
}

#[test]
fn implicit_boolean_key_keeps_no_separator() {
    let input = concat!(
        "[core]\n",     //
        "  autocrlf\n", // an implicit boolean has no separator
    );
    insta::assert_snapshot!(show_control_characters(&norm(input)), "implicit booleans have no separator", @r"
    [core]\n
    \tautocrlf\n
    ");
}

#[test]
fn comments_are_preserved() {
    let input = concat!(
        "; top comment\n",           // exercises root-level comments
        "[core]\n",                  //
        "# inner\n",                 // exercises section-local comments
        "\teditor = vim ; inline\n", // exercises inline value comments
    );
    insta::assert_snapshot!(show_control_characters(&norm(input)), "top-level, standalone, and inline comments are preserved", @r"
    ; top comment\n
    [core]\n
    \t# inner\n
    \teditor = vim ; inline\n
    ");
}

#[test]
fn quoted_subsection_and_value_verbatim() {
    let input = concat!(
        r#"[test "sub \"x\""]"#, // escaped subsection bytes must survive
        "\n",                    //
        "\t",                    // keep indentation explicit outside the raw string
        r#"path = "C:\\root""#,  // escaped value bytes must survive
        "\n",                    //
    );
    assert_eq!(
        semantic_triples(&norm(input)),
        semantic_triples(input),
        "quoted subsections and values must retain their meaning"
    );
}

#[test]
fn crlf_is_detected_and_normalized_without_accumulating_carriage_returns() {
    let input = concat!(
        "; top comment\r\n",   // establishes CRLF and exercises an attached carriage return
        "[core]\n",            // inconsistent LF after a section must be normalized
        "# inner\r\n",         // exercises an attached carriage return on a section comment
        "editor=x ; inline\n", // inconsistent LF after an inline comment must be normalized
    );
    let opts = Options {
        newline: Newline::Detect,
        ..Options::default()
    };
    let once = normalize(input, opts);
    insta::assert_snapshot!(show_control_characters(&once), "detected CRLF is used throughout and comments retain one carriage return", @r"
    ; top comment\r\n
    [core]\r\n
    \t# inner\r\n
    \teditor = x ; inline\r\n
    ");
    assert_eq!(
        normalize(&once, opts),
        once,
        "formatting CRLF comments must be idempotent"
    );
}

#[test]
fn blank_lines_left_alone_by_default() {
    let input = concat!(
        "[a]\n",   //
        "x = 1\n", //
        "\n",      // this blank-line run must be retained
        "\n",      //
        "[b]\n",   //
        "y = 2\n", //
    );
    insta::assert_snapshot!(show_control_characters(&norm(input)), "default formatting preserves blank-line runs", @r"
    [a]\n
    \tx = 1\n
    \n
    \n
    [b]\n
    \ty = 2\n
    ");
}

#[test]
fn blank_lines_collapsed_when_requested() {
    let input = concat!(
        "[a]\n",   //
        "x = 1\n", //
        "\n",      // this oversized blank-line run must be capped
        "\n",      //
        "\n",      //
        "[b]\n",   //
        "y = 2\n", //
    );
    let opts = Options {
        max_consecutive_blank_lines: Some(1),
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, opts)), "blank-line runs are capped at one", @r"
    [a]\n
    \tx = 1\n
    \n
    [b]\n
    \ty = 2\n
    ");
}

#[test]
fn whitespace_only_blank_lines_are_counted_in_the_same_run() {
    let input = concat!(
        "[a]\n",   //
        "x = 1\n", //
        " \n",     // spaces alone must count as a blank line
        "\t\n",    // tabs alone must join the same blank-line run
        "[b]\n",   //
        "y = 2\n", //
    );
    for max_blank in [0, 1] {
        let opts = Options {
            max_consecutive_blank_lines: Some(max_blank),
            ..Options::default()
        };
        let actual = normalize(input, opts);
        match max_blank {
            0 => {
                insta::assert_snapshot!(show_control_characters(&actual), "a zero cap removes the entire whitespace-only run", @r"
                [a]\n
                \tx = 1\n
                [b]\n
                \ty = 2\n
                ");
            }
            1 => {
                insta::assert_snapshot!(show_control_characters(&actual), "a one-line cap treats whitespace-only lines as one run", @r"
                [a]\n
                \tx = 1\n
                \n
                [b]\n
                \ty = 2\n
                ");
            }
            _ => unreachable!("the test only exercises zero and one"),
        }
    }
}

#[test]
fn leading_blank_line_cap_has_boundaries_and_is_independent() {
    let input = concat!(
        " \n",      // leading spaces count as the first blank line
        "\t\n",     // a leading tab joins the same run
        "\n",       // a plain blank line completes the leading run
        "; root\n", // the first content ends the leading run
        "\n",       // first interior blank line
        " \n",      // whitespace-only lines join the interior run
        "\t\n",     //
        "[core]\n", //
        "x=1\n",    //
    );

    for max_leading_blank_lines in [None, Some(0), Some(1), Some(2)] {
        let opts = Options {
            max_leading_blank_lines,
            max_consecutive_blank_lines: Some(1),
            ..Options::default()
        };
        let actual = show_control_characters(&normalize(input, opts));
        match max_leading_blank_lines {
            None => insta::assert_snapshot!(
                actual,
                "None preserves the entire leading run while the interior cap remains active",
                @r"
            \n
            \n
            \n
            ; root\n
            \n
            [core]\n
            \tx = 1\n
            "
            ),
            Some(0) => insta::assert_snapshot!(
                actual,
                "a zero cap removes the entire leading run while retaining one interior blank",
                @r"
            ; root\n
            \n
            [core]\n
            \tx = 1\n
            "
            ),
            Some(1) => insta::assert_snapshot!(
                actual,
                "a cap of one retains exactly one leading and one interior blank",
                @r"
            \n
            ; root\n
            \n
            [core]\n
            \tx = 1\n
            "
            ),
            Some(2) => insta::assert_snapshot!(
                actual,
                "a cap of two retains two leading blanks without changing the interior cap",
                @r"
            \n
            \n
            ; root\n
            \n
            [core]\n
            \tx = 1\n
            "
            ),
            Some(_) => unreachable!("the test only exercises boundary values up to two"),
        }
    }
}

#[test]
fn spaces_around_separator_can_be_disabled() {
    let input = concat!(
        "[core]\n",         //
        "  editor = vim\n", // existing separator spaces must be removable
    );
    let opts = Options {
        spaces_around_separator: false,
        key_value_indent: Indent::Spaces(0),
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, opts)), "separator and indentation spaces are disabled", @r"
    [core]\n
    editor=vim\n
    ");
}

#[test]
fn tabs_indentation_option() {
    let input = concat!(
        "[core]\n",         //
        "    editor=vim\n", // existing spaces must be replaced with two tabs
    );
    let opts = Options {
        key_value_indent: Indent::Tabs(2),
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, opts)), "two tabs indent key-value lines", @r"
    [core]\n
    \t\teditor = vim\n
    ");
}

#[test]
fn root_and_section_comment_indentation_are_independent() {
    let input = concat!(
        "  ; top\n",             // existing root indentation can be replaced or preserved
        "[core]\n",              //
        "# standalone\n",        // section comments must share key-value indentation
        "editor=vim ; inline\n", // inline comments retain one separating space
    );
    let opts = Options {
        root_comment_indent: Some(Indent::Tabs(2)),
        key_value_indent: Indent::Spaces(4),
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, opts)), "root and section comments use independent configured indentation", @r"
    \t\t; top\n
    [core]\n
        # standalone\n
        editor = vim ; inline\n
    ");

    let opts = Options {
        root_comment_indent: None,
        key_value_indent: Indent::Spaces(4),
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, opts)), "None preserves existing root-comment indentation", @r"
      ; top\n
    [core]\n
        # standalone\n
        editor = vim ; inline\n
    ");
}

#[test]
fn zero_tabs_or_spaces_mean_no_indentation() {
    let input = concat!(
        "[core]\n",         //
        "    editor=vim\n", // zero-width indentation must remove these spaces
    );
    let opts = Options {
        key_value_indent: Indent::Tabs(0),
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, opts)), "zero tabs emit no indentation", @r"
    [core]\n
    editor = vim\n
    ");

    let opts = Options {
        key_value_indent: Indent::Spaces(0),
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, opts)), "zero spaces emit no indentation", @r"
    [core]\n
    editor = vim\n
    ");
}

#[test]
fn force_lf_newline() {
    let input = concat!(
        "[core]\r\n",         // input uses CRLF
        "  editor = vim\r\n", //
    );
    let opts = Options {
        newline: Newline::Lf,
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, opts)), "LF replaces all input newlines", @r"
    [core]\n
    \teditor = vim\n
    ");
}

#[test]
fn force_crlf_newline() {
    let input = concat!(
        "[core]\n",         // input uses LF
        "  editor = vim\n", //
    );
    let opts = Options {
        newline: Newline::CrLf,
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, opts)), "CRLF replaces all input newlines", @r"
    [core]\r\n
    \teditor = vim\r\n
    ");
}

#[test]
fn forced_newlines_apply_inside_continued_values() {
    let input = concat!(
        "[alias]\r\n",  // input uses CRLF
        r#"save=one\"#, // continued CRLF value starts
        "\r\n",         //
        "  two\r\n",    // continued CRLF value ends
    );
    let lf = Options {
        newline: Newline::Lf,
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, lf)), "LF is forced inside continued values", @r"
    [alias]\n
    \tsave = one\\n
      two\n
    ");

    let input = concat!(
        "[alias]\n",    // input uses LF
        r#"save=one\"#, // continued LF value starts
        "\n",           //
        "  two\n",      // continued LF value ends
    );
    let crlf = Options {
        newline: Newline::CrLf,
        ..Options::default()
    };
    insta::assert_snapshot!(show_control_characters(&normalize(input, crlf)), "CRLF is forced inside continued values", @r"
    [alias]\r\n
    \tsave = one\\r\n
      two\r\n
    ");
}

#[test]
fn idempotent() {
    let input = concat!(
        "[core]\n",         //
        "  editor=vim\n",   //
        "[remote \"o\"]\n", //
        "url = x\n",        //
    );
    let once = norm(input);
    let twice = norm(&once);
    assert_eq!(once, twice, "formatting an already formatted config must not change it");
}

fn norm(input: &str) -> String {
    normalize(input, Options::default())
}

fn normalize(input: &str, options: Options) -> String {
    let out = format::normalize(input.as_bytes(), options).expect("valid config");
    String::from_utf8(out.into()).expect("UTF-8 input produces UTF-8 output")
}

fn show_control_characters(input: &str) -> String {
    input.replace('\t', "\\t").replace('\r', "\\r").replace('\n', "\\n\n")
}

/// Collect (section, name, value) triples from a config's event stream so two configs can be
/// compared for *meaning* rather than bytes.
fn semantic_triples(input: &str) -> Vec<(String, String, String)> {
    use gix_config::parse::EventRef;
    let events = Events::from_str(input).expect("valid");
    let mut out = Vec::new();
    let mut section = String::new();
    let mut pending_name: Option<String> = None;
    let mut value = String::new();
    for ev in events.iter() {
        match ev {
            EventRef::SectionHeader { name, .. } => section = name.to_string(),
            EventRef::SectionValueName(name) => {
                if let Some(name) = pending_name.take() {
                    out.push((section.clone(), name, std::mem::take(&mut value)));
                }
                pending_name = Some(name.to_string());
                value.clear();
            }
            EventRef::Value(_) | EventRef::ValueDone(_) | EventRef::ValueNotDone(_) => {
                value.push_str(&ev.to_bstr_lossy().to_string());
            }
            _ => {}
        }
    }
    if let Some(name) = pending_name.take() {
        out.push((section, name, value));
    }
    out
}
