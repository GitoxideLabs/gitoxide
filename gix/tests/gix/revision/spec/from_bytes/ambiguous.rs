use super::{missing_reference_names, repo};
use crate::Result;
use crate::{
    revision::spec::from_bytes::{
        normalize_repo_path, parse_spec, parse_spec_better_than_baseline, parse_spec_no_baseline,
        parse_spec_no_baseline_opts, parse_spec_opts, rev_parse,
    },
    util::hex_to_id_sha1_only,
};
use gix::{
    prelude::{ObjectIdExt, RevSpecExt},
    revision::{
        Spec,
        spec::parse::{CandidateInfo, Error, Options, RefsHint},
    },
};

#[test]
fn prefix() -> Result {
    let mut error_snapshots = Vec::new();
    {
        let repo = repo("blob.prefix")?;
        for input in ["dead", "beef"] {
            let err = parse_spec(input, &repo).expect_err("two blobs match the prefix");
            let Some(ambiguity @ Error::AmbiguousPrefix { prefix, candidates }) = err.downcast_any_ref::<Error>()
            else {
                panic!("expected a typed ambiguity error: {err}");
            };
            assert_eq!(
                prefix.to_string(),
                input,
                "the original prefix is available for recovery"
            );
            assert_eq!(candidates.len(), 2, "both candidates remain available");
            assert!(
                candidates[0].0 < candidates[1].0,
                "same-kind candidates are ordered by id"
            );
            assert!(err.is_validation(), "an ambiguous prefix is invalid input");
            assert!(
                gix_error::classify(ambiguity).is_validation(),
                "the ambiguity variant is intrinsically classified without a tag"
            );
            assert!(
                err.classify()
                    .any(|classification| classification.error().is::<Error>()),
                "the ambiguity classification identifies the concrete recovery error"
            );
            error_snapshots.push(err.probable_cause().to_string());
        }
    }

    {
        let repo = repo("blob.bad")?;
        let err = parse_spec("bad0", &repo).expect_err("both prefix candidates are malformed");
        let Some(Error::AmbiguousPrefix { candidates, .. }) = err.downcast_any_ref::<Error>() else {
            panic!("expected a typed ambiguity error: {err}");
        };
        assert_eq!(candidates.len(), 2, "failed lookups do not discard candidates");
        for (_, info) in candidates {
            let CandidateInfo::FindError { source } = info else {
                panic!("the malformed candidate must retain its lookup error");
            };
            assert!(
                source.downcast_any_ref::<gix_error::Message>().is_some(),
                "the owned lookup error retains concrete causes"
            );
        }
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&format_args!("{}", normalize_repo_path(&format!("{err:#?}"), &repo)), &[]), "ambiguous prefixes retain the lookup failure for each malformed candidate", @"
        Short id bad0 is ambiguous. Candidates are:
        \tbad0853 lookup error: Could not read loose object, \"path\"=\"$GIT_DIR/objects/ba/d0853730d9d114ac789f0ce89039d224bf66c9\"
        \tbad0bd4 lookup error: Could not read loose object, \"path\"=\"$GIT_DIR/objects/ba/d0bd4672dee1b4d3b8088534ed5a0362bc8d59\"
        ");
    };
    insta::assert_debug_snapshot!(error_snapshots, "ambiguous object prefixes list the matching candidates", @r#"
    [
        "Short id dead is ambiguous. Candidates are:\n\tdead7b2 blob\n\tdead9d3 blob",
        "Short id beef is ambiguous. Candidates are:\n\tbeef2b0 blob\n\tbeefc9b blob",
    ]
    "#);
    Ok(())
}

