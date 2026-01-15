use gix::{prelude::ObjectIdExt, revision::Spec};

use crate::{
    revision::spec::from_bytes::{parse_spec, parse_spec_no_baseline, repo},
    util::hex_to_id,
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
        assert_eq!(parsed.first_reference().expect("present").name.as_bstr(), prior_branch);
        assert_eq!(parsed.second_reference(), None);
    }

    assert_eq!(
        parse_spec("@{-6}", &repo).unwrap_err().probable_cause().to_string(),
        "HEAD has 5 prior checkouts and checkout number 6 is out of range"
    );
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
            Spec::from_id(hex_to_id("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(repo))
        );
        assert_eq!(
            spec.first_reference().expect("set").name.as_bstr(),
            "refs/heads/main",
            "it sets the reference name even if it is implied"
        );
        assert_eq!(spec.second_reference(), None);
    }

    {
        let spec = parse_spec("HEAD@{5}", repo).unwrap();
        assert_eq!(
            spec,
            Spec::from_id(hex_to_id("5b3f9e24965d0b28780b7ce5daf2b5b7f7e0459f").attach(repo))
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
        Spec::from_id(hex_to_id("9f9eac6bd1cd4b4cc6a494f044b28c985a22972b").attach(&repo))
    );
}
