use gix_revision::spec;

use crate::spec::parse::{try_parse, try_parse_opts, Options};

#[test]
fn cannot_declare_ranges_multiple_times() {
    for invalid_spec in ["^HEAD..", "^HEAD..."] {
        let err = try_parse(invalid_spec).unwrap_err();
        assert!(err.message.contains("cannot set spec kind more than once"));
    }
}

#[test]
fn delegate_can_refuse_spec_kinds() {
    let err = try_parse_opts(
        "^HEAD",
        Options {
            reject_kind: true,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(
        err.message.contains("delegate did not indicate success"),
        "Delegates can refuse spec kind changes to abort parsing early in case they want single-specs only"
    );
}

mod include_parents {
    use gix_revision::spec;

    use crate::spec::parse::{kind::prefix, parse, try_parse, Call};

    #[test]
    fn trailing_caret_at_symbol() {
        let rec = parse("HEAD^@");
        assert_eq!(rec.kind.unwrap(), spec::Kind::IncludeReachableFromParents);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Kind]);
        assert!(rec.done);

        let rec = parse("abcd^@");
        assert_eq!(rec.kind.unwrap(), spec::Kind::IncludeReachableFromParents);
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.order, [Call::DisambiguatePrefix, Call::Kind]);
        assert!(rec.done);

        let rec = parse("r1^@");
        assert_eq!(rec.kind.unwrap(), spec::Kind::IncludeReachableFromParents);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Kind]);
        assert!(rec.done);
    }

    #[test]
    fn trailing_caret_exclamation_mark_must_end_the_input() {
        let err = try_parse("r1^@~1").unwrap_err();
        assert!(err.message.contains("unconsumed input"));
    }
}

mod exclude_parents {
    use gix_revision::spec;

    use crate::spec::parse::{kind::prefix, parse, try_parse, Call};

    #[test]
    fn freestanding() {
        let rec = parse("^!");
        assert_eq!(
            rec.kind,
            Some(gix_revision::spec::Kind::ExcludeReachable),
            "the delegate has to be able to deal with this"
        );
    }

    #[test]
    fn trailing_caret_exclamation_mark() {
        let rec = parse("HEAD^!");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ExcludeReachableFromParents);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Kind]);
        assert!(rec.done);

        let rec = parse("abcd^!");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ExcludeReachableFromParents);
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.order, [Call::DisambiguatePrefix, Call::Kind]);
        assert!(rec.done);

        let rec = parse("r1^!");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ExcludeReachableFromParents);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Kind]);
        assert!(rec.done);
    }

    #[test]
    fn trailing_caret_exclamation_mark_must_end_the_input() {
        let err = try_parse("r1^!~1").unwrap_err();
        assert!(err.message.contains("unconsumed input"));
    }
}

mod exclusive {
    use gix_revision::spec;

    use crate::spec::parse::{kind::prefix, parse};

    #[test]
    fn freestanding() {
        let rec = parse("^");
        assert_eq!(
            rec.kind,
            Some(gix_revision::spec::Kind::ExcludeReachable),
            "the delegate has to be able to deal with this"
        );
    }

    #[test]
    fn leading_caret() {
        let rec = parse("^HEAD");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ExcludeReachable);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.calls, 2);

        let rec = parse("^abcd");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ExcludeReachable);
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.calls, 2);

        let rec = parse("^r1");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ExcludeReachable);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.calls, 2);

        let rec = parse("^hello-0-gabcd-dirty");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ExcludeReachable);
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.calls, 2);
    }
}

mod range {
    use gix_revision::{spec, spec::parse::delegate::Traversal};

    use crate::spec::parse::{kind::prefix, parse, try_parse, Call};

