use gix_config::parse::{
    Events,
    format::{self, Indentation, Newline, Options},
};

fn norm(input: &str) -> String {
    let out = format::normalize(input.as_bytes(), &Options::default()).expect("valid config");
    String::from_utf8(out.into()).expect("utf8")
}

/// Collect (section, name, value) triples from a config's event stream so two configs can be
/// compared for *meaning* rather than bytes.
fn semantic_triples(input: &str) -> Vec<(String, String, String)> {
    use gix_config::parse::Event;
    let events = Events::from_str(input).expect("valid").into_vec();
    let mut out = Vec::new();
    let mut section = String::new();
    let mut pending_name: Option<String> = None;
    let mut value = String::new();
    for ev in &events {
        match ev {
            Event::SectionHeader(h) => section = h.to_bstring().to_string(),
            Event::SectionValueName(_) => {
                if let Some(name) = pending_name.take() {
                    out.push((section.clone(), name, std::mem::take(&mut value)));
                }
                pending_name = Some(ev.to_bstr_lossy().to_string());
                value.clear();
            }
            Event::Value(_) | Event::ValueDone(_) | Event::ValueNotDone(_) => {
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

#[test]
fn default_policy_basic() {
    // 4-space indent collapses to the 2-space default; trailing whitespace and tight `=` are fixed.
    let input = "[core]\n    editor=vim   \n";
    assert_eq!(norm(input), "[core]\n  editor = vim\n");
}

#[test]
fn meaning_is_preserved() {
    for input in [
        "[core]\n  editor=vim\n",
        "[remote \"origin\"]\n\turl = https://example.com/x.git\n",
        "[a]\nx=1\ny = 2\n[b]\nz=3\n",
        "[user]\n\tname = A B   ; trailing comment\n",
    ] {
        assert_eq!(
            semantic_triples(&norm(input)),
            semantic_triples(input),
            "formatting must not change meaning for: {input:?}"
        );
    }
}

#[test]
fn line_continuation_value_is_untouched() {
    // The continued line's leading whitespace is part of the value and must survive verbatim.
    let input = "[alias]\nsave = \"!f() { \\\n    git status; \\\n}; f\"\n";
    let out = norm(input);
    assert_eq!(
        semantic_triples(&out),
        semantic_triples(input),
        "continuation value bytes must be preserved"
    );
}

#[test]
fn trailing_backslash_at_eof() {
    let input = "[core]\na=hello\\";
    // Must parse and round-trip without panicking or corrupting the continuation.
    let out = norm(input);
    assert_eq!(semantic_triples(&out), semantic_triples(input));
}

#[test]
fn implicit_boolean_key_keeps_no_separator() {
    let input = "[core]\n  autocrlf\n";
    assert_eq!(norm(input), "[core]\n  autocrlf\n");
}

#[test]
fn comments_are_preserved() {
    let input = "; top comment\n[core]\n# inner\n\teditor = vim ; inline\n";
    let out = norm(input);
    assert!(out.contains("; top comment"));
    assert!(out.contains("# inner"));
    assert!(out.contains("; inline"));
}

#[test]
fn quoted_subsection_and_value_verbatim() {
    let input = "[test \"sub \\\"x\\\"\"]\n\tpath = \"C:\\\\root\"\n";
    assert_eq!(semantic_triples(&norm(input)), semantic_triples(input));
}

#[test]
fn crlf_is_detected_and_normalized() {
    let input = "[core]\r\n  editor=vim\r\n";
    assert_eq!(norm(input), "[core]\r\n  editor = vim\r\n");
}

#[test]
fn blank_lines_left_alone_by_default() {
    let input = "[a]\nx = 1\n\n\n[b]\ny = 2\n";
    assert_eq!(norm(input), "[a]\n  x = 1\n\n\n[b]\n  y = 2\n");
}

#[test]
fn blank_lines_collapsed_when_requested() {
    let opts = Options {
        max_consecutive_blank_lines: Some(1),
        ..Options::default()
    };
    let out = format::normalize("[a]\nx = 1\n\n\n\n[b]\ny = 2\n".as_bytes(), &opts).unwrap();
    assert_eq!(String::from_utf8(out.into()).unwrap(), "[a]\n  x = 1\n\n[b]\n  y = 2\n");
}

#[test]
fn spaces_around_separator_can_be_disabled() {
    let opts = Options {
        spaces_around_separator: false,
        indentation: Indentation::None,
        ..Options::default()
    };
    let out = format::normalize("[core]\n  editor = vim\n".as_bytes(), &opts).unwrap();
    assert_eq!(String::from_utf8(out.into()).unwrap(), "[core]\neditor=vim\n");
}

#[test]
fn tab_indentation_option() {
    let opts = Options {
        indentation: Indentation::Tab,
        ..Options::default()
    };
    let out = format::normalize("[core]\n    editor=vim\n".as_bytes(), &opts).unwrap();
    assert_eq!(String::from_utf8(out.into()).unwrap(), "[core]\n\teditor = vim\n");
}

#[test]
fn no_indentation_option() {
    let opts = Options {
        indentation: Indentation::None,
        ..Options::default()
    };
    let out = format::normalize("[core]\n    editor=vim\n".as_bytes(), &opts).unwrap();
    assert_eq!(String::from_utf8(out.into()).unwrap(), "[core]\neditor = vim\n");
}

#[test]
fn force_lf_newline() {
    let opts = Options {
        newline: Newline::Lf,
        ..Options::default()
    };
    let out = format::normalize("[core]\r\n  editor = vim\r\n".as_bytes(), &opts).unwrap();
    assert_eq!(String::from_utf8(out.into()).unwrap(), "[core]\n  editor = vim\n");
}

#[test]
fn force_crlf_newline() {
    let opts = Options {
        newline: Newline::CrLf,
        ..Options::default()
    };
    let out = format::normalize("[core]\n  editor = vim\n".as_bytes(), &opts).unwrap();
    assert_eq!(String::from_utf8(out.into()).unwrap(), "[core]\r\n  editor = vim\r\n");
}

#[test]
fn idempotent() {
    let input = "[core]\n  editor=vim\n[remote \"o\"]\nurl = x\n";
    let once = norm(input);
    let twice = norm(&once);
    assert_eq!(once, twice);
}
