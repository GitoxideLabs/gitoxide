use std::path::{Path, PathBuf};

use gix_testtools::TestResult;

fn status(repo: &gix::Repository) -> gix::Result<(Vec<String>, gix::status::Outcome)> {
    collect(repo.status(gix::progress::Discard)?.into_index_worktree_iter(None)?)
}

fn collect(mut iter: gix::status::index_worktree::Iter) -> gix::Result<(Vec<String>, gix::status::Outcome)> {
    let mut paths = iter
        .by_ref()
        .map(|item| item.map(|item| item.rela_path().to_string()))
        .collect::<gix::Result<Vec<_>>>()?;
    paths.sort();
    Ok((
        paths,
        iter.into_outcome().expect("fully consumed status has an outcome"),
    ))
}

fn fixture_with(version: u32, mut setup: impl FnMut(&Path) -> gix_testtools::Result) -> TestResult<PathBuf> {
    let (root, ()) =
        gix_testtools::scripted_fixture_read_only_with_post("make_status_untracked_cache.sh", version, |fixture| {
            if let gix_testtools::FixtureState::Uninitialized(path) = fixture {
                setup(path)?;
            }
            Ok(())
        })?;
    Ok(root)
}

fn prime_cache(root: &Path) -> gix_testtools::Result {
    gix_testtools::git(root, "update-index --no-untracked-cache")?;
    gix_testtools::git(root, "status --porcelain")?;
    gix_testtools::git(root, "status --porcelain")?;
    Ok(())
}

fn writable_fixture() -> TestResult<gix_testtools::tempfile::TempDir> {
    let fixture = gix_testtools::scripted_fixture_writable("make_status_untracked_cache.sh")?;
    // Copying the fixture changes the worktree identity and its absolute exclude path.
    let output = gix_testtools::git_command(fixture.path())
        .args(["config", "core.excludesFile"])
        .arg(fixture.path().join(".git/global-excludes"))
        .output()?;
    assert!(
        output.status.success(),
        "Git updates the exclude path: {:?}",
        output.stderr
    );
    prime_cache(fixture.path())?;
    Ok(fixture)
}

fn git_paths(root: &Path, extra: &[&str], use_cache: bool) -> TestResult<Vec<String>> {
    let mut command = gix_testtools::git_command(root);
    if !use_cache {
        command.args(["-c", "core.fsmonitor=false", "-c", "core.untrackedCache=false"]);
    }
    let output = command
        .args(["status", "--porcelain=v1", "-z"])
        .args(extra)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()?;
    assert!(output.status.success(), "Git status succeeds: {:?}", output.stderr);
    let mut paths = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| String::from_utf8(record[3..].to_vec()).map(|path| path.trim_end_matches('/').to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    Ok(paths)
}

#[test]
fn git_untracked_cache_avoids_directory_reads_without_losing_tracked_names() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("make_status_untracked_cache.sh")?;
    let repo = gix::open_opts(&root, gix::open::Options::isolated())?;
    let (actual, outcome) = status(&repo)?;
    assert_eq!(
        actual,
        ["mixed/deep/visible", "mixed/visible", "stable/visible", "untracked"]
    );
    let walk = outcome
        .index_worktree
        .dirwalk
        .expect("untracked status walks directories");
    assert!(
        walk.cached_dir_listings >= 3,
        "the root and tracked-only branch avoid directory reads: {walk:?}"
    );
    let precious = gix::open_opts(
        &root,
        gix::open::Options::isolated().config_overrides(["gitoxide.parsePrecious=true"]),
    )?;
    let (paths, outcome) = status(&precious)?;
    assert_eq!(paths, actual, "precious parsing preserves ordinary ignore rules");
    assert_eq!(
        outcome.index_worktree.dirwalk.expect("dirwalk is enabled"),
        walk,
        "enabling precious parsing alone must not disable the cache"
    );
    #[cfg(windows)]
    {
        let repo = gix::open_opts(root.canonicalize()?, gix::open::Options::isolated())?;
        let (paths, outcome) = status(&repo)?;
        assert_eq!(paths, actual, "a verbatim worktree path has the same status");
        let walk = outcome.index_worktree.dirwalk.expect("dirwalk is enabled");
        assert!(
            walk.cached_dir_listings >= 3,
            "Windows path aliases retain the warmed cache: {walk:?}"
        );
    }
    let uncached = gix::open_opts(
        &root,
        gix::open::Options::isolated().config_overrides(["core.fsmonitor=false", "core.untrackedCache=false"]),
    )?;
    assert!(
        walk.read_dir_calls
            < status(&uncached)?
                .1
                .index_worktree
                .dirwalk
                .expect("dirwalk is enabled")
                .read_dir_calls,
        "eligible cached directories reduce actual filesystem reads: {walk:?}"
    );
    Ok(())
}

