use std::io::Write as _;

use gix::{prelude::ObjectIdExt, revision::Spec};

use crate::{
    revision::spec::from_bytes::{parse_spec, parse_spec_no_baseline, repo},
    util::hex_to_id_sha1_only,
};

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

    assert_eq!(
        parse_spec("@{-6}", &repo).unwrap_err().probable_cause().to_string(),
        "HEAD has 5 prior checkouts and checkout number 6 is out of range"
    );
}

#[test]
fn nth_prior_checkout_to_deleted_branch_fails_like_git() -> crate::Result {
    let repo = repo("deleted_prior_checkout")?;
    let err = parse_spec("@{-1}", &repo).expect_err("deleted prior checkout branch must not resolve by object id");
    assert!(
        err.probable_cause()
            .to_string()
            .contains("Previous checkout 'prev-target' does not resolve"),
        "error should explain that the reflog name no longer resolves"
    );
    Ok(())
}

#[test]
fn nth_prior_checkout_to_deleted_branch_named_like_object_matches_git() -> crate::Result {
    let repo = repo("deleted_prior_checkout_named_like_object")?;
    assert_eq!(
        parse_spec("@{-1}", &repo)?,
        Spec::from_id(hex_to_id_sha1_only("0123456789012345678901234567890123456789").attach(&repo)),
        "full object ids are accepted as previous checkout names, even without matching objects"
    );
    Ok(())
}

#[test]
fn by_index_unborn_head() {
    let repo = &repo("new").unwrap();

    assert_eq!(
        parse_spec("@{1}", repo).unwrap_err().probable_cause().to_string(),
        "Unborn heads do not have a reflog yet"
    );
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

    assert_eq!(
        parse_spec("main@{12345}", repo)
            .unwrap_err()
            .probable_cause()
            .to_string(),
        "Reference 'refs/heads/main' has 4 ref-log entries and entry number 12345 is out of range"
    );
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

#[test]
fn malformed_reflog_entries_are_not_reported_as_missing_queries() -> crate::Result {
    let fixture = gix_testtools::scripted_fixture_writable("make_rev_spec_parse_repos.sh")?;
    let repository_path = fixture.path().join("complex_graph");
    for log_path in [
        repository_path.join(".git/logs/refs/heads/main"),
        repository_path.join(".git/logs/HEAD"),
    ] {
        std::fs::OpenOptions::new()
            .append(true)
            .open(&log_path)?
            .write_all(b"not a reflog entry\n")?;
    }
    let repo = gix::open_opts(&repository_path, crate::restricted())?;

    for spec in ["main@{0}", "main@{42 +0030}", "@{-1}"] {
        let error =
            parse_spec_no_baseline(spec, &repo).expect_err("malformed reflog entries must fail revision parsing");
        let rendered = format!("{error:#}");
        assert!(
            rendered.contains("read a reference log in reverse"),
            "{spec} preserves the reflog decoding failure: {rendered}"
        );
        assert!(
            !rendered.contains("out of range") && !rendered.contains("does not contain any entries"),
            "{spec} does not disguise corruption as an absent query: {rendered}"
        );
    }
    Ok(())
}