#[test]
fn fully_failed_disambiguation_still_yields_an_ambiguity_error() -> Result {
    let repo = repo("ambiguous_blob_tree_commit")?;
    let err = parse_spec("0000000000^{tag}", &repo).expect_err("none of the candidates can peel to a tag");

    insta::assert_debug_snapshot!(err, "candidate origins distinguish failures that reach the same object", @"
    delegate.peel_until(ObjectKind(Tag)) failed, \"input\"=\"{tag}\"
    |
    └─ Short id 0000000000 is ambiguous. Candidates are:
    \t0000000000e commit 2005-04-07 \"a2onsxbvj\"
    \t0000000000c tree
    \t0000000000b blob
        |
        └─ Could not transform candidate 0000000000b
        |   |
        |   └─ Last encountered object 0000000000b was blob while trying to peel to tag
        |
        └─ Could not transform candidate 0000000000c
        |   |
        |   └─ Last encountered object 0000000000c was tree while trying to peel to tag
        |
        └─ Could not transform candidate 0000000000e
            |
            └─ Last encountered object 0000000000c was tree while trying to peel to tag
    ");

    assert!(
        err.is_validation(),
        "candidate context preserves validation classification"
    );
    assert!(
        matches!(err.downcast_any_ref::<Error>(), Some(Error::AmbiguousPrefix { .. })),
        "failed transformations retain the typed ambiguity for recovery"
    );
    use std::error::Error as _;
    insta::assert_snapshot!(err.source().expect("ambiguity error").to_string().replace('\t', "\\t"), "the ambiguity remains the immediate source of the failed transformation", @r#"
    Short id 0000000000 is ambiguous. Candidates are:
    \t0000000000e commit 2005-04-07 "a2onsxbvj"
    \t0000000000c tree
    \t0000000000b blob
    "#);
    Ok(())
}

#[test]
fn ranges_are_auto_disambiguated_by_committish() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    let id = hex_to_id_sha1_only("0000000000e4f9fbd19cf1e932319e5ad0d1d00b");
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
fn resolved_ambiguity_does_not_hide_a_missing_symbolic_referent() -> Result {
    let fixture = gix_testtools::scripted_fixture_writable("make_rev_spec_parse_repos.sh")?;
    let repo = gix::open_opts(fixture.path().join("ambiguous_blob_tree_commit"), crate::restricted())?;
    std::fs::write(repo.git_dir().join("refs/heads/alias"), b"ref: refs/heads/missing\n")?;

    let err = repo
        .rev_parse("000000000..alias")
        .expect_err("the left endpoint resolves to a commit, but the right referent is missing");
    assert!(
        err.is_not_found(),
        "resolved ambiguity must not mask the missing referent: {err}"
    );
    assert!(
        err.iter_errors().any(|cause| matches!(
            cause.downcast_ref::<Error>(),
            Some(Error::MissingReference { name }) if name == std::path::Path::new("refs/heads/missing")
        )),
        "rejected candidates do not hide the parser-owned missing-reference error"
    );
    assert_eq!(
        err.downcast_any_ref::<gix::refs::file::find::NotFound>()
            .expect("the lookup error survives rejected-candidate errors")
            .name,
        std::path::Path::new("refs/heads/missing"),
        "the final spec conversion retains the actual lookup failure"
    );
    insta::assert_debug_snapshot!(err, @r#"
    The rev-spec is malformed and misses a ref name
    |
    └─ Last encountered object 0000000000b was blob while trying to peel to commit
    |
    └─ Last encountered object 0000000000c was tree while trying to peel to commit
    |
    └─ Could not peel 'refs/heads/alias' to obtain its target
        |
        └─ Reference "refs/heads/missing" could not be found
        |
        └─ The ref partially named "refs/heads/missing" could not be found
    "#);
    Ok(())
}

#[test]
fn missing_references_survive_other_endpoint_ambiguity() -> Result {
    let fixture = gix_testtools::scripted_fixture_writable("make_rev_spec_parse_repos.sh")?;
    for (fixture_name, remains_ambiguous) in [
        ("ambiguous_blob_tree_commit", false),
        ("duplicate_ambiguous_objects", true),
    ] {
        let repo = gix::open_opts(fixture.path().join(fixture_name), crate::restricted())?;
        std::fs::write(repo.git_dir().join("refs/heads/alias"), b"ref: refs/heads/missing\n")?;
        for input in ["0000000000..alias", "0000000000..alias:README.md"] {
            let err = repo
                .rev_parse(input)
                .expect_err("the right endpoint's symbolic referent is missing");
            assert_eq!(
                missing_reference_names(&err),
                [std::path::Path::new("refs/heads/missing")],
                "{fixture_name}: {input} retains missing references during callbacks and finalization: {err}"
            );
            assert_eq!(
                err.downcast_any_ref::<gix::refs::file::find::NotFound>()
                    .expect("the original lookup cause is retained alongside parser recovery errors")
                    .name,
                std::path::Path::new("refs/heads/missing"),
                "aggregation does not discard the original missing-reference cause"
            );
            assert_eq!(
                err.iter_errors()
                    .any(|cause| matches!(cause.downcast_ref::<Error>(), Some(Error::AmbiguousPrefix { .. }))),
                remains_ambiguous,
                "{fixture_name}: {input} reports only ambiguity that survived disambiguation: {err}"
            );
        }
    }
    Ok(())
}

#[test]
fn blob_and_tree_can_be_disambiguated_by_type() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    insta::assert_snapshot!(parse_spec("0000000000", &repo)
            .expect_err("in theory one could disambiguate with 0000000000^{{tree}} (which works in git) or 0000000000^{{blob}} which doesn't work for some reason.")
            .probable_cause().to_string().replace('\t', "\\t"), "ambiguous prefixes retain their human-readable candidate listing", @r#"
    Short id 0000000000 is ambiguous. Candidates are:
    \t0000000000e commit 2005-04-07 "a2onsxbvj"
    \t0000000000c tree
    \t0000000000b blob
    "#);

    assert_eq!(
        parse_spec("0000000000cdc^{tree}", &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("0000000000cdcf04beb2fab69e65622616294984").attach(&repo)),
        "this is unambiguous anyway, but also asserts for tree which is naturally the case"
    );

    assert_eq!(
        parse_spec_better_than_baseline("0000000000^{tree}", &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("0000000000cdcf04beb2fab69e65622616294984").attach(&repo)),
        "the commit refers to the tree which also starts with this prefix, so ultimately the result is unambiguous. Git can't do that yet."
    );

    assert_eq!(
        parse_spec("0000000000^{commit}", &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo)),
        "disambiguation with committish"
    );

    assert_eq!(
        parse_spec("0000000000e", &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo)),
        "no disambiguation needed here"
    );
}

#[test]
fn trees_can_be_disambiguated_by_blob_access() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    let actual = parse_spec_better_than_baseline("0000000000:a0blgqsjc", &repo).unwrap();
    assert_eq!(
        actual,
        Spec::from_id(hex_to_id_sha1_only("0000000000b36b6aa7ea4b75318ed078f55505c3").attach(&repo)),
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
            Spec::from_id(hex_to_id_sha1_only("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo))
        );
    }
}

#[test]
fn tags_can_be_disambiguated_with_commit_specific_transformations() {
    let repo = repo("ambiguous_commits").unwrap();
    assert_eq!(
        parse_spec_better_than_baseline("0000000000^{tag}", &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("0000000000f8f5507ab27a0d7bd3c75c0f64ffe0").attach(&repo)),
        "disambiguation is possible by type, and git can't do that for some reason"
    );
}

#[test]
fn duplicates_are_deduplicated_across_all_odb_types() -> Result {
    let repo = repo("duplicate_ambiguous_objects")?;
    let err = parse_spec_no_baseline("0000000000", &repo).expect_err("multiple distinct candidates match");
    let Some(Error::AmbiguousPrefix { candidates, .. }) = err.downcast_any_ref::<Error>() else {
        panic!("expected a typed ambiguity error: {err}");
    };
    assert_eq!(
        candidates.len(),
        15,
        "objects present in both loose and packed storage appear once"
    );
    assert!(
        matches!(&candidates[0].1, CandidateInfo::Tag { name } if name == "v1.0.0"),
        "tags retain their byte-oriented names"
    );
    assert!(
        matches!(&candidates[1].1, CandidateInfo::Commit { title, date } if title == "czy8f73t" && !date.is_empty()),
        "commits retain their subject and date"
    );
    let ordering: Vec<_> = candidates
        .iter()
        .map(|(object_id, info)| {
            (
                match info {
                    CandidateInfo::Tag { .. } => 0,
                    CandidateInfo::Commit { .. } => 1,
                    CandidateInfo::Object {
                        kind: gix_object::Kind::Tree,
                    } => 2,
                    CandidateInfo::Object {
                        kind: gix_object::Kind::Blob,
                    } => 3,
                    other => panic!("unexpected candidate: {other:?}"),
                },
                *object_id,
            )
        })
        .collect();
    assert!(
        ordering.windows(2).all(|pair| pair[0] < pair[1]),
        "candidates are ordered by kind, then object id"
    );
    insta::assert_snapshot!(err.probable_cause().to_string().replace('\t', "\\t"), "deduplicated candidates keep their human-readable listing", @r#"
    Short id 0000000000 is ambiguous. Candidates are:
    \t0000000000f8 tag "v1.0.0"
    \t000000000004 commit 2005-04-07 "czy8f73t"
    \t00000000006 commit 2005-04-07 "ad2uee"
    \t00000000008 commit 2005-04-07 "ioiley5o"
    \t0000000000e commit 2005-04-07 "a2onsxbvj"
    \t000000000002 tree
    \t00000000005 tree
    \t00000000009 tree
    \t0000000000c tree
    \t0000000000fd tree
    \t00000000001 blob
    \t00000000003 blob
    \t0000000000a blob
    \t0000000000b blob
    \t0000000000f2 blob
    "#);
    Ok(())
}

#[test]
fn malformed_commit_and_tag_candidates_retain_decode_errors() -> Result {
    use gix_object::Write;

    let fixture = gix_testtools::scripted_fixture_writable("make_rev_spec_parse_repos.sh")?;
    let repo = gix::open_opts(fixture.path().join("ambiguous_blob_tree_commit"), crate::restricted())?;
    let objects = repo.objects.store_ref().path();
    // Replace one existing commit and add a tag at the same prefix. The loose headers remain readable,
    // but their bodies cannot be decoded, exercising candidate reporting after object lookup succeeds.
    for (kind, candidate_id) in [
        (gix_object::Kind::Commit, "0000000000e4f9fbd19cf1e932319e5ad0d1d00b"),
        (gix_object::Kind::Tag, "0000000000f00000000000000000000000000000"),
    ] {
        let malformed_object_id = repo.objects.write_buf(kind, b"malformed object body")?;
        let hex = malformed_object_id.to_string();
        let source = objects.join(&hex[..2]).join(&hex[2..]);
        let destination = objects.join(&candidate_id[..2]).join(&candidate_id[2..]);
        if destination.exists() {
            std::fs::remove_file(&destination)?;
        }
        std::fs::rename(source, destination)?;
    }
    let err = repo
        .rev_parse("0000000000")
        .expect_err("the prefix remains ambiguous despite malformed candidates");
    let Some(Error::AmbiguousPrefix { candidates, .. }) = err.downcast_any_ref::<Error>() else {
        panic!("expected a typed ambiguity error: {err}");
    };
    assert_eq!(candidates.len(), 4, "malformed candidates remain in the listing");
    for (_, info) in &candidates[..2] {
        let CandidateInfo::FindError { source } = info else {
            panic!("malformed tag and commit bodies must retain decoding failures");
        };
        assert!(
            source.downcast_any_ref::<gix_error::Message>().is_some(),
            "candidate diagnostics retain the decoder's concrete error"
        );
    }
    Ok(())
}

fn opts_ref_hint(hint: RefsHint) -> Options {
    Options {
        refs_hint: hint,
        object_kind_hint: None,
    }
}

fn assert_ref_and_object_ambiguity<'a>(err: &'a gix::Error, input: &str, expected_candidates: &[&str]) -> &'a Error {
    let Some(
        ambiguity @ Error::AmbiguousRefAndObject {
            prefix,
            reference,
            candidates,
        },
    ) = err.downcast_any_ref::<Error>()
    else {
        panic!("expected a typed reference/object ambiguity: {err}");
    };
    assert_eq!(prefix.to_string(), input, "the colliding prefix is preserved");
    assert_eq!(
        reference.as_bstr(),
        format!("refs/heads/{input}").as_str(),
        "the matching reference can be selected independently of the objects"
    );
    assert_eq!(
        candidates.len(),
        expected_candidates.len(),
        "all object candidates remain available, including a single match"
    );
    for ((candidate, _), expected) in candidates.iter().zip(expected_candidates) {
        assert_eq!(
            candidate.to_string(),
            *expected,
            "candidate ordering matches ordinary object-prefix ambiguity"
        );
    }
    assert!(
        err.is_validation(),
        "a rejected reference/object collision is invalid input"
    );
    assert!(
        gix_error::classify(ambiguity).is_validation(),
        "the collision variant is intrinsically classified"
    );
    assert!(
        !err.iter_errors()
            .any(|cause| matches!(cause.downcast_ref::<Error>(), Some(Error::AmbiguousPrefix { .. }))),
        "a reference/object collision is not also reported as object-only ambiguity"
    );
    ambiguity
}

