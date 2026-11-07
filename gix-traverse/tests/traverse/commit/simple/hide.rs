use super::*;
use crate::Result;
use crate::util::{CountingFind, commit_graph, fixture, odb_at, rev_list_baseline};
use std::{cell::Cell, rc::Rc};

fn assert_simple_repo_graph(repo_dir: &std::path::Path) -> Result {
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
fn disjoint_hidden_and_interesting() -> gix_testtools::TestResult {
    let (repo_dir, odb) = named_fixture("make_repos.sh", "disjoint_branches")?;

    let tip = hex_to_id("e07cf1277ff7c43090f1acfc85a46039e7de1272"); // b3
    let hidden = [hex_to_id("b5665181bf4c338ab16b10da0524d81b96aff209")]; // a3
    let baseline = rev_list_baseline(&repo_dir, "rev-list")?;

    for sorting in all_sortings() {
        let result = traverse_both([tip], &odb, sorting, Parents::All, hidden)?;
        assert_eq!(
            result, baseline,
            "sorting = {sorting:?}: hiding main preserves every commit on the disjoint branch"
        );
    }
    Ok(())
}

#[test]
fn all_hidden() -> gix_testtools::TestResult {
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
fn some_hidden_and_all_hidden() -> gix_testtools::TestResult {
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

fn hidden_bug_repo(name: &str) -> Result<(std::path::PathBuf, gix_odb::Handle)> {
    let dir = fixture("make_repo_for_hidden_bug.sh")?;
    let repo_path = dir.join(name);
    let odb = odb_at(repo_path.join(".git").join("objects"))?;
    Ok((repo_path, odb))
}

#[test]
fn skewed_dates_do_not_expose_hidden_ancestors() -> gix_testtools::TestResult {
    let (repo_dir, odb) = hidden_bug_repo("skewed_dates")?;
    let commits = parse_commit_names(&repo_dir)?;
    for (visible, hidden, baseline_name) in [
        (
            vec![commits["visible"], commits["root"]],
            vec![commits["hidden"]],
            "rev-list",
        ),
        (
            vec![commits["visible"], commits["root"], commits["hidden"]],
            vec![commits["hidden"], commits["hidden"]],
            "rev-list-repeated-exclusions",
        ),
    ] {
        let baseline = rev_list_baseline(&repo_dir, baseline_name)?;
        for sorting in all_sortings() {
            for parents in [Parents::All, Parents::First] {
                assert_eq!(
                    traverse_both(visible.clone(), &odb, sorting, parents, hidden.clone())?,
                    baseline,
                    "{baseline_name}, {sorting:?}: dates must not limit hidden ancestry in either parent mode"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn skewed_hidden_boundaries_match_git() -> gix_testtools::TestResult {
    for (repo_name, visible, hidden) in [
        ("skewed_merge", vec!["visible"], vec!["hidden"]),
        ("skewed_lookahead", vec!["visible"], vec!["hidden"]),
        (
            "crossed_hidden_boundaries",
            vec!["visible-tip", "visible-middle"],
            vec!["hidden-base", "hidden-top"],
        ),
    ] {
        let (repo_dir, odb) = hidden_bug_repo(repo_name)?;
        let commits = parse_commit_names(&repo_dir)?;
        let baseline = rev_list_baseline(&repo_dir, "rev-list")?;
        let visible = visible.into_iter().map(|name| commits[name]).collect::<Vec<_>>();
        let hidden = hidden.into_iter().map(|name| commits[name]).collect::<Vec<_>>();
        for sorting in all_sortings() {
            for parents in [Parents::All, Parents::First] {
                assert_eq!(
                    traverse_both(visible.clone(), &odb, sorting, parents, hidden.clone())?,
                    baseline,
                    "{repo_name}, {sorting:?}: every hidden boundary applies to every visible path"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn hidden_tip_with_longer_path_to_shared_ancestor() -> gix_testtools::TestResult {
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

    let baseline = rev_list_baseline(&repo_path, "rev-list")?;

    for sorting in all_sortings() {
        let result = traverse([tip_a], &odb, sorting, Parents::All, [hidden_h])?;
        assert_eq!(
            result, baseline,
            "sorting = {sorting:?}: 'shared' ({shared}) should NOT be returned because it's \
                     reachable from hidden tip H"
        );
    }

    Ok(())
}

#[test]
fn interesting_tip_with_longer_path_to_shared_ancestor() -> gix_testtools::TestResult {
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

    let baseline = rev_list_baseline(&repo_path, "rev-list")?;

    for sorting in all_sortings() {
        let result = traverse([tip_a], &odb, sorting, Parents::All, [hidden_h])?;
        assert_eq!(
            result, baseline,
            "sorting = {sorting:?}: 'D' ({d}) should NOT be returned because it's \
                     reachable from hidden tip H"
        );
    }

    Ok(())
}

#[test]
fn without_commit_graph_still_hides_single_visible_tip_correctly() -> gix_testtools::TestResult {
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
        .collect::<std::result::Result<Vec<_>, _>>()?;

    assert!(result.is_empty(), "c2 is reachable from hidden c5");
    Ok(())
}

#[test]
fn commit_graph_reduces_odb_lookups_when_hidden_tips_cover_visible_tips() -> gix_testtools::TestResult {
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
    .collect::<std::result::Result<Vec<_>, _>>()?;

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
    .collect::<std::result::Result<Vec<_>, _>>()?;

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
fn hide_and_commit_graph_call_order_do_not_matter() -> gix_testtools::TestResult {
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
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let graph_then_hide: Vec<_> = Simple::new([tip_merge], &odb)
        .sorting(Sorting::BreadthFirst)?
        .parents(Parents::All)
        .commit_graph(commit_graph(odb.store_ref()))
        .hide(hidden_branches)?
        .map(|res| res.map(|info| info.id))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    assert_eq!(hide_then_graph, graph_then_hide);
    Ok(())
}

#[test]
fn short_visible_range_does_not_walk_shared_history_without_commit_graph() -> gix_testtools::TestResult {
    let (repo_dir, odb) = hidden_bug_repo("short_visible_range")?;
    let commits = parse_commit_names(&repo_dir)?;
    let baseline = rev_list_baseline(&repo_dir, "rev-list")?;
    let lookups = Rc::new(Cell::new(0));
    let actual = Simple::new(
        [commits["visible"]],
        CountingFind {
            inner: &odb,
            lookups: Rc::clone(&lookups),
        },
    )
    .commit_graph(None)
    .hide([commits["shared-63"]])?
    .map(|item| item.map(|info| info.id))
    .collect::<gix_error::Result<Vec<_>>>()?;
    assert_eq!(actual, baseline, "hiding the parent matches Git without a commit graph");
    assert!(
        lookups.get() <= 10,
        "a one-commit range only needs bounded lookahead beyond its hidden boundary: {} reads",
        lookups.get()
    );
    Ok(())
}

#[test]
fn merged_visible_range_stops_near_hidden_boundaries() -> gix_testtools::TestResult {
    let (repo_dir, odb) = hidden_bug_repo("merged_visible_range")?;
    let commits = parse_commit_names(&repo_dir)?;
    for (tip, baseline_prefix, max_lookups) in [("visible", "rev-list", 30), ("visible-wide", "rev-list-wide", 260)] {
        for (parents, suffix) in [(Parents::All, ""), (Parents::First, "-first-parent")] {
            let baseline_name = format!("{baseline_prefix}{suffix}");
            let mut baseline = rev_list_baseline(&repo_dir, &baseline_name)?;
            baseline.sort();
            for sorting in all_sortings() {
                for graph in [None, commit_graph(odb.store_ref())] {
                    let lookups = Rc::new(Cell::new(0));
                    let mut actual = Simple::new(
                        [commits[tip]],
                        CountingFind {
                            inner: &odb,
                            lookups: Rc::clone(&lookups),
                        },
                    )
                    .sorting(sorting)?
                    .parents(parents)
                    .commit_graph(graph)
                    .hide([commits["hidden"]])?
                    .map(|item| item.map(|info| info.id))
                    .collect::<gix_error::Result<Vec<_>>>()?;
                    actual.sort();
                    assert_eq!(
                        actual, baseline,
                        "{baseline_name}, {sorting:?}: both merge arms match Git"
                    );
                    assert!(
                        lookups.get() <= max_lookups,
                        "painting and walking {tip} must stop near the hidden merge boundaries: {} reads",
                        lookups.get()
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn already_hidden_tips_do_not_walk_their_ancestry() -> gix_testtools::TestResult {
    let (repo_dir, odb) = hidden_bug_repo("skewed_dates")?;
    let commits = parse_commit_names(&repo_dir)?;
    let tip = commits["hidden"];
    let lookups = Rc::new(Cell::new(0));
    let actual = Simple::new(
        [tip],
        CountingFind {
            inner: &odb,
            lookups: Rc::clone(&lookups),
        },
    )
    .hide([tip])?
    .collect::<gix_error::Result<Vec<_>>>()?;
    assert!(actual.is_empty(), "a tip hidden from itself yields no commits");
    assert!(
        lookups.get() <= 2,
        "hiding a tip from itself must not read its ancestry: {} lookups",
        lookups.get()
    );
    Ok(())
}

/// Walk each range of `(tips, hidden tips, baseline prefix)` in `repo_name` in every sorting and parent mode,
/// and expect the commits that `git rev-list` printed. Without a commit-graph, hidden painting can't use
/// generation numbers to stop early, and [`traverse_both()`] asserts that the walk with one yields the same
/// commits in the same order.
fn assert_hidden_ranges_match_git(repo_name: &str, ranges: &[(&[&str], &[&str], &str)]) -> gix_testtools::TestResult {
    let (repo_dir, odb) = hidden_bug_repo(repo_name)?;
    let commits = parse_commit_names(&repo_dir)?;
    for (tips, hidden, baseline_prefix) in ranges {
        let tip_commit_ids = tips.iter().map(|name| commits[*name]).collect::<Vec<_>>();
        let hidden_commit_ids = hidden.iter().map(|name| commits[*name]).collect::<Vec<_>>();
        for (parents, mode) in [(Parents::All, "do-walk"), (Parents::First, "first-parent")] {
            let baseline_name = format!("rev-list-{baseline_prefix}{mode}");
            let mut baseline = rev_list_baseline(&repo_dir, &baseline_name)?;
            baseline.sort();
            for sorting in all_sortings() {
                let mut actual = traverse_both(
                    tip_commit_ids.clone(),
                    &odb,
                    sorting,
                    parents,
                    hidden_commit_ids.clone(),
                )?;
                actual.sort();
                assert_eq!(actual, baseline, "{repo_name}, {baseline_name}, {sorting:?}");
            }
        }
    }
    Ok(())
}

#[test]
fn unrelated_history_on_either_side_matches_git() -> gix_testtools::TestResult {
    for repo_name in ["unrelated_history", "unrelated_history_same_date"] {
        assert_hidden_ranges_match_git(
            repo_name,
            &[
                (&["topic-1"], &["main-39"], "topic-"),
                (&["main-39"], &["topic-1"], "main-"),
                (&["combined"], &["main-39"], "combined-"),
                (&["combined"], &["main-29"], "combined-base-"),
            ],
        )?;
    }
    Ok(())
}

#[test]
fn hidden_commits_missing_from_the_object_database_match_git() -> gix_testtools::TestResult {
    assert_hidden_ranges_match_git(
        "unrelated_history_missing_commit",
        &[(&["topic-1"], &["main-39"], "topic-")],
    )
}

#[test]
fn criss_cross_merges_match_git() -> gix_testtools::TestResult {
    assert_hidden_ranges_match_git(
        "criss_cross",
        &[
            (&["a3"], &["b3"], "a3-b3-"),
            (&["b3"], &["a3"], "b3-a3-"),
            (&["both"], &["a2"], "both-a2-"),
            (&["a3", "b3"], &["a2", "b2"], "a3-b3-a2-b2-"),
        ],
    )
}

#[test]
fn skewed_commits_outside_the_commit_graph_match_git() -> gix_testtools::TestResult {
    assert_hidden_ranges_match_git("skewed_partial_graph", &[(&["visible"], &["hidden", "base"], "")])
}

#[test]
fn uncomputed_generations_in_the_commit_graph_match_git() -> gix_testtools::TestResult {
    let (repo_dir, odb) = hidden_bug_repo("uncomputed_generation")?;
    let commits = parse_commit_names(&repo_dir)?;
    let patched = gix_testtools::tempfile::tempdir()?;
    let graph_path = patched.path().join("commit-graph");
    write_with_uncomputed_generation(
        &odb.store_ref().path().join("info").join("commit-graph"),
        &graph_path,
        &commits["hidden"],
    )?;
    let load_graph = || gix_commitgraph::Graph::from_file(&graph_path, gix_testtools::object_hash());
    let hidden_generation = load_graph()?
        .commit_by_id(commits["hidden"])
        .expect("the commit-graph contains the hidden tip")
        .generation();
    assert_eq!(hidden_generation, 0, "the hidden tip has a legacy generation");

    // The legacy generation is seen on the hidden tip itself, or on the parent of one.
    for hidden in ["hidden", "hidden-child"] {
        let baseline = rev_list_baseline(&repo_dir, &format!("rev-list-{hidden}"))?;
        for sorting in all_sortings() {
            for graph in [None, Some(load_graph()?)] {
                let with_graph = graph.is_some();
                let actual = Simple::new([commits["visible"]], &odb)
                    .sorting(sorting)?
                    .commit_graph(graph)
                    .hide([commits[hidden], commits["other"]])?
                    .map(|item| item.map(|info| info.id))
                    .collect::<gix_error::Result<Vec<_>>>()?;
                assert_eq!(
                    actual, baseline,
                    "{hidden}, {sorting:?}, commit-graph: {with_graph}: a generation of zero doesn't order commits"
                );
            }
        }
    }
    Ok(())
}

/// Copy the commit-graph file at `source` to `destination` with the generation of `commit_id` set to zero,
/// as Git wrote it before computing generation numbers. The two high bits of its commit time stay.
fn write_with_uncomputed_generation(
    source: &std::path::Path,
    destination: &std::path::Path,
    commit_id: &gix_hash::oid,
) -> gix_testtools::TestResult {
    let file = gix_commitgraph::File::at(source)?;
    let position = file.lookup(commit_id).expect("the commit-graph contains the commit");
    let hash_len = file.object_hash().len_in_bytes();
    drop(file);

    let mut data = std::fs::read(source)?;
    let chunk_count = usize::from(data[6]);
    let commit_data_offset = data[8..8 + 12 * chunk_count]
        .as_chunks::<12>()
        .0
        .iter()
        .find(|entry| entry[..4] == *b"CDAT")
        .map(|entry| u64::from_be_bytes(entry[4..].try_into().expect("chunk offsets have 8 bytes")))
        .expect("the commit-graph has commit data");
    let generation_offset =
        usize::try_from(commit_data_offset)? + usize::try_from(position.0)? * (hash_len + 16) + hash_len + 8;
    let generation_and_time = u32::from_be_bytes(data[generation_offset..][..4].try_into()?);
    data[generation_offset..][..4].copy_from_slice(&(generation_and_time & 0b11).to_be_bytes());
    std::fs::write(destination, data)?;
    Ok(())
}
