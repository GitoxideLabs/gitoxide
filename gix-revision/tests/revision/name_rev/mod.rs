use bstr::ByteSlice;
use gix_error::Exn;
use gix_revision::{
    name_rev,
    name_rev::{Error, Outcome, Tip},
};
use std::{borrow::Cow, path::PathBuf};

use crate::hex_to_id;

fn run_test(
    options: impl Fn(gix_hash::ObjectId) -> gix_revision::name_rev::Options<'static>,
    run_assertions: impl Fn(
        Result<Option<Outcome<'static>>, Exn<Error>>,
        gix_hash::ObjectId,
    ) -> Result<(), gix_error::Error>,
) -> Result<(), gix_error::Error> {
    let store = odb_at(".");
    let commit_id = hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b");
    for use_commitgraph in [false, true] {
        let cache = use_commitgraph
            .then(|| gix_commitgraph::Graph::from_info_dir(&store.store_ref().path().join("info")).ok())
            .flatten();
        let mut graph = gix_revision::Graph::new(&store, cache.as_ref());
        run_assertions(
            gix_revision::name_rev(&commit_id, &mut graph, options(commit_id)),
            commit_id,
        )?;
    }
    Ok(())
}

#[test]
fn direct_tip_match() -> Result<(), gix_error::Error> {
    run_test(
        |id| name_rev::Options {
            tips: vec![tip(id, "main", false)],
            ..Default::default()
        },
        |res, id| {
            let res = res?.expect("name found");
            assert_eq!(res.id, id);
            assert_eq!(res.name.as_deref(), Some("main".as_bytes().as_bstr()));
            assert_eq!(res.into_format(7).to_string(), "main");
            Ok(())
        },
    )
}

#[test]
fn first_parent_ancestor_is_named_by_generation() -> Result<(), gix_error::Error> {
    run_test(
        |id| name_rev::Options {
            tips: vec![tip(id, "main", false)],
            ..Default::default()
        },
        |_res, _id| {
            let store = odb_at(".");
            let mut graph = gix_revision::Graph::new(&store, None);
            let res = gix_revision::name_rev(
                &hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"),
                &mut graph,
                name_rev::Options {
                    tips: vec![tip(
                        hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"),
                        "main",
                        false,
                    )],
                    ..Default::default()
                },
            )?
            .expect("name found");
            assert_eq!(res.into_format(7).to_string(), "main~1");
            Ok(())
        },
    )
}

#[test]
fn side_parent_paths_are_preserved() -> Result<(), gix_error::Error> {
    run_test(
        |id| name_rev::Options {
            tips: vec![tip(id, "main", false)],
            ..Default::default()
        },
        |_res, _id| {
            let store = odb_at(".");
            let mut graph = gix_revision::Graph::new(&store, None);
            let res = gix_revision::name_rev(
                &hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"),
                &mut graph,
                name_rev::Options {
                    tips: vec![tip(
                        hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"),
                        "main",
                        false,
                    )],
                    ..Default::default()
                },
            )?
            .expect("name found");
            assert_eq!(res.into_format(7).to_string(), "main^2~1");
            Ok(())
        },
    )
}

#[test]
fn tags_are_preferred_over_branches() -> Result<(), gix_error::Error> {
    run_test(
        |id| name_rev::Options {
            tips: vec![
                tip(id, "main", false),
                tip(
                    hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"),
                    "tags/at-c5",
                    true,
                ),
            ],
            ..Default::default()
        },
        |_res, _id| {
            let store = odb_at(".");
            let mut graph = gix_revision::Graph::new(&store, None);
            let res = gix_revision::name_rev(
                &hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"),
                &mut graph,
                name_rev::Options {
                    tips: vec![
                        tip(hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"), "main", false),
                        tip(
                            hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"),
                            "tags/at-c5",
                            true,
                        ),
                    ],
                    ..Default::default()
                },
            )?
            .expect("name found");
            assert_eq!(res.into_format(7).to_string(), "tags/at-c5");
            Ok(())
        },
    )
}

#[test]
fn fallback_if_configured_but_no_name_matches() -> Result<(), gix_error::Error> {
    run_test(
        |_| name_rev::Options {
            fallback_to_oid: true,
            ..Default::default()
        },
        |res, _id| {
            let res = res?.expect("fallback active");
            assert!(res.name.is_none(), "no symbolic name was found");
            assert_eq!(res.into_format(7).to_string(), "01ec18a");
            Ok(())
        },
    )
}

fn tip(id: gix_hash::ObjectId, name: &'static str, from_tag: bool) -> Tip<'static> {
    Tip {
        id,
        name: Cow::Borrowed(name.as_bytes().as_bstr()),
        taggerdate: 0,
        from_tag,
        deref: false,
    }
}

fn odb_at(name: &str) -> gix_odb::Handle {
    gix_odb::at(fixture_path().join(name).join(".git/objects")).unwrap()
}

fn fixture_path() -> PathBuf {
    gix_testtools::scripted_fixture_read_only("make_repo_with_branches.sh").unwrap()
}
