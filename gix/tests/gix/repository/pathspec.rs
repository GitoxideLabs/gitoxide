use gix::{bstr::ByteSlice, config::tree::gitoxide};
use gix_worktree::stack::state::attributes::Source;

use crate::util::named_repo;

#[test]
fn defaults_are_taken_from_repo_config() -> crate::Result {
    let mut repo = named_repo("make_basic_repo.sh")?;
    repo.config_snapshot_mut()
        .set_value(&gitoxide::Pathspec::ICASE, "true")?;
    let inherit_ignore_case = true;
    let empty_pathspecs_match_prefix = true;
    let mut pathspec = repo.pathspec(
        empty_pathspecs_match_prefix,
        [
            "hi",
            ":!hip",
            gix::path::to_unix_separators_on_windows(gix::path::into_bstr(
                repo.workdir().expect("present").join("for-normalization"),
            ))
            .to_str_lossy()
            .as_ref(),
        ],
        inherit_ignore_case,
        &**repo.index()?,
        Source::WorktreeThenIdMapping.adjust_for_bare(repo.is_bare()),
    )?;
    assert!(pathspec.is_included("hi", Some(false)));
    assert!(!pathspec.is_included("ho", Some(false)));
    assert!(!pathspec.is_included("hip", Some(false)));
    assert!(pathspec
        .pattern_matching_relative_path("hip", Some(false))
        .expect("match")
        .is_excluded());

    assert!(
        pathspec.is_included("HI", Some(false)),
        "icase is enabled, so filesystem doesn't matter"
    );
    Ok(())
}
