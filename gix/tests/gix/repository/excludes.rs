use crate::Result;
use gix_worktree::stack::state::ignore::Source;

use crate::util::named_subrepo_opts;

#[test]
fn empty_core_excludes() -> Result {
    let mut error_snapshots = Vec::new();
    let repo = named_subrepo_opts(
        "make_basic_repo.sh",
        "empty-core-excludes",
        gix::open::Options::isolated().strict_config(true),
    )?;
    let index = repo.index_or_empty()?;
    match repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped) {
        Ok(_) => {
            unreachable!("Should fail due to empty excludes path")
        }
        Err(err) => {
            error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        }
    }

    let repo = gix::open_opts(repo.git_dir(), repo.open_options().clone().strict_config(false))?;
    repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)
        .expect("empty paths are now just skipped");
    insta::assert_debug_snapshot!(error_snapshots, "empty core excludes", @"
    [
        The value for `core.excludesFile` could not be read from configuration
        |
        └─ path is missing,
    ]
    ");
    Ok(())
}

#[test]
fn missing_core_excludes_is_ignored() -> Result {
    let mut repo = named_subrepo_opts(
        "make_basic_repo.sh",
        "empty-core-excludes",
        gix::open::Options::isolated().strict_config(true),
    )?;
    repo.config_snapshot_mut()
        .set_value(&gix::config::tree::Core::EXCLUDES_FILE, "definitely-missing")?;

    let index = repo.index_or_empty()?;
    repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)
        .expect("the call works as missing excludes files are ignored");
    Ok(())
}

#[test]
fn worktree_info_exclude_from_common_dir() -> Result {
    let repo = named_subrepo_opts(
        "make_worktree_repo_with_info_exclude.sh",
        "worktree",
        gix::open::Options::isolated().strict_config(true),
    )?;
    let index = repo.index_or_empty()?;
    let mut excludes = repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)?;
    assert!(
        excludes.at_path("ignored-file", None)?.is_excluded(),
        "file matching pattern in <common_dir>/info/exclude should be excluded in worktree"
    );
    Ok(())
}