#[test]
fn precious_patterns_cannot_reuse_git_ignore_decisions() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for (version, source) in [
        (170, ".gitignore"),
        (171, "tracked-only/.gitignore"),
        (172, ".git/global-excludes"),
        (173, ".git/info/exclude"),
    ] {
        let root = fixture_with(version, |root| {
            // Git hides the literal dollar-prefixed name, while precious parsing reveals it.
            std::fs::write(root.join(source), "$literal\n")?;
            std::fs::write(root.join("tracked-only/$literal"), "untracked\n")?;
            prime_cache(root)
        })?;
        let repo = gix::open_opts(
            &root,
            gix::open::Options::isolated().config_overrides(["gitoxide.parsePrecious=true"]),
        )?;
        let (actual, outcome) = status(&repo)?;
        let uncached = gix::open_opts(
            &root,
            gix::open::Options::isolated()
                .config_overrides(["gitoxide.parsePrecious=true", "core.untrackedCache=false"]),
        )?;
        assert_eq!(actual, status(&uncached)?.0, "{source}: cached and fresh walks agree");
        assert!(
            actual.iter().any(|path| path == "tracked-only/$literal"),
            "{source}: precious parsing reveals the name omitted by Git"
        );
        assert_eq!(
            outcome
                .index_worktree
                .dirwalk
                .as_ref()
                .expect("dirwalk is enabled")
                .cached_dir_listings,
            0,
            "{source}: different ignore semantics require fresh listings"
        );
        if source.starts_with(".git/") {
            assert!(
                !outcome.has_changes(),
                "different parser settings alone must not invalidate Git's cache"
            );
        }
    }
    Ok(())
}

#[test]
fn differently_cased_cached_directories_fall_back_without_duplicate_paths() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for (version, from, to) in [(160, "mixed", "Mixed"), (161, "mixed/deep", "mixed/Deep")] {
        let root = fixture_with(version, |root| {
            gix_testtools::git(root, "config core.ignoreCase true")?;
            // Keep unchanged tracked files clean on both case-sensitive and insensitive filesystems.
            gix_testtools::git(root, "update-index --assume-unchanged mixed/tracked mixed/deep/tracked")?;
            let intermediate = root.join("case-rename-intermediate");
            std::fs::rename(root.join(from), &intermediate)?;
            std::fs::rename(&intermediate, root.join(to))?;
            gix_testtools::git(root, "update-index --fsmonitor-valid mixed/tracked mixed/deep/tracked")?;
            if ["mixed/tracked", "mixed/deep/tracked"]
                .iter()
                .all(|path| root.join(path).is_file())
            {
                gix_testtools::git(
                    root,
                    "update-index --no-assume-unchanged mixed/tracked mixed/deep/tracked",
                )?;
            }
            prime_cache(root)
        })?;
        let repo = gix::open_opts(&root, gix::open::Options::isolated())?;
        let index = repo.index()?;
        if ["mixed/tracked", "mixed/deep/tracked"]
            .iter()
            .all(|path| root.join(path).is_file())
        {
            assert!(
                index
                    .entries()
                    .iter()
                    .all(|entry| !entry.flags.contains(gix::index::entry::Flags::ASSUME_VALID)),
                "case-insensitive filesystems exercise the spelling guard without assume-valid flags"
            );
        }
        let cache = index.untracked().expect("Git primed UNTR after the case-only rename");
        let basename = to.rsplit('/').next().expect("renamed directory has a name");
        assert!(
            cache.directories().iter().any(|dir| dir.name() == basename),
            "UNTR retains the directory spelling on disk"
        );
        assert!(
            cache.directories()[0].stat().is_some(),
            "the parent listing can otherwise be reused"
        );

        let (actual, outcome) = status(&repo)?;
        assert_eq!(
            actual,
            git_paths(&root, &[], false)?,
            "index spelling cannot create another directory"
        );
        let uncached = gix::open_opts(
            &root,
            gix::open::Options::isolated().config_overrides(["core.fsmonitor=false", "core.untrackedCache=false"]),
        )?;
        assert_eq!(
            actual,
            status(&uncached)?.0,
            "cached and uncached gix agree on path spelling"
        );
        let walk = outcome
            .index_worktree
            .dirwalk
            .expect("untracked status walks directories");
        assert_eq!(
            walk.cached_dir_listings, 0,
            "case mismatches fall back before merging tracked names"
        );
        assert!(walk.read_dir_calls > 0, "the walk obtains the actual directory names");
    }
    Ok(())
}

