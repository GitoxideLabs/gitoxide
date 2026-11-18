use bstr::BStr;
use gix_error::{Result, TestResult};
use gix_merge::{
    blob::builtin_driver::text::Labels,
    submodule::Query,
    tree::{Context, ResolveWith, TreatAsUnresolved},
};
use gix_object::{Find, Write};

use super::{baseline, basic_merge_options, new_blob_merge_platform, new_diff_resource_cache};

fn fixture() -> gix_testtools::Result<Option<std::path::PathBuf>> {
    gix_testtools::scripted_fixture_read_only_with_git_version("tree-submodule-history.sh", |version| {
        version >= (2, 38, 0)
    })
}

fn ids(path: &std::path::Path) -> gix_testtools::Result<Vec<gix_hash::ObjectId>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(|line| gix_hash::ObjectId::from_hex(line.as_bytes()))
        .collect::<std::result::Result<_, _>>()?)
}

fn submodule_objects(root: &std::path::Path) -> gix_testtools::Result<Option<gix_odb::Handle>> {
    let objects = root.join(if root.ends_with("old-form") {
        "module/.git/objects"
    } else {
        ".git/modules/module/objects"
    });
    Ok(objects
        .exists()
        .then(|| gix_odb::at(objects, gix_testtools::object_hash()))
        .transpose()?)
}

fn query(objects: Option<&gix_odb::Handle>, request: Query<'_>) -> Result<Option<bool>> {
    let Some(objects) = objects else {
        return Ok(None);
    };
    Ok(Some(match request {
        Query::CommitsAvailable(commit_ids) => {
            let mut buffer = Vec::new();
            for commit_id in commit_ids {
                let Some(commit) = objects.try_find(commit_id, &mut buffer)? else {
                    return Ok(Some(false));
                };
                if commit.kind != gix_object::Kind::Commit {
                    return Ok(Some(false));
                }
                commit.decode()?;
            }
            true
        }
        Query::IsAncestor { ancestor, descendant } => gix_revision::merge_base::is_ancestor(
            ancestor.to_owned(),
            descendant.to_owned(),
            &mut gix_revwalk::Graph::new(objects, None),
        )?,
    }))
}

fn tree_merge<'a>(
    root: &std::path::Path,
    objects: &'a (impl gix_object::FindObjectOrHeader + Write),
    tree_ids: [gix_hash::ObjectId; 3],
    context: Context<'_>,
    options: gix_merge::tree::Options,
) -> Result<gix_merge::tree::Outcome<'a>> {
    gix_merge::tree(
        &tree_ids[0],
        &tree_ids[1],
        &tree_ids[2],
        Labels::default(),
        objects,
        |buf| objects.write_buf(gix_object::Kind::Blob, buf),
        &mut Default::default(),
        &mut new_diff_resource_cache(root),
        &mut new_blob_merge_platform(root, None),
        context,
        options,
    )
}

