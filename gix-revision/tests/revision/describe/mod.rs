use gix_error::Exn;
use gix_object::bstr::ByteSlice;
use gix_revision::{
    describe,
    describe::{Error, Outcome},
};
use std::{borrow::Cow, path::PathBuf};

use crate::hex_to_id;

mod format;

fn run_test(
    transform_odb: impl FnOnce(gix_odb::Handle) -> gix_odb::Handle,
    options: impl Fn(gix_hash::ObjectId) -> gix_revision::describe::Options<'static>,
    run_assertions: impl Fn(
        Result<Option<Outcome<'static>>, Exn<Error>>,
        gix_hash::ObjectId,
    ) -> Result<(), gix_error::Error>,
) -> Result<(), gix_error::Error> {
    let store = odb_at(".");
    let store = transform_odb(store);
    let commit_id = hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b");
    for use_commitgraph in [false, true] {
        let cache = use_commitgraph
            .then(|| gix_commitgraph::Graph::from_info_dir(&store.store_ref().path().join("info")).ok())
            .flatten();
        let mut graph = gix_revision::Graph::new(&store, cache.as_ref());
        run_assertions(
            gix_revision::describe(&commit_id, &mut graph, options(commit_id)),
            commit_id,
        )?;
    }
    Ok(())
}

#[test]
fn option_none_if_no_tag_found() -> Result<(), gix_error::Error> {
    run_test(
        std::convert::identity,
        |_| Default::default(),
        |res, _id| {
            assert!(res?.is_none(), "cannot find anything if there's no candidate");
            Ok(())
        },
    )
}

#[test]
fn fallback_if_configured_in_options_but_no_candidate_or_names() -> Result<(), gix_error::Error> {
    run_test(
        std::convert::identity,
        |_| describe::Options {
            fallback_to_oid: true,
            ..Default::default()
        },
        |res, _id| {
            let res = res?.expect("fallback active");
            assert!(res.name.is_none(), "no name can be found");
            assert_eq!(res.depth, 0, "just a default, not relevant as there is no name");
            assert_eq!(
                res.commits_seen, 0,
                "a traversal is isn't performed as name map is empty, and that's the whole point"
            );
            assert_eq!(res.into_format(7).to_string(), "01ec18a");
            Ok(())
        },
    )
}

#[test]
fn fallback_if_configured_in_options_and_max_candidates_zero() -> Result<(), gix_error::Error> {
    run_test(
        std::convert::identity,
        |_| describe::Options {
            fallback_to_oid: true,
            max_candidates: 0,
            ..Default::default()
        },
        |res, _id| {
            let res = res?.expect("fallback active");
            assert!(res.name.is_none(), "no name can be found");
            assert_eq!(res.depth, 0, "just a default, not relevant as there is no name");
            assert_eq!(res.commits_seen, 0, "we don't do any traversal");
            assert_eq!(res.into_format(7).to_string(), "01ec18a");
            Ok(())
        },
    )
}

#[test]
fn not_enough_candidates() -> Result<(), gix_error::Error> {
    let name = Cow::Borrowed(b"at-c5".as_bstr());
    run_test(
        std::convert::identity,
        |_| describe::Options {
            name_by_oid: vec![
                (hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), name.clone()),
                (
                    hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"),
                    b"at-b1c1".as_bstr().into(),
                ),
            ]
            .into_iter()
            .collect(),
            max_candidates: 1,
            ..Default::default()
        },
        |res, id| {
            let res = res?.expect("candidate found");
            assert_eq!(res.name, Some(name.clone()), "it finds the youngest/most-recent name");
            assert_eq!(res.id, id);
            assert_eq!(res.commits_seen, 6, "it has to traverse commits");
            assert_eq!(
                res.depth, 3,
                "it calculates the final number of commits even though it aborted early"
            );
            Ok(())
        },
    )
}

#[test]
fn typical_usecases() -> Result<(), gix_error::Error> {
    let name = Cow::Borrowed(b"main".as_bstr());
    run_test(
        std::convert::identity,
        |id| describe::Options {
            name_by_oid: vec![(id, name.clone())].into_iter().collect(),
            max_candidates: 0,
            ..Default::default()
        },
        |res, id| {
            let res = res?.expect("candidate found");
            assert_eq!(
                res.name,
                Some(name.clone()),
                "this is an exact match, and it's found despite max-candidates being 0 (one lookup is always performed)"
            );
            assert_eq!(res.id, id);
            assert_eq!(res.depth, 0);
            assert_eq!(res.commits_seen, 0);
            Ok(())
        },
    )?;

    let name = Cow::Borrowed(b"at-c5".as_bstr());
    run_test(
        std::convert::identity,
        |_| describe::Options {
            name_by_oid: vec![
                (hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), name.clone()),
                (
                    hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"),
                    b"at-b1c1".as_bstr().into(),
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        },
        |res, id| {
            let res = res?.expect("candidate found");
            assert_eq!(
                res.name,
                Some(name.clone()),
                "a match to a tag 1 commit away with 2 commits on the other side of the merge/head"
            );
            assert_eq!(res.id, id);
            assert_eq!(res.depth, 3);
            assert_eq!(res.commits_seen, 6);
            Ok(())
        },
    )?;

    run_test(
        std::convert::identity,
        |_| describe::Options {
            name_by_oid: vec![
                (hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), name.clone()),
                (
                    hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"),
                    b"at-b1c1".as_bstr().into(),
                ),
            ]
            .into_iter()
            .collect(),
            first_parent: true,
            ..Default::default()
        },
        |res, id| {
            let res = res?.expect("candidate found");
            assert_eq!(res.name, Some(name.clone()));
            assert_eq!(res.id, id);
            assert_eq!(res.depth, 1);
            assert_eq!(res.commits_seen, 2);
            assert_eq!(res.into_format(7).to_string(), "at-c5-1-g01ec18a");
            Ok(())
        },
    )
}

#[test]
fn shallow_yields_no_result_if_provided_refs_are_in_truncated_part_of_history() -> Result<(), gix_error::Error> {
    run_test(
        |_| odb_at("shallow-1-clone"),
        |_| describe::Options {
            name_by_oid: vec![(
                hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"),
                Cow::Borrowed(b"at-c5".as_bstr()),
            )]
            .into_iter()
            .collect(),
            first_parent: true,
            ..Default::default()
        },
        |res, _id| {
            let res = res?;
            assert!(
                res.is_none(),
                "no candidate found on truncated history, and it doesn't crash"
            );
            Ok(())
        },
    )
}

#[test]
fn shallow_yields_result_if_refs_are_available() -> Result<(), gix_error::Error> {
    let name = Cow::Borrowed(b"at-c5".as_bstr());
    run_test(
        |_| odb_at("shallow-2-clone"),
        |_| describe::Options {
            name_by_oid: vec![(hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), name.clone())]
                .into_iter()
                .collect(),
            first_parent: true,
            ..Default::default()
        },
        |res, id| {
            let res = res?.expect("found candidate");
            assert_eq!(res.name, Some(name.clone()));
            assert_eq!(res.id, id);
            assert_eq!(res.depth, 1);
            assert_eq!(res.commits_seen, 2);
            assert_eq!(res.into_format(7).to_string(), "at-c5-1-g01ec18a");
            Ok(())
        },
    )
}

fn odb_at(name: &str) -> gix_odb::Handle {
    gix_odb::at(fixture_path().join(name).join(".git/objects")).unwrap()
}

fn fixture_path() -> PathBuf {
    gix_testtools::scripted_fixture_read_only("make_repo_with_branches.sh").unwrap()
}
