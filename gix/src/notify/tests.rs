use gix_error::TestResult;

use super::*;

#[test]
fn dependency_classifier_ignores_unrelated_metadata() {
    let mut layout = layout();
    layout.dependencies.push(Dependency {
        path: "/repo/.git/includes/missing/config".into(),
        kind: DependencyKind::Configuration,
    });
    layout.dependencies.push(Dependency {
        path: "/repo/.git/info/exclude".into(),
        kind: DependencyKind::Ignore,
    });
    for path in [
        ".git/config",
        ".git/config.worktree",
        ".git/HEAD",
        ".git/commondir",
        ".git/gitdir",
        ".git/includes",
        ".git/includes/missing/config",
        ".git/info/exclude",
        ".git",
    ] {
        let (changes, refresh) = classify_dependencies(&Path::new("/repo").join(path), &layout, false);
        assert!(refresh, "{path} can change ignore-aware directory coverage");
        assert_eq!(
            changes.worktree,
            Some(Scope::All),
            "{path} invalidates worktree selection"
        );
    }
    for path in [
        "refs",
        "refs/heads/main",
        "rebase-merge",
        "worktrees",
        "worktrees/other/HEAD",
        "info/attributes",
        "logs/HEAD",
        "FETCH_HEAD",
    ] {
        assert_eq!(
            classify_dependencies(&layout.git_dir.join(path), &layout, false),
            (Changes::default(), false),
            "{path} neither publishes metadata nor schedules an unrelated inventory refresh"
        );
    }
}

#[test]
fn all_coverage_classifies_literal_paths_without_inventory() {
    let mut monitor = monitor();
    monitor.options.metadata = false;
    monitor.options.worktree_coverage = Some(WorktreeCoverage::All);
    monitor.layout.common_dir = "/repo/admin".into();
    monitor.layout.index = "/repo/selected-index".into();
    let now = Instant::now();
    let paths = [
        "ignored/deep/file",
        "nested/inside/file",
        "src/.gitignore",
        "src/.gitattributes",
        ".gitmodules",
        "selected-index",
        "selected-index.lock",
    ];
    for path in paths {
        monitor.observe(&modified(Path::new("/repo").join(path)), true, now);
    }
    for path in [
        ".git/index",
        ".git/config",
        "nested/.git/HEAD",
        "nested/.git",
        "admin/refs/main",
    ] {
        monitor.observe(&modified(Path::new("/repo").join(path)), true, now);
    }
    monitor.observe(
        &event("/repo/new-directory", EventKind::Create, PathKind::Directory),
        true,
        now,
    );
    let mut expected: Vec<_> = paths
        .into_iter()
        .map(|path| path_change(path, EventKind::Modify, PathKind::File))
        .collect();
    expected.push(path_change("new-directory", EventKind::Create, PathKind::Directory));
    assert_eq!(
        monitor.pending_worktree,
        Changes {
            worktree: Some(Scope::Paths(expected)),
            ..Changes::default()
        },
        "all coverage includes ignored and nested content but excludes all administrative paths"
    );
    assert!(
        !monitor.inventory_dirty && !monitor.index_dirty && monitor.maintenance.is_none(),
        "recursive native coverage needs no index/ignore inventory for changed files or new directories"
    );
}

#[test]
fn worktree_only_failures_and_manual_refresh_publish_no_metadata() {
    for manual_refresh in [false, true] {
        let now = Instant::now();
        let mut monitor = monitor();
        monitor.options.metadata = false;
        if manual_refresh {
            monitor.rescan();
        } else {
            monitor.coverage_lost(false, now);
            monitor.coverage_lost(true, now);
        }
        let outcome = monitor.service(Instant::now(), || Err(message("unavailable repository").raise()));
        assert_eq!(
            outcome.changes,
            Changes {
                worktree: Some(Scope::All),
                ..Changes::default()
            },
            "loss, manual refresh and recovery errors never publish metadata in worktree-only mode"
        );
        assert_eq!(
            outcome.errors.len(),
            1,
            "filtering metadata does not discard diagnostics"
        );
    }
}

#[test]
fn disabling_worktree_only_releases_dependency_state() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_repo.sh")?;
    let repo = open(&fixture.join("unignored"))?;
    let mut monitor = monitor();
    monitor.options.metadata = false;
    monitor.metadata.watcher = Some(plumbing::Watcher::new(plumbing::Options::default())?);
    monitor.worktree.watcher = Some(plumbing::Watcher::new(plumbing::Options::default())?);
    let options = Options {
        worktree_coverage: None,
        ..monitor.options().clone()
    };
    drop(monitor);
    let mut monitor = repo.monitor(options)?;
    monitor.rescan();
    let now = Instant::now();
    assert!(
        monitor
            .service(now, || panic!("disabled monitoring must not reopen"))
            .changes
            .is_empty(),
        "neither domain publishes when both selections are disabled"
    );
    assert!(
        monitor.metadata.watcher.is_none() && monitor.worktree.watcher.is_none(),
        "disabling worktree-only monitoring also releases internal dependencies"
    );
    assert_eq!(
        monitor.next_timeout(now),
        None,
        "disabled monitoring needs no maintenance wakeups"
    );
    Ok(())
}

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
        dependencies: Vec::new(),
    }
}

fn monitor() -> RepositoryMonitor {
    RepositoryMonitor {
        layout: layout(),
        options: Options::default(),
        metadata: Domain::default(),
        worktree: Domain {
            registered: BTreeMap::from([(PathBuf::from("/repo"), false), (PathBuf::from("/repo/src"), false)]),
            ..Domain::default()
        },
        directories: inventory::directory_lookup([PathBuf::from("/repo"), PathBuf::from("/repo/src")]),
        nested_repositories: HashSet::new(),
        index_fingerprint: None,
        directory_inventory: None,
        changed_directories: HashSet::new(),
        pending_metadata: Pending::default(),
        pending_worktree: Changes::default(),
        pending_statistics: Statistics::default(),
        metadata_dirty: false,
        inventory_dirty: false,
        inventory_verify: false,
        index_dirty: false,
        maintenance: None,
        backlog: false,
        started: false,
        precompose_unicode: false,
        ignore_case: false,
        shared_metadata: false,
        other_worktrees: BTreeMap::new(),
        retired_worktrees: Vec::new(),
        worktree_transition: None,
        maintenance_failed: false,
        undiscovered_worktrees: BTreeMap::new(),
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

fn path_change(path: impl Into<BString>, kind: EventKind, path_kind: PathKind) -> PathChange {
    PathChange {
        path: path.into(),
        kind,
        path_kind,
    }
}

fn fixture() -> TestResult<gix_testtools::tempfile::TempDir> {
    let directory = gix_testtools::tempfile::tempdir()?;
    let status = gix_testtools::git_command(directory.path())
        .args(["init", "--quiet", "--initial-branch=main"])
        .status()?;
    assert!(status.success(), "the disposable repository initializes");
    Ok(directory)
}

fn open(path: &Path) -> Result<Repository> {
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
        head.references && head.configuration && rediscover,
        "HEAD must invalidate and rediscover branch-conditioned configuration"
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
        Some(Scope::Paths(vec![path_change(
            "src/[literal]*",
            EventKind::Modify,
            PathKind::File
        )])),
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
            Some(Scope::Paths(vec![path_change(
                vec![b'n', 0xff],
                EventKind::Modify,
                PathKind::File
            )])),
            "non-UTF-8 filenames are preserved"
        );
    }
}

#[test]
fn literal_scopes_preserve_native_hints_and_both_rename_endpoints() {
    let now = Instant::now();
    for coverage in [WorktreeCoverage::IgnoreAware, WorktreeCoverage::All] {
        for kind in [
            EventKind::Create,
            EventKind::Modify,
            EventKind::Remove,
            EventKind::Rename,
        ] {
            for path_kind in [PathKind::File, PathKind::Directory, PathKind::Any] {
                let mut monitor = monitor();
                monitor.options.worktree_coverage = Some(coverage);
                monitor.nested_repositories.insert("/repo/nested".into());
                let mut observed = event("/repo/src/[literal]*", kind, path_kind);
                let mut expected = vec![path_change("src/[literal]*", kind, path_kind)];
                if kind == EventKind::Rename {
                    observed.paths.push("/repo/src/[destination]*".into());
                    expected.insert(0, path_change("src/[destination]*", kind, path_kind));
                }
                monitor.observe(&observed, true, now);
                monitor.observe(&observed, true, now);
                assert_eq!(
                    monitor.take_changes(now).worktree,
                    Some(Scope::Paths(expected)),
                    "{coverage:?}: duplicate {kind:?}/{path_kind:?} observations preserve literal endpoints and hints"
                );
                assert!(
                    monitor.take_changes(now).is_empty(),
                    "publication drains the observed paths"
                );
                monitor.observe(&event("/repo/nested", kind, path_kind), true, now);
                assert_eq!(
                    monitor.take_changes(now).worktree,
                    Some(Scope::Paths(vec![path_change("nested", kind, path_kind)])),
                    "an event on the sentinel itself is not widened and keeps its native hints"
                );
            }
        }
    }
}

#[test]
fn filesystem_directory_inference_preserves_native_hints() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_repo.sh")?;
    let layout = Layout::from_repository(&open(&fixture.join("unignored"))?)?;
    let directory = layout
        .worktree
        .as_ref()
        .expect("fixture has a worktree")
        .join("tracked");
    assert!(
        directory.is_dir(),
        "the read-only fixture supplies filesystem directory evidence"
    );
    let now = Instant::now();
    for kind in [
        EventKind::Create,
        EventKind::Modify,
        EventKind::Remove,
        EventKind::Rename,
    ] {
        for path_kind in [PathKind::File, PathKind::Any] {
            let mut monitor = monitor();
            monitor.layout = layout.clone();
            monitor.directories.clear();
            monitor.observe(&event(&directory, kind, path_kind), true, now);
            assert!(
                !monitor.changed_directories.is_empty(),
                "filesystem evidence still requests directory maintenance"
            );
            assert_eq!(
                monitor.take_changes(now).worktree,
                Some(Scope::Paths(vec![path_change("tracked", kind, path_kind)])),
                "filesystem evidence must not rewrite {kind:?}/{path_kind:?} from the watcher"
            );
        }
    }
    Ok(())
}