#[test]
fn every_index_parent_must_match_the_cached_directory_spelling() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = fixture_with(162, |root| {
        gix_testtools::git(root, "config core.ignoreCase true")?;
        gix_testtools::git(root, "update-index --force-remove mixed/deep/tracked")?;
        gix_testtools::git(root, "update-index --assume-unchanged mixed/tracked")?;
        std::fs::rename(root.join("mixed"), root.join("intermediate"))?;
        std::fs::rename(root.join("intermediate"), root.join("MIXED"))?;
        let index = gix::index::File::at(
            root.join(".git/index"),
            gix_testtools::object_hash(),
            false,
            Default::default(),
        )?;
        let blob_id = index
            .entry_by_path("mixed/tracked".into())
            .expect("the fixture tracks this file")
            .id;
        std::fs::write(root.join("MIXED/another"), "tracked\n")?;
        let output = gix_testtools::git_command(root)
            .args(["update-index", "--add", "--cacheinfo", "100644"])
            .arg(blob_id.to_string())
            .arg("MIXED/another")
            .output()?;
        assert!(
            output.status.success(),
            "Git accepts the explicitly cased index path: {:?}",
            output.stderr
        );
        gix_testtools::git(root, "update-index --fsmonitor-valid mixed/tracked")?;
        if root.join("mixed/tracked").is_file() {
            gix_testtools::git(root, "update-index --no-assume-unchanged mixed/tracked")?;
        }
        prime_cache(root)
    })?;
    let repo = gix::open_opts(&root, gix::open::Options::isolated())?;
    let index = repo.index()?;
    let lookup = index.prepare_icase_backing();
    assert!(
        index
            .entry_closest_to_directory_or_directory_icase("MIXED".into(), true, &lookup)
            .expect("the directory is tracked")
            .path(&index)
            .starts_with(b"MIXED/"),
        "the canonical lookup alone matches the cache and misses the second spelling"
    );
    let uncached = gix::open_opts(
        &root,
        gix::open::Options::isolated().config_overrides(["core.fsmonitor=false", "core.untrackedCache=false"]),
    )?;
    let (actual, outcome) = status(&repo)?;
    assert_eq!(
        actual,
        status(&uncached)?.0,
        "no extra index spelling may become a directory entry"
    );
    assert_eq!(
        outcome
            .index_worktree
            .dirwalk
            .expect("dirwalk is enabled")
            .cached_dir_listings,
        0
    );
    Ok(())
}

#[test]
fn absent_skipped_entries_do_not_become_cached_siblings() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for (version, flag) in [(163, "--skip-worktree"), (164, "--assume-unchanged")] {
        let root = fixture_with(version, |root| {
            gix_testtools::git(root, &format!("update-index {flag} stable/tracked"))?;
            std::fs::remove_file(root.join("stable/tracked"))?;
            std::fs::remove_file(root.join("stable/visible"))?;
            std::fs::create_dir(root.join("stable/untracked-directory"))?;
            std::fs::write(root.join("stable/untracked-directory/visible"), "untracked\n")?;
            prime_cache(root)?;
            let mut index = gix::index::File::at(
                root.join(".git/index"),
                gix_testtools::object_hash(),
                false,
                Default::default(),
            )?;
            let token = index
                .fs_monitor()
                .expect("the fixture has a monitor checkpoint")
                .token()
                .clone();
            let cache = index.set_untracked(None);
            index
                .entry_mut_by_path_and_stage("stable/tracked".into(), gix::index::entry::Stage::Unconflicted)
                .expect("the absent file is still tracked")
                .flags
                .insert(gix::index::entry::Flags::FSMONITOR_VALID);
            index.set_fs_monitor_token(Some(token));
            index.set_untracked(cache);
            index.write(Default::default())?;
            Ok(())
        })?;
        let repo = gix::open_opts(&root, gix::open::Options::isolated())?;
        let uncached = gix::open_opts(
            &root,
            gix::open::Options::isolated().config_overrides(["core.fsmonitor=false", "core.untrackedCache=false"]),
        )?;
        let (actual, outcome) = status(&repo)?;
        assert_eq!(
            actual,
            status(&uncached)?.0,
            "{flag}: absent entries do not affect collapsing"
        );
        assert!(
            outcome.has_changes(),
            "{flag}: an absent entry cannot leave its directory cache valid"
        );
    }
    Ok(())
}

