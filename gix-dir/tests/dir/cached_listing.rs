use std::sync::atomic::AtomicBool;

use bstr::BString;
use gix_dir::{
    EntryRef,
    entry::{self, Kind::*, Status::Untracked},
    walk,
    walk::{EmissionMode::*, ForDeletionMode},
};
use gix_testtools::TestResult;

use crate::walk_utils::{Options, options, options_emit_all, try_collect_filtered_opts_collect};

fn listing(includes_ignored: bool) -> walk::CachedListing {
    let mut listing = walk::CachedListing {
        includes_ignored,
        directories: [
            (
                "",
                vec![
                    ("top-untracked", File),
                    (".gitignore", File),
                    ("mixed", Directory),
                    ("uncached", Directory),
                    ("empty", Directory),
                ],
            ),
            ("mixed", vec![("tracked", File), ("new", File)]),
            ("uncached", vec![("file", File)]),
            ("empty", vec![]),
        ]
        .into_iter()
        .map(|(dir, entries)| {
            (
                dir.into(),
                entries
                    .into_iter()
                    .map(|(name, kind)| (name.into(), kind))
                    .collect::<Vec<_>>()
                    .into(),
            )
        })
        .collect(),
    };
    if includes_ignored {
        listing
            .directories
            .get_mut(&BString::default())
            .expect("the root has a listing")
            .entries
            .extend([(".git".into(), Directory), ("generated".into(), Directory)]);
        listing
            .directories
            .get_mut(&BString::from("mixed"))
            .expect("mixed has a listing")
            .entries
            .push(("secret.ignored".into(), File));
        listing
            .directories
            .insert("generated".into(), vec![("data".into(), File)].into());
    }
    listing
}

#[test]
fn complete_listings_preserve_output_and_avoid_directory_reads() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("cached-listing.sh")?;
    assert_eq!(
        std::fs::read_to_string(root.join(".git/untracked-files"))?,
        "mixed/new
top-untracked
uncached/file
",
        "Git excludes ignored files and preserves the tracked child of mixed"
    );
    let listing = listing(true);
    for (options, pattern) in [
        (options(), None),
        (
            walk::Options {
                emit_untracked: CollapseDirectory,
                ..options()
            },
            None,
        ),
        (options_emit_all(), None),
        (
            walk::Options {
                for_deletion: Some(ForDeletionMode::IgnoredDirectoriesCanHideNestedRepositories),
                ..options_emit_all()
            },
            None,
        ),
        (options_emit_all(), Some("mixed/*")),
    ] {
        let ((uncached, _), expected) = try_collect_filtered_opts_collect(
            &root,
            None,
            |keep, ctx| walk(&root, ctx, options, keep),
            pattern,
            Options::default(),
        )?;
        let ((cached, _), actual) = try_collect_filtered_opts_collect(
            &root,
            None,
            |keep, ctx| walk(&root, ctx, options, keep),
            pattern,
            Options {
                cached_listing: Some(&listing),
                ..Default::default()
            },
        )?;
        assert_eq!(
            actual, expected,
            "cached entries retain all classifications and collapse behavior"
        );
        assert_eq!(
            cached.read_dir_calls, 0,
            "complete listings replace every directory read"
        );
        assert_eq!(cached.cached_dir_listings, uncached.read_dir_calls);
        assert!(
            actual.iter().any(|(entry, _)| entry.rela_path == "mixed/new"),
            "the tracked sibling prevents collapsing mixed into an untracked directory"
        );
    }
    Ok(())
}

