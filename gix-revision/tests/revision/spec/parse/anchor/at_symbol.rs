use gix_revision::spec;

use crate::spec::parse::{parse, try_parse};

#[test]
fn braces_must_be_closed() {
    for unclosed_spec in ["@{something", "@{", "@{..@"] {
        let err = try_parse(unclosed_spec).unwrap_err();
        assert_eq!(
            err.input.as_ref().map(|i| i.as_ref()),
            Some(unclosed_spec[1..].as_bytes())
        );
        assert!(err.message.contains("unclosed brace pair"));
    }
}

#[test]
#[cfg(target_pointer_width = "64")] // Only works this way on 64-bit systems.
fn fuzzed() {
    let rec = parse("@{-9223372036854775808}");
    assert_eq!(rec.nth_checked_out_branch, [Some(9223372036854775808), None]);
}

#[test]
fn reflog_by_entry_for_current_branch() {
    for (spec, expected_entry) in [("@{0}", 0), ("@{42}", 42), ("@{00100}", 100)] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(
            rec.prefix[0], None,
            "neither ref nor prefixes are set, straight to navigation"
        );
        assert_eq!(rec.current_branch_reflog_entry[0], Some(expected_entry.to_string()));
        assert_eq!(rec.calls, 1);
    }
}

#[test]
fn reflog_by_date_for_current_branch() {
    let rec = parse("@{42 +0030}");

    assert!(rec.kind.is_none());
    assert_eq!(rec.find_ref[0], None);
    assert_eq!(
        rec.prefix[0], None,
        "neither ref nor prefixes are set, straight to navigation"
    );
    assert_eq!(rec.current_branch_reflog_entry[0], Some("42 +0030".to_string()));
    assert_eq!(rec.calls, 1);
}

#[test]
fn reflog_by_unix_timestamp_for_current_branch() {
    let rec = parse("@{100000000}");

    assert!(rec.kind.is_none());
    assert_eq!(rec.find_ref[0], None);
    assert_eq!(
        rec.prefix[0], None,
        "neither ref nor prefixes are set, straight to navigation"
    );
    assert_eq!(
        rec.current_branch_reflog_entry[0],
        Some("100000000 +0000".to_string()),
        "This number is the first to count as date"
    );
    assert_eq!(rec.calls, 1);

    let rec = parse("@{99999999}");
    assert_eq!(
        rec.current_branch_reflog_entry[0],
        Some("99999999".to_string()),
        "one less is an offset though"
    );
}

#[test]
fn reflog_by_date_with_date_parse_failure() {
    let err = try_parse("@{foo}").unwrap_err();
    assert_eq!(err.input.as_ref().map(|i| i.as_ref()), Some(b"foo".as_ref()));
    assert!(err.message.contains("could not parse time for reflog lookup"));
}

#[test]
fn reflog_by_date_for_hash_is_invalid() {
    for (spec, full_name) in [
        ("1234@{42 +0030}", "1234"),
        ("abcd-dirty@{42 +0030}", "abcd-dirty"),
        ("v1.2.3-0-g1234@{42 +0030}", "v1.2.3-0-g1234"),
    ] {
        let err = try_parse(spec).unwrap_err();
        assert_eq!(
            err.input.as_ref().map(|i| i.as_ref()),
            Some(full_name.as_bytes())
        );
        assert!(err.message.contains("reflog entries require a ref name"));
    }
}

#[test]
fn reflog_by_date_for_given_ref_name() {
    for (spec, expected_ref) in [
        ("main@{42 +0030}", "main"),
        ("refs/heads/other@{42 +0030}", "refs/heads/other"),
        ("refs/worktree/feature/a@{42 +0030}", "refs/worktree/feature/a"),
    ] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.get_ref(0), expected_ref);
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.current_branch_reflog_entry[0], Some("42 +0030".to_string()));
        assert_eq!(rec.calls, 2, "first the ref, then the reflog entry");
    }
}

