use super::repo;
use crate::{
    revision::spec::from_bytes::{
        parse_spec, parse_spec_better_than_baseline, parse_spec_no_baseline, parse_spec_no_baseline_opts,
        parse_spec_opts, rev_parse,
    },
    util::hex_to_id,
};
use gix::{
    prelude::{ObjectIdExt, RevSpecExt},
    revision::{
        spec::parse::{Options, RefsHint},
        Spec,
    },
};

#[test]
fn prefix() {
    {
        let repo = repo("blob.prefix").unwrap();
        assert_eq!(
            parse_spec("dead", &repo).unwrap_err().probable_cause().to_string(),
            "Short id dead is ambiguous. Candidates are:\n\tdead7b2 blob\n\tdead9d3 blob"
        );
        assert_eq!(
            parse_spec("beef", &repo).unwrap_err().probable_cause().to_string(),
            "Short id beef is ambiguous. Candidates are:\n\tbeef2b0 blob\n\tbeefc9b blob"
        );
    }

    {
        let repo = repo("blob.bad").unwrap();
        assert_eq!(
            parse_spec("bad0", &repo).unwrap_err().probable_cause().to_string(),
            "Short id bad0 is ambiguous. Candidates are:\n\tbad0853 lookup error: An error occurred while obtaining an object from the loose object store\n\tbad0bd4 lookup error: An error occurred while obtaining an object from the loose object store",
            "git provides a much worse hint messages and fails to provide information for both bad objects"
        );
    }
}

#[test]
fn fully_failed_disambiguation_still_yields_an_ambiguity_error() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    let err = parse_spec("0000000000^{tag}", &repo).unwrap_err();

    insta::assert_debug_snapshot!(err, @r#"
    delegate.peel_until(ObjectKind(Tag)) failed: {tag}
    |
    └─ Short id 0000000000 is ambiguous. Candidates are:
    	0000000000e commit 2005-04-07 "a2onsxbvj"
    	0000000000c tree
    	0000000000b blob
    |
    └─ Last encountered object 0000000000c was tree while trying to peel to tag
    "#);

    insta::assert_snapshot!(format!("{err:#}"), @r#"
    ParseError { message: "delegate.peel_until(ObjectKind(Tag)) failed", input: Some("{tag}") }
    |
    └─ Message("Short id 0000000000 is ambiguous. Candidates are:\n\t0000000000e commit 2005-04-07 \"a2onsxbvj\"\n\t0000000000c tree\n\t0000000000b blob")
        |
        └─ NotFound { oid: Prefix { bytes: Sha1(0000000000c00000000000000000000000000000), hex_len: 11 }, actual: Tree, expected: Tag }
    "#);
    use std::error::Error;
    assert_eq!(
        err.source().expect("ambiguity error").to_string(),
        "Short id 0000000000 is ambiguous. Candidates are:\n\t0000000000e commit 2005-04-07 \"a2onsxbvj\"\n\t0000000000c tree\n\t0000000000b blob",
        "without special treatment, one would see a bunch of failed transformations with the impression that the first of them is the root cause, which isn't correct."
    );
}

#[test]
fn ranges_are_auto_disambiguated_by_committish() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    let id = hex_to_id("0000000000e4f9fbd19cf1e932319e5ad0d1d00b");
    let expected = gix_revision::Spec::Range { from: id, to: id }.attach(&repo);

    for spec in ["000000000..000000000", "..000000000", "000000000.."] {
        assert_eq!(
            parse_spec(spec, &repo).unwrap(),
            expected,
            "as ranges need a commit, this is assumed when disambiguating"
        );
    }

    let expected = gix_revision::Spec::Merge { theirs: id, ours: id }.attach(&repo);
    for spec in ["000000000...000000000", "...000000000", "000000000..."] {
        assert_eq!(parse_spec(spec, &repo).unwrap(), expected);
    }
}