#[test]
fn partial_listings_only_apply_to_options_that_do_not_need_omitted_entries() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("cached-listing.sh")?;
    let mut listing = listing(false);
    let mixed = listing
        .directories
        .get_mut(&BString::from("mixed"))
        .expect("fixture directory");
    mixed.entries.retain(|(name, _)| name != "tracked");
    mixed.omitted_tracked = true;
    let collapsed = walk::Options {
        emit_untracked: CollapseDirectory,
        ..options()
    };
    for (options, pattern, use_cache) in [
        (collapsed, None, true),
        (
            walk::Options {
                recurse_repositories: true,
                ..collapsed
            },
            None,
            true,
        ),
        (
            walk::Options {
                emit_tracked: true,
                ..collapsed
            },
            None,
            true,
        ),
        (
            walk::Options {
                emit_ignored: Some(Matching),
                ..collapsed
            },
            None,
            false,
        ),
        (
            walk::Options {
                emit_pruned: true,
                ..collapsed
            },
            None,
            false,
        ),
        (
            walk::Options {
                emit_empty_directories: true,
                ..collapsed
            },
            None,
            false,
        ),
        (
            walk::Options {
                emit_collapsed: Some(walk::CollapsedEntriesEmissionMode::All),
                ..collapsed
            },
            None,
            false,
        ),
        (
            walk::Options {
                for_deletion: Some(Default::default()),
                ..collapsed
            },
            None,
            false,
        ),
        (collapsed, Some("mixed/*"), false),
    ] {
        let ((uncached, _), expected) = try_collect_filtered_opts_collect(
            &root,
            None,
            |keep, ctx| walk(&root, ctx, options, keep),
            pattern,
            Options::default(),
        )?;
        let ((cached, _), actual) = try_collect_filtered_opts_collect(
            &root,
            None,
            |keep, ctx| walk(&root, ctx, options, keep),
            pattern,
            Options {
                cached_listing: Some(&listing),
                ..Default::default()
            },
        )?;
        assert_eq!(
            actual, expected,
            "omitting ignored entries cannot change observable output"
        );
        if use_cache {
            assert_eq!(
                cached.read_dir_calls,
                1 + u32::from(options.emit_tracked || options.recurse_repositories),
                "the empty directory and any listing missing requested tracked entries need disk reads"
            );
            assert!(cached.cached_dir_listings > 0);
            assert!(actual.iter().any(|(entry, _)| entry.rela_path == "mixed/new"));
        } else {
            assert_eq!(
                cached.cached_dir_listings, 0,
                "these options require the complete disk listing"
            );
            assert_eq!(cached.read_dir_calls, uncached.read_dir_calls);
        }
    }
    Ok(())
}

#[test]
fn cached_and_uncached_directories_can_be_mixed() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("cached-listing.sh")?;
    let mut listing = listing(true);
    listing.directories.remove(&BString::from("uncached"));
    let ((out, _), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| walk(&root, ctx, options(), keep),
        None::<&str>,
        Options {
            cached_listing: Some(&listing),
            ..Default::default()
        },
    )?;
    assert_eq!(
        out.read_dir_calls, 1,
        "only the directory absent from the cache is read"
    );
    assert_eq!(
        out.cached_dir_listings, 3,
        "root, mixed, and empty use their cached listings"
    );
    assert!(
        entries
            .iter()
            .any(|(entry, _)| entry.rela_path == "uncached/file" && entry.status == Untracked),
        "uncached children still contribute their files"
    );
    Ok(())
}

#[test]
fn partial_listings_are_bypassed_when_ignores_are_disabled() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("cached-listing.sh")?;
    let listing = listing(false);
    let ((out, _), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, mut ctx| {
            ctx.excludes = None;
            walk(&root, ctx, options(), keep)
        },
        None::<&str>,
        Options {
            cached_listing: Some(&listing),
            ..Default::default()
        },
    )?;
    assert_eq!(
        out.cached_dir_listings, 0,
        "without ignores, omitted names become observable"
    );
    assert!(
        entries
            .iter()
            .any(|(entry, _)| entry.rela_path == "mixed/secret.ignored")
    );
    assert!(entries.iter().any(|(entry, _)| entry.rela_path == "generated/data"));
    Ok(())
}

#[test]
fn cached_names_cannot_escape_their_directory() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("cached-listing.sh")?;
    for name in [
        "",
        ".",
        "..",
        "../outside",
        "/absolute",
        "nested/file",
        "trailing/",
        "nul\0",
    ] {
        let listing = walk::CachedListing {
            includes_ignored: true,
            directories: [(BString::default(), vec![(name.into(), File)].into())].into(),
        };
        let err = try_collect_filtered_opts_collect(
            &root,
            None,
            |keep, ctx| walk(&root, ctx, options(), keep),
            None::<&str>,
            Options {
                cached_listing: Some(&listing),
                ..Default::default()
            },
        )
        .expect_err("cached entries must have the same single-component names as directory entries");
        assert!(err.is_validation(), "the malformed name {name:?} is rejected");
    }
    Ok(())
}