#[test]
fn widened_scopes_do_not_claim_native_events_on_the_parent() {
    let now = Instant::now();
    for (path, parent, sentinel) in [
        ("src/.gitignore", "src", false),
        ("nested/.git", "nested", false),
        ("nested/.git", "nested", true),
        ("nested/inside/file", "nested", true),
    ] {
        for kind in [
            EventKind::Create,
            EventKind::Modify,
            EventKind::Remove,
            EventKind::Rename,
        ] {
            for path_kind in [PathKind::File, PathKind::Directory, PathKind::Any] {
                let mut monitor = monitor();
                if sentinel {
                    monitor.nested_repositories.insert("/repo/nested".into());
                }
                monitor.observe(&event(Path::new("/repo").join(path), kind, path_kind), true, now);
                assert_eq!(
                    monitor.take_changes(now).worktree,
                    Some(Scope::Paths(vec![path_change(
                        parent,
                        EventKind::Any,
                        PathKind::Directory
                    )])),
                    "widening {path} to {parent} cannot claim the parent's {kind:?}/{path_kind:?} event happened"
                );
            }
        }
    }
}

#[test]
fn duplicate_path_hints_coalesce_independently_and_stay_unknown() {
    use EventKind::{Create, Modify, Remove};
    use PathKind::{Any, Directory, File};

    let now = Instant::now();
    for (hints, expected_kind, expected_path_kind) in [
        ([(Create, File), (Create, File), (Create, File)], Create, File),
        ([(Create, File), (Modify, File), (Create, File)], EventKind::Any, File),
        ([(Modify, File), (Modify, Directory), (Modify, File)], Modify, Any),
        ([(Modify, Directory), (Modify, File), (Modify, Directory)], Modify, Any),
        ([(Modify, Any), (Modify, File), (Modify, Directory)], Modify, Any),
        ([(Modify, File), (Modify, Any), (Modify, File)], Modify, Any),
        (
            [(Create, File), (Remove, Directory), (Create, File)],
            EventKind::Any,
            Any,
        ),
    ] {
        let mut monitor = monitor();
        for (kind, path_kind) in hints {
            monitor.observe(&event("/repo/file", kind, path_kind), true, now);
        }
        assert_eq!(
            monitor.take_changes(now).worktree,
            Some(Scope::Paths(vec![path_change(
                "file",
                expected_kind,
                expected_path_kind
            )])),
            "{hints:?} coalesces each hint separately without restoring certainty or invalidating all paths"
        );
    }
}

#[test]
fn merging_changes_preserves_equal_hints_and_sticky_conflicts() {
    let initial = Changes {
        worktree: Some(Scope::Paths(vec![
            path_change("equal", EventKind::Create, PathKind::File),
            path_change("event-conflict", EventKind::Create, PathKind::File),
            path_change("path-conflict", EventKind::Modify, PathKind::File),
            path_change("unknown", EventKind::Any, PathKind::Any),
        ])),
        ..Changes::default()
    };
    let incoming = Changes {
        worktree: Some(Scope::Paths(vec![
            path_change("equal", EventKind::Create, PathKind::File),
            path_change("event-conflict", EventKind::Remove, PathKind::File),
            path_change("path-conflict", EventKind::Modify, PathKind::Directory),
            path_change("unknown", EventKind::Modify, PathKind::File),
        ])),
        ..Changes::default()
    };
    let expected = Changes {
        worktree: Some(Scope::Paths(vec![
            path_change("equal", EventKind::Create, PathKind::File),
            path_change("event-conflict", EventKind::Any, PathKind::File),
            path_change("path-conflict", EventKind::Modify, PathKind::Any),
            path_change("unknown", EventKind::Any, PathKind::Any),
        ])),
        ..Changes::default()
    };
    for (mut changes, other) in [(initial.clone(), incoming.clone()), (incoming, initial.clone())] {
        changes.merge(other);
        assert_eq!(changes, expected, "merging hints is independent of batch order");
        changes.merge(initial.clone());
        assert_eq!(
            changes, expected,
            "later concrete hints cannot undo either kind of uncertainty"
        );
    }
}

#[test]
fn published_scopes_are_ordered_by_path_bytes() {
    let now = Instant::now();
    let mut monitor = monitor();
    for (path, kind, path_kind) in [
        ("z", EventKind::Create, PathKind::Directory),
        ("a/[literal]*", EventKind::Modify, PathKind::File),
        ("a", EventKind::Rename, PathKind::Any),
        ("Z", EventKind::Remove, PathKind::File),
    ] {
        monitor.observe(&event(Path::new("/repo").join(path), kind, path_kind), true, now);
    }
    let expected = vec![
        path_change("Z", EventKind::Remove, PathKind::File),
        path_change("a", EventKind::Rename, PathKind::Any),
        path_change("a/[literal]*", EventKind::Modify, PathKind::File),
        path_change("z", EventKind::Create, PathKind::Directory),
    ];
    #[cfg(unix)]
    let expected = {
        use std::os::unix::ffi::OsStringExt;
        let mut expected = expected;
        for byte in [0xff, 0x80] {
            let path = std::ffi::OsString::from_vec(vec![b'n', byte]);
            monitor.observe(&modified(Path::new("/repo").join(path)), true, now);
        }
        expected.insert(3, path_change(vec![b'n', 0x80], EventKind::Modify, PathKind::File));
        expected.insert(4, path_change(vec![b'n', 0xff], EventKind::Modify, PathKind::File));
        expected
    };
    assert_eq!(
        monitor.take_changes(now).worktree,
        Some(Scope::Paths(expected)),
        "publication sorts literal path bytes, not native event order, hints, or decoded Unicode"
    );
}