#[test]
fn ambiguous_40hex_refs_are_ignored_and_we_prefer_the_object_of_the_same_name() {
    let repo = repo("ambiguous_refs").unwrap();
    let spec = "0000000000e4f9fbd19cf1e932319e5ad0d1d00b";
    assert_eq!(
        parse_spec(spec, &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only(spec).attach(&repo)),
        "git shows an advisory here and ignores the ref, which makes it easy to just ignore it too. We are unable to show anything though, maybe traces?"
    );

    assert_eq!(
        parse_spec_opts(spec, &repo, opts_ref_hint(RefsHint::PreferObject)).unwrap(),
        Spec::from_id(hex_to_id_sha1_only(spec).attach(&repo)),
        "preferring objects yields the same result here"
    );

    assert_eq!(
        parse_spec_no_baseline_opts(spec, &repo, opts_ref_hint(RefsHint::PreferRef)).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("cc60d25ccfee90e4a4105e73df36059db383d5ce").attach(&repo)),
        "we can prefer refs in any case, too"
    );

    let err = parse_spec_no_baseline_opts(spec, &repo, opts_ref_hint(RefsHint::Fail))
        .expect_err("full-length object names can also collide with references");
    assert_ref_and_object_ambiguity(&err, spec, &["0000000000e"]);
}