    #[test]
    fn minus_with_n_omitted() {
        let rec = parse("r1^-");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.traversal, [Traversal::NthParent(1)], "default is 1");
        assert_eq!(rec.get_ref(1), "r1");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Traverse, Call::Kind, Call::FindRef]);
        assert!(rec.done);

        let rec = parse("@^-");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.traversal, [Traversal::NthParent(1)], "default is 1");
        assert_eq!(rec.get_ref(1), "HEAD");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Traverse, Call::Kind, Call::FindRef]);
        assert!(rec.done);

        let rec = parse("abcd^-");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.traversal, [Traversal::NthParent(1)], "default is 1");
        assert_eq!(rec.prefix[1], prefix("abcd").into());
        assert_eq!(
            rec.order,
            [
                Call::DisambiguatePrefix,
                Call::Traverse,
                Call::Kind,
                Call::DisambiguatePrefix
            ]
        );
        assert!(rec.done);
    }

    #[test]
    fn minus_with_n() {
        let rec = parse("r1^-42");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.traversal, [Traversal::NthParent(42)]);
        assert_eq!(rec.get_ref(1), "r1");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Traverse, Call::Kind, Call::FindRef]);
        assert!(rec.done);

        let rec = parse("@^-42");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.traversal, [Traversal::NthParent(42)]);
        assert_eq!(rec.get_ref(1), "HEAD");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Traverse, Call::Kind, Call::FindRef]);
        assert!(rec.done);

        let rec = parse("abcd^-42");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.traversal, [Traversal::NthParent(42)]);
        assert_eq!(rec.prefix[1], prefix("abcd").into());
        assert_eq!(
            rec.order,
            [
                Call::DisambiguatePrefix,
                Call::Traverse,
                Call::Kind,
                Call::DisambiguatePrefix
            ]
        );
        assert!(rec.done);
    }

    #[test]
    fn minus_with_n_omitted_has_to_end_there() {
        let err = try_parse("r1^-^").unwrap_err();
        assert!(err.message.contains("unconsumed input"));
    }

    #[test]
    fn minus_with_n_has_to_end_there() {
        let err = try_parse("r1^-42^").unwrap_err();
        assert!(err.message.contains("unconsumed input"));
    }

    #[test]
    fn minus_with_n_has_to_end_there_and_handle_range_suffix() {
        let err = try_parse("r1^-42..").unwrap_err();
        assert!(err.message.contains("unconsumed input"));
    }

    #[test]
    fn minus_with_n_omitted_has_to_end_there_and_handle_range_suffix() {
        let err = try_parse("r1^-..").unwrap_err();
        assert!(err.message.contains("unconsumed input"));
    }

    #[test]
    fn freestanding_dot_dot() {
        let rec = parse("..");
        assert_eq!(
            rec.kind,
            Some(gix_revision::spec::Kind::RangeBetween),
            "the delegate has to be able to deal with this"
        );
    }

    #[test]
    fn trailing_dot_dot() {
        let rec = parse("r1..");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.get_ref(1), "HEAD");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Kind, Call::FindRef]);
    }

    #[test]
    fn leading_dot_dot() {
        let rec = parse("..r2");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.get_ref(1), "r2");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.order, [Call::FindRef, Call::Kind, Call::FindRef]);
    }

    #[test]
    fn middle_dot_dot() {
        let rec = parse("@..r2");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.get_ref(1), "r2");
        assert_eq!(rec.calls, 3);

        let rec = parse("r1..r2");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.get_ref(1), "r2");
        assert_eq!(rec.calls, 3);

        let rec = parse("abcd..1234");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.prefix[1], prefix("1234").into());
        assert_eq!(rec.calls, 3);

        let rec = parse("r1..abcd");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.calls, 3);

        let rec = parse("abcd-dirty..v1.2-42-g1234");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.prefix[1], prefix("1234").into());
        assert_eq!(rec.calls, 3);

        let rec = parse("v1.2-42-g1234..abcd-dirty");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], prefix("1234").into());
        assert_eq!(rec.prefix[1], prefix("abcd").into());
        assert_eq!(rec.calls, 3);

        let rec = parse("v1.2.4@{1}~~^10..r1@{2}~10^2");
        assert_eq!(rec.kind.unwrap(), spec::Kind::RangeBetween);
        assert_eq!(rec.get_ref(0), "v1.2.4");
        assert_eq!(rec.get_ref(1), "r1");
        assert_eq!(&rec.prefix, &[None, None]);
        assert_eq!(
            rec.traversal,
            [
                Traversal::NthAncestor(1),
                Traversal::NthAncestor(1),
                Traversal::NthParent(10),
                Traversal::NthAncestor(10),
                Traversal::NthParent(2)
            ]
        );
        assert_eq!(rec.calls, 10);
        assert!(rec.done);
    }
}

mod mergebase {
    use gix_revision::{spec, spec::parse::delegate::Traversal};

    use crate::spec::parse::{kind::prefix, parse};

    #[test]
    fn freestanding_dot_dot_dot() {
        let rec = parse("...");
        assert_eq!(
            rec.kind,
            Some(gix_revision::spec::Kind::ReachableToMergeBase),
            "the delegate has to be able to deal with this"
        );
    }

    #[test]
    fn trailing_dot_dot_dot() {
        let rec = parse("HEAD...");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ReachableToMergeBase);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.calls, 3);
    }

    #[test]
    fn leading_dot_dot_dot() {
        let rec = parse("...r2");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ReachableToMergeBase);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.get_ref(1), "r2");
        assert_eq!(rec.prefix[0], None);
        assert_eq!(rec.calls, 3);
    }

    #[test]
    fn middle_dot_dot_dot() {
        let rec = parse("HEAD...@");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ReachableToMergeBase);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.get_ref(1), "HEAD");
        assert_eq!(rec.calls, 3);

        let rec = parse("@...HEAD");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ReachableToMergeBase);
        assert_eq!(rec.get_ref(0), "HEAD");
        assert_eq!(rec.get_ref(1), "HEAD");
        assert_eq!(rec.calls, 3);

        let rec = parse("r1...abcd");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ReachableToMergeBase);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.prefix[0], prefix("abcd").into());
        assert_eq!(rec.calls, 3);

        let rec = parse("v1.2.3-beta.1-42-g1234-dirty...abcd-dirty");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ReachableToMergeBase);
        assert_eq!(rec.find_ref[0], None);
        assert_eq!(rec.prefix[0], prefix("1234").into());
        assert_eq!(rec.prefix[1], prefix("abcd").into());
        assert_eq!(rec.calls, 3);

        let rec = parse("r1@{1}~~^10...@{2}~10^2");
        assert_eq!(rec.kind.unwrap(), spec::Kind::ReachableToMergeBase);
        assert_eq!(rec.get_ref(0), "r1");
        assert_eq!(rec.find_ref[1], None, "HEAD is implied");
        assert_eq!(&rec.prefix, &[None, None]);
        assert_eq!(
            rec.traversal,
            [
                Traversal::NthAncestor(1),
                Traversal::NthAncestor(1),
                Traversal::NthParent(10),
                Traversal::NthAncestor(10),
                Traversal::NthParent(2)
            ]
        );
        assert_eq!(rec.calls, 9);
        assert!(rec.done);
    }
}

fn prefix(hex: &str) -> gix_hash::Prefix {
    gix_hash::Prefix::from_hex(hex).unwrap()
}