#[test]
fn special_worktree_files_request_the_correct_refresh() {
    let now = Instant::now();
    for (path, scope, ignores, attributes, inventory) in [
        (
            "src/.gitignore",
            Scope::Paths(vec![path_change("src", EventKind::Any, PathKind::Directory)]),
            true,
            false,
            true,
        ),
        (".gitignore", Scope::All, true, false, true),
        ("src/.gitattributes", Scope::All, false, true, false),
        (".gitmodules", Scope::All, false, true, false),
        (
            "nested/.git",
            Scope::Paths(vec![path_change("nested", EventKind::Any, PathKind::Directory)]),
            false,
            false,
            true,
        ),
    ] {
        let mut monitor = monitor();
        monitor.observe(&modified(Path::new("/repo").join(path)), true, now);
        assert_eq!(
            monitor.pending_worktree.worktree,
            Some(scope),
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
        Some(Scope::Paths(vec![path_change("ä", EventKind::Modify, PathKind::File)])),
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
        monitor.worktree.rebuild && !monitor.changed_directories.is_empty(),
        "remembered directory topology survives missing filesystem metadata"
    );
    assert_eq!(
        monitor.pending_worktree.worktree,
        Some(Scope::Paths(vec![path_change("src", EventKind::Rename, PathKind::Any)])),
        "renamed directories invalidate all descendants without replacing the native path-kind hint"
    );
    for (path, ignore_case, rebuild) in [
        ("/repo//./src/", false, true),
        ("/REPO//./SRC/", true, true),
        ("/REPO/src", false, false),
        ("/repo/src/file", true, false),
        ("/repo/src-other", true, false),
    ] {
        let mut monitor = self::monitor();
        monitor.ignore_case = ignore_case;
        monitor.observe(&event(path, EventKind::Remove, PathKind::Any), true, now);
        assert_eq!(
            monitor.worktree.rebuild, rebuild,
            "directory membership respects path components and ignoreCase for {path}"
        );
    }
}

#[test]
fn index_and_metadata_dependencies_are_separate() {
    let now = Instant::now();
    let mut monitor = monitor();
    monitor.observe(&modified("/repo/.git/index"), true, now);
    assert!(
        monitor.index_dirty && !monitor.inventory_dirty,
        "an index event compares topology before deciding to walk"
    );
    assert!(monitor.pending_worktree.index, "index changes are explicitly surfaced");
    monitor.layout.dependencies.push(Dependency {
        path: "/repo/.git/info/exclude".into(),
        kind: DependencyKind::Ignore,
    });
    monitor.observe(&modified("/repo/.git/info/exclude"), false, now);
    assert!(
        monitor.pending_metadata.changes.ignores && monitor.inventory_dirty,
        "repository-local ignore dependencies invalidate status and watches"
    );
    assert_eq!(
        monitor.pending_metadata.changes.worktree,
        Some(Scope::All),
        "repository-local excludes affect the complete worktree"
    );
    monitor.observe(&event("/repo/new", EventKind::Create, PathKind::Directory), true, now);
    assert!(
        monitor.changed_directories.contains(Path::new("/repo/new")),
        "pending ignore reevaluation must not hide a simultaneous directory creation"
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
        event("/repo/file", EventKind::Any, PathKind::File),
        event("/repo/src", EventKind::Any, PathKind::Directory),
    ] {
        let mut monitor = monitor();
        monitor.observe(&event, true, now);
        assert_eq!(
            monitor.pending_worktree.worktree,
            Some(Scope::All),
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
        Some(Scope::All),
        "metadata loss can hide changed configuration or ignores"
    );
}

#[test]
fn scope_memory_is_bounded() {
    let mut changes = Changes::default();
    assert_eq!(changes.worktree, None, "default changes have no worktree scope");
    assert!(changes.is_empty(), "default changes have no invalidations");
    changes.merge(Changes {
        worktree: Some(Scope::Paths(Vec::new())),
        ..Changes::default()
    });
    assert_eq!(
        changes.worktree, None,
        "merging no paths does not create a worktree scope"
    );
    assert!(
        changes.is_empty(),
        "merging no paths does not introduce an invalidation"
    );
    for worktree in [
        None,
        Some(Scope::Paths(vec![path_change(
            "file",
            EventKind::Modify,
            PathKind::File,
        )])),
        Some(Scope::All),
    ] {
        let expected = Changes {
            worktree,
            references: true,
            ..Changes::default()
        };
        let mut merged = expected.clone();
        merged.merge(Changes::default());
        assert_eq!(merged, expected, "an absent scope preserves existing invalidations");
        let mut merged = Changes::default();
        merged.merge(expected.clone());
        assert_eq!(merged, expected, "an absent scope accepts incoming invalidations");
    }
    for i in 0..MAX_SCOPES {
        changes.insert_worktree_path(path_change(format!("file-{i}"), EventKind::Modify, PathKind::File));
    }
    changes.insert_worktree_path(path_change("file-0", EventKind::Create, PathKind::Directory));
    assert!(
        matches!(&changes.worktree, Some(Scope::Paths(paths)) if paths.len() == MAX_SCOPES
            && paths[0] == path_change("file-0", EventKind::Any, PathKind::Any)),
        "the exact path-count budget permits duplicates and coalesces their hints without counting them again"
    );
    changes.insert_worktree_path(path_change("one-too-many", EventKind::Create, PathKind::File));
    assert_eq!(
        changes.worktree,
        Some(Scope::All),
        "too many scopes promote to a full invalidation"
    );
    changes.insert_worktree_path(path_change("file-0", EventKind::Modify, PathKind::File));
    changes.merge(Changes {
        worktree: Some(Scope::Paths(vec![path_change(
            "new",
            EventKind::Create,
            PathKind::File,
        )])),
        ..Changes::default()
    });
    assert_eq!(
        changes.worktree,
        Some(Scope::All),
        "insertion and merging cannot narrow a full invalidation"
    );
    let mut partial = Changes::default();
    partial.insert_worktree_path(path_change("file", EventKind::Modify, PathKind::File));
    partial.merge(changes);
    assert_eq!(
        partial.worktree,
        Some(Scope::All),
        "an incoming full invalidation replaces path hints"
    );
    let mut changes = Changes::default();
    changes.insert_worktree_path(path_change(
        vec![b'x'; MAX_SCOPE_BYTES + 1],
        EventKind::Modify,
        PathKind::File,
    ));
    assert_eq!(
        changes.worktree,
        Some(Scope::All),
        "a single long path cannot exceed the scope budget"
    );
    let mut changes = Changes::default();
    for byte in *b"xy" {
        changes.insert_worktree_path(path_change(
            vec![byte; MAX_SCOPE_BYTES / 2],
            EventKind::Modify,
            PathKind::File,
        ));
    }
    changes.merge(Changes {
        worktree: Some(Scope::Paths(vec![path_change(
            vec![b'x'; MAX_SCOPE_BYTES / 2],
            EventKind::Remove,
            PathKind::File,
        )])),
        ..Changes::default()
    });
    assert!(
        matches!(&changes.worktree, Some(Scope::Paths(paths)) if paths.len() == 2
            && paths.iter().map(|change| change.path.len()).sum::<usize>() == MAX_SCOPE_BYTES
            && paths[0].kind == EventKind::Any && paths[0].path_kind == PathKind::File),
        "the byte budget counts only distinct path bytes, not hints or duplicate observations merged at the limit"
    );
    changes.merge(Changes {
        worktree: Some(Scope::Paths(vec![path_change("z", EventKind::Create, PathKind::File)])),
        ..Changes::default()
    });
    assert_eq!(
        changes.worktree,
        Some(Scope::All),
        "one more path byte exceeds the cumulative budget"
    );
}

#[test]
fn pending_diagnostics_survive_metadata_debounce() {
    let now = Instant::now();
    let mut monitor = monitor();
    let event = modified("/repo/.git/refs/heads/main");
    monitor.pending_statistics.received = 1;
    monitor.observe(&event, false, now);
    let initial = monitor.service(now, || panic!("ordinary refs do not change watches"));
    assert!(
        initial.changes.is_empty() && initial.statistics.received == 0,
        "metadata debounce retains its diagnostics"
    );
    let later = monitor.service(now + Duration::from_millis(100), || {
        panic!("ordinary refs do not change watches")
    });
    assert!(later.changes.references, "the due reference invalidation is delivered");
    assert_eq!(
        later.statistics.received, 1,
        "publication retains the original event count"
    );
}

#[test]
fn reopening_failures_keep_invalidations_and_retry_deadlines() {
    let now = Instant::now();
    let mut monitor = monitor();
    monitor.coverage_lost(false, now);
    let outcome = monitor.service(now, || Err(message("temporarily unavailable").raise()));
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
fn idle_monitor_does_not_refresh_by_elapsed_time() {
    let now = Instant::now();
    let mut monitor = monitor();
    for now in [now, now + Duration::from_secs(3600)] {
        let outcome = monitor.service(now, || panic!("idle monitoring must not reopen the repository"));
        assert!(outcome.changes.is_empty(), "elapsed time does not invalidate caches");
        assert_eq!(
            monitor.next_timeout(now),
            None,
            "idle monitoring has no refresh deadline"
        );
    }
}

#[test]
fn manual_refresh_invalidates_all_caches_even_when_native_recovery_is_delayed() -> TestResult {
    let mut monitor = monitor();
    let mut watcher = plumbing::Watcher::new(plumbing::Options::default())?;
    watcher.set_waker(|| panic!("simulate a queued worktree notification error"));
    monitor.worktree.watcher = Some(watcher);
    monitor.rescan();
    let now = Instant::now();
    let outcome = monitor.service(now, || panic!("native recovery must respect its retry delay"));
    assert_eq!(outcome.errors.len(), 1, "the queued worktree failure is reported");
    let mut expected = Changes::metadata_all();
    expected.merge(Changes::worktree_all());
    assert_eq!(
        outcome.changes, expected,
        "manual refresh invalidates every cache even when a worktree error postpones maintenance"
    );
    assert_eq!(
        monitor.next_timeout(now),
        Some(monitor.options.retry_interval),
        "full invalidation does not bypass native recovery backoff"
    );
    Ok(())
}

#[test]
fn inventory_honors_nested_ignores_and_nested_repository_boundaries() -> TestResult {
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
    let inventory = inventory::worktree_directories(&repo, &index)?;
    let recursive = inventory::recursive_root_watch(root)?;
    assert_eq!(
        recursive,
        plumbing::Watch {
            path: root.canonicalize()?,
            recursive: true,
        },
        "all coverage uses one native canonical root, not the ignore-aware directory inventory"
    );
    let shallow = inventory.watches_for_backend(false);
    assert_eq!(
        inventory.watches_for_backend(true),
        inventory.watches,
        "native recursive coverage uses the compact directory plan"
    );
    let directories = inventory.watches;
    let root = Layout::from_repository(&repo)?
        .worktree
        .expect("fixture has a worktree");
    assert_eq!(
        shallow,
        ["", "visible", "visible/nested", "project", "project/fuzz", "nested"]
            .into_iter()
            .map(|path| (root.join(path), false))
            .collect(),
        "emulated recursion uses every included directory found by the walk, without ignored or nested repository descendants"
    );
    for (path, recursive) in [
        ("", false),
        ("visible", true),
        ("project", false),
        ("project/fuzz", false),
        ("nested", false),
    ] {
        assert_eq!(
            directories.get(&root.join(path)),
            Some(&recursive),
            "{path} uses recursive coverage only without ignored or repository boundaries"
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
            !directories.contains_key(&root.join(path)),
            "{path} must not be traversed through ignored or repository boundaries"
        );
    }
    let mut monitor = monitor();
    monitor.layout = Layout::from_repository(&repo)?;
    assert!(
        !directories.contains_key(&root.join("visible/nested")),
        "recursive roots cover their descendants without separate registrations"
    );
    monitor.directories = inventory::directory_lookup(directories.into_keys());
    monitor.nested_repositories = inventory.repositories;
    monitor.observe(&modified(root.join("nested/file")), true, Instant::now());
    assert_eq!(
        monitor.pending_worktree.worktree,
        Some(Scope::Paths(vec![path_change(
            "nested",
            EventKind::Any,
            PathKind::Directory
        )])),
        "sentinel events never request traversal into a nested repository or claim it was modified"
    );
    Ok(())
}

#[test]
fn inventory_rechecks_exclusions_and_rediscovers_only_reenabled_subtrees() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path().canonicalize()?;
    std::fs::create_dir_all(root.join("visible/deep"))?;
    std::fs::create_dir_all(root.join("ignored/deep"))?;
    std::fs::write(root.join("ignored/deep/file"), "tracked later\n")?;
    std::fs::write(root.join(".gitignore"), "ignored/\n")?;
    let repo = open(&root)?;
    let index = repo.index_or_empty()?;
    let mut inventory = inventory::worktree_directories(&repo, &index)?;
    assert!(
        !inventory.watches.contains_key(&root.join("ignored")),
        "fully ignored directories have no watches"
    );

    std::fs::write(root.join(".gitignore"), "# Expose the subtree.\n")?;
    let discover = inventory.recheck(&open(&root)?, &index)?;
    assert_eq!(
        discover,
        HashSet::from([root.join("ignored")]),
        "only never-discovered subtrees need an initial walk"
    );
    assert_eq!(
        inventory.read_dir_calls, 0,
        "rule reevaluation uses the recorded directory inventory"
    );
    inventory = inventory::worktree_directories_reusing(&open(&root)?, &index, Some(&inventory), &discover)?;
    assert_eq!(
        inventory.read_dir_calls, 3,
        "discovery reads the root and newly exposed subtree, skipping visible/"
    );

    std::fs::write(root.join(".gitignore"), "ignored/\n")?;
    assert!(
        inventory.recheck(&open(&root)?, &index)?.is_empty(),
        "excluding a known directory requires no discovery"
    );
    assert_eq!(
        inventory.read_dir_calls, 0,
        "exclusion changes use the directory inventory without a walk"
    );
    std::fs::create_dir_all(root.join("ignored/new/cache/deep"))?;
    std::fs::write(root.join("ignored/new/.gitignore"), "cache/\n")?;
    std::fs::write(root.join(".gitignore"), "# Reenable the subtree.\n")?;
    let discover = inventory.recheck(&open(&root)?, &index)?;
    assert_eq!(
        discover,
        HashSet::from([root.join("ignored")]),
        "changes inside unwatched subtrees require local rediscovery"
    );
    inventory = inventory::worktree_directories_reusing(&open(&root)?, &index, Some(&inventory), &discover)?;
    assert!(
        !inventory.watches.contains_key(&root.join("ignored/new/cache")),
        "rediscovery applies ignore files created while unwatched"
    );
    std::fs::write(root.join(".gitignore"), "ignored/\n")?;
    inventory.recheck(&open(&root)?, &index)?;
    let before = inventory.index_fingerprint(&index, false)?;
    gix_testtools::git(&root, "add -f -- ignored/deep/file")?;
    let repo = open(&root)?;
    let index = repo.index_or_empty()?;
    assert_ne!(
        inventory.index_fingerprint(&index, false)?,
        before,
        "tracking an ignored path changes coverage"
    );
    let discover = inventory.recheck(&repo, &index)?;
    assert_eq!(
        discover,
        HashSet::from([root.join("ignored")]),
        "index changes rediscover only the newly covered ignored subtree"
    );
    inventory = inventory::worktree_directories_reusing(&repo, &index, Some(&inventory), &discover)?;
    assert!(
        inventory.watches.contains_key(&root.join("ignored")),
        "tracked paths retain coverage despite ignores"
    );
    Ok(())
}

#[test]
fn directory_changes_reuse_unaffected_recursive_subtrees() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path().canonicalize()?;
    std::fs::create_dir_all(root.join("visible/deep"))?;
    std::fs::create_dir_all(root.join("other/deep/deeper"))?;
    let repo = open(&root)?;
    let index = repo.index_or_empty()?;
    let inventory = inventory::worktree_directories(&repo, &index)?;
    std::fs::create_dir_all(root.join("visible/new/a/cache/deep"))?;
    std::fs::write(root.join("visible/new/.gitignore"), "cache/\n")?;
    let refreshed = inventory::worktree_directories_reusing(
        &repo,
        &index,
        Some(&inventory),
        &HashSet::from([root.join("visible/new")]),
    )?;
    assert_eq!(
        refreshed.read_dir_calls, 4,
        "only the changed branch and its containing directories are read"
    );
    assert!(
        !refreshed.watches.contains_key(&root.join("visible/new/a/cache")),
        "new nested ignores prune complete directories"
    );
    assert_eq!(
        refreshed.watches.get(&root.join("other")),
        Some(&true),
        "unaffected subtrees keep their recursive watches"
    );
    assert_eq!(
        refreshed.watches.get(&root.join("visible/deep")),
        Some(&true),
        "splitting a recursive root preserves its known siblings"
    );
    Ok(())
}

#[test]
fn directory_lookup_hashes_components_and_resolves_case_collisions() -> TestResult {
    let paths = [PathBuf::from("/repo/Src"), PathBuf::from("/repo/Ä")].into_iter();
    #[cfg(unix)]
    let paths = paths.chain([gix_path::from_bstr(crate::bstr::BStr::new(b"/repo/\xffDir"))?.into_owned()]);
    let paths: Vec<_> = paths.collect();
    let lookup = inventory::directory_lookup(paths.clone());
    for directory in paths {
        assert!(
            lookup
                .find(inventory::path_hash(&directory), |path| same_path(
                    path, &directory, false
                ))
                .is_some(),
            "native byte paths survive insertion without text conversion"
        );
    }
    for (query, ignore_case, found) in [
        ("/repo//./Src/", false, true),
        ("/REPO/src", true, true),
        ("/repo/src", false, false),
        ("/repo/ä", true, false),
        ("/repo/Src/file", true, false),
    ] {
        let query = Path::new(query);
        assert_eq!(
            lookup
                .find(inventory::path_hash(query), |path| same_path(query, path, ignore_case))
                .is_some(),
            found,
            "hash lookup uses native components and ASCII-only case rules for {query:?}"
        );
    }
    #[cfg(windows)]
    for (stored, query) in [
        (r"\\server\share\dir", r"//server/share/dir"),
        (r"\\.\COM42\dir", r"//./COM42/dir"),
        (r"C:\dir", r"c:/dir"),
    ] {
        let query = Path::new(query);
        let lookup = inventory::directory_lookup([PathBuf::from(stored)]);
        assert!(
            lookup
                .find(inventory::path_hash(query), |path| same_path(query, path, false))
                .is_some(),
            "equivalent Windows prefixes hash equally even when their raw spellings differ"
        );
    }
    Ok(())
}

#[test]
fn index_fingerprint_tracks_only_ignore_and_repository_entries() -> TestResult {
    use gix_index::entry::{Flags, Mode, Stage};

    let project = |entries: &[(&[u8], Mode, Flags)]| {
        let mut index = gix_index::State::new(gix_hash::Kind::Sha1);
        for &(path, mode, flags) in entries {
            index.dangerously_push_entry(
                Default::default(),
                gix_hash::ObjectId::null(gix_hash::Kind::Sha1),
                flags,
                mode,
                path.into(),
            );
        }
        index.sort_entries();
        inventory::index_fingerprint(&index)
    };
    let file = (b"dir/\xff".as_slice(), Mode::FILE, Flags::empty());
    let before = project(&[file])?;
    for (entries, changed, reason) in [
        (
            vec![(b"a".as_slice(), Mode::FILE, Flags::empty()), file],
            false,
            "ordinary file entries do not affect the coverage fingerprint",
        ),
        (
            vec![file, (b"z".as_slice(), Mode::FILE, Flags::empty())],
            false,
            "ordinary file entries do not affect the coverage fingerprint",
        ),
        (
            vec![(b"dir/x".as_slice(), file.1, file.2)],
            false,
            "ordinary file entries do not affect the coverage fingerprint",
        ),
        (
            vec![(file.0, Mode::SYMLINK, file.2)],
            false,
            "ordinary file entries do not affect the coverage fingerprint",
        ),
        (
            vec![(file.0, file.1, Flags::SKIP_WORKTREE)],
            false,
            "ordinary file entries do not affect the coverage fingerprint",
        ),
        (
            vec![(file.0, file.1, Flags::from_stage(Stage::Ours))],
            false,
            "ordinary file entries do not affect the coverage fingerprint",
        ),
        (
            vec![(file.0, file.1, Flags::FSMONITOR_VALID)],
            false,
            "unrelated flags do not affect coverage",
        ),
        (
            vec![(file.0, file.1, Flags::ASSUME_VALID)],
            false,
            "ordinary file entries do not affect the coverage fingerprint",
        ),
        (
            vec![(file.0, file.1, Flags::UPTODATE)],
            false,
            "ordinary file entries do not affect the coverage fingerprint",
        ),
        (
            vec![(b"submodule".as_slice(), Mode::COMMIT, Flags::empty()), file],
            true,
            "root submodule boundaries affect coverage",
        ),
        (
            vec![(b"sparse".as_slice(), Mode::DIR, Flags::SKIP_WORKTREE), file],
            true,
            "root sparse directories affect coverage",
        ),
        (vec![], false, "ordinary file removals do not affect coverage"),
    ] {
        let after = project(&entries)?;
        assert_eq!(before != after, changed, "{reason}");
    }
    let stages = [
        (file.0, Mode::SYMLINK, Flags::from_stage(Stage::Base)),
        (file.0, Mode::FILE, Flags::from_stage(Stage::Ours)),
    ];
    assert_eq!(
        project(&stages)?,
        project(&[stages[1], stages[0]])?,
        "backing storage order is irrelevant, including conflict stages with different modes"
    );
    let entries = [(b"a".as_slice(), file.1, file.2), file];
    assert_eq!(
        project(&entries)?,
        project(&[entries[1], entries[0]])?,
        "path offsets do not contribute to the fingerprint"
    );
    Ok(())
}

#[test]
fn index_fingerprint_tracks_ignore_blob_ids() -> TestResult {
    for (path, changed) in [
        (".gitignore", true),
        ("dir/.gitignore", true),
        ("dir/.GITIGNORE", true),
        ("dir/ordinary", false),
        ("dir/other.gitignore", false),
    ] {
        let mut index = gix_index::State::new(gix_hash::Kind::Sha1);
        index.dangerously_push_entry(
            Default::default(),
            gix_hash::ObjectId::null(gix_hash::Kind::Sha1),
            gix_index::entry::Flags::SKIP_WORKTREE,
            gix_index::entry::Mode::FILE,
            path.into(),
        );
        let before = inventory::index_fingerprint(&index)?;
        index.entries_mut()[0].id = gix_hash::ObjectId::empty_blob(gix_hash::Kind::Sha1);
        assert_eq!(
            before != inventory::index_fingerprint(&index)?,
            changed,
            "only ignore-rule contents can change coverage for {path}"
        );
    }
    Ok(())
}

#[test]
fn index_fingerprint_ignores_content_and_tracks_directory_topology() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    std::fs::write(root.join("file"), "one\n")?;
    let stage = |path| -> TestResult {
        let status = gix_testtools::git_command(root).args(["add", path]).status()?;
        assert!(status.success(), "git stages {path} in the fixture");
        Ok(())
    };
    stage("file")?;
    let before = inventory::index_fingerprint(&*open(root)?.index_or_empty()?)?;
    std::fs::write(root.join("file"), "two\n")?;
    stage("file")?;
    let content = inventory::index_fingerprint(&*open(root)?.index_or_empty()?)?;
    assert_eq!(
        before, content,
        "object IDs and stat caches do not change watch topology"
    );
    std::fs::create_dir(root.join("new"))?;
    std::fs::write(root.join("new/file"), "new\n")?;
    stage("new/file")?;
    let directory = inventory::index_fingerprint(&*open(root)?.index_or_empty()?)?;
    assert_eq!(
        content, directory,
        "ordinary staging leaves directory coverage unchanged"
    );
    Ok(())
}