#[test]
fn untyped_untracked_files_use_directory_reads() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = fixture_with(154, |root| {
        #[cfg(unix)]
        std::os::unix::fs::symlink("visible", root.join("stable/link"))?;
        prime_cache(root)
    })?;
    let repo = gix::open_opts(&root, gix::open::Options::isolated())?;
    let mut iter = repo.status(gix::progress::Discard)?.into_index_worktree_iter(None)?;
    let mut kinds = Vec::new();
    for item in iter.by_ref() {
        if let gix::status::index_worktree::Item::DirectoryContents { entry, .. } = item? {
            kinds.push((entry.rela_path, entry.disk_kind));
        }
    }
    assert!(
        kinds
            .iter()
            .any(|(path, kind)| path == "stable/visible" && *kind == Some(gix::dir::entry::Kind::File))
    );
    #[cfg(unix)]
    assert!(
        kinds
            .iter()
            .any(|(path, kind)| path == "stable/link" && *kind == Some(gix::dir::entry::Kind::Symlink)),
        "file and symlink types remain distinct"
    );
    let walk = iter
        .into_outcome()
        .expect("status completed")
        .index_worktree
        .dirwalk
        .expect("dirwalk is enabled");
    assert!(
        (2..=4).contains(&walk.cached_dir_listings),
        "the tracked-only leaves and eligible ancestors have cached type information: {walk:?}"
    );
    assert!(
        walk.read_dir_calls >= 4,
        "untracked file types come from read_dir rather than per-entry stat calls"
    );
    Ok(())
}

#[test]
fn uncertain_unicode_composition_falls_back_to_disk_names() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for (version, tracked) in [(155, false), (156, true)] {
        let root = fixture_with(version, |root| {
            gix_testtools::git(root, "config core.precomposeUnicode true")?;
            std::fs::write(root.join("stable/caf\u{e9}"), "unicode\n")?;
            if tracked {
                gix_testtools::git(root, "add stable")?;
            }
            prime_cache(root)
        })?;
        for precompose in ["core.precomposeUnicode=false", "core.precomposeUnicode=true"] {
            let repo = gix::open_opts(&root, gix::open::Options::isolated().config_overrides([precompose]))?;
            let uncached = gix::open_opts(
                &root,
                gix::open::Options::isolated().config_overrides([
                    precompose,
                    "core.fsmonitor=false",
                    "core.untrackedCache=false",
                ]),
            )?;
            let (actual, outcome) = status(&repo)?;
            assert_eq!(
                actual,
                status(&uncached)?.0,
                "the configured walk decides filename composition"
            );
            assert_eq!(
                outcome
                    .index_worktree
                    .dirwalk
                    .as_ref()
                    .expect("dirwalk is enabled")
                    .cached_dir_listings,
                0,
                "UNTR does not record the Unicode policy used to produce its names"
            );
            assert!(
                outcome.has_changes(),
                "an advanced checkpoint must not preserve uncertain Unicode listings"
            );
        }
    }
    Ok(())
}