#[test]
fn ambiguous_short_refs_are_dereferenced() {
    let repo = repo("ambiguous_refs").unwrap();
    let spec = "0000000000e";
    assert_eq!(
        parse_spec(spec, &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("cc60d25ccfee90e4a4105e73df36059db383d5ce").attach(&repo)),
        "git shows a warning here and we show nothing but have dials to control how to handle these cases"
    );

    assert_eq!(
        parse_spec_opts(spec, &repo, opts_ref_hint(RefsHint::PreferRef)).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("cc60d25ccfee90e4a4105e73df36059db383d5ce").attach(&repo)),
        "this does the same, but independently of the length of the ref"
    );

    assert_eq!(
        parse_spec_no_baseline_opts(spec, &repo, opts_ref_hint(RefsHint::PreferObject)).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo)),
        "we can always prefer objects, too"
    );

    let err = parse_spec_no_baseline_opts(spec, &repo, opts_ref_hint(RefsHint::Fail))
        .expect_err("the caller can reject reference/object collisions instead of choosing a preference");
    let ambiguity = assert_ref_and_object_ambiguity(&err, spec, &["0000000000e"]);
    insta::assert_snapshot!(ambiguity.to_string().replace('\t', "\\t"), "reference/object collisions retain a human-readable reference and candidate list", @r#"
    The object-id prefix 0000000000e matched both the reference refs/heads/0000000000e and at least one object. Candidates are:
    \t0000000000e commit 2005-04-07 "a2onsxbvj"
    "#);

    let err = parse_spec_no_baseline_opts("0000000000^{commit}..0000000000e", &repo, opts_ref_hint(RefsHint::Fail))
        .expect_err("the left endpoint resolves, but the right endpoint has a reference/object collision");
    assert_ref_and_object_ambiguity(&err, spec, &["0000000000e"]);
}

