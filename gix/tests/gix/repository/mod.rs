use gix::Repository;

#[cfg(feature = "blame")]
mod blame;
mod config;
#[cfg(feature = "excludes")]
mod excludes;
#[cfg(feature = "attributes")]
mod filter;
#[cfg(feature = "merge")]
mod merge;
mod object;
mod open;
#[cfg(feature = "attributes")]
mod pathspec;
mod reference;
mod remote;
mod shallow;
mod state;
#[cfg(feature = "attributes")]
mod submodule;
mod worktree;

#[cfg(feature = "revision")]
mod revision {
    use crate::util::hex_to_id;

    #[test]
    fn date() -> crate::Result {
        let repo = crate::named_repo("make_rev_parse_repo.sh")?;
        let actual = repo
            .rev_parse_single("old@{20 years ago}")
            .expect("it returns the oldest possible rev when overshooting");
        assert_eq!(actual, hex_to_id("be2f093f0588eaeb71e1eff7451b18c2a9b1d765"));

        let actual = repo
            .rev_parse_single("old@{1732184844}")
            .expect("it finds something in the middle");
        assert_eq!(
            actual,
            hex_to_id("b29405fe9147a3a366c4048fbe295ea04de40fa6"),
            "It also figures out that we don't mean an index, but a date"
        );
        Ok(())
    }
}

#[cfg(feature = "index")]
mod index {
    #[test]
    fn basics() -> crate::Result {
        let repo = crate::named_subrepo_opts("make_basic_repo.sh", "unborn", gix::open::Options::isolated())?;
        assert!(
            repo.index_or_load_from_head().is_err(),
            "can't read index if `HEAD^{{tree}}` can't be resolved"
        );
        assert!(
            repo.index_or_load_from_head_or_empty()?.entries().is_empty(),
            "an empty index is created on the fly"
        );
        assert_eq!(
            repo.is_pristine(),
            Some(false),
            "not pristine as it things the initial ref was changed to 'main'"
        );
        assert_eq!(
            repo.refs.is_pristine("refs/heads/main".try_into()?),
            Some(true),
            "This is a quirk of default values in gix and the way we override the initial branch for test fixtures"
        );
        Ok(())
    }
}

#[cfg(feature = "dirwalk")]
mod dirwalk {
    use std::{process::Command, sync::atomic::AtomicBool};

    use gix::config::tree::Core;
    use gix_dir::{entry::Kind::*, walk::EmissionMode};
    use gix_testtools::tempfile;

    #[test]
    fn basics() -> crate::Result {
        let repo = crate::named_repo("make_basic_repo.sh")?;
        let untracked_only = repo.dirwalk_options()?.emit_untracked(EmissionMode::CollapseDirectory);
        let mut collect = gix::dir::walk::delegate::Collect::default();
        let index = repo.index()?;
        repo.dirwalk(
            &index,
            None::<&str>,
            &AtomicBool::default(),
            untracked_only,
            &mut collect,
        )?;
        let expected = [
            ("all-untracked".to_string(), Repository),
            ("bare-repo-with-index.git".to_string(), Directory),
            ("bare.git".into(), Directory),
            ("empty-core-excludes".into(), Repository),
            ("non-bare-repo-without-index".into(), Repository),
            ("non-bare-without-worktree".into(), Directory),
            ("some".into(), Directory),
            ("unborn".into(), Repository),
        ];
        assert_eq!(
            collect
                .into_entries_by_path()
                .into_iter()
                .map(|e| (e.0.rela_path.to_string(), e.0.disk_kind.expect("kind is known")))
                .collect::<Vec<_>>(),
            expected,
            "note how bare repos are just directories by default"
        );
        let mut iter = repo.dirwalk_iter(index, None::<&str>, Default::default(), untracked_only)?;
        let mut actual: Vec<_> = iter
            .by_ref()
            .map(Result::unwrap)
            .map(|item| {
                (
                    item.entry.rela_path.to_string(),
                    item.entry.disk_kind.expect("kind is known"),
                )
            })
            .collect();
        actual.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(actual, expected, "the iterator works the same");
        let out = iter.into_outcome().expect("iteration done and no error");
        assert_eq!(
            out.dirwalk.returned_entries,
            expected.len(),
            "just a minor sanity check, assuming everything else works as well"
        );
        Ok(())
    }