#[test]
fn blob_and_tree_can_be_disambiguated_by_type() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    assert_eq!(
        parse_spec("0000000000", &repo).unwrap_err().probable_cause().to_string(),
        "Short id 0000000000 is ambiguous. Candidates are:\n\t0000000000e commit 2005-04-07 \"a2onsxbvj\"\n\t0000000000c tree\n\t0000000000b blob",
        "in theory one could disambiguate with 0000000000^{{tree}} (which works in git) or 0000000000^{{blob}} which doesn't work for some reason."
    );

    assert_eq!(
        parse_spec("0000000000cdc^{tree}", &repo).unwrap(),
        Spec::from_id(hex_to_id("0000000000cdcf04beb2fab69e65622616294984").attach(&repo)),
        "this is unambiguous anyway, but also asserts for tree which is naturally the case"
    );

    assert_eq!(
        parse_spec_better_than_baseline("0000000000^{tree}", &repo).unwrap(),
        Spec::from_id(hex_to_id("0000000000cdcf04beb2fab69e65622616294984").attach(&repo)),
        "the commit refers to the tree which also starts with this prefix, so ultimately the result is unambiguous. Git can't do that yet."
    );

    assert_eq!(
        parse_spec("0000000000^{commit}", &repo).unwrap(),
        Spec::from_id(hex_to_id("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo)),
        "disambiguation with committish"
    );

    assert_eq!(
        parse_spec("0000000000e", &repo).unwrap(),
        Spec::from_id(hex_to_id("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo)),
        "no disambiguation needed here"
    );
}

#[test]
fn trees_can_be_disambiguated_by_blob_access() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    let actual = parse_spec_better_than_baseline("0000000000:a0blgqsjc", &repo).unwrap();
    assert_eq!(
        actual,
        Spec::from_id(hex_to_id("0000000000b36b6aa7ea4b75318ed078f55505c3").attach(&repo)),
        "we can disambiguate by providing a path, but git cannot"
    );
    assert_eq!(
        actual.path_and_mode().expect("set"),
        ("a0blgqsjc".into(), gix_object::tree::EntryKind::Blob.into())
    );
}

#[test]
fn commits_can_be_disambiguated_with_commit_specific_transformations() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    for spec in ["0000000000^0", "0000000000^{commit}"] {
        assert_eq!(
            parse_spec(spec, &repo).unwrap(),
            Spec::from_id(hex_to_id("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo))
        );
    }
}

#[test]
fn tags_can_be_disambiguated_with_commit_specific_transformations() {
    let repo = repo("ambiguous_commits").unwrap();
    assert_eq!(
        parse_spec_better_than_baseline("0000000000^{tag}", &repo).unwrap(),
        Spec::from_id(hex_to_id("0000000000f8f5507ab27a0d7bd3c75c0f64ffe0").attach(&repo)),
        "disambiguation is possible by type, and git can't do that for some reason"
    );
}

#[test]
fn duplicates_are_deduplicated_across_all_odb_types() {
    let repo = repo("duplicate_ambiguous_objects").unwrap();
    assert_eq!(
        parse_spec_no_baseline("0000000000", &repo).unwrap_err().probable_cause().to_string(),
        "Short id 0000000000 is ambiguous. Candidates are:\n\t0000000000f8 tag \"v1.0.0\"\n\t000000000004 commit 2005-04-07 \"czy8f73t\"\n\t00000000006 commit 2005-04-07 \"ad2uee\"\n\t00000000008 commit 2005-04-07 \"ioiley5o\"\n\t0000000000e commit 2005-04-07 \"a2onsxbvj\"\n\t000000000002 tree\n\t00000000005 tree\n\t00000000009 tree\n\t0000000000c tree\n\t0000000000fd tree\n\t00000000001 blob\n\t00000000003 blob\n\t0000000000a blob\n\t0000000000b blob\n\t0000000000f2 blob",
        "One day we want to see 16 objects here, and not 32 just because they exist in the loose and the packed odb"
    );
}

fn opts_ref_hint(hint: RefsHint) -> Options {
    Options {
        refs_hint: hint,
        object_kind_hint: None,
    }
}

#[test]
fn ambiguous_40hex_refs_are_ignored_and_we_prefer_the_object_of_the_same_name() {
    let repo = repo("ambiguous_refs").unwrap();
    let spec = "0000000000e4f9fbd19cf1e932319e5ad0d1d00b";
    assert_eq!(
        parse_spec(spec, &repo).unwrap(),
        Spec::from_id(hex_to_id(spec).attach(&repo)),
        "git shows an advisory here and ignores the ref, which makes it easy to just ignore it too. We are unable to show anything though, maybe traces?"
    );

    assert_eq!(
        parse_spec_opts(spec, &repo, opts_ref_hint(RefsHint::PreferObject)).unwrap(),
        Spec::from_id(hex_to_id(spec).attach(&repo)),
        "preferring objects yields the same result here"
    );

    assert_eq!(
        parse_spec_no_baseline_opts(spec, &repo, opts_ref_hint(RefsHint::PreferRef)).unwrap(),
        Spec::from_id(hex_to_id("cc60d25ccfee90e4a4105e73df36059db383d5ce").attach(&repo)),
        "we can prefer refs in any case, too"
    );

    assert_eq!(
        parse_spec_no_baseline_opts(
            spec,
            &repo,
            opts_ref_hint(RefsHint::Fail)
        )
            .unwrap_err().probable_cause()
            .to_string(),
        "The short hash 0000000000e4f9fbd19cf1e932319e5ad0d1d00b matched both the reference refs/heads/0000000000e4f9fbd19cf1e932319e5ad0d1d00b and at least one object"
    );
}

#[test]
fn ambiguous_short_refs_are_dereferenced() {
    let repo = repo("ambiguous_refs").unwrap();
    let spec = "0000000000e";
    assert_eq!(
        parse_spec(spec, &repo).unwrap(),
        Spec::from_id(hex_to_id("cc60d25ccfee90e4a4105e73df36059db383d5ce").attach(&repo)),
        "git shows a warning here and we show nothing but have dials to control how to handle these cases"
    );

    assert_eq!(
        parse_spec_opts(spec, &repo, opts_ref_hint(RefsHint::PreferRef)).unwrap(),
        Spec::from_id(hex_to_id("cc60d25ccfee90e4a4105e73df36059db383d5ce").attach(&repo)),
        "this does the same, but independently of the length of the ref"
    );

    assert_eq!(
        parse_spec_no_baseline_opts(spec, &repo, opts_ref_hint(RefsHint::PreferObject)).unwrap(),
        Spec::from_id(hex_to_id("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo)),
        "we can always prefer objects, too"
    );

    assert_eq!(
        parse_spec_no_baseline_opts(spec, &repo, opts_ref_hint(RefsHint::Fail))
            .unwrap_err()
            .probable_cause()
            .to_string(),
        "The short hash 0000000000e matched both the reference refs/heads/0000000000e and at least one object",
        "users who don't want this ambiguity, could fail like this."
    );
}

#[test]
fn repository_local_disambiguation_hints_disambiguate() {
    let r = repo("ambiguous_objects_disambiguation_config_committish").unwrap();
    assert_eq!(
        rev_parse("0000000000f", &r).unwrap(),
        Spec::from_id(hex_to_id("0000000000f8f5507ab27a0d7bd3c75c0f64ffe0").attach(&r)),
        "we read the 'core.disambiguate' value and apply it to auto-disambiguate"
    );
    let err = rev_parse("0000000000", &r).unwrap_err();
    insta::assert_debug_snapshot!(err, @r#"
    Short id 0000000000 is ambiguous. Candidates are:
    	0000000000f8 tag "v1.0.0"
    	000000000004 commit 2005-04-07 "czy8f73t"
    	00000000006 commit 2005-04-07 "ad2uee"
    	00000000008 commit 2005-04-07 "ioiley5o"
    	0000000000e commit 2005-04-07 "a2onsxbvj"
    "#);
    assert_eq!(
        err.to_string(),
        "Short id 0000000000 is ambiguous. Candidates are:\n\t0000000000f8 tag \"v1.0.0\"\n\t000000000004 commit 2005-04-07 \"czy8f73t\"\n\t00000000006 commit 2005-04-07 \"ad2uee\"\n\t00000000008 commit 2005-04-07 \"ioiley5o\"\n\t0000000000e commit 2005-04-07 \"a2onsxbvj\""
    );

    let r = repo("ambiguous_objects_disambiguation_config_treeish").unwrap();
    let err = rev_parse("0000000000f", &r).unwrap_err();
    insta::assert_debug_snapshot!(err, @r#"
    Short id 0000000000f is ambiguous. Candidates are:
    	0000000000f8 tag "v1.0.0"
    	0000000000fd tree
    "#);
    assert_eq!(
        err.to_string(),
        "Short id 0000000000f is ambiguous. Candidates are:\n\t0000000000f8 tag \"v1.0.0\"\n\t0000000000fd tree",
        "disambiguation might not always work either."
    );

    {
        let id = hex_to_id("00000000000434887f772f53e14e39497f7747d3");
        let expected = gix_revision::Spec::Range { from: id, to: id }.attach(&r);
        assert_eq!(
            rev_parse("00000000000..00000000000", &r).unwrap(),
            expected,
            "we know commits are needed here so we don't fall back to repo-config which would look for trees"
        );
    }

    let r = repo("ambiguous_objects_disambiguation_config_tree").unwrap();
    assert_eq!(
        rev_parse("0000000000f", &r).unwrap(),
        Spec::from_id(hex_to_id("0000000000fd8bcc566027a4d16bde8434cac1a4").attach(&r)),
        "disambiguation may work precisely even with a simple object type constraint"
    );

    let r = repo("ambiguous_objects_disambiguation_config_commit").unwrap();
    assert_eq!(
        rev_parse("0000000000f", &r).unwrap_err().to_string(),
        "Short id 0000000000f is ambiguous. Candidates are:\n\t0000000000f8 tag \"v1.0.0\"\n\t0000000000fd tree\n\t0000000000f2 blob",
    );
    assert_eq!(
        rev_parse("0000000000", &r).unwrap_err().to_string(),
        "Short id 0000000000 is ambiguous. Candidates are:\n\t000000000004 commit 2005-04-07 \"czy8f73t\"\n\t00000000006 commit 2005-04-07 \"ad2uee\"\n\t00000000008 commit 2005-04-07 \"ioiley5o\"\n\t0000000000e commit 2005-04-07 \"a2onsxbvj\"",
    );

    let r = repo("ambiguous_objects_disambiguation_config_blob").unwrap();
    assert_eq!(
        rev_parse("0000000000f", &r).unwrap(),
        Spec::from_id(hex_to_id("0000000000f2fdf63f36c0d76aece18a79ab64f2").attach(&r)),
    );
}

#[test]
fn repository_local_disambiguation_hints_are_overridden_by_specific_ones() {
    let repo = repo("ambiguous_objects_disambiguation_config_committish").unwrap();
    let err = rev_parse("0000000000f^{tree}", &repo).unwrap_err();
    insta::assert_debug_snapshot!(err, @r"
    Short id 0000000000f is ambiguous. Candidates are:
    	0000000000c tree
    	0000000000fd tree
    ");
    assert_eq!(
        err.to_string(),
        "Short id 0000000000f is ambiguous. Candidates are:\n\t0000000000c tree\n\t0000000000fd tree",
        "spec overrides overrule the configuration value, which makes this particular object ambiguous between tree and tag"
    );
}