#[test]
fn metadata_watches_exclude_ancestors_and_external_configuration() -> TestResult {
    let fixture = fixture()?;
    let external = gix_testtools::tempfile::tempdir()?;
    let config = external.path().join("config");
    let ignore = external.path().join("ignore");
    let attributes = external.path().join("attributes");
    std::fs::write(&config, "[monitor]\nvalue = before\n")?;
    std::fs::write(&ignore, "ignored\n")?;
    std::fs::write(&attributes, "* text\n")?;
    let open = || {
        let mut options = crate::open::Options::isolated()
            .system_config_path(&config)
            .config_overrides([
                format!("core.excludesFile={}", ignore.display()),
                format!("core.attributesFile={}", attributes.display()),
            ]);
        options.permissions.config.system = true;
        crate::open_opts(fixture.path(), options)
    };
    let repo = open()?;
    let mut layout = Layout::from_repository(&repo)?;
    let root = layout.worktree.clone();
    let is_local = |path: &Path| {
        path.starts_with(&layout.git_dir)
            || path.starts_with(&layout.common_dir)
            || root.as_ref().is_some_and(|root| path.starts_with(root))
    };
    assert!(
        layout
            .dependencies
            .iter()
            .all(|dependency| is_local(&dependency.path) || dependency.kind == DependencyKind::Ignore),
        "only external ignore files become notification dependencies: {:?}",
        layout.dependencies
    );
    let external_root = external.path().canonicalize()?;
    for worktree in [root, None] {
        layout.worktree = worktree;
        let watches = inventory::metadata_watches(&layout)?;
        assert!(
            watches.keys().all(|path| path == &external_root
                || path.starts_with(&layout.git_dir)
                || path.starts_with(&layout.common_dir)
                || layout.worktree.as_ref().is_some_and(|root| path.starts_with(root))),
            "normal and bare views never watch repository ancestors or external configuration directories: {watches:?}"
        );
        assert!(
            watches.contains_key(&layout.git_dir),
            "local HEAD, configuration and index files remain covered"
        );
        assert!(
            watches.contains_key(&layout.common_dir.join("info")),
            "repository-local ignore and attribute files remain covered"
        );
    }
    std::fs::write(&config, "[monitor]\nvalue = after\n")?;
    assert_eq!(
        open()?
            .config_snapshot()
            .string("monitor.value")
            .map(|value| value.to_vec()),
        Some(b"after".to_vec()),
        "reopening naturally reads external configuration without a watcher"
    );
    drop(repo);
    layout.dependencies.retain(|dependency| {
        layout.git_dir.starts_with(&dependency.path) || dependency.path.starts_with(&layout.git_dir)
    });
    std::fs::remove_dir_all(&layout.git_dir)?;
    assert!(
        inventory::metadata_watches(&layout)?.is_empty(),
        "a missing repository root does not broaden coverage to its parent"
    );
    Ok(())
}

