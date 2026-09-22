use gix_revision::spec::parse::delegate::Traversal;

use crate::spec::parse::{PeelToOwned as PeelTo, parse, try_parse};

#[test]
fn single_is_first_parent() {
    let rec = parse("@^");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(rec.prefix[0], None);
    assert_eq!(rec.traversal[0], Traversal::NthParent(1));
    assert_eq!(rec.calls, 2);
}

#[test]
fn multiple_calls_stack() {
    let rec = parse("@^^^10^0^{tag}^020");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(rec.prefix[0], None);
    assert_eq!(
        rec.traversal,
        vec![
            Traversal::NthParent(1),
            Traversal::NthParent(1),
            Traversal::NthParent(10),
            Traversal::NthParent(20),
        ]
    );
    assert_eq!(
        rec.peel_to,
        vec![
            PeelTo::ObjectKind(gix_object::Kind::Commit),
            PeelTo::ObjectKind(gix_object::Kind::Tag)
        ]
    );
    assert_eq!(rec.calls, 7);
}

#[test]
fn followed_by_zero_is_peeling_to_commit() {
    let rec = parse("@^0");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(rec.prefix[0], None);
    assert_eq!(rec.traversal.len(), 0, "traversals by parent are never zero");
    assert_eq!(
        rec.peel_to,
        vec![PeelTo::ObjectKind(gix_object::Kind::Commit)],
        "instead 0 serves as shortcut"
    );
    assert_eq!(rec.calls, 2);
}

#[test]
fn explicitly_positive_numbers_are_invalid() {
    let err = try_parse("@^+1").unwrap_err().into_inner();
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::from(b"+1".as_ref()))
    );
    insta::assert_debug_snapshot!(err, "explicitly positive numbers are invalid", @r#"
    Message {
        message: "explicitly positive numbers are invalid here",
        class: Validation,
        values: {"input": Bytes("+1")},
    }
    "#);
}

#[test]
fn explicit_parent_number() {
    for (spec, expected) in [
        ("HEAD^1", 1),
        ("abcd^10", 10),
        ("v1.3.4^123", 123),
        ("v1.3.4-12-g1234^1000", 1000),
    ] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert!(rec.find_ref[0].as_ref().is_some() || rec.prefix[0].is_some());
        assert_eq!(rec.traversal, vec![Traversal::NthParent(expected)]);
        assert_eq!(rec.calls, 2);
    }
}

#[test]
fn peel_to_object_type() {
    for (spec, expected) in [
        ("HEAD^{commit}", PeelTo::ObjectKind(gix_object::Kind::Commit)),
        ("abcd^{tree}", PeelTo::ObjectKind(gix_object::Kind::Tree)),
        ("v1.3.4^{blob}", PeelTo::ObjectKind(gix_object::Kind::Blob)),
        ("v1.3.4-12-g1234^{tag}", PeelTo::ObjectKind(gix_object::Kind::Tag)),
        ("v1.3.4-12-g1234^{object}", PeelTo::ExistingObject),
    ] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert!(rec.find_ref[0].as_ref().is_some() || rec.prefix[0].is_some());
        assert_eq!(rec.peel_to, vec![expected]);
        assert_eq!(rec.calls, 2);
    }
}

#[test]
fn regex_backslash_rules() {
    for (spec, regex, msg) in [
        (
            r#"@^{/with count{1}}"#,
            r#"with count{1}"#,
            "matching inner parens do not need escaping",
        ),
        (
            r"@^{/with count\{1\}}",
            r#"with count{1}"#,
            "escaped parens are entirely ignored",
        ),
        (r"@^{/1\}}", r#"1}"#, "unmatched closing parens need to be escaped"),
        (r"@^{/2\{}", r#"2{"#, "unmatched opening parens need to be escaped"),
        (
            r"@^{/3{\{}}",
            r#"3{{}"#,
            "unmatched nested opening parens need to be escaped",
        ),
        (
            r"@^{/4{\}}}",
            r#"4{}}"#,
            "unmatched nested closing parens need to be escaped",
        ),
        (r"@^{/a\b\c}", r"a\b\c", "single backslashes do not need to be escaped"),
        (
            r"@^{/a\b\c\\}",
            r"a\b\c\",
            "single backslashes do not need to be escaped, trailing",
        ),
        (
            r"@^{/a\\b\\c\\}",
            r"a\b\c\",
            "backslashes can be escaped nonetheless, trailing",
        ),
        (
            r"@^{/5\\{}}",
            r"5\{}",
            "backslashes in front of parens must be escaped or they would unbalance the brace pair",
        ),
    ] {
        let rec = try_parse(spec).expect(msg);

        assert!(rec.kind.is_none());
        assert!(rec.find_ref[0].as_ref().is_some() || rec.prefix[0].is_some());
        assert_eq!(rec.patterns, vec![(regex.into(), false)], "{msg}");
        assert_eq!(rec.calls, 2);
    }
}

#[test]
fn regex_with_revision_starting_point_and_negation() {
    for (spec, (regex, negated)) in [
        ("HEAD^{/simple}", ("simple", false)),
        ("abcd^{/!-negated}", ("negated", true)),
        ("v1.3.4^{/^from start}", ("^from start", false)),
        (
            "v1.3.4-12-g1234^{/!!leading exclamation mark}",
            ("!leading exclamation mark", false),
        ),
        ("v1.3.4-12-g1234^{/with count{1}}", ("with count{1}", false)),
    ] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert!(rec.find_ref[0].as_ref().is_some() || rec.prefix[0].is_some());
        assert_eq!(rec.patterns, vec![(regex.into(), negated)]);
        assert_eq!(rec.calls, 2);
    }
}

#[test]
fn empty_braces_deref_a_tag() {
    let rec = parse("v1.2^{}");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "v1.2");
    assert_eq!(rec.peel_to, vec![PeelTo::RecursiveTagObject]);
    assert_eq!(rec.calls, 2);
}

