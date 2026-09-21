use super::*;

#[test]
fn metadata_debounce_has_a_maximum_latency() {
    let now = Instant::now();
    let options = Options::default();
    let mut pending = Pending::default();
    let references = || Changes {
        references: true,
        ..Changes::default()
    };
    pending.add(references(), now, &options, false);
    pending.add(references(), now + Duration::from_millis(90), &options, false);
    pending.add(references(), now + Duration::from_millis(180), &options, false);
    assert!(
        pending.take_due(now + Duration::from_millis(249)).is_empty(),
        "a continuing burst remains pending before its maximum age"
    );
    assert!(
        pending.take_due(now + Duration::from_millis(250)).references,
        "continuous events cannot postpone publication indefinitely"
    );
}

fn layout() -> Layout {
    Layout {
        git_dir: "/repo/.git".into(),
        common_dir: "/repo/.git".into(),
        worktree: Some("/repo".into()),
        index: "/repo/.git/index".into(),
        sources: Vec::new(),
    }
}

fn monitor() -> RepositoryMonitor {
    RepositoryMonitor {
        layout: layout(),
        options: Options {
            safety_interval: None,
            ..Options::default()
        },
        metadata: Domain::default(),
        worktree: Domain::default(),
        directories: HashSet::from([PathBuf::from("/repo"), PathBuf::from("/repo/src")]),
        nested_repositories: HashSet::new(),
        projection: Vec::new(),
        pending_metadata: Pending::default(),
        pending_worktree: Changes::default(),
        pending_statistics: Statistics::default(),
        metadata_dirty: false,
        inventory_dirty: false,
        index_dirty: false,
        maintenance: None,
        safety: None,
        backlog: false,
        precompose_unicode: false,
        ignore_case: false,
    }
}

fn event(path: impl Into<PathBuf>, kind: EventKind, path_kind: PathKind) -> Event {
    Event {
        paths: vec![path.into()],
        kind,
        path_kind,
    }
}

fn modified(path: impl Into<PathBuf>) -> Event {
    event(path, EventKind::Modify, PathKind::File)
}

fn fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    let directory = gix_testtools::tempfile::tempdir()?;
    let status = gix_testtools::git_command(directory.path())
        .args(["init", "--quiet", "--initial-branch=main"])
        .status()?;
    assert!(status.success(), "the disposable repository initializes");
    Ok(directory)
}

fn open(path: &Path) -> Result<Repository, crate::open::Error> {
    crate::open_opts(path, crate::open::Options::isolated())
}

#[test]
fn immediate_metadata_does_not_get_delayed_by_a_later_event() {
    let now = Instant::now();
    let mut pending = Pending::default();
    pending.add(Changes::metadata_all(), now, &Options::default(), true);
    pending.add(
        Changes {
            references: true,
            ..Changes::default()
        },
        now,
        &Options::default(),
        false,
    );
    assert!(
        pending.take_due(now).configuration,
        "a coverage loss is delivered immediately even alongside debounced refs"
    );
}