#[test]
fn metadata_inventory_limits_linked_worktrees_to_administrative_roots() -> TestResult {
    let fixture = fixture()?;
    let repo = open(fixture.path())?;
    let mut layout = Layout::from_repository(&repo)?;
    let worktrees = layout.common_dir.join("worktrees");
    for path in [
        "current/refs/worktree/tix/pins",
        "current/rebase-merge",
        "other/refs/worktree/tix/pins",
        "other/logs/refs/worktree/tix",
        "other/rebase-merge",
        "incomplete",
    ] {
        std::fs::create_dir_all(worktrees.join(path))?;
    }
    for git_dir in [layout.common_dir.clone(), worktrees.join("current")] {
        layout.git_dir = git_dir;
        layout.index = layout.git_dir.join("index");
        let dependencies = inventory::dependency_watches(&layout);
        assert!(
            dependencies.values().all(|recursive| !recursive),
            "ignore-aware dependencies never recursively inventory refs or operation state"
        );
        for directory in [&layout.git_dir, &layout.common_dir] {
            assert!(
                dependencies.contains_key(directory),
                "private and shared roots cover HEAD, configuration and layout dependencies"
            );
        }
        assert!(
            !dependencies.contains_key(&worktrees) && !dependencies.contains_key(&worktrees.join("other")),
            "worktree-only coverage does not enumerate linked-worktree membership"
        );
        let watches = inventory::metadata_watches(&layout)?;
        assert_eq!(
            watches.get(&worktrees),
            Some(&false),
            "worktree membership needs only immediate directory entries"
        );
        for name in ["current", "other", "incomplete"] {
            let directory = worktrees.join(name);
            assert_eq!(
                watches.get(&directory),
                Some(&false),
                "each administrative root is watched even before its gitdir file exists"
            );
            if directory != layout.git_dir {
                for relative in ["refs", "logs", "rebase-merge"] {
                    let nested = directory.join(relative);
                    assert!(
                        !watches
                            .iter()
                            .any(|(root, recursive)| *recursive && nested.starts_with(root)),
                        "foreign {name}/{relative} must not have recursive coverage"
                    );
                }
            }
        }
        for directory in [&layout.common_dir, &layout.git_dir] {
            assert_eq!(
                watches.get(&directory.join("refs")),
                Some(&true),
                "shared and current-worktree references retain recursive coverage"
            );
        }
        if layout.git_dir != layout.common_dir {
            assert_eq!(
                watches.get(&layout.git_dir.join("rebase-merge")),
                Some(&true),
                "the current worktree's operation state retains recursive coverage"
            );
        }
    }
    Ok(())
}

#[test]
fn linked_worktree_membership_reconciles_shallow_watches() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let mut monitor = open(root)?.monitor(Options {
        worktree_coverage: None,
        ..Options::default()
    })?;
    let now = Instant::now();
    for _ in 0..2 {
        let outcome = monitor.service(now, || open(root));
        assert!(
            outcome.errors.is_empty(),
            "startup installs and verifies metadata watches"
        );
    }

    let worktrees = monitor.layout.common_dir.join("worktrees");
    let added = worktrees.join("new");
    std::fs::create_dir_all(&added)?;
    monitor.observe(&event(&worktrees, EventKind::Create, PathKind::Directory), false, now);
    let outcome = monitor.service(now, || open(root));
    assert!(outcome.errors.is_empty(), "first-worktree discovery succeeds");
    assert_eq!(
        monitor.metadata.registered.get(&worktrees),
        Some(&false),
        "the first linked worktree installs shallow membership coverage"
    );
    assert_eq!(
        monitor.metadata.registered.get(&added),
        Some(&false),
        "an incomplete worktree is covered before HEAD and gitdir are written"
    );

    let head = added.join("HEAD");
    std::fs::write(&head, "ref: refs/heads/topic\n")?;
    monitor.observe(&modified(&head), false, now);
    let now = now + Duration::from_millis(250);
    let outcome = monitor.service(now, || open(root));
    assert!(outcome.errors.is_empty(), "completing HEAD preserves native coverage");
    assert!(
        outcome.changes.references && outcome.changes.layout,
        "HEAD changes remain visible after a shallow subscription is installed"
    );

    std::fs::remove_dir_all(&added)?;
    monitor.observe(&event(&added, EventKind::Remove, PathKind::Directory), false, now);
    let outcome = monitor.service(now, || open(root));
    assert!(
        outcome.errors.iter().all(|error| error.is_conflict()
            || error
                .downcast_any_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound)),
        "removing a native watch can report lost coverage or its missing root: {:?}",
        outcome.errors
    );
    let outcome = monitor.service(now + monitor.options.retry_interval, || open(root));
    assert!(
        outcome.errors.is_empty(),
        "worktree removal restores coverage after any native loss: {:?}",
        outcome.errors
    );
    assert!(
        !monitor.metadata.registered.contains_key(&added),
        "removed administrative directories release their subscriptions"
    );
    assert_eq!(
        monitor.metadata.registered.get(&worktrees),
        Some(&false),
        "membership remains observable after the last linked worktree disappears"
    );
    Ok(())
}

#[test]
fn metadata_inventory_reports_unreadable_worktree_membership() -> TestResult {
    let fixture = fixture()?;
    let repo = open(fixture.path())?;
    let layout = Layout::from_repository(&repo)?;
    std::fs::write(layout.common_dir.join("worktrees"), "not a directory\n")?;
    assert!(
        inventory::dependency_watches(&layout)
            .values()
            .all(|recursive| !recursive),
        "worktree-only dependencies do not inspect unrelated linked-worktree membership"
    );
    assert!(
        inventory::metadata_watches(&layout).is_err(),
        "an unreadable worktree inventory cannot silently certify incomplete coverage"
    );
    std::fs::write(&layout.index, "invalid index\n")?;
    let mut monitor = monitor();
    monitor.options.metadata = false;
    monitor.options.worktree_coverage = Some(WorktreeCoverage::All);
    let mut statistics = Statistics::default();
    let mut errors = Vec::new();
    monitor.maintain(&repo, Instant::now(), &mut statistics, &mut errors)?;
    assert!(
        errors.is_empty(),
        "all coverage needs neither the index nor metadata inventory"
    );
    assert!(
        monitor.metadata.watcher.is_none()
            && monitor.layout.dependencies.is_empty()
            && monitor.index_fingerprint.is_none(),
        "all coverage retains no ignore/config dependencies or index fingerprint"
    );
    assert_eq!(
        statistics.added, 1,
        "classified all coverage needs only a single recursive root watch"
    );
    Ok(())
}

