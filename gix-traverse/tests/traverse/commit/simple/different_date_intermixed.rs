use super::*;
use gix_traverse::commit::simple::CommitTimeOrder;

fn intermixed_repo() -> crate::Result<(std::path::PathBuf, gix_odb::Handle)> {
    named_fixture("make_repos.sh", "intermixed")
}

#[test]
fn head_breadth_first() -> crate::Result {
    let (repo_dir, odb) = intermixed_repo()?;

    // Timestamps show the intermixed ordering: b1 and b2 commits are interleaved
    // with main branch commits by time.
    insta::assert_snapshot!(git_graph_with_time(&repo_dir)?, @r"
        *-.   58912d92944087dcb09dca79cdd2a937cc158bed 1112912413 (HEAD -> main) merge
        |\ \  
        | | * a9c28710e058af4e5163699960234adb9fb2abc7 1112912293 (branch2) b2c2
        | | * b648f955b930ca95352fae6f22cb593ee0244b27 1112912173 b2c1
        | * | 0f6632a5a7d81417488b86692b729e49c1b73056 1112912353 (branch1) b1c2
        | * | 77fd3c6832c0cd542f7a39f3af9250c3268db979 1112912233 b1c1
        | |/  
        * / 2dce37be587e07caef8c4a5ab60b423b13a8536a 1112912413 c3
        |/  
        * ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83 1112912113 c2
        * 65d6af66f60b8e39fd1ba6a1423178831e764ec5 1112912053 c1
        ");

    let tip = hex_to_id("58912d92944087dcb09dca79cdd2a937cc158bed"); // merge

    // This is very different from what git does as it keeps commits together,
    // whereas we spread them out breadth-first.
    let expected = [
        tip,
        hex_to_id("2dce37be587e07caef8c4a5ab60b423b13a8536a"), // c3
        hex_to_id("0f6632a5a7d81417488b86692b729e49c1b73056"), // b1c2
        hex_to_id("a9c28710e058af4e5163699960234adb9fb2abc7"), // b2c2
        hex_to_id("ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83"), // c2
        hex_to_id("77fd3c6832c0cd542f7a39f3af9250c3268db979"), // b1c1
        hex_to_id("b648f955b930ca95352fae6f22cb593ee0244b27"), // b2c1
        hex_to_id("65d6af66f60b8e39fd1ba6a1423178831e764ec5"), // c1
    ];

    let result = traverse_both([tip], &odb, Sorting::BreadthFirst, Parents::All, [])?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn head_date_order() -> crate::Result {
    let (_repo_dir, odb) = intermixed_repo()?;
    // Graph with timestamps shown in `head_breadth_first`
    let tip = hex_to_id("58912d92944087dcb09dca79cdd2a937cc158bed"); // merge

    // NewestFirst - exactly what git shows
    let expected_newest = [
        tip,
        hex_to_id("2dce37be587e07caef8c4a5ab60b423b13a8536a"), // c3
        hex_to_id("0f6632a5a7d81417488b86692b729e49c1b73056"), // b1c2
        hex_to_id("a9c28710e058af4e5163699960234adb9fb2abc7"), // b2c2
        hex_to_id("77fd3c6832c0cd542f7a39f3af9250c3268db979"), // b1c1
        hex_to_id("b648f955b930ca95352fae6f22cb593ee0244b27"), // b2c1
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
        hex_to_id("a9c28710e058af4e5163699960234adb9fb2abc7"), // b2c2
        hex_to_id("b648f955b930ca95352fae6f22cb593ee0244b27"), // b2c1
        hex_to_id("ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83"), // c2
        hex_to_id("65d6af66f60b8e39fd1ba6a1423178831e764ec5"), // c1
        hex_to_id("0f6632a5a7d81417488b86692b729e49c1b73056"), // b1c2
        hex_to_id("77fd3c6832c0cd542f7a39f3af9250c3268db979"), // b1c1
        hex_to_id("2dce37be587e07caef8c4a5ab60b423b13a8536a"), // c3
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