#[test]
fn invalid_object_type() {
    let err = try_parse("@^{invalid}").unwrap_err().into_inner();
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::from(b"invalid".as_ref()))
    );
    insta::assert_debug_snapshot!(err, "invalid object type", @r#"
    Message {
        message: "cannot peel to unknown target",
        class: Validation,
        values: {"input": Bytes("invalid")},
    }
    "#);

    let err = try_parse("@^{Commit}").unwrap_err().into_inner();
    insta::assert_debug_snapshot!(err, "these types are case sensitive", @r#"
    Message {
        message: "cannot peel to unknown target",
        class: Validation,
        values: {"input": Bytes("Commit")},
    }
    "#);
    assert!(
        err.values.get("input") == Some(&gix_error::MetadataValue::from(b"Commit".as_ref())),
        "these types are case sensitive"
    );
}

#[test]
fn invalid_caret_without_previous_refname() {
    let mut message_diagnostics = Vec::new();
    let rec = parse(r"^^");
    assert_eq!(rec.calls, 2);
    assert_eq!(rec.kind, Some(gix_revision::spec::Kind::ExcludeReachable));
    assert_eq!(
        rec.traversal,
        [Traversal::NthParent(1)],
        "This can trip off an implementation as it's actually invalid, but looks valid"
    );

    for revspec in ["^^^HEAD", "^^HEAD"] {
        let err = try_parse(revspec).unwrap_err().into_inner();
        assert_eq!(
            err.values.get("input"),
            Some(&gix_error::MetadataValue::from(b"HEAD".as_ref()))
        );
        message_diagnostics.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
    }
    insta::assert_debug_snapshot!(message_diagnostics, "invalid caret without previous refname", @r#"
    [
        Message {
            message: "unconsumed input",
            class: Validation,
            values: {"input": Bytes("HEAD")},
        },
        Message {
            message: "unconsumed input",
            class: Validation,
            values: {"input": Bytes("HEAD")},
        },
    ]
    "#);
}

#[test]
fn incomplete_escaped_braces_in_regex_are_invalid() {
    let err = try_parse(r"@^{/a\{1}}").unwrap_err().into_inner();
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::from(b"}".as_ref()))
    );
    insta::assert_debug_snapshot!(err, "incomplete escaped braces in regex are invalid", @r#"
    Message {
        message: "unconsumed input",
        class: Validation,
        values: {"input": Bytes("}")},
    }
    "#);

    let err = try_parse(r"@^{/a{1\}}").unwrap_err().into_inner();
    insta::assert_debug_snapshot!(err, "incomplete escaped braces in regex are invalid", @r#"
    Message {
        message: "unclosed brace pair",
        class: Validation,
        values: {"input": Bytes("{/a{1\\}}")},
    }
    "#);
    assert!(
        err.values.get("input") == Some(&gix_error::MetadataValue::from(br"{/a{1\}}".as_ref())),
        "incomplete escaped braces in regex are invalid"
    );
}

#[test]
fn regex_with_empty_exclamation_mark_prefix_is_invalid() {
    let err = try_parse(r#"@^{/!hello}"#).unwrap_err().into_inner();
    assert_eq!(
        err.values.get("input"),
        Some(&gix_error::MetadataValue::from(b"!hello".as_ref()))
    );
    insta::assert_debug_snapshot!(err, "regex with empty exclamation mark prefix is invalid", @r#"
    Message {
        message: "need one character after /!, typically -",
        class: Validation,
        values: {"input": Bytes("!hello")},
    }
    "#);
}

#[test]
fn bad_escapes_can_cause_brace_mismatch() {
    let err = try_parse(r"@^{\}").unwrap_err().into_inner();
    insta::assert_debug_snapshot!(err, "bad escapes can cause brace mismatch", @r#"
    Message {
        message: "unclosed brace pair",
        class: Validation,
        values: {"input": Bytes("{\\}")},
    }
    "#);
    assert!(
        err.values.get("input") == Some(&gix_error::MetadataValue::from(br"{\}".as_ref())),
        "bad escapes can cause brace mismatch"
    );

    let err = try_parse(r"@^{{\}}").unwrap_err().into_inner();
    // The raw string r"{{\}}" contains actual backslashes, so the input would be r"{{\}}"
    insta::assert_debug_snapshot!(err, "bad escapes can cause brace mismatch", @r#"
    Message {
        message: "unclosed brace pair",
        class: Validation,
        values: {"input": Bytes("{{\\}}")},
    }
    "#);
    assert!(
        err.values.get("input") == Some(&gix_error::MetadataValue::from(br"{{\}}".as_ref())),
        "bad escapes can cause brace mismatch"
    );
}

#[test]
fn empty_regex_is_passed_to_the_delegate() {
    let rec = parse("@^{/}");

    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), "HEAD");
    assert_eq!(
        rec.patterns,
        vec![("".into(), false)],
        "empty regexes (will) match everything, so Git finds the anchor itself, peeled to a commit"
    );
    assert_eq!(rec.calls, 2);

    let rec = parse("@^{/!-}");

    assert_eq!(
        rec.patterns,
        vec![("".into(), true)],
        "negated empty regexes (will) match nothing"
    );
    assert_eq!(rec.calls, 2);
}
