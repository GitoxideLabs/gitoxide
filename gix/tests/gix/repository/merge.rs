use crate::util::named_repo;
use gix::merge::tree::DirectoryRenames;
use gix_testtools::TestResult;

#[test]
fn tree_merge_options() -> TestResult {
    let repo = named_repo("make_basic_repo.sh")?;
    let opts: gix::merge::plumbing::tree::Options = repo.tree_merge_options()?.into();
    assert_eq!(
        opts.rewrites,
        Some(gix::diff::Rewrites::default()),
        "If merge options aren't set, it defaults to diff options, and these default to doing rename tracking"
    );
    assert_eq!(
        opts.directory_renames,
        DirectoryRenames::Conflict,
        "repository-derived options use Git's conflict default, like the plumbing options"
    );
    Ok(())
}

#[test]
fn tree_merge_options_directory_renames() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;
    for renames in ["merge.renames=false", "diff.renames=false"] {
        for directory_renames in ["merge.directoryRenames=bogus", "merge.directoryRenames=true"] {
            let repo = gix::open_opts(
                &fixture,
                crate::util::restricted()
                    .strict_config(true)
                    .config_overrides([renames, directory_renames]),
            )?;
            let opts: gix::merge::plumbing::tree::Options = repo.tree_merge_options()?.into();
            assert_eq!(
                opts.rewrites, None,
                "directory renames do not re-enable rewrite tracking"
            );
            assert_eq!(
                opts.directory_renames,
                DirectoryRenames::Disabled,
                "disabled rewrite tracking ignores even invalid directory rename configuration"
            );
        }
    }
    let repo = gix::open_opts(
        &fixture,
        crate::util::restricted()
            .strict_config(true)
            .config_overrides(["merge.renames=true", "merge.directoryRenames=bogus"]),
    )?;
    assert!(
        repo.tree_merge_options().is_err(),
        "strict configuration still rejects invalid directory rename settings when rewrites are enabled"
    );
    Ok(())
}
