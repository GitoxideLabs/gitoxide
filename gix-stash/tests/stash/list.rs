use crate::{git_dir, open_ref_store};

/// Fixture: three stashes pushed.
fn list_repo() -> gix_testtools::Result<std::path::PathBuf> {
    gix_testtools::scripted_fixture_read_only("make_list_repo.sh")
}

/// Fixture: repo with no stashes at all.
fn list_empty_repo() -> gix_testtools::Result<std::path::PathBuf> {
    gix_testtools::scripted_fixture_read_only("make_list_empty_repo.sh")
}

#[test]
fn lists_entries_newest_first() -> gix_testtools::Result {
    let worktree = list_repo()?;
    let refs = open_ref_store(&git_dir(&worktree));

    let outcome = gix_stash::list(&refs)?;

    assert_eq!(outcome.entries.len(), 3, "expected 3 stash entries");

    // Entries are newest-first: index 0 = most recently pushed ("third stash").
    assert_eq!(outcome.entries[0].index, 0);
    assert_eq!(outcome.entries[1].index, 1);
    assert_eq!(outcome.entries[2].index, 2);

    // The messages should follow the stack order (newest first).
    // git stash push -m "third stash" was the last push.
    let msg0 = outcome.entries[0].message.to_string();
    let msg2 = outcome.entries[2].message.to_string();
    assert!(
        msg0.contains("third"),
        "entries[0] should contain 'third', got: {msg0:?}"
    );
    assert!(
        msg2.contains("first"),
        "entries[2] should contain 'first', got: {msg2:?}"
    );

    Ok(())
}

#[test]
fn empty_repo_returns_empty_outcome() -> gix_testtools::Result {
    let worktree = list_empty_repo()?;
    let refs = open_ref_store(&git_dir(&worktree));

    let outcome = gix_stash::list(&refs)?;

    assert!(
        outcome.entries.is_empty(),
        "no stashes should produce an empty outcome, not an error"
    );
    Ok(())
}

#[test]
fn time_seconds_is_positive() -> gix_testtools::Result {
    let worktree = list_repo()?;
    let refs = open_ref_store(&git_dir(&worktree));

    let outcome = gix_stash::list(&refs)?;

    for entry in &outcome.entries {
        assert!(
            entry.time_seconds > 0,
            "stash entry {} has non-positive time: {}",
            entry.index,
            entry.time_seconds
        );
    }
    Ok(())
}