#[test]
fn missing_dependencies_follow_the_nearest_existing_ancestor() -> TestResult {
    let fixture = fixture()?;
    let repo = open(fixture.path())?;
    let mut layout = Layout::from_repository(&repo)?;
    let root = layout.worktree.clone().expect("fixture has a worktree");
    let dependency = root.join("missing/deeper/config");
    layout.dependencies.push(Dependency {
        path: dependency.clone(),
        kind: DependencyKind::Configuration,
    });
    assert!(
        inventory::metadata_watches(&layout)?.contains_key(&root),
        "missing parents are observed through an existing ancestor"
    );
    std::fs::create_dir_all(root.join("missing/deeper"))?;
    let (changes, refresh) = classify_metadata(&root.join("missing"), &layout, false);
    assert!(
        changes.configuration && refresh,
        "creating an ancestor rediscovers the dependency's parent"
    );
    assert!(
        inventory::metadata_watches(&layout)?.contains_key(dependency.parent().expect("dependency has parent")),
        "registration advances as missing ancestors appear"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn physical_roots_and_config_symlink_replacements_are_both_observed() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let target = root.join("actual-config");
    std::fs::rename(root.join(".git/config"), &target)?;
    std::os::unix::fs::symlink(&target, root.join(".git/config"))?;
    let repo = open(root)?;
    let layout = Layout::from_repository(&repo)?;
    let physical_root = root.canonicalize()?;
    assert_eq!(
        layout.worktree.as_ref(),
        Some(&physical_root),
        "native registrations use the physical root spelling"
    );
    for path in [physical_root.join(".git/config"), physical_root.join("actual-config")] {
        assert!(
            layout.dependencies.iter().any(|dependency| dependency.path == path),
            "{} remains a configuration dependency",
            path.display()
        );
        assert!(
            classify_metadata(&path, &layout, false).0.configuration,
            "both symlink replacement and target edits invalidate configuration"
        );
    }
    let relative = absolute_path(Path::new("missing/config"), &physical_root, false)?;
    assert_eq!(
        relative,
        physical_root.join("missing/config"),
        "relative missing paths are made absolute"
    );
    Ok(())
}

#[test]
fn startup_verifies_registration_once_and_unchanged_sets_are_stable() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let mut monitor = open(root)?.monitor(Options::default())?;
    let now = Instant::now();
    let mut opens = 0;
    let mut opener = || {
        opens += 1;
        open(root)
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
fn head_changes_rediscover_branch_conditioned_dependencies() -> TestResult {
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
    let mut monitor = open()?.monitor(Options::default())?;
    assert!(
        monitor
            .layout
            .dependencies
            .iter()
            .any(|dependency| dependency.path.ends_with("main-config")),
        "initial dependencies reflect the current branch"
    );
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/other\n")?;
    let now = Instant::now();
    monitor.observe(&modified(monitor.layout.git_dir.join("HEAD")), false, now);
    let outcome = monitor.service(now, open);
    assert!(
        outcome.errors.is_empty(),
        "branch dependency rediscovery succeeds: {:?}",
        outcome.errors
    );
    assert!(
        outcome.changes.configuration,
        "switching conditional includes invalidates loaded configuration"
    );
    assert!(
        monitor
            .layout
            .dependencies
            .iter()
            .any(|dependency| dependency.path.ends_with("other-config")),
        "the newly active missing include is observed"
    );
    assert!(
        !monitor
            .layout
            .dependencies
            .iter()
            .any(|dependency| dependency.path.ends_with("main-config")),
        "inactive include paths are removed"
    );
    Ok(())
}

#[test]
fn head_changes_refresh_coverage_with_unchanged_dependency_paths() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    let config = root.join(".git/config");
    let mut contents = std::fs::read_to_string(&config)?;
    contents.push_str(
        "
[include]
    path = case-config
[core]
    ignoreCase = true
[includeIf \"onbranch:other\"]
    path = case-config
",
    );
    std::fs::write(config, contents)?;
    std::fs::write(
        root.join(".git/case-config"),
        "[core]
    ignoreCase = false
",
    )?;
    std::fs::write(root.join(".gitignore"), "*.ignored\n")?;
    std::fs::create_dir(root.join("FILE.IGNORED"))?;
    let open = || {
        let mut options = crate::open::Options::isolated();
        options.permissions.config.includes = true;
        crate::open_opts(root, options)
    };
    let mut monitor = open()?.monitor(Options::default())?;
    let now = Instant::now();
    for _ in 0..2 {
        let out = monitor.service(now, open);
        assert!(out.errors.is_empty(), "initial coverage succeeds: {:?}", out.errors);
    }
    let dependencies = monitor.layout.dependencies.clone();
    assert!(
        !monitor.worktree.registered.contains_key(&root.join("FILE.IGNORED")),
        "the initial effective case setting excludes the uppercase directory"
    );

    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/other\n")?;
    let repo = open()?;
    assert!(
        !repo.filesystem_options()?.ignore_case,
        "the repeated conditional include changes effective configuration without adding a source path"
    );
    monitor.observe(&modified(monitor.layout.git_dir.join("HEAD")), false, now);
    monitor.maintain(&repo, now, &mut Statistics::default(), &mut Vec::new())?;
    assert_eq!(
        monitor.layout.dependencies, dependencies,
        "deduplicated dependency paths cannot detect changed include precedence"
    );
    assert_eq!(
        monitor.worktree.registered.get(
            &monitor
                .layout
                .worktree
                .as_ref()
                .expect("fixture has a worktree")
                .join("FILE.IGNORED")
        ),
        Some(&plumbing::Watcher::supports_native_recursive_watches()),
        "HEAD refreshes backend-appropriate coverage even when configuration dependency paths are unchanged"
    );
    let changes = monitor.take_changes(now + monitor.options.metadata_debounce);
    assert!(
        changes.configuration && changes.worktree == Some(Scope::All),
        "consumers must re-evaluate configuration and worktree state after branch-conditioned settings change"
    );
    Ok(())
}

#[test]
fn all_worktrees_share_metadata_and_settle_without_reopening() -> TestResult {
    // Writable fixture copies emit FSEvents on their source; isolate this synthetic timeline.
    let (fixture, ()) = gix_testtools::scripted_fixture_read_only_with_post("make_notify_worktrees.sh", 3, |_| Ok(()))?;
    let root = fixture.join("main");
    let mut monitor = open(&root)?.monitor(Options {
        all_worktrees: true,
        ..Options::default()
    })?;
    let now = Instant::now();
    let initial = monitor.service(now, || open(&root));
    assert!(
        initial.errors.is_empty(),
        "initial union registration succeeds: {:?}",
        initial.errors
    );
    assert_eq!(
        monitor.other_worktrees.len(),
        2,
        "both sibling checkouts are discovered"
    );
    assert_eq!(
        initial.other_worktrees.len(),
        2,
        "each discovered checkout has one baseline"
    );
    assert!(
        initial
            .other_worktrees
            .iter()
            .all(|change| matches!(change.kind, WorktreeChange::Added)),
        "Added establishes the baseline without a redundant startup Changed event"
    );
    let registrations = monitor.metadata.registered.len()
        + monitor.worktree.registered.len()
        + monitor
            .other_worktrees
            .values()
            .map(|other| other.worktree.registered.len())
            .sum::<usize>();
    assert_eq!(
        initial.statistics.added, registrations,
        "only the owner registers shared metadata"
    );
    assert_eq!(
        monitor.metadata.registered.get(&monitor.layout.common_dir.join("refs")),
        Some(&true),
        "shared refs have one recursive registration"
    );
    for other in monitor.other_worktrees.values() {
        assert!(
            other.metadata.watcher.is_none(),
            "additional checkouts never allocate a metadata watcher"
        );
        assert!(
            other
                .worktree
                .registered
                .keys()
                .all(|path| path.starts_with(other.layout.worktree.as_ref().expect("checkout"))),
            "private index subscriptions belong to the union, not checkout streams"
        );
    }
    let verified = monitor.service(now, || open(&root));
    assert!(
        verified.errors.is_empty(),
        "inventory verification succeeds: {:?}",
        verified.errors
    );
    assert_eq!(
        (verified.statistics.added, verified.statistics.removed),
        (0, 0),
        "verification leaves registrations unchanged"
    );
    assert!(
        monitor.maintenance.is_none()
            && monitor
                .other_worktrees
                .values()
                .all(|other| other.maintenance.is_none()),
        "every inventory settles after verification"
    );
    monitor.service(now, || panic!("idle monitors must not reopen repositories"));
    monitor.coverage_lost(false, now);
    assert!(
        monitor
            .other_worktrees
            .values()
            .all(|other| other.pending_worktree.worktree == Some(Scope::All)),
        "union loss invalidates every checkout baseline"
    );
    let options = Options {
        worktree_coverage: None,
        ..monitor.options().clone()
    };
    drop(monitor);
    monitor = open(&root)?.monitor(options)?;
    assert!(
        monitor.other_worktrees.is_empty(),
        "disabling checkout coverage drops all additional checkout streams"
    );
    let disabled = monitor.service(Instant::now(), || open(&root));
    assert!(
        disabled.errors.is_empty() && disabled.other_worktrees.is_empty(),
        "metadata-only service does not rediscover checkout streams"
    );
    let options = Options {
        worktree_coverage: Some(WorktreeCoverage::IgnoreAware),
        ..monitor.options().clone()
    };
    drop(monitor);
    monitor = open(&root)?.monitor(options)?;
    let enabled = monitor.service(Instant::now(), || open(&root));
    assert!(
        enabled.errors.is_empty()
            && enabled.other_worktrees.len() == 2
            && enabled
                .other_worktrees
                .iter()
                .all(|change| matches!(change.kind, WorktreeChange::Added)),
        "enabling coverage establishes fresh Added baselines"
    );
    Ok(())
}

#[test]
fn all_worktrees_retry_backoff_covers_overdue_child_maintenance() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let root = fixture.join("main");
    let mut monitor = open(&root)?.monitor(Options {
        all_worktrees: true,
        ..Options::default()
    })?;
    let now = Instant::now();
    for _ in 0..2 {
        let out = monitor.service(now, || open(&root));
        assert!(out.errors.is_empty(), "startup succeeds: {:?}", out.errors);
    }
    monitor
        .other_worktrees
        .values_mut()
        .next()
        .expect("linked checkout")
        .maintenance = Some(now);
    let out = monitor.service(now, || {
        Err(message("temporarily unavailable selected repository").raise())
    });
    assert_eq!(out.errors.len(), 1, "opening failure is reported once");
    assert!(
        monitor.next_timeout(now).is_some_and(|timeout| !timeout.is_zero()),
        "overdue children cannot bypass the coordinator's retry interval"
    );
    monitor.service(now, || panic!("failed maintenance must wait for retry backoff"));
    Ok(())
}

#[test]
fn all_worktrees_retry_backoff_covers_overdue_undiscovered_worktrees() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let root = fixture.join("main");
    let mut monitor = open(&root)?.monitor(Options {
        all_worktrees: true,
        ..Options::default()
    })?;
    let mut now = Instant::now();
    for _ in 0..2 {
        let out = monitor.service(now, || open(&root));
        assert!(out.errors.is_empty(), "startup succeeds: {:?}", out.errors);
    }
    let git_dir = monitor.layout.common_dir.join("worktrees/linked-a");
    monitor
        .other_worktrees
        .remove(&git_dir)
        .expect("discovered linked checkout");
    monitor.undiscovered_worktrees.insert(git_dir.clone(), now);
    monitor.maintenance = Some(now);
    for _ in 0..2 {
        let out = monitor.service(now, || {
            Err(message("temporarily unavailable selected repository").raise())
        });
        assert_eq!(out.errors.len(), 1, "the coordinator reports its opening failure once");
        let retry = now + monitor.options.retry_interval;
        assert_eq!(
            monitor.undiscovered_worktrees.get(&git_dir),
            Some(&retry),
            "coordinator failure postpones overdue undiscovered entries just like known children"
        );
        assert_eq!(
            monitor.maintenance,
            Some(retry),
            "the old map deadline cannot override retry backoff"
        );
        assert!(
            monitor.next_timeout(now).is_some_and(|timeout| !timeout.is_zero()),
            "undiscovered entries cannot cause a busy loop after coordinator failure"
        );
        let idle = monitor.service(now, || panic!("failed discovery must wait for retry backoff"));
        assert!(
            idle.errors.is_empty(),
            "service before the retry deadline does not repeat the error"
        );
        now = retry;
    }
    Ok(())
}

