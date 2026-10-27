use gix::{prelude::ObjectIdExt, revision::Spec};
use gix_testtools::TestResult;

use crate::{
    revision::spec::from_bytes::{parse_spec, parse_spec_no_baseline, repo},
    util::hex_to_id_sha1_only,
};

fn reflog_edge_case_matches_git(name: &str) -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_reflog_edge_cases.sh")?;
    let repo = gix::open_opts(fixture.join(name), crate::restricted())?;
    let baseline = std::fs::read_to_string(fixture.join(name).join("baseline.git"))?;
    for spec in baseline.lines().step_by(2) {
        if let Ok(actual) = parse_spec(spec, &repo) {
            assert!(
                !actual.single().expect("a reflog lookup yields one object").is_null(),
                "{name}: {spec} must name an object"
            );
        }
    }
    Ok(())
}

#[test]
fn reflog_expired_history_matches_git() -> TestResult {
    reflog_edge_case_matches_git("expired")
}

#[test]
fn reflog_deletion_matches_git() -> TestResult {
    reflog_edge_case_matches_git("deletion")
}

#[test]
fn reflog_stale_symbolic_log_matches_git() -> TestResult {
    reflog_edge_case_matches_git("stale")
}

#[test]
fn reflog_dangling_ref_matches_git() -> TestResult {
    reflog_edge_case_matches_git("dangling")?;
    reflog_edge_case_matches_git("null-log")
}

#[test]
fn symbolic_references_use_their_own_log_or_the_final_targets() -> gix_testtools::TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_symbolic_ref_reflogs.sh")?;
    let repo = gix::open_opts(fixture, crate::restricted())?;
    for name in ["refs/symref", "refs/symref-chain"] {
        assert!(
            !repo.find_reference(name)?.log_exists()?,
            "{name} must exercise fallback to the final target's reflog"
        );
        for query in ["0", "1", "2", "1979-02-26 00:00:00 +0000"] {
            let revspec = format!("{name}@{{{query}}}");
            assert_eq!(
                parse_spec(&revspec, &repo)?.single(),
                parse_spec(format!("main@{{{query}}}"), &repo)?.single(),
                "{revspec} uses the final target's reflog"
            );
        }
    }
    assert!(
        repo.find_reference("refs/heads/symref")?.log_exists()?,
        "a symbolic branch has its own reflog"
    );
    assert!(
        parse_spec("refs/heads/symref@{1}", &repo).is_err(),
        "an existing reflog takes precedence over the target's longer log"
    );
    Ok(())
}

#[test]
fn nth_prior_checkout() {
    let repo = repo("complex_graph").unwrap();

    for (spec, prior_branch) in [
        ("@{-1}", "refs/heads/i"),
        ("@{-2}", "refs/heads/main"),
        ("@{-3}", "refs/heads/e"),
        ("@{-4}", "refs/heads/j"),
        ("@{-5}", "refs/heads/h"),
    ] {
        let parsed = parse_spec(spec, &repo).unwrap_or_else(|_| panic!("{spec} to be parsed successfully"));
        assert_eq!(parsed.first_reference().expect("present"), prior_branch);
        assert_eq!(parsed.second_reference(), None);
    }

    insta::assert_debug_snapshot!(parse_spec("@{-6}", &repo).expect_err("nth prior checkout").probable_cause(), "nth prior checkout", @r#"
    Message {
        message: "HEAD has 5 prior checkouts and checkout number 6 is out of range",
    }
    "#);
}

#[test]
fn nth_prior_checkout_to_deleted_branch_fails_like_git() -> TestResult {
    let repo = repo("deleted_prior_checkout")?;
    let err = parse_spec("@{-1}", &repo).expect_err("deleted prior checkout branch must not resolve by object id");
    insta::assert_debug_snapshot!(err.probable_cause(), "error should explain that the reflog name no longer resolves", @r#"
    Message {
        message: "Previous checkout 'prev-target' does not resolve to an existing revision",
    }
    "#);
    Ok(())
}

#[test]
fn nth_prior_checkout_to_deleted_branch_named_like_object_matches_git() -> TestResult {
    let repo = repo("deleted_prior_checkout_named_like_object")?;
    assert_eq!(
        parse_spec("@{-1}", &repo)?,
        Spec::from_id(hex_to_id_sha1_only("0123456789012345678901234567890123456789").attach(&repo)),
        "full object ids are accepted as previous checkout names, even without matching objects"
    );
    Ok(())
}

#[test]
fn nth_prior_checkout_preserves_annotated_tag_matches_git() -> TestResult {
    for fixture in ["prior_checkout_tag_loose", "prior_checkout_tag_packed"] {
        let repo = repo(fixture)?;
        for (prior, named, kind) in [
            ("@{-1}", "main", gix::objs::Kind::Tag),
            ("@{-1}^{}", "main^{}", gix::objs::Kind::Commit),
        ] {
            let actual = parse_spec(prior, &repo)?.single().expect("one revision");
            assert_eq!(
                actual,
                parse_spec(named, &repo)?.single().expect("one revision"),
                "the prior checkout resolves just like its name"
            );
            assert_eq!(actual.object()?.kind, kind, "only an explicit peel follows the tag");
        }
    }
    Ok(())
}

#[test]
fn by_index_unborn_head() {
    let repo = &repo("new").unwrap();

    insta::assert_debug_snapshot!(parse_spec("@{1}", repo).expect_err("by index unborn head").probable_cause(), "by index unborn head", @r#"
    Message {
        message: "Unborn heads do not have a reflog yet",
    }
    "#);
}

#[test]
fn by_index() {
    let repo = &repo("complex_graph").unwrap();
    {
        let spec = parse_spec("@{0}", repo).unwrap();
        assert_eq!(
            spec,
            Spec::from_id(hex_to_id_sha1_only("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(repo))
        );
        assert_eq!(
            spec.first_reference().expect("set"),
            "refs/heads/main",
            "it sets the reference name even if it is implied"
        );
        assert_eq!(spec.second_reference(), None);
    }

    {
        let spec = parse_spec("HEAD@{5}", repo).unwrap();
        assert_eq!(
            spec,
            Spec::from_id(hex_to_id_sha1_only("5b3f9e24965d0b28780b7ce5daf2b5b7f7e0459f").attach(repo))
        );
        assert_eq!(
            spec.first_reference().map(|r| r.name.to_string()),
            Some("HEAD".into()),
            "explicit references are picked up as usual"
        );
        assert_eq!(spec.second_reference(), None);
    }

    insta::assert_debug_snapshot!(parse_spec("main@{12345}", repo)
            .expect_err("by index")
            .probable_cause(), "by index", @r#"
    Message {
        message: "Reference 'refs/heads/main' has 4 ref-log entries and entry number 12345 is out of range",
    }
    "#);
}

#[test]
fn by_date() {
    let repo = repo("complex_graph").unwrap();

    let spec = parse_spec_no_baseline("main@{42 +0030}", &repo).unwrap();

    assert_eq!(
        spec,
        Spec::from_id(hex_to_id_sha1_only("9f9eac6bd1cd4b4cc6a494f044b28c985a22972b").attach(&repo))
    );
}