#[test]
fn reference_collisions_retain_multiple_object_candidates() -> Result {
    let fixture = gix_testtools::scripted_fixture_writable("make_rev_spec_parse_repos.sh")?;
    let repo = gix::open_opts(fixture.path().join("ambiguous_blob_tree_commit"), crate::restricted())?;
    repo.reference(
        "refs/heads/0000000000",
        repo.head_id()?,
        gix::refs::transaction::PreviousValue::Any,
        "",
    )?;

    let input = "0000000000";
    let err = parse_spec_no_baseline_opts(input, &repo, opts_ref_hint(RefsHint::Fail))
        .expect_err("the prefix matches a reference as well as multiple objects");
    assert_ref_and_object_ambiguity(&err, input, &["0000000000e", "0000000000c", "0000000000b"]);

    let resolved = parse_spec_no_baseline_opts(input, &repo, opts_ref_hint(RefsHint::PreferRef))?;
    assert_eq!(
        resolved.single().expect("the reference resolves to one object"),
        repo.head_id()?,
        "retrying with a reference preference resolves the collision"
    );
    let err = parse_spec_no_baseline_opts(input, &repo, opts_ref_hint(RefsHint::PreferObject))
        .expect_err("preferring objects still requires disambiguating the object candidates");
    assert!(
        matches!(err.downcast_any_ref::<Error>(), Some(Error::AmbiguousPrefix { .. })),
        "object-only ambiguity remains distinct after choosing objects over the reference"
    );
    Ok(())
}

