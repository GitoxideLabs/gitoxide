use crate::spec::parse::{parse, try_parse};

#[test]
fn regex_parsing_ignores_ranges_as_opposed_to_git() {
    for spec in [":/a..b", ":/a...b"] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(
            rec.patterns,
            vec![(spec[2..].into(), false)],
            "git parses ranges but I think it's merely coincidental rather than intended, not doing so allows to use '.' more liberally"
        );
    }
}

#[test]
fn index_lookups_ignores_ranges_as_opposed_to_git() {
    for spec in [":a..b", ":a...b"] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(
            rec.index_lookups,
            vec![(spec[1..].into(), 0)],
            "git parses ranges but it's never useful as these specs only ever produce blob ids"
        );
    }
}

#[test]
fn various_forms_of_regex() {
    for (spec, (regex, negated)) in [
        (":/simple", ("simple", false)),
        (":/!-negated", ("negated", true)),
        (":/^from start", ("^from start", false)),
        (":/!!leading exclamation mark", ("!leading exclamation mark", false)),
        (":/with count{1}", ("with count{1}", false)),
        (
            ":/all-consuming makes navigation impossible^5~10",
            ("all-consuming makes navigation impossible^5~10", false),
        ),
    ] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.patterns, vec![(regex.into(), negated)]);
        assert_eq!(rec.calls, 1);
    }
}

#[test]
fn regex_do_not_get_any_backslash_processing() {
    for (spec, regex) in [(r#":/{"#, "{"), (r":/\{\}", r"\{\}"), (r":/\\\\\}", r"\\\\\}")] {
        let rec = parse(spec);

        assert_eq!(rec.patterns, vec![(regex.into(), false)]);
        assert_eq!(rec.calls, 1);
    }
}

#[test]
fn various_valid_index_lookups_by_path() {
    for spec in [
        ":path",
        ":dir/path",
        ":./relative-to.cwd",
        ":../relative-to-cwd-too",
        ":navigation/is/ignored~10^^^",
    ] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.index_lookups, vec![(spec[1..].into(), 0)]);
        assert_eq!(rec.peel_to, vec![], "peeling only works for anchors");
        assert_eq!(rec.calls, 1);
    }
}

#[test]
fn various_valid_index_lookups_by_path_and_stage() {
    for (spec, path, stage) in [
        (":0:path", "path", 0),
        (":1:dir/path", "dir/path", 1),
        (":2:dir/path@{part-of-path}", "dir/path@{part-of-path}", 2),
    ] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.index_lookups, vec![(path.into(), stage)]);
        assert_eq!(rec.peel_to, vec![], "peeling only works for anchors");
        assert_eq!(rec.calls, 1);
    }
}

#[test]
fn empty_top_level_regex_are_invalid() {
    let err = try_parse(":/").unwrap_err();
    assert!(
        err.message
            .contains("colon-slash must be followed by a regular expression"),
        "git also can't do it, finds nothing instead. It could be the youngest commit in theory, but isn't"
    );
}

#[test]
fn regex_with_empty_exclamation_mark_prefix_is_invalid() {
    let err = try_parse(r#":/!hello"#).unwrap_err();
    assert_eq!(err.input.as_ref().map(|i| i.as_ref()), Some(b"!hello".as_ref()));
    assert!(err.message.contains("need one character after /!"));
}

#[test]
fn needs_suffix() {
    let err = try_parse(":").unwrap_err();
    assert!(
        err.message
            .contains("colon must be followed by either slash and regex or path"),
        "git also can't do it, finds nothing instead. It could be the youngest commit in theory, but isn't"
    );
}

#[test]
fn invalid_index_stage_is_part_of_path() {
    for spec in [":4:file", ":5:file", ":01:file", ":10:file"] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.patterns, vec![]);
        assert_eq!(
            rec.index_lookups,
            vec![(spec[1..].into(), 0)],
            "these count as stage 0 lookups"
        );
        assert_eq!(rec.peel_to, vec![]);
        assert_eq!(rec.calls, 1);
    }
}