#[test]
fn cached_walks_obey_interrupts_and_stop_after_the_delegate_breaks() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("cached-listing.sh")?;
    let listing = listing(true);
    let should_interrupt = AtomicBool::new(true);
    let err = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| walk(&root, ctx, options(), keep),
        None::<&str>,
        Options {
            should_interrupt: Some(&should_interrupt),
            cached_listing: Some(&listing),
            ..Default::default()
        },
    )
    .expect_err("the cache cannot bypass cancellation");
    assert!(err.is_cancelled());

    struct Stop;
    impl walk::Delegate for Stop {
        fn emit(&mut self, entry: EntryRef<'_>, _: Option<entry::Status>) -> walk::Action {
            assert_eq!(
                entry.rela_path.as_ref(),
                "top-untracked",
                "the first cached entry is emitted first"
            );
            std::ops::ControlFlow::Break(())
        }
    }
    let ((out, _), _) = try_collect_filtered_opts_collect(
        &root,
        None,
        |_, ctx| walk(&root, ctx, options(), &mut Stop),
        None::<&str>,
        Options {
            cached_listing: Some(&listing),
            ..Default::default()
        },
    )?;
    assert_eq!(
        out.cached_dir_listings, 1,
        "breaking in the root never visits cached children"
    );
    assert_eq!(out.read_dir_calls, 0, "breaking does not start any disk traversal");
    assert_eq!(out.returned_entries, 1);
    Ok(())
}

#[test]
fn empty_partial_listings_do_not_make_submodule_roots_appear_empty() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("cached-listing-empty-submodule.sh")?.join("submodule");
    let listing = walk::CachedListing {
        directories: [(BString::default(), Vec::new().into())].into(),
        includes_ignored: false,
    };
    for cached_listing in [None, Some(&listing)] {
        let ((out, _), entries) = try_collect_filtered_opts_collect(
            &root,
            None,
            |keep, ctx| walk(&root, ctx, options(), keep),
            None::<&str>,
            Options {
                git_dir: Some("../gitdir"),
                cached_listing,
                ..Default::default()
            },
        )?;
        assert!(entries.is_empty(), "the only disk entry is the pruned .git file");
        assert_eq!(
            out.read_dir_calls, 1,
            "an omitted .git file still makes the root nonempty"
        );
        assert_eq!(out.cached_dir_listings, 0, "empty partial listings are read from disk");
    }
    Ok(())
}

#[test]
fn partial_listings_preserve_collapses_with_empty_and_ignored_children() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("cached-listing-partial-collapse.sh")?;
    let listing = walk::CachedListing {
        includes_ignored: false,
        directories: [
            (
                "",
                vec![
                    (".gitignore", File),
                    ("parent", Directory),
                    ("with-pruned", Directory),
                    ("nested", Directory),
                    ("only-empty", Directory),
                    ("leaf", Directory),
                ],
            ),
            ("parent", vec![("empty", Directory)]),
            ("with-pruned", vec![(".git", File), ("empty", Directory)]),
            ("nested", vec![("parent", Directory)]),
            ("nested/parent", vec![("empty", Directory)]),
            ("only-empty", vec![("empty", Directory)]),
            (
                "leaf",
                vec![("tracked", File), ("untracked", File), ("empty", Directory)],
            ),
        ]
        .into_iter()
        .map(|(path, entries)| {
            (
                path.into(),
                entries
                    .into_iter()
                    .map(|(name, kind)| (name.into(), kind))
                    .collect::<Vec<_>>()
                    .into(),
            )
        })
        .collect(),
    };
    for emit_untracked in [Matching, CollapseDirectory] {
        let options = walk::Options {
            emit_untracked,
            ..options()
        };
        let ((uncached, _), expected) = try_collect_filtered_opts_collect(
            &root,
            None,
            |keep, ctx| walk(&root, ctx, options, keep),
            None::<&str>,
            Options::default(),
        )?;
        let ((cached, _), actual) = try_collect_filtered_opts_collect(
            &root,
            None,
            |keep, ctx| walk(&root, ctx, options, keep),
            None::<&str>,
            Options {
                cached_listing: Some(&listing),
                ..Default::default()
            },
        )?;
        assert_eq!(
            actual, expected,
            "omitted ignored files cannot make a parent look empty"
        );
        let ((with_tracked, _), mut visible) = try_collect_filtered_opts_collect(
            &root,
            None,
            |keep, ctx| {
                walk(
                    &root,
                    ctx,
                    walk::Options {
                        emit_tracked: true,
                        ..options
                    },
                    keep,
                )
            },
            None::<&str>,
            Options {
                cached_listing: Some(&listing),
                ..Default::default()
            },
        )?;
        visible.retain(|(entry, _)| entry.status != entry::Status::Tracked);
        assert_eq!(visible, actual, "hidden tracked paths still prevent ancestor collapse");
        assert_eq!(
            with_tracked.seen_entries, cached.seen_entries,
            "hidden tracked paths count as seen without being retained for output"
        );
        assert!(
            cached.cached_dir_listings >= 2,
            "a tracked file keeps its parent listing reusable even beside a subdirectory: {cached:?}"
        );
        assert!(cached.read_dir_calls < uncached.read_dir_calls);
        if emit_untracked == CollapseDirectory {
            assert!(actual.iter().any(|(entry, _)| entry.rela_path == "parent"));
            assert!(actual.iter().any(|(entry, _)| entry.rela_path == "with-pruned"));
            assert!(actual.iter().any(|(entry, _)| entry.rela_path == "nested"));
            assert!(!actual.iter().any(|(entry, _)| entry.rela_path == "only-empty"));
        }
    }
    Ok(())
}

