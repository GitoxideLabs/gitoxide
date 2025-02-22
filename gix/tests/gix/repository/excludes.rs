use crate::util::named_subrepo_opts;
use gix_worktree::stack::state::ignore::Source;

#[test]
fn empty_core_excludes() -> crate::Result {
    let repo = named_subrepo_opts(
        "make_basic_repo.sh",
        "empty-core-excludes",
        gix::open::Options::default().strict_config(true),
    )?;
    let index = repo.index_or_empty()?;
    match repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped) {
        Ok(_) => {
            unreachable!("Should fail due to empty excludes path")
        }
        Err(err) => {
            assert_eq!(
                err.to_string(),
                "The value for `core.excludesFile` could not be read from configuration"
            );
        }
    }

    let repo = gix::open_opts(repo.git_dir(), repo.open_options().clone().strict_config(false))?;
    repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)
        .expect("empty paths are now just skipped");
    Ok(())
}