#[test]
fn metadata_classifier_separates_linked_worktrees_and_ignores_housekeeping() {
    let mut layout = layout();
    layout.git_dir = layout.common_dir.join("worktrees/current");
    layout.index = layout.git_dir.join("index");
    for path in [
        "index",
        "index.lock",
        "logs/HEAD",
        "objects/ab/cdef",
        "description",
        "worktrees/other/index",
        "worktrees/other/logs/HEAD",
        "worktrees/other/refs/worktree/private",
    ] {
        assert!(
            classify_metadata(&layout.common_dir.join(path), &layout, false)
                .0
                .is_empty(),
            "{path} does not invalidate references"
        );
    }
    for path in [
        layout.common_dir.join("refs/heads/main"),
        layout.common_dir.join("worktrees/other/HEAD"),
        layout.git_dir.join("refs/worktree/private"),
    ] {
        assert!(
            classify_metadata(&path, &layout, false).0.references,
            "{} affects reference views",
            path.display()
        );
    }
    let (head, rediscover) = classify_metadata(&layout.git_dir.join("HEAD"), &layout, false);
    assert!(
        head.references && rediscover,
        "HEAD must rediscover branch-conditioned configuration includes"
    );
    let (config, rediscover) = classify_metadata(&layout.git_dir.join("config.worktree"), &layout, false);
    assert!(
        config.configuration && rediscover,
        "worktree configuration has its own invalidation"
    );
    let (operation, rediscover) = classify_metadata(&layout.git_dir.join("rebase-merge"), &layout, false);
    assert!(
        operation.operations && rediscover,
        "new operation directories need recursive registration"
    );
    assert!(
        classify_metadata(&layout.common_dir.join("worktrees/new"), &layout, false)
            .0
            .layout,
        "linked-worktree membership is observed"
    );
}

#[test]
fn file_scopes_are_literal_and_do_not_reopen_a_repository() {
    let now = Instant::now();
    let mut monitor = monitor();
    monitor.observe(&modified("/repo/src/[literal]*"), true, now);
    monitor.observe(&modified("/repo/.git/HEAD"), true, now);
    monitor.observe(&modified("/outside/file"), true, now);
    let changes = monitor
        .service(now, || panic!("ordinary files do not require repository maintenance"))
        .changes;
    assert_eq!(
        changes.worktree,
        Scope::Paths(vec!["src/[literal]*".into()]),
        "scope paths are repository-relative bytes, not pathspecs"
    );
    assert!(
        !monitor.inventory_dirty,
        "ordinary file contents do not change native registration"
    );
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let name = std::ffi::OsString::from_vec(vec![b'n', 0xff]);
        monitor.observe(&modified(Path::new("/repo").join(name)), true, now);
        assert_eq!(
            monitor.pending_worktree.worktree,
            Scope::Paths(vec![vec![b'n', 0xff].into()]),
            "non-UTF-8 filenames are preserved"
        );
    }
}

#[test]
fn special_worktree_files_request_the_correct_refresh() {
    let now = Instant::now();
    for (path, scope, ignores, attributes, inventory) in [
        ("src/.gitignore", Scope::Paths(vec!["src".into()]), true, false, true),
        (".gitignore", Scope::All, true, false, true),
        ("src/.gitattributes", Scope::All, false, true, false),
        (".gitmodules", Scope::All, false, true, false),
        ("nested/.git", Scope::Paths(vec!["nested".into()]), false, false, true),
    ] {
        let mut monitor = monitor();
        monitor.observe(&modified(Path::new("/repo").join(path)), true, now);
        assert_eq!(
            monitor.pending_worktree.worktree, scope,
            "{path} invalidates its required scope"
        );
        assert_eq!(
            monitor.pending_worktree.ignores, ignores,
            "{path} identifies ignore changes"
        );
        assert_eq!(
            monitor.pending_worktree.attributes, attributes,
            "{path} identifies attribute/submodule changes"
        );
        assert_eq!(
            monitor.inventory_dirty, inventory,
            "{path} requests registration maintenance only when necessary"
        );
    }
}

#[test]
fn case_and_unicode_settings_apply_to_event_paths() {
    let now = Instant::now();
    let mut monitor = monitor();
    monitor.ignore_case = true;
    monitor.precompose_unicode = true;
    monitor.observe(&modified("/REPO/src/.GITIGNORE"), true, now);
    assert!(
        monitor.pending_worktree.ignores,
        "case-insensitive filesystems recognize special filenames"
    );
    monitor.pending_worktree = Changes::default();
    monitor.observe(&modified("/repo/a\u{308}"), true, now);
    assert_eq!(
        monitor.pending_worktree.worktree,
        Scope::Paths(vec!["ä".into()]),
        "precomposed event paths match Git's index spelling"
    );
    assert!(
        classify_metadata(Path::new("/REPO/.GIT/head"), &monitor.layout, true)
            .0
            .references,
        "metadata comparisons honor ignoreCase too"
    );
    assert!(
        classify_metadata(Path::new("/repo/.git/head"), &monitor.layout, false)
            .0
            .is_empty(),
        "case-sensitive filesystems do not treat lowercase head as HEAD"
    );
}