#[test]
fn custom_delegates_keep_directory_callbacks_without_opt_in() -> TestResult {
    #[derive(Default)]
    struct RecordDirectories {
        paths: Vec<BString>,
        omit_ignored: bool,
    }
    impl walk::Delegate for RecordDirectories {
        fn can_omit_ignored_directories(&self) -> bool {
            self.omit_ignored
        }

        fn emit(&mut self, _: EntryRef<'_>, _: Option<entry::Status>) -> walk::Action {
            std::ops::ControlFlow::Continue(())
        }

        fn can_recurse(
            &mut self,
            entry: EntryRef<'_>,
            for_deletion: Option<ForDeletionMode>,
            worktree_root_is_repository: bool,
        ) -> bool {
            self.paths.push(entry.rela_path.as_ref().to_owned());
            entry.status.can_recurse(
                entry.disk_kind,
                entry.pathspec_match,
                for_deletion,
                worktree_root_is_repository,
            )
        }
    }

    let root = gix_testtools::scripted_fixture_read_only("cached-listing.sh")?;
    let collect_callbacks = |cached_listing: Option<&walk::CachedListing>, omit_ignored| -> TestResult<_> {
        let mut delegate = RecordDirectories {
            omit_ignored,
            ..Default::default()
        };
        let ((outcome, _), _) = try_collect_filtered_opts_collect(
            &root,
            None,
            |_, ctx| walk(&root, ctx, options(), &mut delegate),
            None::<&str>,
            Options {
                cached_listing,
                ..Default::default()
            },
        )?;
        delegate.paths.sort();
        Ok((outcome, delegate.paths))
    };
    let (uncached, expected) = collect_callbacks(None, false)?;
    assert!(
        expected.iter().any(|path| path == "generated"),
        "ignored directories are visible to the custom recursion callback"
    );
    for includes_ignored in [false, true] {
        let listing = listing(includes_ignored);
        let (cached, actual) = collect_callbacks(Some(&listing), false)?;
        assert_eq!(
            actual, expected,
            "caching must preserve the delegate's directory callbacks"
        );
        if includes_ignored {
            assert_eq!(cached.read_dir_calls, 0, "complete listings need no delegate opt-in");
            assert!(cached.cached_dir_listings > 0);
        } else {
            assert_eq!(
                cached.cached_dir_listings, 0,
                "partial listings require explicit opt-in"
            );
            assert_eq!(cached.read_dir_calls, uncached.read_dir_calls);
        }
    }
    let mut compact = listing(false);
    compact.directories.retain(|path, _| path.is_empty());
    let root = compact.directories.get_mut(&BString::default()).expect("root listing");
    root.entries.retain(|(name, _)| name != ".gitignore");
    root.omitted_tracked = true;
    let (out, actual) = collect_callbacks(Some(&compact), true)?;
    assert_eq!(
        out.cached_dir_listings, 0,
        "permission to omit ignored directories cannot omit tracked ones"
    );
    assert_eq!(actual, expected, "the delegate still sees every directory on disk");
    Ok(())
}
