//! Same dates are somewhat special as they show how sorting-details on priority queues affects ordering
use super::*;
use crate::util::fixture;
use gix_hash::oid;
use gix_traverse::commit::simple::CommitTimeOrder;

fn same_date_repo() -> crate::Result<(std::path::PathBuf, gix_odb::Handle)> {
    let dir = fixture("make_traversal_repo_for_commits_same_date.sh")?;
    let odb = gix_odb::at(dir.join(".git").join("objects"))?;
    Ok((dir, odb))
}

#[test]
fn c4_breadth_first() -> crate::Result {
    let (repo_dir, odb) = same_date_repo()?;

    insta::assert_snapshot!(git_graph(&repo_dir)?, @r"
        *   01ec18a3ebf2855708ad3c9d244306bc1fae3e9b  (HEAD -> main) m1b1
        |\  
        | * ce2e8ffaa9608a26f7b21afc1db89cadb54fd353  (branch1) b1c2
        | * 9152eeee2328073cf23dcf8e90c949170b711659  b1c1
        * | efd9a841189668f1bab5b8ebade9cd0a1b139a37  c5
        |/  
        * 9556057aee5abb06912922e9f26c46386a816822  c4
        * 17d78c64cef6c33a10a604573fd2c429e477fd63  c3
        * 9902e3c3e8f0c569b4ab295ddf473e6de763e1e7  c2
        * 134385f6d781b7e97062102c6a483440bfda2a03  c1
        ");

    let tip = hex_to_id("9556057aee5abb06912922e9f26c46386a816822"); // c4

    let expected = [
        tip,
        hex_to_id("17d78c64cef6c33a10a604573fd2c429e477fd63"), // c3
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
    ];

    let result = traverse_both([tip], &odb, Sorting::BreadthFirst, Parents::All, [])?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn head_breadth_first() -> crate::Result {
    let (_repo_dir, odb) = same_date_repo()?;
    // Graph shown in `c4_breadth_first`
    let tip = hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"); // m1b1

    // We always take the first parent first, then the second, and so on.
    // Deviation: git for some reason displays b1c2 *before* c5, but I think it's better
    //            to have a strict parent order.
    let expected = [
        tip,
        hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), // c5
        hex_to_id("ce2e8ffaa9608a26f7b21afc1db89cadb54fd353"), // b1c2
        hex_to_id("9556057aee5abb06912922e9f26c46386a816822"), // c4
        hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"), // b1c1
        hex_to_id("17d78c64cef6c33a10a604573fd2c429e477fd63"), // c3
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
    ];

    let result = traverse_both([tip], &odb, Sorting::BreadthFirst, Parents::All, [])?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn head_date_order() -> crate::Result {
    let (_repo_dir, odb) = same_date_repo()?;
    // Graph shown in `c4_breadth_first`
    let tip = hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"); // m1b1

    let expected = [
        tip,
        hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), // c5
        hex_to_id("ce2e8ffaa9608a26f7b21afc1db89cadb54fd353"), // b1c2
        hex_to_id("9556057aee5abb06912922e9f26c46386a816822"), // c4
        hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"), // b1c1
        hex_to_id("17d78c64cef6c33a10a604573fd2c429e477fd63"), // c3
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
    assert_eq!(result, expected);

    let result = traverse_both(
        [tip],
        &odb,
        Sorting::ByCommitTime(CommitTimeOrder::OldestFirst),
        Parents::All,
        [],
    )?;
    assert_eq!(result, expected);

    Ok(())
}

#[test]
fn head_first_parent_only_breadth_first() -> crate::Result {
    let (_repo_dir, odb) = same_date_repo()?;
    // Graph shown in `c4_breadth_first`
    let tip = hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"); // m1b1

    let expected = [
        tip,
        hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), // c5
        hex_to_id("9556057aee5abb06912922e9f26c46386a816822"), // c4
        hex_to_id("17d78c64cef6c33a10a604573fd2c429e477fd63"), // c3
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
    ];

    let result = traverse_both([tip], &odb, Sorting::BreadthFirst, Parents::First, [])?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn head_c4_breadth_first() -> crate::Result {
    let (_repo_dir, odb) = same_date_repo()?;
    // Graph shown in `c4_breadth_first`
    let tips = [
        hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"), // m1b1
        hex_to_id("9556057aee5abb06912922e9f26c46386a816822"), // c4
    ];

    let expected = [
        tips[0],
        tips[1],
        hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), // c5
        hex_to_id("ce2e8ffaa9608a26f7b21afc1db89cadb54fd353"), // b1c2
        hex_to_id("17d78c64cef6c33a10a604573fd2c429e477fd63"), // c3
        hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"), // b1c1
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
    ];

    let result = traverse_both(tips, &odb, Sorting::BreadthFirst, Parents::All, [])?;
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn filtered_commit_does_not_block_ancestors_reachable_from_another_commit() -> crate::Result {
    // I don't see a use case for the predicate returning false for a commit but return true for
    // at least one of its ancestors, so this test is kind of dubious. But we do want
    // `Ancestors` to not eagerly blacklist all of a commit's ancestors when blacklisting that
    // one commit, and this test happens to check that.
    let (_repo_dir, odb) = same_date_repo()?;
    // Graph shown in `c4_breadth_first`
    let tip = hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"); // m1b1
    let filter_out = hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"); // b1c1

    let expected = [
        tip,
        hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), // c5
        hex_to_id("ce2e8ffaa9608a26f7b21afc1db89cadb54fd353"), // b1c2
        hex_to_id("9556057aee5abb06912922e9f26c46386a816822"), // c4
        hex_to_id("17d78c64cef6c33a10a604573fd2c429e477fd63"), // c3
        hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"), // c2
        hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"), // c1
    ];

    let graph = commit_graph(odb.store_ref());
    let result: Vec<_> = Simple::filtered([tip], &odb, move |id: &oid| id != filter_out)
        .sorting(Sorting::BreadthFirst)?
        .parents(Parents::All)
        .hide([])?
        .commit_graph(graph)
        .map(|res| res.map(|info| info.id))
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn predicate_only_called_once_even_if_fork_point() -> crate::Result {
    // The `self.seen` check should come before the `self.predicate` check, as we don't know how
    // expensive calling `self.predicate` may be.
    let (_repo_dir, odb) = same_date_repo()?;
    // Graph shown in `c4_breadth_first`
    let tip = hex_to_id("01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"); // m1b1
    let filter_out = hex_to_id("9556057aee5abb06912922e9f26c46386a816822"); // c4

    let expected = [
        tip,
        hex_to_id("efd9a841189668f1bab5b8ebade9cd0a1b139a37"), // c5
        hex_to_id("ce2e8ffaa9608a26f7b21afc1db89cadb54fd353"), // b1c2
        hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"), // b1c1
    ];

    let mut seen = false;
    let graph = commit_graph(odb.store_ref());
    let result: Vec<_> = Simple::filtered([tip], &odb, move |id: &oid| {
        if id == filter_out {
            assert!(!seen, "predicate should only be called once for c4");
            seen = true;
            false
        } else {
            true
        }
    })
    .sorting(Sorting::BreadthFirst)?
    .parents(Parents::All)
    .hide([])?
    .commit_graph(graph)
    .map(|res| res.map(|info| info.id))
    .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(result, expected);
    Ok(())
}
