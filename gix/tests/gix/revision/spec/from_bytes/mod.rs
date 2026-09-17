use gix::{prelude::ObjectIdExt, revision::Spec};
pub use util::*;

use crate::util::hex_to_id_sha1_only;

mod ambiguous;
mod regex;
mod util;

mod reflog;
mod traverse;

mod peel;

mod sibling_branch {
    use crate::{
        revision::spec::from_bytes::{parse_spec, repo},
        util::hex_to_id_sha1_only,
    };

    #[test]
    fn push_and_upstream() -> crate::Result {
        let repo = repo("complex_graph").unwrap();
        for op in ["upstream", "push"] {
            for branch in ["", "main"] {
                let actual = parse_spec(format!("{branch}@{{{op}}}"), &repo)?;
                assert_eq!(actual.first_reference().expect("set"), "refs/remotes/origin/main");
                assert_eq!(actual.second_reference(), None);
                assert_eq!(
                    actual.single().expect("just one"),
                    hex_to_id_sha1_only("55e825ebe8fd2ff78cad3826afb696b96b576a7e")
                );
            }
        }
        Ok(())
    }
}

mod index {
    use gix::{prelude::ObjectIdExt, revision::Spec};

    use crate::{
        revision::spec::from_bytes::{parse_spec, repo},
        util::hex_to_id_sha1_only,
    };

