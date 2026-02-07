use super::*;
use gix_traverse::commit::simple::CommitTimeOrder;

fn simple_repo() -> crate::Result<(std::path::PathBuf, gix_odb::Handle)> {
    named_fixture("make_repos.sh", "simple")
}

#[test]
fn head_breadth_first() -> crate::Result {
    let (repo_dir, odb) = simple_repo()?;

    // Timestamps show branch1 commits are newer than branch2, with c5 being the newest.
    insta::assert_snapshot!(git_graph_with_time(&repo_dir)?, @r"
        *-.   f49838d84281c3988eeadd988d97dd358c9f9dc4 1112912533 (HEAD -> main) merge
        |\ \  
        | | * 48e8dac19508f4238f06c8de2b10301ce64a641c 1112912353 (branch2) b2c2
        | | * cb6a6befc0a852ac74d74e0354e0f004af29cb79 1112912293 b2c1
        | * | 66a309480201c4157b0eae86da69f2d606aadbe7 1112912473 (branch1) b1c2
        | * | 80947acb398362d8236fcb8bf0f8a9dac640583f 1112912413 b1c1
        | |/  
        * / 0edb95c0c0d9933d88f532ec08fcd405d0eee882 1112912533 c5
        |/  
        * 8cb5f13b66ce52a49399a2c49f537ee2b812369c 1112912233 c4
        * 33aa07785dd667c0196064e3be3c51dd9b4744ef 1112912173 c3
        * ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83 1112912113 c2
        * 65d6af66f60b8e39fd1ba6a1423178831e764ec5 1112912053 c1
        ");

    let tip = hex_to_id("f49838d84281c3988eeadd988d97dd358c9f9dc4"); // merge

    // This is very different from what git does as it keeps commits together,
    // whereas we spread them out breadth-first.
    let expected = [
        tip,
        hex_to_id("0edb95c0c0d9933d88f532ec08fcd405d0eee882"), // c5
        hex_to_id("66a309480201c4157b0eae86da69f2d606aadbe7"), // b1c2
        hex_to_id("48e8dac19508f4238f06c8de2b10301ce64a641c"), // b2c2
        hex_to_id("8cb5f13b66ce52a49399a2c49f537ee2b812369c"), // c4
        hex_to_id("80947acb398362d8236fcb8bf0f8a9dac640583f"), // b1c1
        hex_to_id("cb6a6befc0a852ac74d74e0354e0f004af29cb79"), // b2c1
        hex_to_id("33aa07785dd667c0196064e3be3c51dd9b4744ef"), // c3
        hex_to_id("ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83"), // c2
        hex_to_id("65d6af66f60b8e39fd1ba6a1423178831e764ec5"), // c1
    ];

    let result = traverse_both([tip], &odb, Sorting::BreadthFirst, Parents::All, [])?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn head_date_order() -> crate::Result {
    let (_repo_dir, odb) = simple_repo()?;
    // Graph with timestamps shown in `head_breadth_first`
    let tip = hex_to_id("f49838d84281c3988eeadd988d97dd358c9f9dc4"); // merge

    // NewestFirst - exactly what git shows
    let expected_newest = [
        tip,
        hex_to_id("0edb95c0c0d9933d88f532ec08fcd405d0eee882"), // c5
        hex_to_id("66a309480201c4157b0eae86da69f2d606aadbe7"), // b1c2
        hex_to_id("80947acb398362d8236fcb8bf0f8a9dac640583f"), // b1c1
        hex_to_id("48e8dac19508f4238f06c8de2b10301ce64a641c"), // b2c2
        hex_to_id("cb6a6befc0a852ac74d74e0354e0f004af29cb79"), // b2c1
        hex_to_id("8cb5f13b66ce52a49399a2c49f537ee2b812369c"), // c4
        hex_to_id("33aa07785dd667c0196064e3be3c51dd9b4744ef"), // c3
        hex_to_id("ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83"), // c2
        hex_to_id("65d6af66f60b8e39fd1ba6a1423178831e764ec5"), // c1
    ];
    let result = traverse_both(
        [tip],
        &odb,
        Sorting::ByCommitTime(CommitTimeOrder::NewestFirst),
        Parents::All,
        [],
    )?;
    assert_eq!(result, expected_newest);

    // OldestFirst
    let expected_oldest = [
        tip,
        hex_to_id("48e8dac19508f4238f06c8de2b10301ce64a641c"), // b2c2
        hex_to_id("cb6a6befc0a852ac74d74e0354e0f004af29cb79"), // b2c1
        hex_to_id("8cb5f13b66ce52a49399a2c49f537ee2b812369c"), // c4
        hex_to_id("33aa07785dd667c0196064e3be3c51dd9b4744ef"), // c3
        hex_to_id("ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83"), // c2
        hex_to_id("65d6af66f60b8e39fd1ba6a1423178831e764ec5"), // c1
        hex_to_id("66a309480201c4157b0eae86da69f2d606aadbe7"), // b1c2
        hex_to_id("80947acb398362d8236fcb8bf0f8a9dac640583f"), // b1c1
        hex_to_id("0edb95c0c0d9933d88f532ec08fcd405d0eee882"), // c5
    ];
    let result = traverse_both(
        [tip],
        &odb,
        Sorting::ByCommitTime(CommitTimeOrder::OldestFirst),
        Parents::All,
        [],
    )?;
    assert_eq!(result, expected_oldest);

    Ok(())
}