#[test]
fn reflog_by_entry_for_given_ref_name() {
    for (spec, expected_ref, expected_entry) in [
        ("main@{0}", "main", 0),
        ("refs/heads/other@{42}", "refs/heads/other", 42),
        ("refs/worktree/feature/a@{00100}", "refs/worktree/feature/a", 100),
    ] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.get_ref(0), expected_ref);
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.current_branch_reflog_entry[0], Some(expected_entry.to_string()));
        assert_eq!(rec.calls, 2, "first the ref, then the reflog entry");
    }
}

#[test]
fn reflog_by_entry_for_hash_is_invalid() {
    for (spec, full_name) in [
        ("1234@{0}", "1234"),
        ("abcd-dirty@{1}", "abcd-dirty"),
        ("v1.2.3-0-g1234@{2}", "v1.2.3-0-g1234"),
    ] {
        let err = try_parse(spec).unwrap_err();
        assert_eq!(
            err.input.as_ref().map(|i| i.as_ref()),
            Some(full_name.as_bytes())
        );
        assert!(err.message.contains("reflog entries require a ref name"));
    }
}

#[test]
fn sibling_branch_current_branch() {
    for (spec, kind_name) in [("@{u}", "Upstream"), ("@{push}", "Push"), ("@{UPSTREAM}", "Upstream")] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], None, "neither ref nor prefix are explicitly set");
        assert_eq!(rec.sibling_branch[0].as_deref(), Some(kind_name));
        assert_eq!(rec.calls, 1);
    }
}

#[test]
fn sibling_branch_for_branch_name() {
    for (spec, ref_name, kind_name) in [
        ("r1@{U}", "r1", "Upstream"),
        ("refs/heads/main@{Push}", "refs/heads/main", "Push"),
        ("refs/worktree/private@{UpStreaM}", "refs/worktree/private", "Upstream"),
    ] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.get_ref(0), ref_name);
        assert_eq!(rec.prefix[0], None, "neither ref nor prefix are explicitly set");
        assert_eq!(
            rec.sibling_branch[0].as_deref(),
            Some(kind_name),
            "note that we do not know if something is a branch or not and make the call even if it would not be allowed. Configuration decides"
        );
        assert_eq!(rec.calls, 2);
    }
}

#[test]
fn sibling_branch_for_hash_is_invalid() {
    for (spec, full_name) in [
        ("1234@{u}", "1234"),
        ("abcd-dirty@{push}", "abcd-dirty"),
        ("v1.2.3-0-g1234@{upstream}", "v1.2.3-0-g1234"),
    ] {
        let err = try_parse(spec).unwrap_err();
        assert_eq!(
            err.input.as_ref().map(|i| i.as_ref()),
            Some(full_name.as_bytes())
        );
        assert!(err.message.contains("sibling branches"));
    }
}

#[test]
fn nth_checked_out_branch_for_refname_is_invalid() {
    let err = try_parse("r1@{-1}").unwrap_err();
    assert!(
        err.input.as_ref().map(|i| i.as_ref()) == Some(b"-1".as_ref())
            && err.message.contains("reference name must be followed by positive numbers"),
        "its undefined how to handle negative numbers and specified ref names"
    );
}

#[test]
fn nth_checked_out_branch() {
    for (spec, expected_branch) in [("@{-1}", 1), ("@{-42}", 42), ("@{-00100}", 100)] {
        let rec = parse(spec);

        assert!(rec.kind.is_none());
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(
            rec.prefix[0], None,
            "neither ref nor prefixes are set, straight to navigation"
        );
        assert_eq!(rec.nth_checked_out_branch[0], Some(expected_branch));
        assert_eq!(rec.calls, 1);
    }
}

#[test]
fn numbers_within_braces_cannot_be_negative_zero() {
    let err = try_parse("@{-0}").unwrap_err();
    assert!(
        err.input.as_ref().map(|i| i.as_ref()) == Some(b"-0".as_ref())
            && err.message.contains("negative zero is invalid"),
        "negative zero is not accepted, even though it could easily be defaulted to 0 which is a valid value"
    );
}

#[test]
fn numbers_within_braces_can_be_positive_zero() {
    assert_eq!(
        parse("@{+0}"),
        parse("@{0}"),
        "+ prefixes are allowed though and the same as without it"
    );
}
