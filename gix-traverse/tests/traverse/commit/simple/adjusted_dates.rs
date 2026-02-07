//! Some dates adjusted to be a year apart, but still 'c1' and 'c2' with the same date.
use super::*;
use crate::util::fixture;
use gix_traverse::commit::simple::CommitTimeOrder;

fn adjusted_dates_repo() -> crate::Result<(std::path::PathBuf, gix_odb::Handle)> {
    let dir = fixture("make_traversal_repo_for_commits_with_dates.sh")?;
    let odb = gix_odb::at(dir.join(".git").join("objects"))?;
    Ok((dir, odb))
}

#[test]
fn head_breadth_first() -> crate::Result {
    let (repo_dir, odb) = adjusted_dates_repo()?;

    // Timestamps show b1c1 (978393600) is a year newer than c2 (946771200),
    // explaining why date-order puts b1c1 before c2.
    insta::assert_snapshot!(git_graph_with_time(&repo_dir)?, @r"
        *   288e509293165cb5630d08f4185bdf2445bf6170 1009929600 (HEAD -> main) m1b1
        |\  
        | * bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac 978393600 (branch1) b1c1
        * | 9902e3c3e8f0c569b4ab295ddf473e6de763e1e7 946771200 c2
        |/  
        * 134385f6d781b7e97062102c6a483440bfda2a03 946771200 c1
        ");

    let tip = hex_to_id("288e509293165cb5630d08f4185bdf2445bf6170"); // m1b1

    // Git also shows `b1c1` first, making topo-order similar to date order,
    // even though c2 *is* the first parent.
    let expected = [
        tip,
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac"), // b1c1
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
    ];

    let result = traverse_both([tip], &odb, Sorting::BreadthFirst, Parents::All, [])?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn head_date_order() -> crate::Result {
    let (_repo_dir, odb) = adjusted_dates_repo()?;
    // Graph with timestamps shown in `head_breadth_first`
    let tip = hex_to_id("288e509293165cb5630d08f4185bdf2445bf6170"); // m1b1

    // NewestFirst
    let expected_newest = [
        tip,
        hex_to_id("bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac"), // b1c1
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
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
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
        hex_to_id("bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac"), // b1c1
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

#[test]
fn head_date_order_with_cutoff() -> crate::Result {
    let (_repo_dir, odb) = adjusted_dates_repo()?;
    // Graph shown in `head_breadth_first`
    let tip = hex_to_id("288e509293165cb5630d08f4185bdf2445bf6170"); // m1b1

    let expected = [
        tip,
        hex_to_id("bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac"), // b1c1
    ];

    for order in [CommitTimeOrder::NewestFirst, CommitTimeOrder::OldestFirst] {
        let result = traverse_both(
            [tip],
            &odb,
            Sorting::ByCommitTimeCutoff {
                order,
                seconds: 978393600, // =2001-01-02 00:00:00 +0000
            },
            Parents::All,
            [],
        )?;
        assert_eq!(result, expected, "order = {order:?}");
    }
    Ok(())
}

#[test]
fn head_date_order_with_cutoff_disabled() -> crate::Result {
    let (_repo_dir, odb) = adjusted_dates_repo()?;
    // Graph shown in `head_breadth_first`
    let tip = hex_to_id("288e509293165cb5630d08f4185bdf2445bf6170"); // m1b1
    let very_early = 878393600; // an early date before any commit

    // NewestFirst with early cutoff (effectively disabled)
    let expected_newest = [
        tip,
        hex_to_id("bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac"), // b1c1
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
    ];
    let result = traverse_both(
        [tip],
        &odb,
        Sorting::ByCommitTimeCutoff {
            order: CommitTimeOrder::NewestFirst,
            seconds: very_early,
        },
        Parents::All,
        [],
    )?;
    assert_eq!(result, expected_newest);

    // OldestFirst with early cutoff
    let expected_oldest = [
        tip,
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
        hex_to_id("bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac"), // b1c1
    ];
    let result = traverse_both(
        [tip],
        &odb,
        Sorting::ByCommitTimeCutoff {
            order: CommitTimeOrder::OldestFirst,
            seconds: very_early,
        },
        Parents::All,
        [],
    )?;
    assert_eq!(result, expected_oldest);

    Ok(())
}

#[test]
fn date_order_with_cutoff_is_applied_to_starting_position() -> crate::Result {
    let (_repo_dir, odb) = adjusted_dates_repo()?;
    // Graph shown in `head_breadth_first`
    let tip = hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"); // c2

    for order in [CommitTimeOrder::NewestFirst, CommitTimeOrder::OldestFirst] {
        let graph = commit_graph(odb.store_ref());
        let count = Simple::new([tip], &odb)
            .sorting(Sorting::ByCommitTimeCutoff {
                order,
                seconds: 978393600, // =2001-01-02 00:00:00 +0000
            })?
            .commit_graph(graph)
            .count();
        assert_eq!(
            count, 0,
            "initial tips that don't pass cutoff value are not returned either"
        );
    }
    Ok(())
}

#[test]
fn head_date_order_first_parent_only() -> crate::Result {
    let (_repo_dir, odb) = adjusted_dates_repo()?;
    // Graph shown in `head_breadth_first`
    let tip = hex_to_id("288e509293165cb5630d08f4185bdf2445bf6170"); // m1b1

    let expected = [
        tip,
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
    ];

    for order in [CommitTimeOrder::NewestFirst, CommitTimeOrder::OldestFirst] {
        let result = traverse_both([tip], &odb, Sorting::ByCommitTime(order), Parents::First, [])?;
        assert_eq!(result, expected, "order = {order:?}");
    }
    Ok(())
}
