use gix::{prelude::ObjectIdExt, revision::Spec};
pub use util::*;

use crate::util::hex_to_id;

mod ambiguous;
mod regex;
mod util;

mod reflog;
mod traverse;

mod peel;

mod sibling_branch {
    use crate::{
        revision::spec::from_bytes::{parse_spec, repo},
        util::hex_to_id,
    };

    #[test]
    fn push_and_upstream() -> crate::Result {
        let repo = repo("complex_graph").unwrap();
        for op in ["upstream", "push"] {
            for branch in ["", "main"] {
                let actual = parse_spec(format!("{branch}@{{{op}}}"), &repo)?;
                assert_eq!(
                    actual.first_reference().expect("set").name.as_bstr(),
                    "refs/remotes/origin/main"
                );
                assert_eq!(actual.second_reference(), None);
                assert_eq!(
                    actual.single().expect("just one"),
                    hex_to_id("55e825ebe8fd2ff78cad3826afb696b96b576a7e")
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
        util::hex_to_id,
    };

    #[test]
    fn at_stage() {
        let repo = repo("complex_graph").unwrap();
        let actual = parse_spec(":file", &repo).unwrap();
        assert_eq!(
            actual,
            Spec::from_id(hex_to_id("fe27474251f7f8368742f01fbd3bd5666b630a82").attach(&repo))
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
fn bad_objects_are_valid_until_they_are_actually_read_from_the_odb() {
    {
        let repo = repo("blob.bad").unwrap();
        assert_eq!(
            parse_spec("e328", &repo).unwrap(),
            Spec::from_id(hex_to_id("e32851d29feb48953c6f40b2e06d630a3c49608a").attach(&repo)),
            "we are able to return objects even though they are 'bad' when trying to decode them, like git",
        );
        let err = parse_spec("e328^{object}", &repo).unwrap_err();
        assert_eq!(
            format!("{:?}", err.probable_cause()),
            r#"InvalidObjectKind { kind: "bad" }"#,
            "Now we enforce the object to exist and be valid, as ultimately it wants to match with a certain type"
        );
        insta::assert_debug_snapshot!(err, @r#"
        delegate.peel_until(ValidObject) failed: {object}
        |
        └─ An error occurred while obtaining an object from the loose object store
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
            Spec::from_id(hex_to_id("cafea31147e840161a1860c50af999917ae1536b").attach(&repo))
        );
        let err = parse_spec("cafea^{object}", &repo).unwrap_err();
        insta::assert_snapshot!(format!("{err:#?}").replace('\\', "/").replace("windows", "unix"), @r"
        delegate.peel_until(ValidObject) failed: {object}
        |
        └─ An error occurred while obtaining an object from the loose object store
        |
        └─ decompression of loose object at 'tests/fixtures/generated-do-not-edit/make_rev_spec_parse_repos/2990428670-unix/blob.corrupt/objects/ca/fea31147e840161a1860c50af999917ae1536b' failed
        |
        └─ Could not decode zip stream, status was 'Invalid input data'
        |
        └─ Invalid input data
        ");
    }
}

#[test]
fn access_blob_through_tree() {
    let repo = repo("ambiguous_blob_tree_commit").unwrap();
    let actual = parse_spec("0000000000cdc:a0blgqsjc", &repo).unwrap();
    assert_eq!(
        actual,
        Spec::from_id(hex_to_id("0000000000b36b6aa7ea4b75318ed078f55505c3").attach(&repo))
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
        └─ Could not follow a single level of a symbolic reference
        |   |
        |   └─ The ref partially named "refs/heads/main" could not be found
        |
        └─ Couldn't get object at internal index 0
    "#);

    let err = parse_spec("HEAD", &repo).unwrap_err();
    // The head couldn't be peeled, and there is nothing left to consume, but no
    // object was resolved.
    insta::assert_debug_snapshot!(err, @"The rev-spec is malformed and misses a ref name");
}

#[test]
fn empty_tree_as_full_name() {
    let repo = repo("complex_graph").unwrap();
    assert_eq!(
        parse_spec("4b825dc642cb6eb9a060e54bf8d69288fbee4904", &repo).unwrap(),
        Spec::from_id(hex_to_id("4b825dc642cb6eb9a060e54bf8d69288fbee4904").attach(&repo))
    );
}