#[test]
fn directory_replacement_rebuilds_native_handles() {
    let now = Instant::now();
    let mut monitor = monitor();
    monitor.metadata.registered.insert("/repo/.git/refs".into(), true);
    monitor.observe(
        &event("/repo/.git/refs", EventKind::Remove, PathKind::Directory),
        false,
        now,
    );
    assert!(
        monitor.metadata.rebuild,
        "recreating the same pathname must not retain a stale native directory handle"
    );
    monitor.observe(&event("/repo/src", EventKind::Rename, PathKind::Any), true, now);
    assert!(
        monitor.worktree.rebuild && monitor.inventory_dirty,
        "remembered directory topology survives missing filesystem metadata"
    );
    assert_eq!(
        monitor.pending_worktree.worktree,
        Scope::Paths(vec!["src".into()]),
        "renamed directories invalidate all descendants"
    );
}

#[test]
fn index_and_metadata_sources_are_separate() {
    let now = Instant::now();
    let mut monitor = monitor();
    monitor.observe(&modified("/repo/.git/index"), true, now);
    assert!(
        monitor.index_dirty && !monitor.inventory_dirty,
        "an index event compares topology before deciding to walk"
    );
    assert!(monitor.pending_worktree.index, "index changes are explicitly surfaced");
    monitor.layout.sources.push(Source {
        path: "/outside/ignore".into(),
        kind: SourceKind::Ignore,
    });
    monitor.observe(&modified("/outside/ignore"), false, now);
    assert!(
        monitor.pending_metadata.changes.ignores && monitor.inventory_dirty,
        "external ignore dependencies invalidate status and watches"
    );
    assert_eq!(
        monitor.pending_metadata.changes.worktree,
        Scope::All,
        "global ignores affect the complete worktree"
    );
}

#[test]
fn unknown_pathless_and_lost_coverage_invalidate_conservatively() {
    let now = Instant::now();
    for event in [
        Event {
            paths: Vec::new(),
            kind: EventKind::Modify,
            path_kind: PathKind::File,
        },
        event("/repo/file", EventKind::Any, PathKind::Any),
    ] {
        let mut monitor = monitor();
        monitor.observe(&event, true, now);
        assert_eq!(
            monitor.pending_worktree.worktree,
            Scope::All,
            "incomplete native observations cannot retain incremental state"
        );
        assert_eq!(
            monitor.maintenance,
            Some(now),
            "coverage loss immediately requests inventory repair"
        );
    }
    let mut monitor = monitor();
    monitor.coverage_lost(false, now);
    assert!(
        monitor.pending_metadata.take_due(now).configuration,
        "metadata loss invalidates all semantic metadata"
    );
    assert_eq!(
        monitor.pending_worktree.worktree,
        Scope::All,
        "metadata loss can hide changed configuration or ignores"
    );
}

#[test]
fn scope_and_diagnostic_memory_is_bounded() {
    let mut scope = Scope::None;
    for i in 0..=MAX_SCOPES {
        scope.insert(format!("file-{i}").into());
    }
    assert_eq!(scope, Scope::All, "too many scopes promote to a full invalidation");
    let mut scope = Scope::None;
    scope.insert(vec![b'x'; MAX_SCOPE_BYTES + 1].into());
    assert_eq!(scope, Scope::All, "a single long path cannot exceed the scope budget");
    let mut statistics = Statistics::default();
    for i in 0..100 {
        statistics.observe(&modified(format!("/repo/{i}")));
    }
    assert_eq!(
        statistics.received, 100,
        "event counts remain meaningful when samples are truncated"
    );
    assert_eq!(
        statistics.paths.len(),
        MAX_SAMPLE_PATHS,
        "only a bounded sample is retained"
    );
    assert_eq!(
        statistics.omitted_paths,
        100 - MAX_SAMPLE_PATHS,
        "diagnostics report omitted samples"
    );
}