    #[test]
    fn at_stage() {
        let repo = repo("complex_graph").unwrap();
        let actual = parse_spec(":file", &repo).unwrap();
        assert_eq!(
            actual,
            Spec::from_id(hex_to_id_sha1_only("fe27474251f7f8368742f01fbd3bd5666b630a82").attach(&repo))
        );
        assert_eq!(
            actual.path_and_mode().expect("set"),
            ("file".into(), gix_object::tree::EntryKind::Blob.into()),
            "index paths (that are present) are captured"
        );

        let err = parse_spec(":1:file", &repo).unwrap_err();
        insta::assert_debug_snapshot!(err, @r#"
        Couldn't find index 'file' stage 1
        |
        └─ Path "file" did not exist in index at stage 1. It does exist at stage 0. It exists on disk
        "#);
        assert_eq!(
            err.probable_cause().to_string(),
            "Path \"file\" did not exist in index at stage 1. It does exist at stage 0. It exists on disk",
        );

        assert_eq!(
            parse_spec(":5:file", &repo).unwrap_err().probable_cause().to_string(),
            "Path \"5:file\" did not exist in index at stage 0. It does not exist on disk",
            "invalid stage ids are interpreted as part of the filename"
        );

        assert_eq!(
            parse_spec(":foo", &repo).unwrap_err().probable_cause().to_string(),
            "Path \"foo\" did not exist in index at stage 0. It does not exist on disk",
        );
    }
}

#[test]
fn names_are_made_available_via_references() {
    let repo = repo("complex_graph").unwrap();
    let spec = parse_spec_no_baseline("main..g", &repo).unwrap();
    let (a, b) = spec.clone().into_references();
    assert_eq!(
        a.as_ref().map(|r| r.name().as_bstr().to_string()),
        Some("refs/heads/main".into())
    );
    assert_eq!(
        b.as_ref().map(|r| r.name().as_bstr().to_string()),
        Some("refs/heads/g".into())
    );
    assert_eq!(spec.first_reference(), a.as_ref().map(|r| &r.inner));
    assert_eq!(spec.second_reference(), b.as_ref().map(|r| &r.inner));

    let spec = parse_spec_no_baseline("@", &repo).unwrap();
    assert_eq!(spec.second_reference(), None);
    assert_eq!(
        spec.first_reference().map(|r| r.name.as_bstr().to_string()),
        Some("HEAD".into())
    );
}

#[test]
fn missing_revision_keeps_reference_lookup_error_available_for_path_fallback() -> crate::Result {
    let repo = repo("complex_graph")?;
    let err = repo
        .rev_parse("README.md")
        .expect_err("missing revspec must fail before callers can inspect the error chain");

    assert!(
        err.is_not_found(),
        "rev-parse preserves the reference lookup classification"
    );
    let not_found = err
        .downcast_any_ref::<gix::refs::file::find::NotFound>()
        .expect("reference lookup failure remains available for downcasting after rev-parse");

    assert_eq!(
        not_found.name,
        std::path::Path::new("README.md"),
        "the missing reference carries the unresolved revspec for path fallback"
    );

    Ok(())
}

#[test]
fn missing_symbolic_referents_keep_their_name() -> crate::Result {
    let (repo, _keep) = crate::basic_rw_repo()?;
    std::fs::write(repo.git_dir().join("refs/heads/alias"), b"ref: refs/heads/missing\n")?;

    for revspec in ["alias", "alias..HEAD", "HEAD..alias", "alias...HEAD", "HEAD...alias"] {
        let err = repo.rev_parse(revspec).expect_err("the symbolic referent is missing");
        assert!(
            err.is_not_found(),
            "missing symbolic referents are classified as not found: {err:?}"
        );
        assert_eq!(
            err.downcast_any_ref::<gix::refs::file::find::NotFound>()
                .expect("the missing referent remains available for path fallback")
                .name,
            std::path::Path::new("refs/heads/missing"),
            "the missing reference name is not necessarily the input revspec"
        );
    }
    Ok(())
}

#[test]
fn both_missing_symbolic_referents_are_retained() -> crate::Result {
    let (repo, _keep) = crate::basic_rw_repo()?;
    std::fs::write(
        repo.git_dir().join("refs/heads/first"),
        b"ref: refs/heads/missing-first\n",
    )?;
    std::fs::write(
        repo.git_dir().join("refs/heads/second"),
        b"ref: refs/heads/missing-second\n",
    )?;

    for revspec in ["first..second", "first...second"] {
        let err = repo
            .rev_parse(revspec)
            .expect_err("both symbolic referents are missing");
        let missing_names: Vec<_> = err
            .iter_errors()
            .filter_map(|cause| cause.downcast_ref::<gix::refs::file::find::NotFound>())
            .map(|cause| cause.name.as_path())
            .collect();
        assert_eq!(
            missing_names,
            [
                std::path::Path::new("refs/heads/missing-first"),
                std::path::Path::new("refs/heads/missing-second"),
            ],
            "final spec conversion preserves both lookup failures"
        );
    }
    Ok(())
}

#[test]
fn missing_objects_are_classified_without_a_missing_reference() -> crate::Result {
    let (repo, _keep) = crate::basic_rw_repo()?;
    let mut missing_commit_id = repo.object_hash().null();
    missing_commit_id.as_mut_slice()[0] = 1;
    repo.reference(
        "refs/heads/missing-object",
        missing_commit_id,
        gix::refs::transaction::PreviousValue::Any,
        "",
    )?;

    std::fs::write(
        repo.git_dir().join("refs/heads/alias"),
        b"ref: refs/heads/missing-object\n",
    )?;

    for revspec in ["missing-object^{object}", "missing-object:README.md", "alias"] {
        let err = repo.rev_parse(revspec).expect_err("the referenced object is missing");
        assert!(
            err.is_not_found(),
            "object lookup failures retain their classification: {err}"
        );
        assert!(
            err.downcast_any_ref::<gix::refs::file::find::NotFound>().is_none(),
            "a missing object must not trigger missing-reference path fallback: {err}"
        );
    }
    Ok(())
}

#[test]
fn bad_objects_are_valid_until_they_are_actually_read_from_the_odb() {
    {
        let repo = repo("blob.bad").unwrap();
        assert_eq!(
            parse_spec("e328", &repo).unwrap(),
            Spec::from_id(hex_to_id_sha1_only("e32851d29feb48953c6f40b2e06d630a3c49608a").attach(&repo)),
            "we are able to return objects even though they are 'bad' when trying to decode them, like git",
        );
        let err = parse_spec("e328^{object}", &repo).unwrap_err();
        let cause = err
            .probable_cause()
            .downcast_ref::<gix_error::ValidationError>()
            .expect("invalid object kinds are classified as validation failures");
        assert_eq!(
            (
                cause.message.as_ref(),
                cause.input.as_ref().map(|input| input.as_slice())
            ),
            ("Unknown object kind", Some(b"bad".as_slice())),
            "Now we enforce the object to exist and be valid, as ultimately it wants to match with a certain type"
        );
        insta::assert_snapshot!(normalize_repo_path(&format!("{err:#?}"), &repo), @r#"
        delegate.peel_until(ValidObject) failed: "{object}"
        |
        └─ Could not read loose object, "path"="$GIT_DIR/objects/e3/2851d29feb48953c6f40b2e06d630a3c49608a"
        |
        └─ The object header contained an unknown object kind.
        |
        └─ Unknown object kind: "bad"
        "#);
    }

    {
        let repo = repo("blob.corrupt").unwrap();
        assert_eq!(
            parse_spec("cafea", &repo).unwrap(),
            Spec::from_id(hex_to_id_sha1_only("cafea31147e840161a1860c50af999917ae1536b").attach(&repo))
        );
        let err = parse_spec("cafea^{object}", &repo).unwrap_err();
        insta::assert_snapshot!(normalize_repo_path(&format!("{err:#?}"), &repo), @r#"
        delegate.peel_until(ValidObject) failed: "{object}"
        |
        └─ Could not read loose object, "path"="$GIT_DIR/objects/ca/fea31147e840161a1860c50af999917ae1536b"
        |
        └─ Could not decode zip stream
        |
        └─ Invalid input data
        "#);
    }
}

#[test]
fn access_blob_through_tree() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    let actual = parse_spec("0000000000cdc:a0blgqsjc", &repo).unwrap();
    assert_eq!(
        actual,
        Spec::from_id(hex_to_id_sha1_only("0000000000b36b6aa7ea4b75318ed078f55505c3").attach(&repo))
    );
    assert_eq!(
        actual.path_and_mode().expect("set"),
        ("a0blgqsjc".into(), gix_object::tree::EntryKind::Blob.into()),
        "we capture tree-paths"
    );

    let err = parse_spec("0000000000cdc:missing", &repo).unwrap_err();
    insta::assert_debug_snapshot!(err, @r#"
    delegate.peel_until(Path("missing")) failed
    |
    └─ Could not find path "missing" in tree 0000000000c of parent object 0000000000c
    "#);
    assert_eq!(
        err.probable_cause().to_string(),
        "Could not find path \"missing\" in tree 0000000000c of parent object 0000000000c"
    );
}

#[test]
fn invalid_head() {
    let repo = repo("invalid-head").unwrap();
    let err = parse_spec("HEAD:file", &repo).unwrap_err();
    insta::assert_debug_snapshot!(err, @r#"
    delegate.peel_until(Path("file")) failed
    |
    └─ Could not peel 'HEAD' to obtain its target
        |
        └─ The ref partially named "refs/heads/main" could not be found
        |   |
        |   └─ Reference or object not found
        |
        └─ Couldn't get object at internal index 0
    "#);

    let err = parse_spec("HEAD", &repo).unwrap_err();
    assert!(
        err.is_not_found(),
        "final conversion retains the deferred lookup failure"
    );
    insta::assert_debug_snapshot!(err, @r#"
    The rev-spec is malformed and misses a ref name
    |
    └─ Could not peel 'HEAD' to obtain its target
    |
    └─ The ref partially named "refs/heads/main" could not be found
    "#);
}

#[test]
fn empty_tree_as_full_name() {
    let repo = repo("complex_graph").unwrap();
    let empty_tree_id = repo.object_hash().empty_tree();
    assert_eq!(
        parse_spec(empty_tree_id.to_string(), &repo).unwrap(),
        Spec::from_id(empty_tree_id.attach(&repo))
    );
}
