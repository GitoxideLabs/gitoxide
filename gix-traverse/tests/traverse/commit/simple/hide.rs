use super::*;
use crate::util::{fixture, git_rev_list};

#[test]
fn disjoint_hidden_and_interesting() -> crate::Result {
    let (repo_dir, odb) = named_fixture("make_repos.sh", "disjoint_branches")?;

    insta::assert_snapshot!(git_graph(&repo_dir)?, @"
        * e07cf1277ff7c43090f1acfc85a46039e7de1272  (HEAD -> disjoint) b3
        * 94cf3f3a4c782b672173423e7a4157a02957dd48  b2
        * 34e5ff5ce3d3ba9f0a00d11a7fad72551fff0861  b1
        * b5665181bf4c338ab16b10da0524d81b96aff209  (main) a3
        * f0230ce37b83d8e9f51ea6322ed7e8bd148d8e28  a2
        * 674aca0765b935ac5e7f7e9ab83af7f79272b5b0  a1
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

    insta::assert_snapshot!(git_graph(&repo_dir)?, @r"
        *-.   f49838d84281c3988eeadd988d97dd358c9f9dc4  (HEAD -> main) merge
        |\ \  
        | | * 48e8dac19508f4238f06c8de2b10301ce64a641c  (branch2) b2c2
        | | * cb6a6befc0a852ac74d74e0354e0f004af29cb79  b2c1
        | * | 66a309480201c4157b0eae86da69f2d606aadbe7  (branch1) b1c2
        | * | 80947acb398362d8236fcb8bf0f8a9dac640583f  b1c1
        | |/  
        * / 0edb95c0c0d9933d88f532ec08fcd405d0eee882  c5
        |/  
        * 8cb5f13b66ce52a49399a2c49f537ee2b812369c  c4
        * 33aa07785dd667c0196064e3be3c51dd9b4744ef  c3
        * ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83  c2
        * 65d6af66f60b8e39fd1ba6a1423178831e764ec5  c1
        ");

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
    let odb = gix_odb::at(repo_path.join(".git").join("objects"))?;
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

    insta::assert_snapshot!(git_graph(&repo_path)?, @"
            * b6cf469d740a02645b7b9f7cdb98977a6cd7e5ab  (HEAD -> main) A
            | * 2955979fbddb1bddb9e1b1ca993789cacf612b18  (hidden_branch) H
            | * ae431c4e51a81a1df4ac22a52c4e247734ee3c9d  X
            | * ab31ef4cacc50169f2b1d753c1e4efd55d570bbc  Y
            |/  
            * f1543941113388f8a194164420fd7da96f73c2ce  shared
            ");

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

    insta::assert_snapshot!(git_graph(&repo_path)?, @"
            * 8822f888affa916a2c945ef3b17447f29f8aabff  (HEAD -> main) A
            * 90f80e3c031e9149cfa631493663ffe52d645aab  B
            * 2f353d445c4c552eec8e84f0f6f73999d08a8073  C
            | * 7e0cf8f62783a0eb1043fbe56d220308c3e0289e  (hidden_branch) H
            |/  
            * 359b53df58a6e26b95e276a9d1c9e2b33a3b50bf  D
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