#[test]
fn restricted_status_invalidates_uncertain_unicode_before_writeback() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for partial in [false, true] {
        let fixture = writable_fixture()?;
        let root = fixture.path();
        std::fs::write(root.join("stable/caf\u{e9}"), "unicode\n")?;
        prime_cache(root)?;
        std::fs::write(root.join(".git/fsmonitor-token"), b"after-unicode-change")?;
        let repo = gix::open_opts(root, gix::open::Options::isolated())?;
        let mut platform = repo.status(gix::progress::Discard)?;
        if !partial {
            platform = platform.untracked_files(gix::status::UntrackedFiles::None);
        }
        let (_, mut outcome) = collect(platform.into_index_worktree_iter(partial.then(|| "mixed/".into()))?)?;
        outcome.write_changes().expect("the new checkpoint is pending")?;
        let written = repo.index()?;
        assert_eq!(
            written.fs_monitor().expect("writeback retains FSMN").token(),
            &gix::index::extension::fs_monitor::Token::V2 {
                token: "after-unicode-change".into()
            }
        );
        assert!(
            written
                .untracked()
                .expect("writeback retains the cache structure")
                .directories()
                .iter()
                .all(|dir| dir.stat().is_none()),
            "restricted status cannot advance the checkpoint with uncertain Unicode cache entries"
        );
    }
    Ok(())
}

#[test]
fn all_untracked_mode_reaches_cached_child_only_directories() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = fixture_with(100, |root| {
        std::fs::create_dir_all(root.join("only-untracked/deep"))?;
        std::fs::write(root.join("only-untracked/deep/file"), "untracked\n")?;
        gix_testtools::git(root, "config status.showUntrackedFiles all")?;
        prime_cache(root)
    })?;
    let repo = gix::open_opts(&root, gix::open::Options::isolated())?;
    let all_mode_cached = repo.index()?.untracked().expect("Git populated UNTR").dir_flags() == 0;
    let (actual, outcome) = status(&repo)?;
    assert_eq!(
        actual,
        git_paths(&root, &[], false)?,
        "cached directory nodes remain reachable without tracked names"
    );
    assert!(actual.iter().any(|path| path == "only-untracked/deep/file"));
    let walk = outcome
        .index_worktree
        .dirwalk
        .expect("untracked status walks directories");
    if all_mode_cached {
        assert!(
            walk.cached_dir_listings > 0,
            "compatible all-file listings are reused: {walk:?}"
        );
    } else {
        assert_eq!(
            walk.cached_dir_listings, 0,
            "older Git leaves an incompatible UNTR policy"
        );
        assert!(walk.read_dir_calls > 0, "incompatible listings fall back to disk");
    }
    Ok(())
}

#[test]
fn changed_ignore_sources_reveal_previously_omitted_files() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for (version, source, revealed) in [
        (110, ".gitignore", "mixed/hidden.ignored"),
        (111, ".git/global-excludes", "global"),
        (112, ".git/info/exclude", "info-hidden"),
    ] {
        let root = fixture_with(version, |root| {
            if source == ".git/info/exclude" {
                std::fs::write(root.join(source), "info-hidden\n")?;
                std::fs::write(root.join("info-hidden"), "ignored\n")?;
                prime_cache(root)?;
            }
            std::fs::write(root.join(source), "")?;
            if source == ".gitignore" {
                std::fs::write(root.join(".git/fsmonitor-paths"), b".gitignore\0")?;
            }
            Ok(())
        })?;
        let repo = gix::open_opts(&root, gix::open::Options::isolated())?;
        let (actual, outcome) = status(&repo)?;
        assert_eq!(
            actual,
            git_paths(&root, &[], false)?,
            "changed {source} invalidates omitted names"
        );
        assert!(
            actual.iter().any(|path| path == revealed),
            "{source} no longer hides {revealed}"
        );
        let walk = outcome
            .index_worktree
            .dirwalk
            .expect("untracked status walks directories");
        assert_eq!(
            walk.cached_dir_listings, 0,
            "changed ignore rules force a fresh walk: {source}"
        );
        assert!(walk.read_dir_calls > 0);
    }
    Ok(())
}

