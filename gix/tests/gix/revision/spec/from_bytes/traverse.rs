use gix::{prelude::ObjectIdExt, revision::Spec};

use crate::{
    revision::spec::from_bytes::{parse_spec, parse_spec_no_baseline, repo},
    util::hex_to_id,
};

#[test]
fn complex() -> crate::Result {
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
fn freestanding_negation_yields_descriptive_error() -> crate::Result {
    let repo = repo("complex_graph")?;
    for revspec in ["^^", "^^HEAD"] {
        assert_eq!(
            parse_spec(revspec, &repo).unwrap_err().probable_cause().to_string(),
            "Tried to navigate the commit-graph without providing an anchor first"
        );
    }
    assert_eq!(
        parse_spec("^", &repo).unwrap_err().probable_cause().to_string(),
        "The rev-spec is malformed and misses a ref name"
    );
    let err = parse_spec("^!", &repo).unwrap_err();
    insta::assert_debug_snapshot!(err, @r#"
    couldn't parse revision: !
    |
    └─ The ref partially named "!" could not be found
    "#);
    assert_eq!(
        err.probable_cause().to_string(),
        "The ref partially named \"!\" could not be found"
    );
    Ok(())
}
#[test]
fn freestanding_double_or_triple_dot_defaults_to_head_refs() -> crate::Result {
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
        Spec::from_id(hex_to_id("5b3f9e24965d0b28780b7ce5daf2b5b7f7e0459f").attach(&repo))
    );
    assert_eq!(parse_spec("a", &repo).unwrap(), parse_spec("a^0", &repo).unwrap());
    assert_eq!(
        parse_spec("a^42", &repo).unwrap_err().probable_cause().to_string(),
        "Commit 55e825e has 2 parents and parent number 42 is out of range"
    );
}

#[test]
fn ancestors() {
    let repo = repo("complex_graph").unwrap();
    assert_eq!(
        parse_spec("a~1", &repo).unwrap(),
        Spec::from_id(hex_to_id("5b3f9e24965d0b28780b7ce5daf2b5b7f7e0459f").attach(&repo))
    );
    assert_eq!(parse_spec("a", &repo).unwrap(), parse_spec("a~0", &repo).unwrap());
    assert_eq!(
        parse_spec("a~3", &repo).unwrap(),
        Spec::from_id(hex_to_id("9f9eac6bd1cd4b4cc6a494f044b28c985a22972b").attach(&repo))
    );
    assert_eq!(
        parse_spec("a~42", &repo).unwrap_err().probable_cause().to_string(),
        "Commit 55e825e has 3 ancestors along the first parent and ancestor number 42 is out of range"
    );
}
