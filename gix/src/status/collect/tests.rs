use std::sync::atomic::{AtomicBool, Ordering};

use gix_testtools::Result;

use crate::{
    bstr::BString,
    status::{Item, Platform},
};

fn configured(repo: &crate::Repository) -> Result<Platform<'_, crate::progress::Discard>> {
    Ok(repo
        .status(crate::progress::Discard)?
        .index_worktree_options_mut(|opts| {
            opts.sorting = Some(gix_status::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
        }))
}

fn sorted(mut items: Vec<Item>) -> Vec<Item> {
    items.sort_by(|a, b| {
        a.location()
            .cmp(b.location())
            .then_with(|| matches!(a, Item::IndexWorktree(_)).cmp(&matches!(b, Item::IndexWorktree(_))))
    });
    items
}

#[test]
fn synchronous_collection_matches_fully_drained_iterators() -> Result {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_repos.sh")?;
    for name in ["racy-git", "git-mv", "untracked-only", "added-unborn"] {
        let repo = crate::open_opts(fixture.join(name), crate::open::Options::isolated())?;
        for patterns in [Vec::new(), vec![BString::from("subdir")]] {
            let expected = sorted(
                configured(&repo)?
                    .into_iter(patterns.clone())?
                    .collect::<std::result::Result<Vec<_>, _>>()?,
            );
            for (staged, unstaged) in [(true, true), (true, false), (false, true)] {
                let actual = sorted(
                    configured(&repo)?
                        .collect_internal(patterns.clone(), staged, unstaged, &AtomicBool::new(false))
                        .map_err(gix_error::Exn::into_error)?,
                );
                let expected: Vec<_> = expected
                    .iter()
                    .filter(|item| match item {
                        Item::TreeIndex(_) => staged,
                        Item::IndexWorktree(_) => unstaged,
                    })
                    .cloned()
                    .collect();
                assert_eq!(
                    actual, expected,
                    "{name}: synchronous phases preserve iterator status semantics"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn cancellation_returns_no_partial_snapshot_and_allows_retry() -> Result {
    let fixture = gix_testtools::scripted_fixture_read_only("make_status_repos.sh")?;
    let repo = crate::open_opts(fixture.join("racy-git"), crate::open::Options::isolated())?;
    let interrupt = AtomicBool::new(true);
    let error = configured(&repo)?
        .collect_internal(Vec::new(), true, true, &interrupt)
        .expect_err("a cancelled collection must not return even an empty successful snapshot");
    assert!(
        error.to_string().contains("interrupted"),
        "cancellation is distinguishable from a clean result"
    );
    interrupt.store(false, Ordering::Relaxed);
    let items = configured(&repo)?
        .collect_internal(Vec::new(), true, true, &interrupt)
        .map_err(gix_error::Exn::into_error)?;
    assert_eq!(items.len(), 2, "retry recomputes both staged and unstaged changes");
    Ok(())
}

#[test]
fn nested_submodule_changes_are_fully_collected_synchronously() -> Result {
    use gix_status::index_as_worktree::{Change, EntryStatus};

    let fixture = gix_testtools::scripted_fixture_writable("make_status_monitor_submodules.sh")?;
    let repo = crate::open_opts(fixture.path().join("root"), crate::open::Options::isolated())?;
    let expected = configured(&repo)?
        .into_iter(None)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let actual = configured(&repo)?
        .into_vec(None, &AtomicBool::new(false))
        .map_err(gix_error::Exn::into_error)?;
    assert_eq!(
        actual, expected,
        "nested synchronous status preserves the complete submodule result"
    );

    let mut changes = actual.as_slice();
    for path in ["outer", "inner"] {
        assert_eq!(changes.len(), 1, "only the nested submodule is dirty at this level");
        assert_eq!(
            changes[0].location(),
            path,
            "the changed child is relative to its enclosing worktree"
        );
        let Item::IndexWorktree(crate::status::index_worktree::Item::Modification {
            status: EntryStatus::Change(Change::SubmoduleModification(status)),
            ..
        }) = &changes[0]
        else {
            panic!("the changed child must carry its nested submodule status");
        };
        changes = status
            .changes
            .as_deref()
            .expect("synchronous submodule status includes its worktree changes");
    }
    assert_eq!(changes.len(), 1, "the innermost worktree has one modified file");
    assert_eq!(
        changes[0].location(),
        "file",
        "the nested scan reaches the modified leaf"
    );
    Ok(())
}

#[test]
fn submodule_collection_observes_its_callers_borrowed_interrupt() -> Result {
    let fixture = gix_testtools::scripted_fixture_writable("make_status_monitor_submodules.sh")?;
    let repo = crate::open_opts(fixture.path().join("root"), crate::open::Options::isolated())?;
    let submodule = repo
        .submodules()?
        .expect("fixture has submodules")
        .next()
        .expect("outer submodule exists");
    let interrupt = AtomicBool::new(false);
    let error = submodule
        .status_opts_inner(
            crate::submodule::config::Ignore::None,
            false,
            &mut |platform| {
                interrupt.store(true, Ordering::Relaxed);
                platform
            },
            Some(&interrupt),
        )
        .expect_err("cancellation before nested collection must fail instead of returning partial status");
    assert!(
        matches!(error, crate::submodule::status::Error::SynchronousCollection(_)),
        "the nested collection uses the synchronous path with its caller's cancellation flag"
    );
    Ok(())
}

#[test]
fn synchronous_submodule_status_preserves_repository_open_options() -> Result {
    let fixture = gix_testtools::scripted_fixture_writable("make_status_monitor_submodules.sh")?;
    let repo = crate::open_opts(
        fixture.path().join("root"),
        crate::open::Options::isolated().config_overrides(["diff.ignoreSubmodules=all"]),
    )?;
    let actual = configured(&repo)?
        .into_vec(None, &AtomicBool::new(false))
        .map_err(gix_error::Exn::into_error)?;
    assert!(
        actual.is_empty(),
        "the submodule provider must retain the caller's configuration overrides when reopening without parallelism"
    );
    Ok(())
}