#[test]
fn bare_main_without_checkout_recovers_undiscovered_registration() -> TestResult {
    // Writable fixture copies emit FSEvents on their source; isolate this synthetic timeline.
    let (fixture, ()) = gix_testtools::scripted_fixture_read_only_with_post("make_notify_worktrees.sh", 3, |_| Ok(()))?;
    let root = fixture.join("bare-a");
    let mut monitor = open(&root)?.monitor(Options {
        all_worktrees: true,
        ..Options::default()
    })?;
    let now = Instant::now();
    let main_git_dir = monitor.layout.common_dir.clone();
    for _ in 0..2 {
        let out = monitor.service(now, || open(&root));
        assert!(out.errors.is_empty(), "startup succeeds: {:?}", out.errors);
        assert!(
            out.other_worktrees.iter().all(|change| change.git_dir != main_git_dir),
            "a bare main has no checkout baseline or lifecycle event"
        );
    }
    monitor.undiscovered_worktrees.insert(main_git_dir.clone(), now);
    monitor.maintenance = Some(now);
    let recovered = monitor.service(now, || open(&root));
    assert!(
        recovered.errors.is_empty(),
        "rediscovery of the bare main succeeds: {:?}",
        recovered.errors
    );
    assert!(
        monitor.undiscovered_worktrees.is_empty(),
        "successfully discovering that the main is bare clears its previous failure deadline"
    );
    assert!(
        recovered.changes.is_empty() && recovered.other_worktrees.is_empty(),
        "an absent bare-main checkout neither publishes Added nor invalidates healthy checkouts"
    );
    assert!(
        !monitor.other_worktrees.contains_key(&main_git_dir),
        "a bare main never becomes a checkout monitor"
    );
    assert!(
        monitor.maintenance.is_none()
            && monitor
                .other_worktrees
                .values()
                .all(|other| other.maintenance.is_none()),
        "all inventories settle after rediscovering the bare main"
    );
    let idle = monitor.service(now + monitor.options.retry_interval, || {
        panic!("a recovered bare main must not schedule endless rediscovery")
    });
    assert!(
        idle.errors.is_empty() && idle.changes.is_empty() && idle.other_worktrees.is_empty(),
        "the old retry deadline remains cleared without generating a bare-main baseline"
    );
    Ok(())
}

#[test]
fn disabled_all_worktree_coverage_routes_metadata_as_selected_only() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let root = fixture.join("linked-a");
    for coverage in [None, Some(WorktreeCoverage::All), Some(WorktreeCoverage::IgnoreAware)] {
        let mut monitor = open(&root)?.monitor(Options {
            all_worktrees: true,
            worktree_coverage: coverage,
            metadata_debounce: Duration::ZERO,
            ..Options::default()
        })?;
        let mut now = Instant::now();
        for _ in 0..2 {
            let out = monitor.service(now, || open(&root));
            assert!(out.errors.is_empty(), "startup succeeds: {:?}", out.errors);
        }
        if coverage.is_some() {
            let options = Options {
                worktree_coverage: None,
                ..monitor.options().clone()
            };
            drop(monitor);
            monitor = open(&root)?.monitor(options)?;
            now = Instant::now();
            for _ in 0..2 {
                let out = monitor.service(now, || open(&root));
                assert!(
                    out.errors.is_empty(),
                    "disabling checkout coverage succeeds: {:?}",
                    out.errors
                );
            }
        }
        monitor.observe(&modified(monitor.layout.common_dir.join("MERGE_MSG")), false, now);
        let out = monitor.service(now, || panic!("an operation-state file does not change watch coverage"));
        assert_eq!(
            out.changes,
            Changes {
                operations: true,
                ..Changes::default()
            },
            "{coverage:?}: with checkout coverage disabled, main-private metadata belongs to the selected-only view"
        );
        assert!(
            out.other_worktrees.is_empty() && monitor.other_worktrees.is_empty(),
            "disabled all-worktree coverage has no child to receive main-private operation changes"
        );
        assert!(
            !monitor.metadata_is_shared(),
            "all_worktrees alone does not select shared metadata routing"
        );
    }
    Ok(())
}

#[test]
fn worktree_replace_failure_invalidates_coverage_despite_latched_maintenance_failure() -> TestResult {
    use gix_parallel::{Mutable, OwnShared, lock};

    for coverage in [WorktreeCoverage::All, WorktreeCoverage::IgnoreAware] {
        let fixture = gix_testtools::scripted_fixture_writable("make_notify_worktrees.sh")?;
        let root = fixture.path().join("linked-a");
        let mut monitor = open(&root)?.monitor(Options {
            metadata: false,
            worktree_coverage: Some(coverage),
            ..Options::default()
        })?;
        let now = Instant::now();
        for _ in 0..2 {
            let out = monitor.service(now, || open(&root));
            assert!(out.errors.is_empty(), "startup succeeds: {:?}", out.errors);
        }
        monitor.maintenance_failed(now);
        let failed = monitor.service(now, || panic!("the first failure schedules a future retry"));
        assert_eq!(
            failed.changes.worktree,
            Some(Scope::All),
            "the initial failure publishes its baseline invalidation"
        );
        assert!(
            monitor.maintenance_failed,
            "the failure latch remains set before native replacement"
        );

        // An empty watcher has no backend thread. Arm its waker only after set_waker's initial
        // callback: replace wakes synchronously after losing coverage, before native registration.
        // Removing the checkout there deterministically fails registration, not inventory discovery.
        let remove_on_replace = OwnShared::new(Mutable::new(None::<PathBuf>));
        let mut watcher = plumbing::Watcher::new(plumbing::Options::default())?;
        watcher.set_waker({
            let remove_on_replace = remove_on_replace.clone();
            move || {
                if let Some(root) = lock(&remove_on_replace).take() {
                    std::fs::remove_dir_all(root).expect("the disposable checkout can be removed before registration");
                }
            }
        });
        *lock(&remove_on_replace) = Some(root.clone());
        monitor.worktree = Domain {
            watcher: Some(watcher),
            ..Domain::default()
        };
        monitor.inventory_dirty = true;
        let retry = now + monitor.options.retry_interval;
        let out = monitor.service(retry, || open(&root));
        assert!(!root.try_exists()?, "the waker removed the checkout during replacement");
        assert!(
            lock(&remove_on_replace).is_none(),
            "the one-shot registration failure was injected"
        );
        assert_eq!(
            out.errors.len(),
            1,
            "{coverage:?}: native registration reports the missing checkout: {:?}",
            out.errors
        );
        assert!(
            monitor.worktree.rebuild,
            "failed native replacement requires rebuilding coverage"
        );
        assert_eq!(
            out.changes,
            Changes {
                worktree: Some(Scope::All),
                ..Changes::default()
            },
            "{coverage:?}: new native coverage loss cannot be suppressed by an already-set maintenance failure latch"
        );
        assert_eq!(
            monitor.maintenance,
            Some(retry + monitor.options.retry_interval),
            "native registration failure still backs off"
        );
    }
    Ok(())
}

#[test]
fn all_worktrees_require_a_new_monitor_for_a_different_primary() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_worktrees.sh")?;
    let root = fixture.join("linked-a");
    let mut monitor = open(&root)?.monitor(Options {
        all_worktrees: true,
        ..Options::default()
    })?;
    let now = Instant::now();
    for _ in 0..2 {
        let out = monitor.service(now, || open(&root));
        assert!(out.errors.is_empty(), "startup succeeds: {:?}", out.errors);
    }
    let primary = monitor.layout.git_dir.clone();
    monitor.rescan();
    let out = monitor.service(Instant::now(), || open(&fixture.join("main")));
    assert!(
        out.errors.iter().any(gix_error::Error::is_validation),
        "an implicit primary switch must not duplicate the main checkout"
    );
    assert_eq!(monitor.layout.git_dir, primary, "the existing primary remains selected");
    let options = monitor.options().clone();
    drop(monitor);
    monitor = open(&fixture.join("main"))?.monitor(options)?;
    let out = monitor.service(Instant::now(), || open(&fixture.join("main")));
    assert!(
        out.errors.is_empty() && out.other_worktrees.len() == 2,
        "dropping the old monitor and constructing a new one safely selects another primary"
    );
    Ok(())
}

#[test]
fn native_errors_count_discarded_registrations() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_notify_repo.sh")?;
    let root = fixture.join("unignored");
    let mut monitor = open(&root)?.monitor(Options {
        metadata: false,
        worktree_coverage: Some(WorktreeCoverage::All),
        ..Options::default()
    })?;
    let now = Instant::now();
    let initial = monitor.service(now, || panic!("all-worktree-only coverage does not reopen"));
    assert!(initial.errors.is_empty(), "startup succeeds: {:?}", initial.errors);
    let registered = monitor.worktree.registered.len();
    assert!(registered > 0, "the checkout has native coverage");
    // A failing callback queues a native error without filesystem mutation or timing races.
    monitor
        .worktree
        .watcher
        .as_mut()
        .expect("startup installed the watcher")
        .set_waker(|| panic!("deliberate wake callback failure"));
    let mut out = Outcome::default();
    monitor.poll(now, &mut out);
    assert!(!out.errors.is_empty(), "the callback failure is reported");
    assert!(monitor.worktree.registered.is_empty(), "failed coverage is discarded");
    assert_eq!(
        out.statistics.removed, registered,
        "discarded registrations count as removed"
    );
    Ok(())
}