#[test]
fn pending_diagnostics_survive_metadata_debounce() {
    let now = Instant::now();
    let mut monitor = monitor();
    let event = modified("/repo/.git/refs/heads/main");
    monitor.pending_statistics.observe(&event);
    monitor.observe(&event, false, now);
    let initial = monitor.service(now, || panic!("ordinary refs do not change watches"));
    assert!(
        initial.changes.is_empty() && initial.statistics.paths.is_empty(),
        "metadata debounce retains its diagnostics"
    );
    let later = monitor.service(now + Duration::from_millis(100), || {
        panic!("ordinary refs do not change watches")
    });
    assert!(later.changes.references, "the due reference invalidation is delivered");
    assert_eq!(
        later.statistics.paths, event.paths,
        "publication retains the original trigger sample"
    );
}

#[test]
fn reopening_failures_keep_invalidations_and_retry_deadlines() {
    let now = Instant::now();
    let mut monitor = monitor();
    monitor.coverage_lost(false, now);
    let outcome = monitor.service(now, || Err(message("temporarily unavailable").raise().erased()));
    assert_eq!(
        outcome.errors.len(),
        1,
        "the repository error is delivered with its context"
    );
    assert!(
        outcome.changes.configuration && outcome.changes.index,
        "failure does not discard the conservative invalidation"
    );
    assert_eq!(
        monitor.next_timeout(now),
        Some(monitor.options.retry_interval),
        "recovery waits for the retry interval"
    );
    monitor.service(now + Duration::from_secs(1), || panic!("retry is not due yet"));
}

#[test]
fn safety_refresh_and_worktree_toggles_remain_bounded() {
    let now = Instant::now();
    let mut monitor = monitor();
    monitor.options.safety_interval = Some(Duration::from_secs(60));
    monitor.service(now, || panic!("starting the safety timer does not reopen"));
    assert_eq!(
        monitor.next_timeout(now),
        Some(Duration::from_secs(60)),
        "the default safety interval is one minute"
    );
    let outcome = monitor.service(now + Duration::from_secs(60), || {
        Err(message("offline").raise().erased())
    });
    assert!(
        outcome.changes.references && outcome.changes.index,
        "safety refresh covers every repository domain"
    );
    assert!(
        !monitor.metadata.rebuild,
        "periodic safety refresh reconciles without forcing native restarts"
    );
    monitor.set_worktree_enabled(false);
    assert!(
        !monitor.worktree_enabled() && monitor.directories.is_empty(),
        "disabling releases worktree-only state"
    );
    monitor.set_worktree_enabled(true);
    assert!(
        monitor.inventory_dirty && monitor.index_dirty,
        "reenabling reestablishes complete coverage"
    );
    monitor.rescan();
    assert!(
        monitor.metadata.rebuild && monitor.worktree.rebuild,
        "manual refresh explicitly repairs native registrations"
    );
}