#[test]
fn provided_submodule_history_matches_git_trees_and_index_stages() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(fixture.join("cases"))?;
    for layout in cases.lines().filter(|layout| *layout != "malformed-unavailable") {
        let root = fixture.join(layout);
        let hash = gix_testtools::object_hash();
        let objects = gix_odb::memory::Proxy::new(gix_odb::at(root.join(".git/objects"), hash)?, hash);
        let submodule = submodule_objects(&root)?;
        let commit_ids = ids(&root.join("commits"))?;
        for (current, other, direction) in [(1, 2, "A-B"), (2, 1, "B-A")] {
            let mut requests = Vec::new();
            let mut provide = |path: &BStr, request: Query<'_>| {
                requests.push((path.to_owned(), matches!(request, Query::CommitsAvailable(_))));
                query(submodule.as_ref(), request)
            };
            let context = Context {
                submodule: Some(&mut provide),
            };
            let actual = gix_merge::commit(
                commit_ids[current],
                commit_ids[other],
                Labels::default(),
                &mut gix_revwalk::Graph::new(&objects, None),
                &mut new_diff_resource_cache(&root),
                &mut new_blob_merge_platform(&root, None),
                &objects,
                &mut |commit_id| commit_id.to_hex_with_len(7).to_string(),
                context,
                basic_merge_options(),
            );

            if layout == "corrupt-parent" {
                assert!(
                    actual.is_err(),
                    "{direction}: available commit with a missing parent propagates its ancestry error"
                );
                continue;
            }
            let mut actual = actual?.tree_merge;
            let expected =
                baseline::parse_merge_info(std::fs::read_to_string(root.join(format!("{direction}.merge-info")))?);
            let expected_conflict = std::fs::read_to_string(root.join(format!("{direction}.status")))?.trim() == "1";
            let tree_id = actual.tree.write(|tree| objects.write(tree))?;
            let index = actual
                .index
                .to_index(TreatAsUnresolved::git(), None, &objects, Default::default())?;
            index.verify_entries()?;
            let mut expected_index = gix_index::State::from_tree(&expected.merged_tree, &objects, Default::default())?;
            if let Some(conflicts) = &expected.conflicts {
                baseline::apply_git_index_entries(conflicts, &mut expected_index);
            }
            assert_eq!(
                tree_id, expected.merged_tree,
                "{layout} {direction}: callback-driven tree matches Git"
            );
            assert_eq!(
                baseline::debug_entries(&index),
                baseline::debug_entries(&expected_index),
                "{layout} {direction}: callback-driven conflict stages match Git"
            );
            assert_eq!(
                actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                expected_conflict,
                "{layout} {direction}: callback-driven verdict matches Git"
            );
            assert!(
                actual
                    .conflicts
                    .iter()
                    .all(|conflict| conflict.content_merge().is_none()),
                "{layout} {direction}: gitlinks do not enter blob merging"
            );
            if matches!(
                layout,
                "equal" | "equal-malformed" | "one-side" | "add-add" | "type-file-base" | "type-link-base" | "rename"
            ) {
                assert!(
                    requests.is_empty(),
                    "{layout} {direction}: trivial, missing-base and incompatible-base merges do not inspect submodule history"
                );
            } else {
                assert!(
                    requests.first().is_some_and(|(_, available)| *available),
                    "{layout} {direction}: named commit availability is checked before ancestry"
                );
                assert!(
                    requests.iter().all(|(path, _)| path == "module"),
                    "{layout} {direction}: queries identify the original source path"
                );
            }
            if matches!(
                layout,
                "missing-tip"
                    | "missing-base"
                    | "wrong-kind-tip"
                    | "wrong-kind-base"
                    | "missing-tip-corrupt-parent"
                    | "unavailable"
            ) {
                assert_eq!(
                    requests.len(),
                    1,
                    "{layout} {direction}: unavailable inputs prevent every ancestry walk"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn forward_history_wins_before_tree_favor_and_virtual_base_policy() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    for layout in ["forward", "reverse"] {
        let root = fixture.join(layout);
        let hash = gix_testtools::object_hash();
        let objects = gix_odb::memory::Proxy::new(gix_odb::at(root.join(".git/objects"), hash)?, hash);
        let submodule = submodule_objects(&root)?;
        let tree_ids = ids(&root.join("trees"))?;
        for (current, other, direction) in [(1, 2, "A-B"), (2, 1, "B-A")] {
            let expected = ids(&root.join(format!("{direction}.tree")))?[0];
            for virtual_ancestor in [false, true] {
                for favor in [None, Some(ResolveWith::Ancestor), Some(ResolveWith::Ours)] {
                    let mut provide = |_: &BStr, request: Query<'_>| query(submodule.as_ref(), request);
                    let context = Context {
                        submodule: Some(&mut provide),
                    };
                    let mut options = basic_merge_options().tree_merge;
                    options.tree_conflicts = favor;
                    options.blob_merge.is_virtual_ancestor = virtual_ancestor;
                    let mut actual = tree_merge(
                        &root,
                        &objects,
                        [tree_ids[0], tree_ids[current], tree_ids[other]],
                        context,
                        options,
                    )?;
                    assert_eq!(
                        actual.tree.write(|tree| objects.write(tree))?,
                        expected,
                        "{layout} {direction}, favor={favor:?}, virtual={virtual_ancestor}: fast-forwarded gitlink is retained"
                    );
                    assert!(
                        !actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                        "{layout} {direction}, favor={favor:?}, virtual={virtual_ancestor}: fast-forward resolves before conflict policy"
                    );
                    assert!(
                        actual
                            .index
                            .to_index(TreatAsUnresolved::git(), None, &objects, Default::default())?
                            .entries()
                            .iter()
                            .all(|entry| entry.stage() == gix_index::entry::Stage::Unconflicted),
                        "{layout} {direction}: fast-forward leaves no unmerged stages"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn unavailable_callback_facts_preserve_the_unmerged_gitlink() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    for (layout, query_count) in [("forward", 4), ("reverse", 5)] {
        let root = fixture.join(layout);
        let hash = gix_testtools::object_hash();
        let objects = gix_odb::memory::Proxy::new(gix_odb::at(root.join(".git/objects"), hash)?, hash);
        let tree_ids = ids(&root.join("trees"))?;
        for unavailable_at in std::iter::once(None).chain((0..query_count).map(Some)) {
            let submodule = submodule_objects(&root)?;
            let mut calls = 0;
            let mut provide = |_: &BStr, request: Query<'_>| {
                let this_call = calls;
                calls += 1;
                if unavailable_at == Some(this_call) {
                    Ok(None)
                } else {
                    query(submodule.as_ref(), request)
                }
            };
            let context = Context {
                submodule: unavailable_at.map(|_| &mut provide as &mut gix_merge::submodule::QueryFn<'_>),
            };
            let mut actual = tree_merge(
                &root,
                &objects,
                [tree_ids[0], tree_ids[1], tree_ids[2]],
                context,
                basic_merge_options().tree_merge,
            )?;
            assert_eq!(
                actual.tree.write(|tree| objects.write(tree))?,
                tree_ids[1],
                "{layout}, unavailable query {unavailable_at:?}: unresolved gitlink keeps the current tree"
            );
            assert!(
                actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                "{layout}, unavailable query {unavailable_at:?}: lack of a fact must not resolve the merge"
            );
            let index = actual
                .index
                .to_index(TreatAsUnresolved::git(), None, &objects, Default::default())?;
            assert_eq!(
                index
                    .entries()
                    .iter()
                    .filter(|entry| entry.stage() != gix_index::entry::Stage::Unconflicted)
                    .count(),
                3,
                "{layout}, unavailable query {unavailable_at:?}: all three gitlink inputs remain unmerged"
            );
        }
    }
    Ok(())
}

#[test]
fn incomparable_history_retains_conflicts_until_the_tree_policy_selects_an_entry() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    let root = fixture.join("divergent");
    let hash = gix_testtools::object_hash();
    let objects = gix_odb::memory::Proxy::new(gix_odb::at(root.join(".git/objects"), hash)?, hash);
    let submodule = submodule_objects(&root)?;
    let tree_ids = ids(&root.join("trees"))?;
    for (current, other) in [(1, 2), (2, 1)] {
        for virtual_ancestor in [false, true] {
            for favor in [None, Some(ResolveWith::Ancestor), Some(ResolveWith::Ours)] {
                let mut provide = |_: &BStr, request: Query<'_>| query(submodule.as_ref(), request);
                let context = Context {
                    submodule: Some(&mut provide),
                };
                let mut options = basic_merge_options().tree_merge;
                options.tree_conflicts = favor;
                options.blob_merge.is_virtual_ancestor = virtual_ancestor;
                let mut actual = tree_merge(
                    &root,
                    &objects,
                    [tree_ids[0], tree_ids[current], tree_ids[other]],
                    context,
                    options,
                )?;
                let expected_tree_id = if favor == Some(ResolveWith::Ancestor) {
                    tree_ids[0]
                } else {
                    tree_ids[current]
                };
                assert_eq!(
                    actual.tree.write(|tree| objects.write(tree))?,
                    expected_tree_id,
                    "current={current}, favor={favor:?}, virtual={virtual_ancestor}: policy selects the retained gitlink"
                );
                assert_eq!(
                    actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                    favor.is_none(),
                    "current={current}, favor={favor:?}, virtual={virtual_ancestor}: incomparable history alone remains unresolved"
                );
                let index = actual
                    .index
                    .to_index(TreatAsUnresolved::git(), None, &objects, Default::default())?;
                let unmerged = index
                    .entries()
                    .iter()
                    .filter(|entry| entry.stage() != gix_index::entry::Stage::Unconflicted)
                    .count();
                assert_eq!(
                    unmerged,
                    if favor.is_none() { 3 } else { 0 },
                    "current={current}, favor={favor:?}, virtual={virtual_ancestor}: selected entries and stage plans agree"
                );
            }
        }
    }
    Ok(())
}
