use super::*;
use crate::util::{commit_graph, fixture, git_rev_list, odb_at};
use std::{cell::Cell, rc::Rc};

fn assert_simple_repo_graph(repo_dir: &std::path::Path) -> crate::Result {
    let graph = git_graph(repo_dir)?;

    insta::allow_duplicates! {
        insta::assert_snapshot!(graph, @r"
        *-.   Oid(1)  (HEAD -> main) merge
        |\ \  
        | | * Oid(2)  (branch2) b2c2
        | | * Oid(3)  b2c1
        | * | Oid(4)  (branch1) b1c2
        | * | Oid(5)  b1c1
        | |/  
        * / Oid(6)  c5
        |/  
        * Oid(7)  c4
        * Oid(8)  c3
        * Oid(9)  c2
        * Oid(10)  c1
        ")
    }

    Ok(())
}

#[test]
fn disjoint_hidden_and_interesting() -> crate::Result {
    let (repo_dir, odb) = named_fixture("make_repos.sh", "disjoint_branches")?;

    insta::assert_snapshot!(git_graph(&repo_dir)?, @r"
        * Oid(1)  (HEAD -> disjoint) b3
        * Oid(2)  b2
        * Oid(3)  b1
        * Oid(4)  (main) a3
        * Oid(5)  a2
        * Oid(6)  a1
        ");

    let tip = hex_to_id("e07cf1277ff7c43090f1acfc85a46039e7de1272"); // b3
    let hidden = [hex_to_id("b5665181bf4c338ab16b10da0524d81b96aff209")]; // a3
    let expected = [
        tip,
        hex_to_id("94cf3f3a4c782b672173423e7a4157a02957dd48"), // b2
        hex_to_id("34e5ff5ce3d3ba9f0a00d11a7fad72551fff0861"), // b1
    ];

    for sorting in all_sortings() {
        let result = traverse_both([tip], &odb, sorting, Parents::All, hidden)?;
        assert_eq!(result, expected, "sorting = {sorting:?}");
    }
    Ok(())
}

#[test]
fn all_hidden() -> crate::Result {
    let (_repo_dir, odb) = named_fixture("make_repos.sh", "disjoint_branches")?;
    let tips = [
        hex_to_id("e07cf1277ff7c43090f1acfc85a46039e7de1272"), // b3
        hex_to_id("b5665181bf4c338ab16b10da0524d81b96aff209"), // a3
    ];
    // The start positions are also declared hidden, so nothing should be visible.
    let hidden = tips;

    for sorting in all_sortings() {
        let result = traverse_both(tips, &odb, sorting, Parents::All, hidden)?;
        assert!(result.is_empty(), "sorting = {sorting:?}");
    }
    Ok(())
}

#[test]
fn some_hidden_and_all_hidden() -> crate::Result {
    let (repo_dir, odb) = named_fixture("make_repos.sh", "simple")?;

    assert_simple_repo_graph(&repo_dir)?;

    // Test: Hidden has to catch up with non-hidden
    let tip_c2 = hex_to_id("ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83");
    let hidden_c5 = hex_to_id("0edb95c0c0d9933d88f532ec08fcd405d0eee882");

    for sorting in all_sortings() {
        let result = traverse_both([tip_c2], &odb, sorting, Parents::All, [hidden_c5])?;
        assert!(
            result.is_empty(),
            "c2 is reachable from hidden c5, sorting = {sorting:?}"
        );
    }

    // Test: merge tip with two branch tips hidden
    let tip_merge = hex_to_id("f49838d84281c3988eeadd988d97dd358c9f9dc4");
    let hidden_branches = [
        hex_to_id("48e8dac19508f4238f06c8de2b10301ce64a641c"), // b2c2
        hex_to_id("66a309480201c4157b0eae86da69f2d606aadbe7"), // b1c2
    ];
    let expected = [
        tip_merge,
        hex_to_id("0edb95c0c0d9933d88f532ec08fcd405d0eee882"), // c5
    ];

    for sorting in all_sortings() {
        let result = traverse_both([tip_merge], &odb, sorting, Parents::All, hidden_branches)?;
        assert_eq!(result, expected, "sorting = {sorting:?}");
    }

    // Test: single-parent mode with hidden catching up
    let tip_b1c1 = hex_to_id("80947acb398362d8236fcb8bf0f8a9dac640583f");
    let hidden_merge = hex_to_id("f49838d84281c3988eeadd988d97dd358c9f9dc4");

    let result = traverse_both([tip_b1c1], &odb, Sorting::BreadthFirst, Parents::First, [hidden_merge])?;
    assert!(result.is_empty(), "b1c1 is reachable from hidden merge");

    Ok(())
}

fn hidden_bug_repo(name: &str) -> crate::Result<(std::path::PathBuf, gix_odb::Handle)> {
    let dir = fixture("make_repo_for_hidden_bug.sh")?;
    let repo_path = dir.join(name);
    let odb = odb_at(repo_path.join(".git").join("objects"))?;
    Ok((repo_path, odb))
}

#[test]
fn hidden_tip_with_longer_path_to_shared_ancestor() -> crate::Result {
    // Graph:
    //   A(tip) --> shared
    //            /
    //   H(hidden) --> X --> Y --> shared
    //
    // Expected: only A is returned (shared is reachable from H)
    let (repo_path, odb) = hidden_bug_repo("long_hidden_path")?;

    insta::assert_snapshot!(git_graph(&repo_path)?, @r"
            * Oid(1)  (HEAD -> main) A
            | * Oid(2)  (hidden_branch) H
            | * Oid(3)  X
            | * Oid(4)  Y
            |/  
            * Oid(5)  shared
        "
    );

    let commits = parse_commit_names(&repo_path)?;
    let tip_a = commits["A"];
    let hidden_h = commits["H"];
    let shared = commits["shared"];

    let expected = vec![tip_a];

    for sorting in all_sortings() {
        let result = traverse([tip_a], &odb, sorting, Parents::All, [hidden_h])?;
        assert_eq!(
            result, expected,
            "sorting = {sorting:?}: 'shared' ({shared}) should NOT be returned because it's \
                     reachable from hidden tip H"
        );
    }

    // Verify against git
    let git_output = git_rev_list(&repo_path, &["main", "--not", "hidden_branch"])?;
    assert_eq!(git_output, expected, "git rev-list should show only A");

    Ok(())
}

#[test]
fn interesting_tip_with_longer_path_to_shared_ancestor() -> crate::Result {
    // Graph:
    //   A(tip) --> B --> C --> D(shared)
    //                        /
    //   H(hidden) --------->+
    //
    // Expected: A, B, C are returned (D is reachable from H)
    let (repo_path, odb) = hidden_bug_repo("long_interesting_path")?;

    insta::assert_snapshot!(git_graph(&repo_path)?, @r"
            * Oid(1)  (HEAD -> main) A
            * Oid(2)  B
            * Oid(3)  C
            | * Oid(4)  (hidden_branch) H
            |/  
            * Oid(5)  D
        ");

    let commits = parse_commit_names(&repo_path)?;
    let tip_a = commits["A"];
    let hidden_h = commits["H"];
    let d = commits["D"];

    let expected: Vec<_> = ["A", "B", "C"].iter().map(|name| commits[*name]).collect();

    for sorting in all_sortings() {
        let result = traverse([tip_a], &odb, sorting, Parents::All, [hidden_h])?;
        assert_eq!(
            result, expected,
            "sorting = {sorting:?}: 'D' ({d}) should NOT be returned because it's \
                     reachable from hidden tip H"
        );
    }

    // Verify against git
    let git_output = git_rev_list(&repo_path, &["main", "--not", "hidden_branch"])?;
    assert_eq!(git_output, expected, "git rev-list should show A, B, C");

    Ok(())
}

#[test]
fn without_commit_graph_still_hides_single_visible_tip_correctly() -> crate::Result {
    let (repo_dir, odb) = named_fixture("make_repos.sh", "simple")?;
    assert_simple_repo_graph(&repo_dir)?;
    let tip_c2 = hex_to_id("ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83");
    let hidden_c5 = hex_to_id("0edb95c0c0d9933d88f532ec08fcd405d0eee882");

    let result: Vec<_> = Simple::new([tip_c2], &odb)
        .sorting(Sorting::BreadthFirst)?
        .parents(Parents::All)
        .commit_graph(None)
        .hide([hidden_c5])?
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    assert!(result.is_empty(), "c2 is reachable from hidden c5");
    Ok(())
}

#[test]
fn commit_graph_reduces_odb_lookups_when_hidden_tips_cover_visible_tips() -> crate::Result {
    let (repo_dir, odb) = named_fixture("make_repos.sh", "simple")?;
    assert_simple_repo_graph(&repo_dir)?;
    let tips = [
        hex_to_id("ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83"), // c2
        hex_to_id("33aa07785dd667c0196064e3be3c51dd9b4744ef"), // c3
    ];
    let hidden_c5 = hex_to_id("0edb95c0c0d9933d88f532ec08fcd405d0eee882");
    let without_graph_lookups = Rc::new(Cell::new(0usize));
    let with_graph_lookups = Rc::new(Cell::new(0usize));

    let without_graph: Vec<_> = Simple::new(
        tips,
        CountingFind {
            inner: &odb,
            lookups: Rc::clone(&without_graph_lookups),
        },
    )
    .sorting(Sorting::BreadthFirst)?
    .parents(Parents::All)
    .commit_graph(None)
    .hide([hidden_c5])?
    .map(|res| res.map(|info| info.id))
    .collect::<Result<Vec<_>, _>>()?;

    let with_graph: Vec<_> = Simple::new(
        tips,
        CountingFind {
            inner: &odb,
            lookups: Rc::clone(&with_graph_lookups),
        },
    )
    .sorting(Sorting::BreadthFirst)?
    .parents(Parents::All)
    .commit_graph(commit_graph(odb.store_ref()))
    .hide([hidden_c5])?
    .map(|res| res.map(|info| info.id))
    .collect::<Result<Vec<_>, _>>()?;

    assert!(
        without_graph.is_empty(),
        "both starting tips are reachable from hidden c5"
    );
    assert_eq!(
        without_graph, with_graph,
        "commit-graph must not change traversal results"
    );
    assert!(
        with_graph_lookups.get() < without_graph_lookups.get(),
        "commit-graph should reduce object database lookups for hidden painting",
    );
    Ok(())
}

#[test]
fn hide_and_commit_graph_call_order_do_not_matter() -> crate::Result {
    let (repo_dir, odb) = named_fixture("make_repos.sh", "simple")?;
    assert_simple_repo_graph(&repo_dir)?;
    let tip_merge = hex_to_id("f49838d84281c3988eeadd988d97dd358c9f9dc4");
    let hidden_branches = [
        hex_to_id("48e8dac19508f4238f06c8de2b10301ce64a641c"),
        hex_to_id("66a309480201c4157b0eae86da69f2d606aadbe7"),
    ];

    let hide_then_graph: Vec<_> = Simple::new([tip_merge], &odb)
        .sorting(Sorting::BreadthFirst)?
        .parents(Parents::All)
        .hide(hidden_branches)?
        .commit_graph(commit_graph(odb.store_ref()))
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    let graph_then_hide: Vec<_> = Simple::new([tip_merge], &odb)
        .sorting(Sorting::BreadthFirst)?
        .parents(Parents::All)
        .commit_graph(commit_graph(odb.store_ref()))
        .hide(hidden_branches)?
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(hide_then_graph, graph_then_hide);
    Ok(())
}

struct CountingFind<'a> {
    inner: &'a gix_odb::Handle,
    lookups: Rc<Cell<usize>>,
}

impl gix_object::Find for CountingFind<'_> {
    fn try_find<'a>(
        &self,
        id: &gix_hash::oid,
        buffer: &'a mut Vec<u8>,
    ) -> Result<Option<gix_object::Data<'a>>, gix_object::find::Error> {
        self.lookups.set(self.lookups.get() + 1);
        gix_object::Find::try_find(self.inner, id, buffer)
    }
}