#[test]
fn directory_and_file_events_invalidate_additions_and_removals() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for (version, events) in [
        (120, b"mixed/\0".as_slice()),
        (121, b"mixed/visible\0mixed/deep/added\0".as_slice()),
    ] {
        let root = fixture_with(version, |root| {
            std::fs::remove_file(root.join("mixed/visible"))?;
            std::fs::write(root.join("mixed/deep/added"), "new\n")?;
            std::fs::write(root.join(".git/fsmonitor-paths"), events)?;
            Ok(())
        })?;
        let repo = gix::open_opts(
            &root,
            gix::open::Options::isolated().config_overrides(["core.ignoreCase=false"]),
        )?;
        let (actual, outcome) = status(&repo)?;
        assert_eq!(
            actual,
            git_paths(&root, &[], false)?,
            "events invalidate affected descendants and parents"
        );
        assert!(actual.iter().any(|path| path == "mixed/deep/added"));
        assert!(!actual.iter().any(|path| path == "mixed/visible"));
        let walk = outcome
            .index_worktree
            .dirwalk
            .expect("untracked status walks directories");
        assert!(
            walk.cached_dir_listings > 0,
            "the unchanged stable directory keeps its listing"
        );
        assert!(
            walk.read_dir_calls >= 3,
            "affected root and nested listings are reread: {walk:?}"
        );
    }
    Ok(())
}

#[test]
fn writeback_preserves_valid_siblings_and_git_can_read_the_result() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = writable_fixture()?;
    let root = fixture.path();
    std::fs::remove_file(root.join("mixed/visible"))?;
    std::fs::write(root.join("mixed/deep/added"), "new\n")?;
    std::fs::write(root.join(".git/fsmonitor-paths"), b"mixed/\0")?;
    std::fs::write(root.join(".git/fsmonitor-token"), b"after-mixed-change")?;
    let repo = gix::open_opts(
        root,
        gix::open::Options::isolated().config_overrides(["core.ignoreCase=false"]),
    )?;
    let (expected, mut outcome) = status(&repo)?;
    assert_eq!(expected, git_paths(root, &[], false)?);
    assert!(
        outcome.has_changes(),
        "invalidated directory validity must be persisted"
    );
    outcome.write_changes().expect("the cache invalidation is pending")?;
    let written = repo.index()?;
    assert_eq!(
        written.fs_monitor().expect("writeback retains FSMN").token(),
        &gix::index::extension::fs_monitor::Token::V2 {
            token: "after-mixed-change".into()
        },
        "directory invalidation and token advancement are persisted together"
    );
    let cache = written.untracked().expect("writeback preserves the cache structure");
    assert!(
        cache
            .directories()
            .iter()
            .any(|dir| dir.name() == "stable" && dir.stat().is_some()),
        "the unreported sibling remains valid"
    );
    assert!(
        cache
            .directories()
            .iter()
            .any(|dir| dir.name() == "mixed" && dir.stat().is_none()),
        "the reported directory remains invalid until Git rebuilds its listing"
    );
    assert_eq!(
        git_paths(root, &[], true)?,
        expected,
        "Git accepts and uses the rewritten UNTR extension"
    );

    std::fs::write(root.join(".git/fsmonitor-paths"), b"")?;
    let (actual, repeated) = status(&repo)?;
    assert_eq!(actual, expected);
    assert!(
        !repeated.has_changes(),
        "unchanged token and cache validity do not request another index write"
    );
    Ok(())
}

#[test]
fn failed_and_malformed_monitors_preserve_only_invalidated_cache_data() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for malformed in [false, true] {
        let fixture = writable_fixture()?;
        let root = fixture.path();
        let command = if malformed {
            std::fs::write(root.join(".git/fsmonitor-token"), b"")?;
            ".git/fsmonitor-hook"
        } else {
            ".git/missing-fsmonitor-hook"
        };
        let repo = gix::open_opts(
            root,
            gix::open::Options::isolated().config_overrides([
                format!("core.fsmonitor={command}"),
                "gitoxide.core.externalCommandStderr=false".into(),
            ]),
        )?;
        let (actual, mut outcome) = status(&repo)?;
        assert_eq!(actual, git_paths(root, &[], false)?);
        let walk = outcome.index_worktree.dirwalk.as_ref().expect("dirwalk is enabled");
        assert_eq!(walk.cached_dir_listings, 0, "an unusable monitor cannot validate UNTR");
        assert!(walk.read_dir_calls > 0);
        outcome
            .write_changes()
            .expect("the stale checkpoint must be invalidated")?;
        let written = repo.index()?;
        assert!(written.fs_monitor().is_none(), "no usable checkpoint remains");
        let cache = written
            .untracked()
            .expect("invalidated UNTR remains available for Git to rebuild");
        assert!(
            cache.directories().iter().all(|dir| dir.stat().is_none()),
            "every directory loses validity after monitor failure"
        );
        assert_eq!(
            git_paths(root, &[], true)?,
            actual,
            "Git safely reads the invalidated cache"
        );
    }
    Ok(())
}