#[test]
fn repository_local_disambiguation_hints_disambiguate() {
    let r = repo("ambiguous_objects_disambiguation_config_committish").unwrap();
    assert_eq!(
        rev_parse("0000000000f", &r).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("0000000000f8f5507ab27a0d7bd3c75c0f64ffe0").attach(&r)),
        "we read the 'core.disambiguate' value and apply it to auto-disambiguate"
    );
    let err = rev_parse("0000000000", &r).unwrap_err();
    insta::assert_debug_snapshot!(err, @"
    Short id 0000000000 is ambiguous. Candidates are:
    \t0000000000f8 tag \"v1.0.0\"
    \t000000000004 commit 2005-04-07 \"czy8f73t\"
    \t00000000006 commit 2005-04-07 \"ad2uee\"
    \t00000000008 commit 2005-04-07 \"ioiley5o\"
    \t0000000000e commit 2005-04-07 \"a2onsxbvj\"
    |
    └─ Last encountered object 000000000002 was tree while trying to peel to commit
    |
    └─ Last encountered object 00000000001 was blob while trying to peel to commit
    |
    └─ Last encountered object 00000000003 was blob while trying to peel to commit
    |
    └─ Last encountered object 00000000005 was tree while trying to peel to commit
    |
    └─ Last encountered object 00000000009 was tree while trying to peel to commit
    |
    └─ Last encountered object 0000000000a was blob while trying to peel to commit
    |
    └─ Last encountered object 0000000000b was blob while trying to peel to commit
    |
    └─ Last encountered object 0000000000c was tree while trying to peel to commit
    |
    └─ Last encountered object 0000000000f2 was blob while trying to peel to commit
    |
    └─ Last encountered object 0000000000fd was tree while trying to peel to commit
    ");
    insta::assert_debug_snapshot!(err, "repository local disambiguation hints disambiguate", @"
    Short id 0000000000 is ambiguous. Candidates are:
    \t0000000000f8 tag \"v1.0.0\"
    \t000000000004 commit 2005-04-07 \"czy8f73t\"
    \t00000000006 commit 2005-04-07 \"ad2uee\"
    \t00000000008 commit 2005-04-07 \"ioiley5o\"
    \t0000000000e commit 2005-04-07 \"a2onsxbvj\"
    |
    └─ Last encountered object 000000000002 was tree while trying to peel to commit
    |
    └─ Last encountered object 00000000001 was blob while trying to peel to commit
    |
    └─ Last encountered object 00000000003 was blob while trying to peel to commit
    |
    └─ Last encountered object 00000000005 was tree while trying to peel to commit
    |
    └─ Last encountered object 00000000009 was tree while trying to peel to commit
    |
    └─ Last encountered object 0000000000a was blob while trying to peel to commit
    |
    └─ Last encountered object 0000000000b was blob while trying to peel to commit
    |
    └─ Last encountered object 0000000000c was tree while trying to peel to commit
    |
    └─ Last encountered object 0000000000f2 was blob while trying to peel to commit
    |
    └─ Last encountered object 0000000000fd was tree while trying to peel to commit
    ");

    let r = repo("ambiguous_objects_disambiguation_config_treeish").unwrap();
    let err = rev_parse("0000000000f", &r).unwrap_err();
    insta::assert_debug_snapshot!(err, @"
    Short id 0000000000f is ambiguous. Candidates are:
    \t0000000000f8 tag \"v1.0.0\"
    \t0000000000fd tree
    |
    └─ Last encountered object 0000000000f2 was blob while trying to peel to tree
    ");
    insta::assert_debug_snapshot!(err, "disambiguation might not always work either.", @"
    Short id 0000000000f is ambiguous. Candidates are:
    \t0000000000f8 tag \"v1.0.0\"
    \t0000000000fd tree
    |
    └─ Last encountered object 0000000000f2 was blob while trying to peel to tree
    ");

    {
        let id = hex_to_id_sha1_only("00000000000434887f772f53e14e39497f7747d3");
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
        Spec::from_id(hex_to_id_sha1_only("0000000000fd8bcc566027a4d16bde8434cac1a4").attach(&r)),
        "disambiguation may work precisely even with a simple object type constraint"
    );

    let r = repo("ambiguous_objects_disambiguation_config_commit").unwrap();
    insta::assert_debug_snapshot!(rev_parse("0000000000f", &r).expect_err("repository local disambiguation hints disambiguate"), "repository local disambiguation hints disambiguate", @"
    Short id 0000000000f is ambiguous. Candidates are:
    \t0000000000f8 tag \"v1.0.0\"
    \t0000000000fd tree
    \t0000000000f2 blob
    |
    └─ Object 0000000000f2 was a blob, but needed it to be a commit
    |
    └─ Object 0000000000f8 was a tag, but needed it to be a commit
    |
    └─ Object 0000000000fd was a tree, but needed it to be a commit
    ");
    insta::assert_debug_snapshot!(rev_parse("0000000000", &r).expect_err("repository local disambiguation hints disambiguate"), "repository local disambiguation hints disambiguate", @"
    Short id 0000000000 is ambiguous. Candidates are:
    \t000000000004 commit 2005-04-07 \"czy8f73t\"
    \t00000000006 commit 2005-04-07 \"ad2uee\"
    \t00000000008 commit 2005-04-07 \"ioiley5o\"
    \t0000000000e commit 2005-04-07 \"a2onsxbvj\"
    |
    └─ Object 000000000002 was a tree, but needed it to be a commit
    |
    └─ Object 00000000001 was a blob, but needed it to be a commit
    |
    └─ Object 00000000003 was a blob, but needed it to be a commit
    |
    └─ Object 00000000005 was a tree, but needed it to be a commit
    |
    └─ Object 00000000009 was a tree, but needed it to be a commit
    |
    └─ Object 0000000000a was a blob, but needed it to be a commit
    |
    └─ Object 0000000000b was a blob, but needed it to be a commit
    |
    └─ Object 0000000000c was a tree, but needed it to be a commit
    |
    └─ Object 0000000000f2 was a blob, but needed it to be a commit
    |
    └─ Object 0000000000f8 was a tag, but needed it to be a commit
    |
    └─ Object 0000000000fd was a tree, but needed it to be a commit
    ");

    let r = repo("ambiguous_objects_disambiguation_config_blob").unwrap();
    assert_eq!(
        rev_parse("0000000000f", &r).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("0000000000f2fdf63f36c0d76aece18a79ab64f2").attach(&r)),
    );
}

#[test]
fn repository_local_disambiguation_hints_are_overridden_by_specific_ones() {
    let repo = repo("ambiguous_objects_disambiguation_config_committish").unwrap();
    let err = rev_parse("0000000000f^{tree}", &repo).unwrap_err();
    insta::assert_debug_snapshot!(err, @"
    Short id 0000000000f is ambiguous. Candidates are:
    \t0000000000c tree
    \t0000000000fd tree
    |
    └─ Could not transform candidate 0000000000f2
    |
    └─ Last encountered object 0000000000f2 was blob while trying to peel to tree
    ");
    insta::assert_debug_snapshot!(err, "spec overrides overrule the configuration value, which makes this particular object ambiguous between tree and tag", @"
    Short id 0000000000f is ambiguous. Candidates are:
    \t0000000000c tree
    \t0000000000fd tree
    |
    └─ Could not transform candidate 0000000000f2
    |
    └─ Last encountered object 0000000000f2 was blob while trying to peel to tree
    ");
}