#[test]
fn incomplete_worktrees_do_not_poison_the_shared_metadata_inventory() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_notify_worktrees.sh")?;
    let root = fixture.path().join("main");
    let mut monitor = open(&root)?.monitor(Options {
        all_worktrees: true,
        worktree_coverage: Some(WorktreeCoverage::All),
        ..Options::default()
    })?;
    let now = Instant::now();
    for _ in 0..2 {
        let out = monitor.service(now, || open(&root));
        assert!(out.errors.is_empty(), "startup succeeds: {:?}", out.errors);
    }
    let key = monitor.other_worktrees.keys().next().expect("linked checkout").clone();
    let checkout = monitor.other_worktrees[&key]
        .layout
        .worktree
        .clone()
        .expect("checkout path");
    std::fs::remove_dir_all(&checkout)?;
    std::fs::remove_file(key.join("gitdir"))?;
    monitor
        .other_worktrees
        .get_mut(&key)
        .expect("retained child")
        .maintenance = Some(now);
    monitor.service(now, || open(&root));
    // Native root loss can defer maintenance until the retry deadline.
    let out = monitor.service(now + monitor.options.retry_interval, || open(&root));
    assert!(
        !out.errors.is_empty(),
        "incomplete registration reports degraded coverage"
    );
    assert!(
        !monitor.metadata.rebuild,
        "a degraded child must not prevent rebuilding the metadata union"
    );
    assert!(
        monitor.metadata.registered.keys().all(|path| path.is_dir()),
        "the union does not retain registrations for missing checkout roots"
    );
    assert!(
        monitor.other_worktrees.contains_key(&key)
            && out
                .other_worktrees
                .iter()
                .all(|change| !matches!(change.kind, WorktreeChange::Removed)),
        "inaccessible checkouts are retained, not declared removed"
    );
    Ok(())
}

#[test]
fn stale_worktree_registration_retries_do_not_invalidate_healthy_checkouts() -> TestResult {
    let (fixture, ()) =
        gix_testtools::scripted_fixture_read_only_with_post("make_notify_worktrees.sh", 1, |fixture| {
            if let gix_testtools::FixtureState::Uninitialized(path) = fixture {
                // Keep HEAD, commondir, index and gitdir: Git still lists this complete registration,
                // but opening its missing checkout must fail during every coverage retry.
                std::fs::remove_dir_all(path.join("linked-a"))?;
            }
            Ok(())
        })?;
    let root = fixture.join("main");
    for coverage in [WorktreeCoverage::All, WorktreeCoverage::IgnoreAware] {
        let mut monitor = open(&root)?.monitor(Options {
            all_worktrees: true,
            worktree_coverage: Some(coverage),
            metadata_debounce: Duration::ZERO,
            ..Options::default()
        })?;
        let stale_git_dir = monitor.layout.common_dir.join("worktrees/linked-a");
        let healthy_git_dir = monitor.layout.common_dir.join("worktrees/linked-b");
        let mut now = Instant::now();
        let initial = monitor.service(now, || open(&root));
        assert_eq!(
            initial.errors.len(),
            1,
            "{coverage:?}: the complete-but-stale registration reports its missing checkout: {:?}",
            initial.errors
        );
        assert_eq!(initial.other_worktrees.len(), 2, "both registrations are retained");
        assert!(
            initial
                .other_worktrees
                .iter()
                .all(|change| matches!(change.kind, WorktreeChange::Added)),
            "startup establishes Added baselines, not Changed events: {:?}",
            initial.other_worktrees
        );
        assert!(
            monitor.other_worktrees.contains_key(&stale_git_dir),
            "failed checkout coverage must not retire a complete registration"
        );
        assert!(
            !monitor.metadata.rebuild && monitor.metadata.registered.keys().all(|path| path.is_dir()),
            "a missing checkout does not poison initial union registration"
        );
        let union = monitor.metadata.registered.clone();
        let retry_interval = monitor.options.retry_interval;
        for retry in 0..3 {
            now += retry_interval;
            let mut expected_primary = Changes::default();
            let mut expected_other = Vec::new();
            if retry == 1 {
                // Inject ordinary observations instead of mutating this read-only fixture or
                // forcing a rescan, which would legitimately invalidate the entire shared stream.
                let path = monitor
                    .layout
                    .worktree
                    .as_ref()
                    .expect("primary checkout")
                    .join("tracked/file");
                monitor.observe(&modified(path), true, now);
                monitor.observe(&modified(monitor.layout.common_dir.join("refs/heads/main")), false, now);
                expected_primary.references = true;
                expected_primary.worktree = Some(Scope::Paths(vec![path_change(
                    "tracked/file",
                    EventKind::Modify,
                    PathKind::File,
                )]));
                let other = monitor
                    .other_worktrees
                    .get_mut(&healthy_git_dir)
                    .expect("healthy child");
                let path = other
                    .layout
                    .worktree
                    .as_ref()
                    .expect("healthy checkout")
                    .join("tracked/file");
                other.observe(&modified(path), true, now);
                expected_other.push(healthy_git_dir.clone());
            }
            let out = monitor.service(now, || open(&root));
            assert_eq!(
                out.errors.len(),
                1,
                "{coverage:?}: retry {retry} still reports the child failure: {:?}",
                out.errors
            );
            assert_eq!(
                out.changes, expected_primary,
                "{coverage:?}: retry {retry} neither invents primary changes nor discards healthy observations"
            );
            assert_eq!(
                out.other_worktrees
                    .iter()
                    .map(|change| change.git_dir.clone())
                    .collect::<Vec<_>>(),
                expected_other,
                "{coverage:?}: unchanged child failures do not republish Changed or lifecycle events"
            );
            if let Some(change) = out.other_worktrees.first() {
                assert!(
                    matches!(&change.kind, WorktreeChange::Changed(changes) if *changes == Changes {
                        worktree: Some(Scope::Paths(vec![path_change("tracked/file", EventKind::Modify, PathKind::File)])),
                        ..Changes::default()
                    }),
                    "a healthy sibling's actual path observation survives another child's failed retry: {change:?}"
                );
            }
            assert_eq!(
                monitor.metadata.registered, union,
                "failed retries keep the settled union unchanged"
            );
            assert_eq!(
                (out.statistics.added, out.statistics.removed),
                (0, 0),
                "retries do not churn native registrations"
            );
            assert_eq!(
                monitor.other_worktrees[&stale_git_dir].maintenance,
                Some(now + retry_interval),
                "the failed child waits for its next retry deadline"
            );
            assert!(
                monitor.other_worktrees[&healthy_git_dir].maintenance.is_none(),
                "the healthy sibling settles despite failed retries elsewhere"
            );
            let idle = monitor.service(now, || panic!("a failed child must respect retry backoff"));
            assert!(
                idle.errors.is_empty() && idle.changes.is_empty() && idle.other_worktrees.is_empty(),
                "service before the next retry has neither changes nor repeated diagnostics"
            );
        }
    }
    Ok(())
}

#[test]
fn child_maintenance_failure_and_same_layout_recovery_invalidate_only_that_child() -> TestResult {
    // Copying a read-only fixture for writable tests emits FSEvents on the source files too.
    // A dedicated cache identity keeps native copy notifications out of this synthetic timeline.
    let (fixture, ()) = gix_testtools::scripted_fixture_read_only_with_post("make_notify_worktrees.sh", 2, |_| Ok(()))?;
    let root = fixture.join("main");
    for coverage in [WorktreeCoverage::All, WorktreeCoverage::IgnoreAware] {
        let mut monitor = open(&root)?.monitor(Options {
            all_worktrees: true,
            worktree_coverage: Some(coverage),
            ..Options::default()
        })?;
        let mut now = Instant::now();
        for _ in 0..2 {
            let out = monitor.service(now, || open(&root));
            assert!(out.errors.is_empty(), "startup succeeds: {:?}", out.errors);
        }
        let git_dir = monitor.layout.common_dir.join("worktrees/linked-a");
        let layout = monitor.other_worktrees[&git_dir].layout.clone();
        let union = monitor.metadata.registered.clone();
        let watches = monitor.other_worktrees[&git_dir].worktree.registered.clone();
        let mut full = Changes::metadata_all();
        full.merge(Changes::worktree_all());
        for _ in 0..2 {
            // Model a transient maintenance error without changing any watched path. Filesystem
            // deletion/restoration would change the union and mask a missing recovery invalidation.
            monitor
                .other_worktrees
                .get_mut(&git_dir)
                .expect("retained child")
                .maintenance_failed(now);
            let failed = monitor.service(now, || panic!("the injected failure has a future retry deadline"));
            assert!(
                failed.errors.is_empty(),
                "the state transition does not synthesize diagnostics"
            );
            assert_only_child_change(&failed, &layout, &full);

            monitor
                .other_worktrees
                .get_mut(&git_dir)
                .expect("retained child")
                .maintenance_failed(now);
            let repeated = monitor.service(now, || panic!("repeated failures still respect backoff"));
            assert!(
                repeated.changes.is_empty() && repeated.other_worktrees.is_empty(),
                "{coverage:?}: the first failure invalidates once, not on every retry"
            );

            now += monitor.options.retry_interval;
            let mut opens = 0;
            let recovered = monitor.service(now, || {
                opens += 1;
                open(&root)
            });
            assert_eq!(opens, 1, "the due child recovery uses one freshly opened repository");
            assert!(recovered.errors.is_empty(), "recovery succeeds: {:?}", recovered.errors);
            assert_only_child_change(&recovered, &layout, &full);
            assert_eq!(
                monitor.other_worktrees[&git_dir].layout, layout,
                "recovery retains the same layout"
            );
            assert_eq!(
                monitor.metadata.registered, union,
                "recovery does not alter the shared union"
            );
            assert_eq!(
                monitor.other_worktrees[&git_dir].worktree.registered, watches,
                "recovery retains the same child watches"
            );
            assert_eq!(
                (recovered.statistics.added, recovered.statistics.removed),
                (0, 0),
                "recovery invalidates even without watch replacement"
            );
            let idle = monitor.service(now, || panic!("unchanged recovery needs no further inventory work"));
            assert!(
                idle.changes.is_empty() && idle.other_worktrees.is_empty(),
                "recovery publishes one full invalidation and re-arms the failure latch"
            );
        }
    }
    Ok(())
}

fn assert_only_child_change(out: &Outcome, layout: &Layout, expected: &Changes) {
    assert!(
        out.changes.is_empty(),
        "a child-only failure/recovery must not invalidate the primary: {:?}",
        out.changes
    );
    assert_eq!(
        out.other_worktrees.len(),
        1,
        "only the affected child publishes changes: {:?}",
        out.other_worktrees
    );
    let change = &out.other_worktrees[0];
    assert_eq!(
        change.git_dir, layout.git_dir,
        "the private Git directory owns the failure/recovery"
    );
    assert_eq!(
        Some(&change.worktree),
        layout.worktree.as_ref(),
        "the unchanged checkout keeps its identity"
    );
    assert!(
        matches!(&change.kind, WorktreeChange::Changed(changes) if changes == expected),
        "an existing child's failure/recovery is a full Changed invalidation, not a new baseline: {change:?}"
    );
}