#[test]
fn removed_and_replaced_cached_directories_do_not_leave_stale_children() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for (version, replace_with_file) in [(140, false), (141, true)] {
        let root = fixture_with(version, |root| {
            gix_testtools::git(root, "config status.showUntrackedFiles all")?;
            prime_cache(root)?;
            std::fs::remove_dir_all(root.join("untracked"))?;
            if replace_with_file {
                std::fs::write(root.join("untracked"), "now a file\n")?;
            }
            std::fs::write(root.join(".git/fsmonitor-paths"), b"untracked/\0")?;
            Ok(())
        })?;
        let repo = gix::open_opts(
            &root,
            gix::open::Options::isolated().config_overrides(["core.ignoreCase=false"]),
        )?;
        let (actual, outcome) = status(&repo)?;
        assert_eq!(actual, git_paths(&root, &[], false)?);
        assert!(!actual.iter().any(|path| path == "untracked/visible"));
        assert_eq!(actual.iter().any(|path| path == "untracked"), replace_with_file);
        assert!(
            outcome
                .index_worktree
                .dirwalk
                .expect("dirwalk is enabled")
                .read_dir_calls
                > 0,
            "the reported replacement is inspected on disk"
        );
    }
    Ok(())
}

#[test]
fn disabled_and_foreign_caches_fall_back_to_directory_reads() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("make_status_untracked_cache.sh")?;
    let foreign = fixture_with(130, |root| {
        let mut index = gix::index::File::at(
            root.join(".git/index"),
            gix_testtools::object_hash(),
            false,
            Default::default(),
        )?;
        let mut bytes = Vec::new();
        index
            .untracked()
            .expect("Git populated UNTR")
            .write_to(&mut bytes, index.object_hash())?;
        let position = bytes
            .windows(b"Location ".len())
            .position(|bytes| bytes == b"Location ")
            .expect("Git's cache has an identity");
        bytes[position] = b'X';
        let cache = gix::index::extension::untracked_cache::decode(&bytes[8..], index.object_hash(), None)
            .expect("only the cache identity changed");
        index.set_untracked(Some(cache));
        index.write(Default::default())?;
        Ok(())
    })?;
    for (root, overrides) in [(&root, vec!["core.untrackedCache=false"]), (&foreign, Vec::new())] {
        let repo = gix::open_opts(root, gix::open::Options::isolated().config_overrides(overrides))?;
        let (actual, outcome) = status(&repo)?;
        assert_eq!(actual, git_paths(root, &[], false)?);
        let walk = outcome
            .index_worktree
            .dirwalk
            .expect("untracked status walks directories");
        assert_eq!(
            walk.cached_dir_listings, 0,
            "disabled or foreign cache data cannot be reused"
        );
        assert!(walk.read_dir_calls > 0);
    }
    Ok(())
}

#[test]
fn ignored_output_and_pathspecs_bypass_partial_listings() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::scripted_fixture_read_only("make_status_untracked_cache.sh")?;
    let repo = gix::open_opts(&root, gix::open::Options::isolated())?;
    let (actual, outcome) = collect(
        repo.status(gix::progress::Discard)?
            .dirwalk_options(|options| options.emit_ignored(Some(gix::dir::walk::EmissionMode::Matching)))
            .into_index_worktree_iter(None)?,
    )?;
    assert_eq!(actual, git_paths(&root, &["--ignored=matching"], false)?);
    assert_eq!(
        outcome
            .index_worktree
            .dirwalk
            .expect("dirwalk is enabled")
            .cached_dir_listings,
        0
    );
    let (actual, outcome) = collect(
        repo.status(gix::progress::Discard)?
            .into_index_worktree_iter(Some("mixed/".into()))?,
    )?;
    assert_eq!(actual, git_paths(&root, &["--", "mixed/"], false)?);
    assert_eq!(
        outcome
            .index_worktree
            .dirwalk
            .expect("dirwalk is enabled")
            .cached_dir_listings,
        0
    );
    Ok(())
}