#[test]
fn inventory_honors_nested_ignores_and_nested_repository_boundaries() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    for path in [
        "visible/nested",
        "project/fuzz/target/deep/out",
        "project/fuzz/target/visible",
        "nested/inside",
        "ignored/nested",
    ] {
        std::fs::create_dir_all(root.join(path))?;
    }
    std::fs::write(root.join(".gitignore"), "target/\nignored/\n")?;
    std::fs::write(root.join("project/.gitignore"), "!out/\n")?;
    let status = gix_testtools::git_command(root.join("nested"))
        .args(["init", "--quiet"])
        .status()?;
    assert!(status.success(), "a disposable nested repository initializes");
    let repo = open(root)?;
    let index = repo.index_or_empty()?;
    let inventory = inventory::worktree_directories(&repo, &index).map_err(gix_error::Exn::into_error)?;
    let directories = inventory.paths;
    let root = Layout::from_repository(&repo)
        .map_err(gix_error::Exn::into_error)?
        .worktree
        .expect("fixture has a worktree");
    for path in ["", "visible", "visible/nested", "project", "project/fuzz", "nested"] {
        assert!(
            directories.contains(&root.join(path)),
            "{path} is watched, including the nested repository sentinel"
        );
    }
    for path in [
        "project/fuzz/target",
        "project/fuzz/target/deep/out",
        "ignored",
        "nested/inside",
        ".git",
    ] {
        assert!(
            !directories.contains(&root.join(path)),
            "{path} must not be traversed through ignored or repository boundaries"
        );
    }
    let mut monitor = monitor();
    monitor.layout = Layout::from_repository(&repo).map_err(gix_error::Exn::into_error)?;
    monitor.directories = directories;
    monitor.nested_repositories = inventory.repositories;
    monitor.observe(&modified(root.join("nested/file")), true, Instant::now());
    assert_eq!(
        monitor.pending_worktree.worktree,
        Scope::Paths(vec!["nested".into()]),
        "sentinel events never request traversal into a nested repository"
    );
    Ok(())
}

#[test]
fn index_projection_ignores_content_and_tracks_directory_topology() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("file"), "one\n")?;
    let stage = |path| -> gix_testtools::Result {
        let status = gix_testtools::git_command(root).args(["add", path]).status()?;
        assert!(status.success(), "git stages {path} in the fixture");
        Ok(())
    };
    stage("file")?;
    let before = inventory::index_projection(&*open(root)?.index_or_empty()?);
    std::fs::write(root.join("file"), "two\n")?;
    stage("file")?;
    let content = inventory::index_projection(&*open(root)?.index_or_empty()?);
    assert_eq!(
        before, content,
        "object IDs and stat caches do not change watch topology"
    );
    std::fs::create_dir(root.join("new"))?;
    std::fs::write(root.join("new/file"), "new\n")?;
    stage("new/file")?;
    let directory = inventory::index_projection(&*open(root)?.index_or_empty()?);
    assert!(
        inventory::index_changes_directories(&content, &directory),
        "a newly tracked directory requests inventory reconciliation"
    );
    Ok(())
}

