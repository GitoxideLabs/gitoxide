use crate::Result;
use gix::{prelude::ObjectIdExt, revision::Spec};

use crate::{
    revision::spec::from_bytes::{parse_spec, parse_spec_no_baseline, repo},
    util::hex_to_id_sha1_only,
};

#[test]
fn complex() -> Result {
    let repo = &repo("complex_graph")?;

    assert_eq!(parse_spec("b", repo)?, parse_spec("a~1", repo)?);
    assert_eq!(parse_spec("b", repo)?, parse_spec("a^", repo)?);
    assert_eq!(parse_spec("c", repo)?, parse_spec("a^2", repo)?);
    assert_eq!(parse_spec("d", repo)?, parse_spec("a^^", repo)?);
    assert_eq!(parse_spec("d", repo)?, parse_spec("a^1^1", repo)?);
    assert_eq!(parse_spec("d", repo)?, parse_spec("a~2", repo)?);
    assert_eq!(parse_spec("e", repo)?, parse_spec("a^^2", repo)?);
    assert_eq!(parse_spec("j", repo)?, parse_spec("b^3^2", repo)?);
    assert_eq!(parse_spec("j", repo)?, parse_spec("a^^3^2", repo)?);
    Ok(())
}

#[test]
fn freestanding_negation_yields_descriptive_error() -> Result {
    let mut error_snapshots = Vec::new();
    let repo = repo("complex_graph")?;
    for revspec in ["^^", "^^HEAD"] {
        error_snapshots.push(gix_testtools::redact_debug_snapshot(
            &(parse_spec(revspec, &repo).unwrap_err().probable_cause()),
            &[],
        ));
    }
    insta::assert_debug_snapshot!(parse_spec("^", &repo).expect_err("freestanding negation yields descriptive error").probable_cause(), "freestanding negation yields descriptive error", @r#"
    Message {
        message: "The rev-spec is malformed and misses a ref name",
    }
    "#);
    let err = parse_spec("^!", &repo).unwrap_err();
    insta::assert_debug_snapshot!(err, @r#"
    couldn't parse revision, "input"="!"
    |
    └─ Reference "!" could not be found
    |
    └─ The ref partially named "!" could not be found
    "#);
    assert!(err.is_not_found(), "the missing anchor reference remains classified");
    insta::assert_debug_snapshot!(error_snapshots, "freestanding negation yields descriptive error", @r#"
    [
        Message {
            message: "Tried to navigate the commit-graph without providing an anchor first",
        },
        Message {
            message: "Tried to navigate the commit-graph without providing an anchor first",
        },
    ]
    "#);
    Ok(())
}
#[test]
fn freestanding_double_or_triple_dot_defaults_to_head_refs() -> Result {
    let repo = repo("complex_graph")?;
    assert_eq!(
        parse_spec_no_baseline("..", &repo)?, // git can't communicate what it does here
        parse_spec("@..@", &repo)?,
    );
    assert_eq!(parse_spec("...", &repo)?, parse_spec("@...@", &repo)?);
    Ok(())
}

#[test]
fn parent() {
    let repo = repo("complex_graph").unwrap();
    assert_eq!(
        parse_spec("a^1", &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("5b3f9e24965d0b28780b7ce5daf2b5b7f7e0459f").attach(&repo))
    );
    assert_eq!(parse_spec("a", &repo).unwrap(), parse_spec("a^0", &repo).unwrap());
    insta::assert_debug_snapshot!(parse_spec("a^42", &repo).expect_err("parent").probable_cause(), "parent", @r#"
    Message {
        message: "Commit 55e825e has 2 parents and parent number 42 is out of range",
    }
    "#);
}

#[test]
fn tags_navigate_from_their_commit() -> Result {
    let repo = repo("complex_graph")?;
    for (spec, expected) in [
        ("b-tag^", "d"),
        ("b-tag^1", "d"),
        ("b-tag^2", "e"),
        ("b-tag~1", "d"),
        ("b-tag~2", "g"),
        ("b-tag~0", "b"),
        ("b-lightweight-tag~0", "b"),
        ("b-tag^{/G}", "g"),
    ] {
        assert_eq!(
            parse_spec(spec, &repo)?,
            parse_spec(expected, &repo)?,
            "{spec} navigates from the commit that the tag points at"
        );
    }
    Ok(())
}

#[test]
fn ancestors() {
    let repo = repo("complex_graph").unwrap();
    assert_eq!(
        parse_spec("a~1", &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("5b3f9e24965d0b28780b7ce5daf2b5b7f7e0459f").attach(&repo))
    );
    assert_eq!(parse_spec("a", &repo).unwrap(), parse_spec("a~0", &repo).unwrap());
    assert_eq!(
        parse_spec("a~3", &repo).unwrap(),
        Spec::from_id(hex_to_id_sha1_only("9f9eac6bd1cd4b4cc6a494f044b28c985a22972b").attach(&repo))
    );
    insta::assert_debug_snapshot!(parse_spec("a~42", &repo).expect_err("ancestors").probable_cause(), "ancestors", @r#"
    Message {
        message: "Commit 55e825e has 3 ancestors along the first parent and ancestor number 42 is out of range",
    }
    "#);
}
