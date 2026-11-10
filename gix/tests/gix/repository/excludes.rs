use gix_testtools::TestResult;
use gix_worktree::stack::state::ignore::Source;

use crate::util::named_subrepo_opts;

#[test]
fn at_entry_infers_directories_only_without_a_mode() -> TestResult {
    use gix::bstr::{BString, ByteSlice};
    use gix::index::entry::Mode;

    let repo = crate::util::named_repo("make_directory_excludes.sh")?;
    let index = repo.index_or_empty()?;
    let mut excludes = repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)?;
    let workdir = repo.workdir().expect("fixture has a worktree");
    let git_baseline = std::fs::read_to_string(repo.git_dir().join("check-ignore.baseline"))?;
    assert!(
        !workdir.join("absent").exists(),
        "queries must not rely on on-disk directories"
    );
    for (path, expected) in [("absent/target/", true), ("absent/target", false)] {
        assert_eq!(
            git_baseline.lines().any(|ignored| ignored == path),
            expected,
            "Git's directory-only match for {path}"
        );
        let path_bytes = BString::from(path);
        let path_native = std::path::PathBuf::from(path);
        for actual in [
            excludes.at_entry(path, None)?.is_excluded(),
            excludes.at_entry(path_bytes.as_bstr(), None)?.is_excluded(),
            excludes.at_entry(&path_bytes, None)?.is_excluded(),
            excludes.at_entry(path_native.as_path(), None)?.is_excluded(),
            excludes.at_entry(path_native, None)?.is_excluded(),
        ] {
            assert_eq!(actual, expected, "all supported path types must match Git for {path}");
        }
        #[cfg(windows)]
        {
            let path_native = std::path::PathBuf::from(path.replace('/', "\\"));
            assert_eq!(
                excludes.at_entry(path_native.as_path(), None)?.is_excluded(),
                expected,
                "native Windows paths infer directories from backslashes too"
            );
            assert!(
                !excludes
                    .at_entry(path_native.as_path(), Some(Mode::FILE))?
                    .is_excluded(),
                "explicit file mode wins over a native Windows separator"
            );
            assert_eq!(
                excludes.at_entry(path_native, None)?.is_excluded(),
                expected,
                "owned native Windows paths preserve trailing separators"
            );
        }
        assert!(
            excludes.at_entry(path, Some(Mode::DIR))?.is_excluded(),
            "explicit directory mode wins for {path}"
        );
        assert!(
            !excludes.at_entry(path, Some(Mode::FILE))?.is_excluded(),
            "explicit file mode wins for {path}"
        );
        assert!(
            !excludes.at_path(path, None)?.is_excluded(),
            "at_path continues to assume a file for {path}"
        );
    }
    assert!(
        !workdir.join("absent").exists(),
        "exclude queries must not create directories"
    );
    Ok(())
}

#[test]
fn empty_core_excludes() -> TestResult {
    let mut error_snapshots = Vec::new();
    let mut repo = named_subrepo_opts(
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

    repo.reload_with(|options| options.strict_config(false))?;
    repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)?;
    insta::assert_debug_snapshot!(error_snapshots, "empty core excludes", @"
    [
        The value for `core.excludesFile` could not be read from configuration
        
        Caused by:
            0: path is missing,
    ]
    ");
    Ok(())
}

#[test]
fn missing_core_excludes_is_ignored() -> TestResult {
    let mut repo = named_subrepo_opts(
        "make_basic_repo.sh",
        "empty-core-excludes",
        gix::open::Options::isolated().strict_config(true),
    )?;
    repo.config_snapshot_mut()
        .set_value(&gix::config::tree::Core::EXCLUDES_FILE, "definitely-missing")?;

    let index = repo.index_or_empty()?;
    repo.excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)?;
    Ok(())
}

#[test]
fn worktree_info_exclude_from_common_dir() -> TestResult {
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