#[test]
fn missing_dependencies_follow_the_nearest_existing_ancestor() -> gix_testtools::Result {
    let fixture = fixture()?;
    let repo = open(fixture.path())?;
    let mut layout = Layout::from_repository(&repo).map_err(gix_error::Exn::into_error)?;
    let root = layout.worktree.clone().expect("fixture has a worktree");
    let source = root.join("missing/deeper/config");
    layout.sources.push(Source {
        path: source.clone(),
        kind: SourceKind::Configuration,
    });
    assert!(
        inventory::metadata_watches(&layout).contains_key(&root),
        "missing parents are observed through an existing ancestor"
    );
    std::fs::create_dir_all(root.join("missing/deeper"))?;
    let (changes, refresh) = classify_metadata(&root.join("missing"), &layout, false);
    assert!(
        changes.configuration && refresh,
        "creating an ancestor rediscovers the dependency's parent"
    );
    assert!(
        inventory::metadata_watches(&layout).contains_key(source.parent().expect("source has parent")),
        "registration advances as missing ancestors appear"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn physical_roots_and_config_symlink_replacements_are_both_observed() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    let target = root.join("actual-config");
    std::fs::rename(root.join(".git/config"), &target)?;
    std::os::unix::fs::symlink(&target, root.join(".git/config"))?;
    let repo = open(root)?;
    let layout = Layout::from_repository(&repo).map_err(gix_error::Exn::into_error)?;
    let physical_root = root.canonicalize()?;
    assert_eq!(
        layout.worktree.as_ref(),
        Some(&physical_root),
        "native registrations use the physical root spelling"
    );
    for source in [physical_root.join(".git/config"), physical_root.join("actual-config")] {
        assert!(
            layout.sources.iter().any(|dependency| dependency.path == source),
            "{} remains a configuration dependency",
            source.display()
        );
        assert!(
            classify_metadata(&source, &layout, false).0.configuration,
            "both symlink replacement and target edits invalidate configuration"
        );
    }
    let relative =
        absolute_path(Path::new("missing/config"), &physical_root, false).map_err(gix_error::Exn::into_error)?;
    assert_eq!(
        relative,
        physical_root.join("missing/config"),
        "relative missing paths are made absolute"
    );
    Ok(())
}

#[test]
fn startup_verifies_registration_once_and_unchanged_sets_are_stable() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    let mut monitor = open(root)?
        .monitor(Options {
            safety_interval: None,
            ..Options::default()
        })
        .map_err(gix_error::Exn::into_error)?;
    let now = Instant::now();
    let mut opens = 0;
    let mut opener = || {
        opens += 1;
        open(root).or_erased()
    };
    let initial = monitor.service(now, &mut opener);
    assert!(
        initial.errors.is_empty(),
        "initial registration succeeds: {:?}",
        initial.errors
    );
    assert!(
        initial.changes.references && initial.changes.index,
        "installing watches requests a complete baseline"
    );
    assert_eq!(
        monitor.next_timeout(now),
        Some(Duration::ZERO),
        "installation races require a later inventory verification"
    );
    let verified = monitor.service(now, &mut opener);
    assert!(
        verified.errors.is_empty(),
        "verification succeeds: {:?}",
        verified.errors
    );
    assert_eq!(opens, 2, "each service call opens at most once");
    assert!(
        monitor.maintenance.is_none(),
        "an unchanged inventory does not keep scheduling scans"
    );
    assert_eq!(
        (verified.statistics.added, verified.statistics.removed),
        (0, 0),
        "verification does not churn native registrations"
    );
    Ok(())
}

#[test]
fn head_changes_rediscover_branch_conditioned_sources() -> gix_testtools::Result {
    let fixture = fixture()?;
    let root = fixture.path();
    let config = root.join(".git/config");
    let mut contents = std::fs::read_to_string(&config)?;
    contents.push_str(
        "\n[includeIf \"onbranch:main\"]\npath = main-config\n[includeIf \"onbranch:other\"]\npath = other-config\n",
    );
    std::fs::write(config, contents)?;
    let open = || {
        let mut options = crate::open::Options::isolated();
        options.permissions.config.includes = true;
        crate::open_opts(root, options)
    };
    let mut monitor = open()?
        .monitor(Options {
            safety_interval: None,
            ..Options::default()
        })
        .map_err(gix_error::Exn::into_error)?;
    assert!(
        monitor
            .layout
            .sources
            .iter()
            .any(|source| source.path.ends_with("main-config")),
        "initial sources reflect the current branch"
    );
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/other\n")?;
    let now = Instant::now();
    monitor.observe(&modified(monitor.layout.git_dir.join("HEAD")), false, now);
    let outcome = monitor.service(now, || open().or_erased());
    assert!(
        outcome.errors.is_empty(),
        "branch source rediscovery succeeds: {:?}",
        outcome.errors
    );
    assert!(
        outcome.changes.configuration,
        "switching conditional includes invalidates loaded configuration"
    );
    assert!(
        monitor
            .layout
            .sources
            .iter()
            .any(|source| source.path.ends_with("other-config")),
        "the newly active missing include is observed"
    );
    assert!(
        !monitor
            .layout
            .sources
            .iter()
            .any(|source| source.path.ends_with("main-config")),
        "inactive include paths are removed"
    );
    Ok(())
}