    #[test]
    // On Windows, NTFS flushes directory metadata asynchronously. Directories modified
    // very recently can report slightly different `LastWriteTime` values depending on
    // when the stat is read, causing the IOUC stat check to fail unpredictably.
    #[cfg_attr(windows, ignore)]
    fn untracked_cache_respects_config_and_allows_overrides() -> crate::Result {
        let mut repo = repo_with_untracked_cache()?;
        let index = repo.index()?;

        repo.config_snapshot_mut().set_value(&Core::UNTRACKED_CACHE, "true")?;
        let out = run_dirwalk(
            &repo,
            &index,
            repo.dirwalk_options()?.emit_untracked(EmissionMode::CollapseDirectory),
        )?;
        assert_eq!(
            out.dirwalk.read_dir_calls, 0,
            "core.untrackedCache=true should enable the fast path"
        );

        let out = run_dirwalk(
            &repo,
            &index,
            repo.dirwalk_options()?
                .emit_untracked(EmissionMode::CollapseDirectory)
                .untracked_cache(gix::dirwalk::UntrackedCache::Ignore),
        )?;
        assert_ne!(
            out.dirwalk.read_dir_calls, 0,
            "callers can explicitly disable the untracked cache"
        );

        repo.config_snapshot_mut().set_value(&Core::UNTRACKED_CACHE, "false")?;
        let out = run_dirwalk(
            &repo,
            &index,
            repo.dirwalk_options()?.emit_untracked(EmissionMode::CollapseDirectory),
        )?;
        assert_ne!(
            out.dirwalk.read_dir_calls, 0,
            "core.untrackedCache=false should disable the fast path"
        );

        let out = run_dirwalk(
            &repo,
            &index,
            repo.dirwalk_options()?
                .emit_untracked(EmissionMode::CollapseDirectory)
                .untracked_cache(gix::dirwalk::UntrackedCache::Use),
        )?;
        assert_eq!(
            out.dirwalk.read_dir_calls, 0,
            "callers can override config to force use of the untracked cache"
        );
        Ok(())
    }

    fn repo_with_untracked_cache() -> crate::Result<gix::Repository> {
        let tmp = tempfile::tempdir()?;
        let root = tmp.path().join("repo");
        std::mem::forget(tmp);
        std::fs::create_dir(&root)?;
        git(&root, ["init"])?;
        git(&root, ["config", "status.showUntrackedFiles", "all"])?;
        git(&root, ["config", "user.name", "a"])?;
        git(&root, ["config", "user.email", "a@example.com"])?;
        git(&root, ["config", "core.untrackedCache", "true"])?;
        std::fs::create_dir(root.join("tracked"))?;
        std::fs::write(root.join("tracked/keep"), "keep")?;
        git(&root, ["add", "tracked/keep"])?;
        git(&root, ["commit", "-m", "init"])?;
        std::fs::create_dir_all(root.join("tracked/new"))?;
        std::fs::create_dir_all(root.join("new"))?;
        std::fs::write(root.join("tracked/new/file"), "tracked-new")?;
        std::fs::write(root.join("new/file"), "new")?;
        git(&root, ["update-index", "--force-untracked-cache"])?;
        git(&root, ["status", "--porcelain"])?;
        // Run status a second time so git validates the recorded directory stats and sets
        // the valid bitmap in the IOUC. Some git versions only populate the structure on
        // the first run and mark entries valid on the second.
        git(&root, ["status", "--porcelain"])?;
        Ok(gix::open_opts(&root, gix::open::Options::isolated())?)
    }

    fn git(cwd: &std::path::Path, args: impl IntoIterator<Item = impl AsRef<std::ffi::OsStr>>) -> crate::Result {
        let status = Command::new("git").args(args).current_dir(cwd).status()?;
        assert!(status.success());
        Ok(())
    }

    fn run_dirwalk<'repo>(
        repo: &'repo gix::Repository,
        index: &gix::worktree::Index,
        options: gix::dirwalk::Options,
    ) -> crate::Result<gix::dirwalk::Outcome<'repo>> {
        let mut collect = gix::dir::walk::delegate::Collect::default();
        Ok(repo.dirwalk(index, None::<&str>, &AtomicBool::default(), options, &mut collect)?)
    }
}

#[test]
fn size_in_memory() {
    let actual_size = std::mem::size_of::<Repository>();
    // Windows currently lays out `Repository` slightly larger than other platforms.
    // Keep the tighter limit elsewhere so regular growth still gets noticed quickly.
    let limit = if cfg!(windows) { 1280 } else { 1250 };
    assert!(
        actual_size <= limit,
        "size of Repository shouldn't change without us noticing, it's meant to be cloned: should have been below {limit:?}, was {actual_size}"
    );
}

#[test]
#[cfg(feature = "parallel")]
fn thread_safe_repository_is_sync() -> crate::Result {
    fn f<T: Send + Sync + Clone>(_t: T) {}
    f(crate::util::basic_repo()?.into_sync());
    Ok(())
}

#[test]
#[cfg(feature = "parallel")]
fn repository_is_send() -> crate::Result {
    fn f<T: Send + Clone>(_t: T) {}
    f(crate::util::basic_repo()?);
    Ok(())
}
