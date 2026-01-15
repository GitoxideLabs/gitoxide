use std::{collections::BTreeSet, sync::atomic::AtomicBool};

use gix_dir::{
    entry,
    entry::{Kind::*, PathspecMatch::*, Property::*, Status::*},
    walk,
    walk::{
        CollapsedEntriesEmissionMode::{All, OnStatusMismatch},
        EmissionMode::*,
        ForDeletionMode,
    },
    EntryRef,
};
use gix_ignore::Kind::*;
use pretty_assertions::assert_eq;

use crate::walk_utils::{
    collect, collect_filtered, collect_filtered_with_cwd, entry, entry_dirstat, entry_nokind, entry_nomatch, entryps,
    entryps_dirstat, fixture, fixture_in, options, options_emit_all, try_collect, try_collect_filtered_opts,
    try_collect_filtered_opts_collect, try_collect_filtered_opts_collect_with_root, EntryExt, Options,
};

#[test]
#[cfg(unix)]
fn root_is_fifo() {
    let root = fixture_in("fifo", "top-level");

    let err = try_collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            gix_dir::walk::Options {
                emit_ignored: Some(Matching),
                ..options()
            },
            keep,
        )
    })
    .unwrap_err();
    assert!(
        matches!(err, gix_dir::walk::Error::WorktreeRootIsFile { .. }),
        "roots simply need to be directories to work"
    );
}

#[test]
#[cfg(unix)]
fn one_top_level_fifo() {
    let root = fixture_in("fifo", "single-top-level-fifo");

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            gix_dir::walk::Options {
                emit_pruned: false,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 2,
        }
    );

    assert_eq!(
        entries,
        &[entry("top", Untracked, Untrackable),],
        "Untrackable entries are like normal files, but with a different state"
    );
}

#[test]
#[cfg(unix)]
fn fifo_in_traversal() {
    let root = fixture_in("fifo", "two-fifos-two-files");

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            gix_dir::walk::Options {
                emit_pruned: true,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );

    assert_eq!(
        entries,
        &[
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry("dir-with-file/nested-file", Untracked, File),
            entry("dir/nested", Untracked, Untrackable),
            entry("file", Untracked, File),
            entry("top", Untracked, Untrackable),
        ],
        "Untrackable entries only differ by their disk-kind"
    );
}

#[test]
fn symlink_to_dir_can_be_excluded() -> crate::Result {
    let root = fixture_in("many-symlinks", "excluded-symlinks-to-dir");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            gix_dir::walk::Options {
                emit_ignored: Some(Matching),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 9,
        }
    );

    assert_eq!(
        entries,
        &[
            entry("file1", Ignored(Expendable), Symlink),
            entry("file2", Untracked, Symlink),
            entry("ignored", Ignored(Expendable), Directory),
            entry("ignored-must-be-dir", Ignored(Expendable), Directory),
            entry("src/file", Untracked, File),
            entry("src1", Ignored(Expendable), Symlink),
            entry("src2", Untracked, Symlink), /* marked as src2/ in .gitignore */
        ],
        "by default, symlinks are counted as files only, even if they point to a directory, when handled by the exclude machinery"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            gix_dir::walk::Options {
                emit_ignored: Some(Matching),
                symlinks_to_directories_are_ignored_like_directories: true,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 9,
        }
    );

    assert_eq!(
        entries,
        &[
            entry("file1", Ignored(Expendable), Symlink),
            entry("file2", Untracked, Symlink),
            entry("ignored", Ignored(Expendable), Directory),
            entry("ignored-must-be-dir", Ignored(Expendable), Directory),
            entry("src/file", Untracked, File),
            entry("src1", Ignored(Expendable), Symlink),
            entry("src2", Ignored(Expendable), Symlink), /* marked as src2/ in .gitignore */
        ],
        "with libgit2 compatibility enabled, symlinks to directories are treated like a directory, not symlink"
    );
    Ok(())
}

#[test]
fn root_may_not_lead_through_symlinks() -> crate::Result {
    for (name, intermediate, expected) in [
        ("immediate-breakout-symlink", "", 0),
        ("breakout-symlink", "hide", 1),
        ("breakout-symlink", "hide/../hide", 1),
    ] {
        let root = fixture_in("many-symlinks", name);
        let troot = root.join(intermediate).join("breakout");
        let err = try_collect_filtered_opts_collect_with_root(
            &root,
            None,
            Some(&troot),
            |keep, ctx| walk(&root, ctx, options(), keep),
            None::<&str>,
            Default::default(),
        )
        .unwrap_err();
        assert!(
            matches!(err, walk::Error::SymlinkInRoot { component_index, .. } if component_index == expected),
            "{name} should have component {expected}"
        );
    }
    Ok(())
}

#[test]
fn root_may_be_a_symlink_if_it_is_the_worktree() -> crate::Result {
    let root = fixture_in("many-symlinks", "worktree-root-is-symlink");
    let ((_out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            gix_dir::walk::Options {
                emit_ignored: Some(Matching),
                symlinks_to_directories_are_ignored_like_directories: true,
                ..options()
            },
            keep,
        )
    });

    assert_eq!(
        entries,
        &[
            entry("file1", Ignored(Expendable), Symlink),
            entry("file2", Untracked, Symlink),
            entry("ignored", Ignored(Expendable), Directory),
            entry("ignored-must-be-dir", Ignored(Expendable), Directory),
            entry("src/file", Untracked, File),
            entry("src1", Ignored(Expendable), Symlink),
            entry("src2", Ignored(Expendable), Symlink), /* marked as src2/ in .gitignore */
        ],
        "it traversed the directory normally - without this capability, symlinked repos can't be traversed"
    );
    Ok(())
}

#[test]
fn should_interrupt_works_even_in_empty_directories() {
    let root = fixture("empty");
    let should_interrupt = AtomicBool::new(true);
    let err = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| walk(&root, ctx, gix_dir::walk::Options { ..options() }, keep),
        None::<&str>,
        Options {
            should_interrupt: Some(&should_interrupt),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, gix_dir::walk::Error::Interrupted));
}

#[test]
fn empty_root() -> crate::Result {
    let root = fixture("empty");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );
    assert_eq!(
        entries.len(),
        0,
        "by default, nothing is shown as the directory is empty"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_empty_directories: true,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );
    assert_eq!(
        entries,
        [entry("", Untracked, Directory).with_property(EmptyDirectory)],
        "this is how we can indicate the worktree is entirely untracked"
    );
    Ok(())
}

#[test]
fn complex_empty() -> crate::Result {
    let root = fixture("complex-empty");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 9,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );
    assert_eq!(
        entries,
        &[
            entry("dirs-and-files/dir/file", Untracked, File),
            entry("dirs-and-files/sub", Untracked, Directory).with_property(EmptyDirectory),
            entry("empty-toplevel", Untracked, Directory).with_property(EmptyDirectory),
            entry("only-dirs/other", Untracked, Directory).with_property(EmptyDirectory),
            entry("only-dirs/sub/subsub", Untracked, Directory).with_property(EmptyDirectory),
        ],
        "we see each and every directory, and get it classified as empty as it's set to be emitted"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_empty_directories: false,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 9,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );
    assert_eq!(
        entries,
        &[entry("dirs-and-files/dir/file", Untracked, File),],
        "by default, no empty directory shows up"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_empty_directories: true,
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 9,
            returned_entries: entries.len(),
            seen_entries: 9,
        }
    );
    assert_eq!(
        entries,
        &[
            entry("dirs-and-files", Untracked, Directory),
            entry("empty-toplevel", Untracked, Directory).with_property(EmptyDirectory),
            entry("only-dirs", Untracked, Directory),
        ],
        "empty directories collapse just fine"
    );
    Ok(())
}

#[test]
fn ignored_with_prefix_pathspec_collapses_just_like_untracked() -> crate::Result {
    let root = fixture("untracked-and-ignored-for-collapse");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_untracked: CollapseDirectory,
                    emit_ignored: Some(CollapseDirectory),
                    ..options()
                },
                keep,
            )
        },
        ["untracked", "no-match"],
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 6,
        }
    );
    assert_eq!(
        entries,
        [entryps("untracked", Untracked, Directory, Prefix)],
        "prefix matches allow untracked directories to collapse"
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_untracked: CollapseDirectory,
                    emit_ignored: Some(CollapseDirectory),
                    ..options()
                },
                keep,
            )
        },
        ["ignored", "ignored-inside"],
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 4,
            returned_entries: entries.len(),
            seen_entries: 8,
        }
    );
    assert_eq!(
        entries,
        [
            entryps("ignored", Ignored(Expendable), Directory, Prefix),
            entryps("ignored-inside", Ignored(Expendable), Directory, Prefix)
        ],
        "prefix matches allow ignored directories to collapse as well"
    );
    Ok(())
}

#[test]
fn ignored_dir_with_cwd_handling() -> crate::Result {
    let root = fixture("untracked-and-ignored-for-collapse");
    let ((out, _root), entries) = collect_filtered_with_cwd(
        &root,
        Some(&root.join("ignored")),
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_ignored: Some(CollapseDirectory),
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3,
        }
    );
    assert_eq!(
        entries,
        [entryps("ignored", Ignored(Expendable), Directory, Prefix)],
        "even if the traversal root is for deletion, unless the CWD is set it will be collapsed (no special cases)"
    );

    let real_root = gix_path::realpath(&root)?;
    let ((out, _root), entries) = collect_filtered_with_cwd(
        &real_root,
        Some(&real_root.join("ignored")),
        Some("ignored"),
        |keep, ctx| {
            walk(
                &real_root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_ignored: Some(CollapseDirectory),
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 2,
        }
    );
    assert_eq!(
        entries,
        [
            entryps("ignored/b", Ignored(Expendable), File, Prefix),
        ],
        "the traversal starts from the top, but we automatically prevent the 'd' directory from being deleted by stopping its collapse."
    );

    let real_root = gix_path::realpath(fixture("subdir-untracked-and-ignored"))?;
    let ((out, _root), entries) = collect_filtered_with_cwd(
        &real_root,
        None,
        Some("d/d/generated"),
        |keep, ctx| {
            walk(
                &real_root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_ignored: Some(CollapseDirectory),
                    emit_pruned: false,
                    ..options()
                },
                keep,
            )
        },
        Some(":/*generated/*"),
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 8,
            returned_entries: entries.len(),
            seen_entries: 26
        }
    );
    assert_eq!(
        entries,
        [
            entryps("d/d/generated/b", Ignored(Expendable), File, WildcardMatch),
            entryps("d/generated", Ignored(Expendable), Directory, WildcardMatch),
            entryps("generated", Ignored(Expendable), Directory, WildcardMatch),
        ],
        "'d/d/generated/b' is there because the parent directory isn't allowed to fold due to the CWD rule."
    );

    Ok(())
}

#[test]
fn ignored_with_cwd_handling() -> crate::Result {
    let root = gix_path::realpath(fixture("ignored-with-empty"))?;
    let ((out, _root), entries) = collect_filtered_with_cwd(
        &root,
        None,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_ignored: Some(CollapseDirectory),
                    emit_empty_directories: true,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 3,
        }
    );

    assert_eq!(
        entries,
        [entry("target", Ignored(Expendable), Directory),],
        "the baseline shows the content"
    );

    let ((out, _root), entries) = collect_filtered_with_cwd(
        &root,
        Some(&root),
        Some("target/empty"),
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_ignored: Some(CollapseDirectory),
                    emit_empty_directories: true,
                    ..options()
                },
                keep,
            )
        },
        Some("target"),
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 5,
            returned_entries: entries.len(),
            seen_entries: 7,
        }
    );

    assert_eq!(
        entries,
        [
            entryps("target/debug", Ignored(Expendable), Directory, Prefix),
            entryps("target/empty", Ignored(Expendable), Directory, Prefix).with_property(EmptyDirectoryAndCWD),
            entryps("target/release", Ignored(Expendable), Directory, Prefix),
        ],
        "it detects empty as CWD (very special case) and lists it as usual, while also preventing collapse to assure \
         to not accidentally end up trying to delete a parent directory"
    );
    Ok(())
}

#[test]
fn only_untracked_with_cwd_handling() -> crate::Result {
    let root = fixture("only-untracked");
    let ((out, _root), entries) = collect_filtered_with_cwd(
        &root,
        None,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_untracked: CollapseDirectory,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 9,
        }
    );
    assert_eq!(
        entries,
        [
            entry("a", Untracked, File),
            entry("b", Untracked, File),
            entry("c", Untracked, File),
            entry("d", Untracked, Directory),
        ],
        "the top-level is never collapsed, as our CWD is the worktree root"
    );

    let ((out, _root), entries) = collect_filtered_with_cwd(
        &root,
        Some(&root.join("d")),
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_untracked: CollapseDirectory,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );
    assert_eq!(
        entries,
        [entryps("d", Untracked, Directory, Prefix),],
        "even if the traversal root is for deletion, unless the CWD is set it will be collapsed (no special cases)"
    );

    let real_root = gix_path::realpath(&root)?;
    let ((out, _root), entries) = collect_filtered_with_cwd(
        &real_root,
        Some(&real_root),
        Some("d"),
        |keep, ctx| {
            walk(
                &real_root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_untracked: CollapseDirectory,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 8,
        }
    );
    assert_eq!(
        entries,
        [
            entry("a", Untracked, File),
            entry("b", Untracked, File),
            entry("c", Untracked, File),
            entry("d/a", Untracked, File),
            entry("d/b", Untracked, File),
            entry("d/d", Untracked, Directory),
        ],
        "the traversal starts from the top, but we automatically prevent the 'd' directory from being deleted by stopping its collapse."
    );

    let ((out, _root), entries) = collect_filtered_with_cwd(
        &real_root,
        None,
        Some("d"),
        |keep, ctx| {
            walk(
                &real_root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_untracked: CollapseDirectory,
                    ..options()
                },
                keep,
            )
        },
        Some("../d/"),
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 4,
        }
    );
    assert_eq!(
        entries,
        [
            entryps("d/a", Untracked, File, Prefix),
            entryps("d/b", Untracked, File, Prefix),
            entryps("d/d", Untracked, Directory, Prefix),
        ],
        "this will correctly detect that the pathspec leads back into our CWD, which wouldn't be the case otherwise"
    );

    Ok(())
}

#[test]
fn only_untracked_with_pathspec() -> crate::Result {
    let root = fixture("only-untracked");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(Default::default()),
                    emit_untracked: CollapseDirectory,
                    ..options()
                },
                keep,
            )
        },
        Some("d/"),
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );
    assert_eq!(
        entries,
        [entryps("d", Untracked, Directory, Prefix)],
        "this is equivalent as if we use a prefix, as we end up starting the traversal from 'd'"
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: None,
                    emit_untracked: CollapseDirectory,
                    ..options()
                },
                keep,
            )
        },
        Some("d/"),
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );
    assert_eq!(
        entries,
        [entryps("d", Untracked, Directory, Prefix)],
        "When not deleting things, it's once again the same effect as with a prefix"
    );
    Ok(())
}

#[test]
fn only_untracked_with_prefix_deletion() -> crate::Result {
    let root = fixture("only-untracked");
    let troot = root.join("d");
    let ((out, _root), entries) = collect(&root, Some(&troot), |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                for_deletion: Some(Default::default()),
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );
    assert_eq!(
        entries,
        [entryps("d", Untracked, Directory, Prefix),],
        "This is like being inside of 'd', but the CWD is now explicit so we happily fold"
    );

    let ((out, _root), entries) = collect(&root, Some(&troot), |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                for_deletion: None,
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );
    assert_eq!(
        entries,
        [entryps("d", Untracked, Directory, Prefix)],
        "However, when not deleting, we can collapse, as we could still add all with 'git add .'"
    );
    Ok(())
}

#[test]
fn only_untracked() -> crate::Result {
    let root = fixture("only-untracked");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 7,
        }
    );
    assert_eq!(
        entries,
        [
            entry("a", Untracked, File),
            entry("b", Untracked, File),
            entry("c", Untracked, File),
            entry("d/a", Untracked, File),
            entry("d/b", Untracked, File),
            entry("d/d/a", Untracked, File),
        ]
    );

    let ((out, _root), entries) =
        collect_filtered(&root, None, |keep, ctx| walk(&root, ctx, options(), keep), Some("d/*"));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3,
        }
    );
    assert_eq!(
        entries,
        [
            entryps("d/a", Untracked, File, WildcardMatch),
            entryps("d/b", Untracked, File, WildcardMatch),
            entryps("d/d/a", Untracked, File, WildcardMatch),
        ]
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 7 + 2,
        },
        "There are 2 extra directories that we fold into, but ultimately discard"
    );
    assert_eq!(
        entries,
        [
            entry("a", Untracked, File),
            entry("b", Untracked, File),
            entry("c", Untracked, File),
            entry("d", Untracked, Directory),
        ]
    );
    Ok(())
}

#[test]
fn only_untracked_explicit_pathspec_selection() -> crate::Result {
    let root = fixture("only-untracked");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_untracked: Matching,
                    ..options()
                },
                keep,
            )
        },
        ["d/a", "d/d/a"],
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3,
        },
    );
    assert_eq!(
        entries,
        [
            entryps("d/a", Untracked, File, Verbatim),
            entryps("d/d/a", Untracked, File, Verbatim)
        ],
        "this works just like expected, as nothing is collapsed anyway"
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_untracked: CollapseDirectory,
                    emit_pruned: true,
                    ..options()
                },
                keep,
            )
        },
        ["d/a", "d/d/a"],
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3,
        },
        "no collapsing happens"
    );
    assert_eq!(
        entries,
        [
            entryps("d/a", Untracked, File, Verbatim),
            entry_nokind("d/b", Pruned),
            entryps("d/d/a", Untracked, File, Verbatim)],
        "we actually want to mention the entries that matched the pathspec precisely, so two of them would be needed here \
        while preventing the directory collapse from happening"
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_untracked: CollapseDirectory,
                    ..options()
                },
                keep,
            )
        },
        Some("d/*"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 2 + 3,
        },
        "collapsing happens just like Git"
    );
    assert_eq!(
        entries,
        [entryps("d", Untracked, Directory, WildcardMatch),],
        "wildcard matches on the top-level without deletion show just the top level"
    );
    Ok(())
}

#[test]
fn expendable_and_precious() {
    let root = fixture("expendable-and-precious");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 6,
            returned_entries: entries.len(),
            seen_entries: 18,
        }
    );
    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry(".gitignore", Tracked, File),
            entry("a.o", Ignored(Expendable), File),
            entry("all-expendable", Ignored(Expendable), Directory),
            entry("all-expendable-by-filematch/e.o", Ignored(Expendable), File),
            entry("all-expendable-by-filematch/f.o", Ignored(Expendable), File),
            entry("all-precious", Ignored(Precious), Directory),
            entry("all-precious-by-filematch/a.precious", Ignored(Precious), File),
            entry("all-precious-by-filematch/b.precious", Ignored(Precious), File),
            entry("mixed/b.o", Ignored(Expendable), File),
            entry("mixed/precious", Ignored(Precious), File),
            entry("precious", Ignored(Precious), File),
            entry("some-expendable/file", Tracked, File),
            entry("some-expendable/file.o", Ignored(Expendable), File),
            entry("some-expendable/new", Untracked, File),
            entry("some-precious/file", Tracked, File),
            entry("some-precious/file.precious", Ignored(Precious), File),
            entry("some-precious/new", Untracked, File),
        ],
        "listing everything is a 'matching' preset, which is among the most efficient."
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                emit_tracked: true,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 6,
            returned_entries: entries.len(),
            seen_entries: 18 + 2,
        }
    );

    assert_eq!(
        entries,
        [
            entry(".gitignore", Tracked, File),
            entry("a.o", Ignored(Expendable), File),
            entry("all-expendable", Ignored(Expendable), Directory),
            entry("all-expendable-by-filematch", Ignored(Expendable), Directory),
            entry("all-precious", Ignored(Precious), Directory),
            entry("all-precious-by-filematch", Ignored(Precious), Directory),
            entry("mixed/b.o", Ignored(Expendable), File),
            entry("mixed/precious", Ignored(Precious), File),
            entry("precious", Ignored(Precious), File),
            entry("some-expendable/file", Tracked, File),
            entry("some-expendable/file.o", Ignored(Expendable), File),
            entry("some-expendable/new", Untracked, File),
            entry("some-precious/file", Tracked, File),
            entry("some-precious/file.precious", Ignored(Precious), File),
            entry("some-precious/new", Untracked, File),
        ],
        "those that have tracked and ignored won't be collapsed, nor will be folders that have mixed precious and ignored files,\
        those with all files of one type will be collapsed though"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: None,
                emit_untracked: CollapseDirectory,
                emit_tracked: false,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 6,
            returned_entries: entries.len(),
            seen_entries: 16 + 2,
        }
    );

    assert_eq!(
        entries,
        [
            entry("some-expendable/new", Untracked, File),
            entry("some-precious/new", Untracked, File),
        ],
        "even with collapsing, once there is a tracked file in the directory, we show the untracked file directly"
    );
}

#[test]
fn subdir_untracked() -> crate::Result {
    let root = fixture("subdir-untracked");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 7,
        }
    );
    assert_eq!(entries, [entry("d/d/a", Untracked, File)]);

    let ((out, actual_root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&root),
        |keep, ctx| walk(&root, ctx, options(), keep),
        Some("d/d/*"),
        Default::default(),
    )?;
    assert_eq!(actual_root, root);
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 7,
        },
        "pruning has no actual effect here as there is no extra directories that could be avoided"
    );
    assert_eq!(entries, &[entryps("d/d/a", Untracked, File, WildcardMatch)]);

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 7 + 1,
        },
        "there is a folded directory we added"
    );
    assert_eq!(entries, [entry("d/d", Untracked, Directory)]);
    Ok(())
}

#[test]
fn only_untracked_from_subdir() -> crate::Result {
    let root = fixture("only-untracked");
    let troot = root.join("d").join("d");
    let ((out, _root), entries) = collect(&root, Some(&troot), |keep, ctx| walk(&root, ctx, options(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );
    assert_eq!(
        entries,
        [entryps("d/d/a", Untracked, File, Prefix)],
        "even from subdirs, paths are worktree relative"
    );
    Ok(())
}

#[test]
fn untracked_and_ignored_pathspec_guidance() -> crate::Result {
    for for_deletion in [None, Some(Default::default())] {
        let root = fixture("subdir-untracked-and-ignored");
        let ((out, _root), entries) = collect_filtered(
            &root,
            None,
            |keep, ctx| {
                walk(
                    &root,
                    ctx,
                    walk::Options {
                        emit_ignored: Some(CollapseDirectory),
                        for_deletion,
                        ..options()
                    },
                    keep,
                )
            },
            Some("d/d/generated/b"),
        );
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 1,
                returned_entries: entries.len(),
                seen_entries: 1,
            },
            "we have to read the parent directory, just like git, as we can't assume a directory"
        );
        assert_eq!(
            entries,
            [entryps("d/d/generated/b", Ignored(Expendable), File, Verbatim)],
            "pathspecs allow reaching into otherwise ignored directories, ignoring the flag to collapse"
        );
    }
    Ok(())
}

#[test]
fn untracked_and_ignored_for_deletion_negative_wildcard_spec() -> crate::Result {
    let root = fixture("subdir-untracked-and-ignored");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    emit_pruned: true,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some(":!*generated*"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 5,
            returned_entries: entries.len(),
            seen_entries: 23,
        },
    );
    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry(".gitignore", Untracked, File),
            entry("a.o", Ignored(Expendable), File),
            entry("b.o", Ignored(Expendable), File),
            entry("c.o", Ignored(Expendable), File),
            entry("d/a.o", Ignored(Expendable), File),
            entry("d/b.o", Ignored(Expendable), File),
            entry("d/d/a", Untracked, File),
            entry("d/d/a.o", Ignored(Expendable), File),
            entry("d/d/b.o", Ignored(Expendable), File),
            entryps("d/d/generated", Ignored(Expendable), Directory, Excluded),
            entryps("d/generated", Ignored(Expendable), Directory, Excluded),
            entryps("generated", Ignored(Expendable), Directory, Excluded),
            entry("objs", Ignored(Expendable), Directory),
        ],
        "'generated' folders are excluded, and collapsing is done where possible. \
         Note that Git wants to incorrectly delete `d/d` as it doesn't see the excluded \
         ignored file inside, which would incorrectly delete something the users didn't want deleted."
    );
    Ok(())
}

#[test]
fn untracked_and_ignored_for_deletion_positive_wildcard_spec() -> crate::Result {
    let root = fixture("subdir-untracked-and-ignored");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    emit_pruned: true,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some("*generated*"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 8,
            returned_entries: entries.len(),
            seen_entries: 27,
        },
    );
    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).no_kind().with_property(DotGit),
            entry_nomatch(".gitignore", Pruned, File),
            entry_nomatch("a.o", Ignored(Expendable), File),
            entry_nomatch("b.o", Ignored(Expendable), File),
            entry_nomatch("c.o", Ignored(Expendable), File),
            entry_nomatch("d/a.o", Ignored(Expendable), File),
            entry_nomatch("d/b.o", Ignored(Expendable), File),
            entry_nomatch("d/d/a", Pruned, File),
            entry_nomatch("d/d/a.o", Ignored(Expendable), File),
            entry_nomatch("d/d/b.o", Ignored(Expendable), File),
            entryps("d/d/generated", Ignored(Expendable), Directory, WildcardMatch),
            entryps("d/generated", Ignored(Expendable), Directory, WildcardMatch),
            entryps("generated", Ignored(Expendable), Directory, WildcardMatch),
            entry_nomatch("objs", Ignored(Expendable), Directory),
        ],
        "'generated' folders are included, and collapsing is done where possible"
    );
    Ok(())
}

#[test]
fn untracked_and_ignored_for_deletion_nonmatching_wildcard_spec() -> crate::Result {
    let root = fixture("subdir-untracked-and-ignored");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    emit_pruned: true,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some("*foo*"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 8,
            returned_entries: entries.len(),
            seen_entries: 28,
        },
    );
    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).no_match().with_property(DotGit),
            entry_nomatch(".gitignore", Pruned, File),
            entry_nomatch("a.o", Ignored(Expendable), File),
            entry_nomatch("b.o", Ignored(Expendable), File),
            entry_nomatch("c.o", Ignored(Expendable), File),
            entry_nomatch("d/a.o", Ignored(Expendable), File),
            entry_nomatch("d/b.o", Ignored(Expendable), File),
            entry_nomatch("d/d", Ignored(Expendable), Directory),
            entry_nomatch("d/d/a", Pruned, File),
            entry_nomatch("d/generated", Ignored(Expendable), Directory),
            entry_nomatch("generated", Ignored(Expendable), Directory),
            entry_nomatch("objs", Ignored(Expendable), Directory),
        ],
        "'generated' folders are included, and collapsing is done where possible"
    );
    Ok(())
}
#[test]
fn nested_precious_repo_respects_wildcards() -> crate::Result {
    let root = fixture("precious-nested-repository");
    for for_deletion in [
        Some(ForDeletionMode::FindNonBareRepositoriesInIgnoredDirectories),
        Some(ForDeletionMode::FindRepositoriesInIgnoredDirectories),
    ] {
        let (_out, entries) = collect_filtered(
            &root,
            None,
            |keep, ctx| {
                walk(
                    &root,
                    ctx,
                    walk::Options {
                        emit_ignored: Some(CollapseDirectory),
                        emit_untracked: CollapseDirectory,
                        emit_pruned: false,
                        for_deletion,
                        ..options()
                    },
                    keep,
                )
            },
            Some("*foo/"),
        );
        // NOTE: do not use `_out` as `.git` directory contents can change, it's controlled by Git, causing flakiness.

        assert_eq!(entries, [], "nothing matches, of course");
    }
    Ok(())
}

#[test]
fn nested_ignored_dirs_for_deletion_nonmatching_wildcard_spec() -> crate::Result {
    let root = fixture("ignored-dir-nested-minimal");
    let (_out, entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    emit_pruned: false,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some("*foo*"),
    );
    // NOTE: do not use `_out` as `.git` directory contents can change, it's controlled by Git, causing flakiness.

    assert_eq!(
        entries,
        [],
        "it figures out that nothing actually matches, even though it has to check everything"
    );

    let (_out, entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    emit_pruned: true,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some("*foo*"),
    );
    // NOTE: do not use `_out` as `.git` directory contents can change, it's controlled by Git, causing flakiness.

    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).with_property(DotGit),
            entry_nomatch(".gitignore", Pruned, File),
            entry_nomatch("bare/HEAD", Pruned, File),
            entry_nomatch("bare/info/exclude", Pruned, File),
            entry_nomatch("bare/objects", Untracked, Directory),
            entry_nomatch("bare/refs", Untracked, Directory),
            entry_nomatch("dir", Ignored(Expendable), Directory),
        ],
        "it's possible to observe pruned entries like before"
    );
    Ok(())
}

#[test]
fn expendable_and_precious_in_ignored_dir_with_pathspec() -> crate::Result {
    let root = fixture("expendable-and-precious-nested-in-ignored-dir");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                emit_untracked: CollapseDirectory,
                emit_pruned: true,
                emit_tracked: true,
                for_deletion: Some(Default::default()),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 5,
        },
    );

    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry(".gitignore", Tracked, File).with_index_kind(File),
            entry("ignored", Ignored(Expendable), Directory),
            entry("other", Ignored(Expendable), Directory),
        ],
        "without pathspec, it collapses completely. \
         It's interesting that 'other' claims to be ignored - due to the collapse of `other/ignored` it inherits the sub-directory status.\
         However, it's what we want, compared to the alternative of leaving it empty, and then detecting it as empty\
         the next time we run."
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    emit_pruned: true,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some("*ignored*"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 9,
            returned_entries: entries.len(),
            seen_entries: 19,
        },
    );

    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).with_property(DotGit).no_match(),
            entry_nokind("ignored/d/.git", Ignored(Expendable) ).with_property(DotGit).with_match(WildcardMatch),
            entryps("ignored/d/.gitignore", Ignored(Expendable), File, WildcardMatch),
            entryps("ignored/d/a.o", Ignored(Expendable), File, WildcardMatch),
            entryps(
                "ignored/d/all-expendable",
                Ignored(Expendable),
                Directory,
                WildcardMatch
            ),
            entryps("ignored/d/all-precious", Ignored(Precious), Directory, WildcardMatch),
            entryps("ignored/d/mixed", Ignored(Expendable), Directory, WildcardMatch),
            entryps("ignored/d/precious", Ignored(Expendable), File, WildcardMatch),
            entryps("other", Ignored(Expendable), Directory, WildcardMatch),
        ],
        "with pathspec, we match what's inside and expect to have all the lowest-level paths that have 'ignored' in them.\
         It seems strange that 'precious' isn't precious, while 'all-precious' is. However, the ignore-search is special
         as it goes backward through directories (using directory-declarations), and aborts if it matched. Thus it finds
         that '$/all-precious/' matched, but in the other cases it matches 'ignored/'.
        'other' gets folded and inherits, just like before.
        Also, look how the ignore-state overrides the prune-default for DotGit kinds, to have more finegrained classification."
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    emit_pruned: true,
                    for_deletion: None,
                    ..options()
                },
                keep,
            )
        },
        Some("*ignored*"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 9,
            returned_entries: entries.len(),
            seen_entries: 19,
        },
    );

    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).with_property(DotGit).no_match(),
            entry_nokind("ignored/d/.git", Pruned)
                .with_property(DotGit)
                .with_match(WildcardMatch),
            entryps("ignored/d/.gitignore", Ignored(Expendable), File, WildcardMatch),
            entryps("ignored/d/a.o", Ignored(Expendable), File, WildcardMatch),
            entryps(
                "ignored/d/all-expendable",
                Ignored(Expendable),
                Directory,
                WildcardMatch
            ),
            entryps("ignored/d/all-precious", Ignored(Precious), Directory, WildcardMatch),
            entryps("ignored/d/mixed", Ignored(Expendable), Directory, WildcardMatch),
            entryps("ignored/d/precious", Ignored(Expendable), File, WildcardMatch),
            entryps("other", Ignored(Expendable), Directory, WildcardMatch),
        ],
        "The same as above, but without delete mode, we don't upgrade the status of ignored dot-git entries"
    );
    Ok(())
}

#[test]
fn untracked_and_ignored() -> crate::Result {
    let root = fixture("subdir-untracked-and-ignored");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(Matching),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 5,
            returned_entries: entries.len(),
            seen_entries: 21,
        },
        "some untracked ones are hidden by default"
    );
    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("a.o", Ignored(Expendable), File),
            entry("b.o", Ignored(Expendable), File),
            entry("c.o", Ignored(Expendable), File),
            entry("d/a.o", Ignored(Expendable), File),
            entry("d/b.o", Ignored(Expendable), File),
            entry("d/d/a", Untracked, File),
            entry("d/d/a.o", Ignored(Expendable), File),
            entry("d/d/b.o", Ignored(Expendable), File),
            entry("d/d/generated", Ignored(Expendable), Directory),
            entry("d/generated", Ignored(Expendable), Directory),
            entry("generated", Ignored(Expendable), Directory),
            entry("objs/a.o", Ignored(Expendable), File),
            entry("objs/b.o", Ignored(Expendable), File),
            entry("objs/sub/other.o", Ignored(Expendable), File),
        ]
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_pruned: true,
                    ..options()
                },
                keep,
            )
        },
        Some("**/a*"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 5,
            returned_entries: entries.len(),
            seen_entries: 21,
        },
        "basically the same result…"
    );

    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).with_property(DotGit).no_match(),
            entry_nomatch(".gitignore", Pruned, File),
            entryps("d/d/a", Untracked, File, WildcardMatch),
        ],
        "…but with different classification as the ignore file is pruned so it's not untracked anymore"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: None,
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 5,
            returned_entries: entries.len(),
            seen_entries: 21 + 1,
        },
        "we still encounter the same amount of entries, and 1 folded directory"
    );
    assert_eq!(
        entries,
        [entry(".gitignore", Untracked, File), entry("d/d", Untracked, Directory)],
        "aggregation kicks in here"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 5,
            returned_entries: entries.len(),
            seen_entries: 21 + 2,
        },
        "some untracked ones are hidden by default, folded directories"
    );
    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("a.o", Ignored(Expendable), File),
            entry("b.o", Ignored(Expendable), File),
            entry("c.o", Ignored(Expendable), File),
            entry("d/a.o", Ignored(Expendable), File),
            entry("d/b.o", Ignored(Expendable), File),
            entry("d/d/a", Untracked, File),
            entry("d/d/a.o", Ignored(Expendable), File),
            entry("d/d/b.o", Ignored(Expendable), File),
            entry("d/d/generated", Ignored(Expendable), Directory),
            entry("d/generated", Ignored(Expendable), Directory),
            entry("generated", Ignored(Expendable), Directory),
            entry("objs", Ignored(Expendable), Directory),
        ],
        "objects are aggregated"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                emit_untracked: CollapseDirectory,
                emit_collapsed: Some(OnStatusMismatch),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 5,
            returned_entries: entries.len(),
            seen_entries: 21 + 3,
        },
        "some untracked ones are hidden by default, and folded directories"
    );
    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("a.o", Ignored(Expendable), File),
            entry("b.o", Ignored(Expendable), File),
            entry("c.o", Ignored(Expendable), File),
            entry("d/a.o", Ignored(Expendable), File),
            entry("d/b.o", Ignored(Expendable), File),
            entry("d/d", Untracked, Directory),
            entryps_dirstat("d/d/a.o", Ignored(Expendable), File, Always, Untracked),
            entryps_dirstat("d/d/b.o", Ignored(Expendable), File, Always, Untracked),
            entryps_dirstat("d/d/generated", Ignored(Expendable), Directory, Always, Untracked),
            entry("d/generated", Ignored(Expendable), Directory),
            entry("generated", Ignored(Expendable), Directory),
            entry("objs", Ignored(Expendable), Directory),
        ],
        "ignored ones are aggregated, and we get the same effect as with `git status --ignored` - collapsing of untracked happens\
        and we still list the ignored files that were inside.\
        Also note the entries that would be dropped in case of `git clean` are marked with `entry_dirstat`, which would display what's\
        done differently."
    );
    Ok(())
}

#[test]
fn untracked_and_ignored_collapse_handling_mixed() -> crate::Result {
    let root = fixture("subdir-untracked-and-ignored");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    for_deletion: None,
                    ..options()
                },
                keep,
            )
        },
        Some("d/d/b.o"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 4,
        },
        "it has to read 'd/d' as 'd/d/b.o' isn't a directory candidate"
    );

    assert_eq!(
        entries,
        [entryps("d/d/b.o", Ignored(Expendable), File, Verbatim)],
        "when files are selected individually, they are never collapsed"
    );

    for (spec, pathspec_match) in [("d/d/*", WildcardMatch), ("d/d", Prefix), ("d/d/", Prefix)] {
        let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
            &root,
            None,
            Some(&root),
            |keep, ctx| {
                walk(
                    &root,
                    ctx,
                    walk::Options {
                        emit_ignored: Some(CollapseDirectory),
                        emit_untracked: CollapseDirectory,
                        for_deletion: None,
                        emit_collapsed: Some(OnStatusMismatch),
                        ..options()
                    },
                    keep,
                )
            },
            Some(spec),
            Default::default(),
        )?;
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 4,
                returned_entries: entries.len(),
                seen_entries: 21,
            },
        );

        assert_eq!(
            entries,
            [
                entryps("d/d", Untracked, Directory, pathspec_match),
                entryps_dirstat("d/d/a.o", Ignored(Expendable), File, pathspec_match, Untracked),
                entryps_dirstat("d/d/b.o", Ignored(Expendable), File, pathspec_match, Untracked),
                entryps_dirstat(
                    "d/d/generated",
                    Ignored(Expendable),
                    Directory,
                    pathspec_match,
                    Untracked
                ),
            ],
            "with wildcard matches, it's OK to collapse though"
        );
    }
    Ok(())
}

#[test]
fn untracked_and_ignored_collapse_handling_mixed_with_prefix() -> crate::Result {
    let root = fixture("subdir-untracked-and-ignored");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    for_deletion: None,
                    emit_collapsed: Some(OnStatusMismatch),
                    ..options()
                },
                keep,
            )
        },
        Some("d/d"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 11
        },
        "this is not a directory, so the prefix is only 'd', not 'd/d'"
    );

    assert_eq!(
        entries,
        [
            entryps("d/d", Untracked, Directory, Prefix),
            entryps_dirstat("d/d/a.o", Ignored(Expendable), File, Prefix, Untracked),
            entryps_dirstat("d/d/b.o", Ignored(Expendable), File, Prefix, Untracked),
            entryps_dirstat("d/d/generated", Ignored(Expendable), Directory, Prefix, Untracked),
        ],
        "as it's not the top-level anymore (which is 'd', not 'd/d'), we will collapse"
    );

    for (spec, pathspec_match) in [("d/d/*", WildcardMatch), ("d/d/", Prefix)] {
        let ((out, _root), entries) = collect_filtered(
            &root,
            None,
            |keep, ctx| {
                walk(
                    &root,
                    ctx,
                    walk::Options {
                        emit_ignored: Some(CollapseDirectory),
                        emit_untracked: CollapseDirectory,
                        for_deletion: None,
                        emit_collapsed: Some(OnStatusMismatch),
                        ..options()
                    },
                    keep,
                )
            },
            Some(spec),
        );
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 2,
                returned_entries: entries.len(),
                seen_entries: 6,
            },
        );

        assert_eq!(
            entries,
            [
                entryps("d/d", Untracked, Directory, pathspec_match),
                entryps_dirstat("d/d/a.o", Ignored(Expendable), File, pathspec_match, Untracked),
                entryps_dirstat("d/d/b.o", Ignored(Expendable), File, pathspec_match, Untracked),
                entryps_dirstat(
                    "d/d/generated",
                    Ignored(Expendable),
                    Directory,
                    pathspec_match,
                    Untracked
                ),
            ],
            "{spec}: with wildcard matches, it's OK to collapse though"
        );
    }
    // TODO: try for deletion, and prefix combinations
    Ok(())
}

#[test]
fn untracked_and_ignored_collapse_handling_for_deletion_with_wildcards() -> crate::Result {
    let root = fixture("subdir-untracked-and-ignored");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some("*.o"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 8,
            returned_entries: entries.len(),
            seen_entries: 26
        },
    );
    assert_eq!(
        entries,
        [
            entryps("a.o", Ignored(Expendable), File, WildcardMatch),
            entryps("b.o", Ignored(Expendable), File, WildcardMatch),
            entryps("c.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/a.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/b.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/d/a.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/d/b.o", Ignored(Expendable), File, WildcardMatch),
            entryps("generated/a.o", Ignored(Expendable), File, WildcardMatch),
            entryps("objs", Ignored(Expendable), Directory, WildcardMatch),
        ],
        "when using wildcards like these, we actually want to see only the suffixed items even if they all match, like Git does. \
        However, we have no way to differentiate `*` from `*.o`, in which case Git decides to delete the directory instead of its \
        contents, so it's not perfect there either. \
        Thus we stick to the rule: if everything in the directory is going to be deleted, we delete the whole directory."
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    for_deletion: Some(Default::default()),
                    emit_collapsed: Some(OnStatusMismatch),
                    ..options()
                },
                keep,
            )
        },
        Some("*"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 8,
            returned_entries: entries.len(),
            seen_entries: 28
        },
    );
    assert_eq!(
        entries,
        [
            entryps(".gitignore", Untracked, File, WildcardMatch),
            entryps("a.o", Ignored(Expendable), File, WildcardMatch),
            entryps("b.o", Ignored(Expendable), File, WildcardMatch),
            entryps("c.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/a.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/b.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/d", Untracked, Directory, WildcardMatch),
            entryps_dirstat("d/d/a.o", Ignored(Expendable), File, WildcardMatch, Untracked),
            entryps_dirstat("d/d/b.o", Ignored(Expendable), File, WildcardMatch, Untracked),
            entryps_dirstat(
                "d/d/generated",
                Ignored(Expendable),
                Directory,
                WildcardMatch,
                Untracked
            ),
            entryps("d/generated", Ignored(Expendable), Directory, WildcardMatch),
            entryps("generated", Ignored(Expendable), Directory, WildcardMatch),
            entryps("objs", Ignored(Expendable), Directory, WildcardMatch),
        ],
        "In this case, Git is doing exactly the same"
    );
    Ok(())
}

#[test]
fn untracked_and_ignored_collapse_handling_for_deletion_with_prefix_wildcards() -> crate::Result {
    let root = fixture("subdir-untracked-and-ignored");
    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some("generated/*.o"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 2,
        },
    );
    assert_eq!(
        entries,
        [entryps("generated/a.o", Ignored(Expendable), File, WildcardMatch)],
        "this is the same result as '*.o', but limited to a subdirectory"
    );
    Ok(())
}

#[test]
fn untracked_and_ignored_collapse_handling_for_deletion_mixed() -> crate::Result {
    let root = fixture("subdir-untracked-and-ignored");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: None,
                emit_untracked: CollapseDirectory,
                for_deletion: Some(Default::default()),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 5,
            returned_entries: entries.len(),
            seen_entries: 21,
        },
    );

    assert_eq!(
        entries,
        [entry(".gitignore", Untracked, File), entry("d/d/a", Untracked, File)],
        "without ignored files, we only see untracked ones, without a chance to collapse. This actually is something Git fails to do."
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                emit_untracked: CollapseDirectory,
                for_deletion: Some(Default::default()),
                emit_collapsed: Some(OnStatusMismatch),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 5,
            returned_entries: entries.len(),
            seen_entries: 24,
        },
    );

    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("a.o", Ignored(Expendable), File),
            entry("b.o", Ignored(Expendable), File),
            entry("c.o", Ignored(Expendable), File),
            entry("d/a.o", Ignored(Expendable), File),
            entry("d/b.o", Ignored(Expendable), File),
            entry("d/d", Untracked, Directory),
            entryps_dirstat("d/d/a.o", Ignored(Expendable), File, Always, Untracked),
            entryps_dirstat("d/d/b.o", Ignored(Expendable), File, Always, Untracked),
            entryps_dirstat("d/d/generated", Ignored(Expendable), Directory, Always, Untracked),
            entry("d/generated", Ignored(Expendable), Directory),
            entry("generated", Ignored(Expendable), Directory),
            entry("objs", Ignored(Expendable), Directory),
        ],
        "with ignored files, we can collapse untracked and ignored like before"
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    for_deletion: Some(Default::default()),
                    emit_collapsed: Some(OnStatusMismatch),
                    ..options()
                },
                keep,
            )
        },
        Some("d/d/*"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 6,
        },
    );

    assert_eq!(
        entries,
        [
            entryps("d/d", Untracked, Directory, WildcardMatch),
            entryps_dirstat("d/d/a.o", Ignored(Expendable), File, WildcardMatch, Untracked),
            entryps_dirstat("d/d/b.o", Ignored(Expendable), File, WildcardMatch, Untracked),
            entryps_dirstat("d/d/generated", Ignored(Expendable), Directory, WildcardMatch, Untracked),
        ],
        "everything is filtered down to the pathspec, otherwise it's like before. Not how all-matching  'generated' collapses, \
        but also how 'd/d' collapses as our current working directory the worktree"
    );

    let real_root = gix_path::realpath(&root)?;
    let ((out, _root), entries) = collect_filtered_with_cwd(
        &real_root,
        Some(&real_root),
        Some("d/d"),
        |keep, ctx| {
            walk(
                &real_root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    for_deletion: Some(Default::default()),
                    emit_collapsed: Some(OnStatusMismatch),
                    ..options()
                },
                keep,
            )
        },
        Some("d/d/*"), // NOTE: this would be '*' in the real world and automatically prefixed, but the test-setup is limited
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 5,
        },
    );

    assert_eq!(
        entries,
        [
            entryps("d/d/a", Untracked, File, WildcardMatch),
            entryps("d/d/a.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/d/b.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/d/generated", Ignored(Expendable), Directory, WildcardMatch),
        ],
        "Now the CWD is 'd/d', which means we can't collapse it."
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    emit_tracked: true,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some("d/d/*.o"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 5,
        },
    );

    assert_eq!(
        entries,
        [
            entryps("d/d/a.o", Ignored(Expendable), File, WildcardMatch),
            entryps("d/d/b.o", Ignored(Expendable), File, WildcardMatch),
        ],
        "If the wildcard doesn't match everything, it can't be collapsed"
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    for_deletion: Some(Default::default()),
                    emit_collapsed: Some(OnStatusMismatch),
                    ..options()
                },
                keep,
            )
        },
        Some("d/d/"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 6,
        },
    );

    assert_eq!(
        entries,
        [
            entryps("d/d", Untracked, Directory, Prefix),
            entryps_dirstat("d/d/a.o", Ignored(Expendable), File, Prefix, Untracked),
            entryps_dirstat("d/d/b.o", Ignored(Expendable), File, Prefix, Untracked),
            entryps_dirstat("d/d/generated", Ignored(Expendable), Directory, Prefix, Untracked),
        ],
        "Now the whole folder is matched and can collapse, as no CWD is set - the prefix-based root isn't special anymore \
        as it is not easily predictable, and has its own rules."
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: None,
                    emit_untracked: CollapseDirectory,
                    for_deletion: Some(Default::default()),
                    ..options()
                },
                keep,
            )
        },
        Some("d/d/"),
    );
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 4,
        },
    );

    assert_eq!(
        entries,
        [entryps("d/d/a", Untracked, File, Prefix)],
        "a prefix match works similarly"
    );
    Ok(())
}

#[test]
fn precious_are_not_expendable() {
    let root = fixture("untracked-and-precious");
    let (_out, entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(Matching),
                emit_untracked: Matching,
                ..options_emit_all()
            },
            keep,
        )
    });
    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry(".gitignore", Tracked, File),
            entry("a.o", Ignored(Expendable), File),
            entry("d/a", Tracked, File),
            entry("d/a.o", Ignored(Expendable), File),
            entry("d/b", Tracked, File),
            entry("d/b.o", Ignored(Expendable), File),
            entry("d/d/a.precious", Ignored(Precious), File),
            entry("d/d/new", Untracked, File),
        ],
        "just to have an overview"
    );
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                emit_untracked: CollapseDirectory,
                emit_collapsed: Some(OnStatusMismatch),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 10,
        },
    );

    assert_eq!(
        entries,
        [
            entry("a.o", Ignored(Expendable), File),
            entry("d/a.o", Ignored(Expendable), File),
            entry("d/b.o", Ignored(Expendable), File),
            entry("d/d", Untracked, Directory),
            entryps_dirstat("d/d/a.precious", Ignored(Precious), File, Always, Untracked),
        ],
        "by default precious files are treated no differently than expendable files, which is fine\
            unless you want to delete `d/d`. Then we shouldn't ever see `d/d` and have to deal with \
            a collapsed precious file."
    );

    let ((out, _root), entries) = collect_filtered(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(CollapseDirectory),
                    emit_untracked: CollapseDirectory,
                    emit_collapsed: Some(OnStatusMismatch),
                    ..options()
                },
                keep,
            )
        },
        Some("d"),
    );

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 10,
        },
        "'d' is assumed to be a file, hence it's stripped to its base '', yielding one more call."
    );

    assert_eq!(
        entries,
        [
            entryps("d/a.o", Ignored(Expendable), File, Prefix),
            entryps("d/b.o", Ignored(Expendable), File, Prefix),
            entryps("d/d", Untracked, Directory, Prefix),
            entryps_dirstat("d/d/a.precious", Ignored(Precious), File, Prefix, Untracked),
        ],
        "should yield the same entries - note how collapsed directories inherit the pathspec"
    );
    for (equivalent_pathspec, expected_match) in [("d/*", WildcardMatch), ("d/", Prefix)] {
        let ((out, _root), entries) = collect_filtered(
            &root,
            None,
            |keep, ctx| {
                walk(
                    &root,
                    ctx,
                    walk::Options {
                        emit_ignored: Some(CollapseDirectory),
                        emit_untracked: CollapseDirectory,
                        emit_collapsed: Some(OnStatusMismatch),
                        ..options()
                    },
                    keep,
                )
            },
            Some(equivalent_pathspec),
        );
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 2,
                returned_entries: entries.len(),
                seen_entries: 7,
            },
            "{equivalent_pathspec}: should yield same result, they also see the 'd' prefix directory"
        );

        assert_eq!(
            entries,
            [
                entryps("d/a.o", Ignored(Expendable), File, expected_match),
                entryps("d/b.o", Ignored(Expendable), File, expected_match),
                entryps("d/d", Untracked, Directory, expected_match),
                entryps_dirstat("d/d/a.precious", Ignored(Precious), File, expected_match, Untracked),
            ],
            "'{equivalent_pathspec}' should yield the same entries - note how collapsed directories inherit the pathspec"
        );
    }

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                emit_untracked: CollapseDirectory,
                for_deletion: Some(Default::default()),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 3,
            returned_entries: entries.len(),
            seen_entries: 9,
        },
    );

    assert_eq!(
        entries,
        [
            entry("a.o", Ignored(Expendable), File),
            entry("d/a.o", Ignored(Expendable), File),
            entry("d/b.o", Ignored(Expendable), File),
            entry("d/d/a.precious", Ignored(Precious), File),
            entryps("d/d/new", Untracked, File, Always),
        ],
        "If collapses are for deletion, we don't treat precious files like expendable/ignored anymore so they show up individually \
        and prevent collapsing into a folder in the first place"
    );
}

#[test]
#[cfg_attr(
    not(target_vendor = "apple"),
    ignore = "Needs filesystem that folds unicode composition"
)]
fn decomposed_unicode_in_directory_is_returned_precomposed() -> crate::Result {
    let root = gix_testtools::tempfile::TempDir::new()?;

    let decomposed = "a\u{308}";
    let precomposed = "ä";
    std::fs::create_dir(root.path().join(decomposed))?;
    std::fs::write(root.path().join(decomposed).join(decomposed), [])?;

    let ((out, _root), entries) = collect(root.path(), None, |keep, ctx| {
        walk(
            root.path(),
            ctx,
            walk::Options {
                precompose_unicode: true,
                ..options()
            },
            keep,
        )
    });

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );
    assert_eq!(
        entries,
        [entry(format!("{precomposed}/{precomposed}").as_str(), Untracked, File)],
        "even root paths are returned precomposed then"
    );

    let troot = root.path().join(decomposed);
    let ((out, _root), entries) = collect(root.path(), Some(&troot), |keep, ctx| {
        walk(
            root.path(),
            ctx,
            walk::Options {
                precompose_unicode: false,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 1,
        },
        "note how it starts directly in the right repository"
    );
    assert_eq!(
        entries,
        [entryps(
            format!("{decomposed}/{decomposed}").as_str(),
            Untracked,
            File,
            Prefix
        )],
        "if disabled, it stays decomposed as provided"
    );
    Ok(())
}

#[test]
fn worktree_root_can_be_symlink() -> crate::Result {
    let root = fixture_in("many-symlinks", "symlink-to-breakout-symlink");
    let troot = root.join("file");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| walk(&root, ctx, options(), keep),
        None::<&str>,
        Default::default(),
    )?;
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );
    assert_eq!(
        entries,
        [entry("file", Untracked, File)],
        "it allows symlinks for the worktree itself"
    );
    Ok(())
}

#[test]
fn root_may_not_go_through_dot_git() -> crate::Result {
    let root = fixture("with-nested-dot-git");
    for (dir, expected_pathspec) in [("", Some(Verbatim)), ("subdir", None)] {
        let troot = root.join("dir").join(".git").join(dir);
        let ((out, _root), entries) = collect(&root, Some(&troot), |keep, ctx| {
            walk(&root, ctx, options_emit_all(), keep)
        });
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 0,
                returned_entries: entries.len(),
                seen_entries: 1,
            }
        );
        assert_eq!(
            entries,
            [{
                let mut e = entry("dir/.git", Pruned, Directory).with_property(DotGit);
                e.0.pathspec_match = expected_pathspec;
                e
            }],
            "{dir}: no traversal happened as root passes though .git"
        );
    }
    Ok(())
}

#[test]
fn root_at_submodule_repository_allows_walk() -> crate::Result {
    let root = fixture("repo-with-submodule");
    let troot = root.join("submodule");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &troot,
        None,
        Some(&troot),
        |keep, ctx| {
            walk(
                &troot,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    emit_untracked: Matching,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options::git_dir("../.git/modules/submodule"),
    )?;

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3,
        }
    );

    assert_eq!(
        entries,
        [entry("dir/file", Tracked, File), entry("untracked", Untracked, File)],
        "this is a special case to allow walking submodules specifically, like a normal repository"
    );
    Ok(())
}

#[test]
fn root_in_submodule_repository_allows_walk() -> crate::Result {
    let root = fixture("repo-with-submodule");
    let troot = root.join("submodule");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &troot,
        None,
        Some(&troot.join("dir")),
        |keep, ctx| {
            walk(
                &troot,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    emit_untracked: Matching,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options::git_dir("../.git/modules/submodule"),
    )?;

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );

    assert_eq!(
        entries,
        [entry("dir/file", Tracked, File)],
        "it's also working if the traversal root is inside the subdmodule"
    );
    Ok(())
}

#[test]
fn root_in_submodule_from_superproject_repository_allows_walk() -> crate::Result {
    let root = fixture("repo-with-submodule");
    let troot = root.join("submodule").join("dir");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| {
            walk(
                &troot,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    emit_untracked: Matching,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Default::default(),
    )?;

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );

    assert_eq!(
        entries,
        [entry("file", Untracked, File)],
        "there is no index that has 'file' in it (it's 'dir/file'), hence it's untracked.\
        But the traversal is possible, even though it might not make the most sense."
    );
    Ok(())
}

#[test]
fn root_enters_directory_with_dot_git_in_reconfigured_worktree_tracked() -> crate::Result {
    let root = fixture("nonstandard-worktree");
    let troot = root.join("dir-with-dot-git").join("inside");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options::git_dir("dir-with-dot-git/.git"),
    )?;

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );

    assert_eq!(
        entries,
        [entry("dir-with-dot-git/inside", Tracked, File)],
        "everything is tracked, so it won't try to detect git repositories anyway"
    );

    let troot = root.join("dir-with-dot-git").join("inside");
    let ((out, _root), entries) = try_collect_filtered_opts_collect(
        &root,
        Some(&troot),
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_tracked: false,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options::git_dir("dir-with-dot-git/.git"),
    )?;

    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: 0,
            seen_entries: 1,
        }
    );

    assert!(entries.is_empty());
    Ok(())
}

#[test]
fn root_enters_directory_with_dot_git_in_reconfigured_worktree_untracked() -> crate::Result {
    let root = fixture("nonstandard-worktree-untracked");
    let troot = root.join("dir-with-dot-git").join("inside");
    let (_out, entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| walk(&root, ctx, options(), keep),
        None::<&str>,
        Options::git_dir("dir-with-dot-git/.git"),
    )?;
    assert_eq!(
        entries,
        [entry("dir-with-dot-git/inside", Untracked, File)],
        "it can enter a dir and treat it as normal even if own .git is inside,\
         which otherwise would be a repository"
    );
    Ok(())
}

#[test]
fn root_may_not_go_through_nested_repository_unless_enabled() -> crate::Result {
    let root = fixture("nested-repository");
    let troot = root.join("nested").join("file");
    let (_out, entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    recurse_repositories: true,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Default::default(),
    )?;
    assert_eq!(
        entries,
        [entry("nested/file", Untracked, File)],
        "it happily enters the repository and lists the file"
    );

    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| walk(&root, ctx, options(), keep),
        None::<&str>,
        Default::default(),
    )?;
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );
    assert_eq!(
        entries,
        [entry("nested", Untracked, Repository)],
        "thus it ends in the directory that is a repository"
    );
    Ok(())
}

#[test]
fn root_may_not_go_through_submodule() -> crate::Result {
    let root = fixture("with-submodule");
    let troot = root.join("submodule").join("dir").join("file");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| walk(&root, ctx, options_emit_all(), keep),
        None::<&str>,
        Default::default(),
    )?;
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        },
    );
    assert_eq!(
        entries,
        [entry("submodule", Tracked, Repository)],
        "it refuses to start traversal in a submodule, thus it ends in the directory that is the submodule, \
        if the root is another repository"
    );
    Ok(())
}

#[test]
fn walk_with_submodule() -> crate::Result {
    let root = fixture("with-submodule");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 4,
        }
    );
    assert_eq!(
        entries,
        [
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry(".gitmodules", Tracked, File),
            entry("dir/file", Tracked, File),
            entry("submodule", Tracked, Repository)
        ],
        "thus it ends in the directory that is the submodule"
    );
    Ok(())
}

#[test]
fn root_that_is_tracked_file_is_returned() -> crate::Result {
    let root = fixture("dir-with-tracked-file");
    let troot = &root.join("dir").join("file");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(troot),
        |keep, ctx| walk(&root, ctx, options_emit_all(), keep),
        None::<&str>,
        Default::default(),
    )?;
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );

    assert_eq!(
        entries,
        [entry("dir/file", Tracked, File)],
        "a tracked file as root just returns that file (even though no iteration is possible)"
    );
    Ok(())
}

#[test]
fn root_that_is_untracked_file_is_returned() -> crate::Result {
    let root = fixture("dir-with-file");
    let troot = root.join("dir").join("file");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| walk(&root, ctx, options(), keep),
        None::<&str>,
        Default::default(),
    )?;
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );

    assert_eq!(
        entries,
        [entry("dir/file", Untracked, File)],
        "an untracked file as root just returns that file (even though no iteration is possible)"
    );
    Ok(())
}

#[test]
fn top_level_root_that_is_a_file() {
    let root = fixture("just-a-file");
    let err = try_collect(&root, None, |keep, ctx| walk(&root, ctx, options(), keep)).unwrap_err();
    assert!(matches!(err, walk::Error::WorktreeRootIsFile { .. }));
}

#[test]
fn root_can_be_pruned_early_with_pathspec() -> crate::Result {
    let root = fixture("dir-with-file");
    let troot = root.join("dir");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| walk(&root, ctx, options_emit_all(), keep),
        Some("no-match/"),
        Default::default(),
    )?;
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );

    assert_eq!(
        entries,
        [entry_nomatch("dir", Pruned, Directory)],
        "the pathspec didn't match the root, early abort"
    );
    Ok(())
}

#[test]
fn submodules() -> crate::Result {
    let root = fixture("multiple-submodules");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );
    let expected_content = [
        entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
        entry(".gitmodules", Tracked, File).with_index_kind(File),
        entry("a/b", Tracked, Repository).with_index_kind(Repository),
        entry("empty", Tracked, File).with_index_kind(File),
        entry("submodule", Tracked, Repository).with_index_kind(Repository),
    ];
    assert_eq!(entries, expected_content, "submodules are detected as repositories");

    let ((out1, _root), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| walk(&root, ctx, options_emit_all(), keep),
        None::<&str>,
        Options {
            fresh_index: false,
            ..Default::default()
        },
    )?;
    assert_eq!(out1, out, "the output matches precisely");
    assert_eq!(
        entries, expected_content,
        "this is also the case if the index isn't considered fresh"
    );

    let ((out2, _root), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    ignore_case: true,
                    ..options_emit_all()
                },
                keep,
            )
        },
        None::<&str>,
        Options {
            fresh_index: false,
            ..Default::default()
        },
    )?;
    assert_eq!(out2, out, "the output matches precisely, even with ignore-case");
    assert_eq!(
        entries, expected_content,
        "ignore case doesn't change anything (even though our search is quite different)"
    );
    Ok(())
}

#[test]
fn cancel_with_collection_does_not_fail() -> crate::Result {
    struct CancelDelegate {
        emits_left_until_cancel: usize,
    }

    impl gix_dir::walk::Delegate for CancelDelegate {
        fn emit(&mut self, _entry: EntryRef<'_>, _collapsed_directory_status: Option<entry::Status>) -> walk::Action {
            if self.emits_left_until_cancel == 0 {
                std::ops::ControlFlow::Break(())
            } else {
                self.emits_left_until_cancel -= 1;
                std::ops::ControlFlow::Continue(())
            }
        }
    }

    for (idx, fixture_name) in [
        "nonstandard-worktree",
        "nonstandard-worktree-untracked",
        "dir-with-file",
        "expendable-and-precious",
        "subdir-untracked-and-ignored",
        "empty-and-untracked-dir",
        "complex-empty",
        "type-mismatch-icase-clash-file-is-dir",
    ]
    .into_iter()
    .enumerate()
    {
        let root = fixture(fixture_name);
        let mut dlg = CancelDelegate {
            emits_left_until_cancel: idx,
        };
        let _out = try_collect_filtered_opts(
            &root,
            None,
            None,
            None,
            |keep, ctx| {
                walk(
                    &root,
                    ctx,
                    walk::Options {
                        emit_untracked: CollapseDirectory,
                        emit_ignored: Some(CollapseDirectory),
                        emit_empty_directories: true,
                        emit_tracked: true,
                        for_deletion: Some(Default::default()),
                        emit_pruned: true,
                        ..options()
                    },
                    keep,
                )
            },
            None::<&str>,
            &mut dlg,
            Options::default(),
        )?;
        // Note that this also doesn't trigger an error - the caller has to deal with that.
    }
    Ok(())
}

#[test]
fn file_root_is_shown_if_pathspec_matches_exactly() -> crate::Result {
    let root = fixture("dir-with-file");
    let troot = root.join("dir").join("file");
    let ((out, _root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| walk(&root, ctx, options(), keep),
        Some("*dir/*"),
        Default::default(),
    )?;
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        },
    );

    assert_eq!(
        entries,
        [entryps("dir/file", Untracked, File, WildcardMatch)],
        "the pathspec matched the root precisely"
    );
    Ok(())
}

#[test]
fn root_that_is_tracked_and_ignored_is_considered_tracked() -> crate::Result {
    let root = fixture("tracked-is-ignored");
    let walk_root = "dir/file";
    let troot = root.join(walk_root);
    let ((out, actual_root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| walk(&root, ctx, options_emit_all(), keep),
        None::<&str>,
        Default::default(),
    )?;
    assert_eq!(actual_root, troot, "it uses the root we provide");
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );

    assert_eq!(
        entries,
        [entry(walk_root, Tracked, File)],
        "tracking is checked first, so we can safe exclude checks for most entries"
    );
    Ok(())
}

#[test]
fn root_with_dir_that_is_tracked_and_ignored() -> crate::Result {
    let root = fixture("tracked-is-ignored");
    for emission in [Matching, CollapseDirectory] {
        let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_ignored: Some(emission),
                    emit_tracked: true,
                    emit_untracked: emission,
                    ..options_emit_all()
                },
                keep,
            )
        });
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 2,
                returned_entries: entries.len(),
                seen_entries: 3,
            }
        );

        assert_eq!(
            entries,
            [
                entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
                entry(".gitignore", Tracked, File),
                entry("dir/file", Tracked, File)
            ],
            "'tracked' is the overriding property here, so we even enter ignored directories if they have tracked contents,\
            otherwise we might permanently miss new untracked files in there. Emission mode has no effect"
        );
    }

    Ok(())
}

#[test]
fn empty_and_nested_untracked() -> crate::Result {
    let root = fixture("empty-and-untracked-dir");
    for for_deletion in [None, Some(Default::default())] {
        let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_untracked: Matching,
                    for_deletion,
                    emit_empty_directories: true,
                    ..options()
                },
                keep,
            )
        });
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 3,
                returned_entries: entries.len(),
                seen_entries: 2,
            }
        );

        assert_eq!(
            entries,
            [
                entry("empty", Untracked, Directory).with_property(EmptyDirectory),
                entry("untracked/file", Untracked, File)
            ],
            "we find all untracked entries, no matter the deletion mode"
        );
        let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_untracked: CollapseDirectory,
                    emit_empty_directories: true,
                    for_deletion,
                    ..options()
                },
                keep,
            )
        });
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 3,
                returned_entries: entries.len(),
                seen_entries: 3,
            }
        );

        assert_eq!(
            entries,
            [
                entry("empty", Untracked, Directory).with_property(EmptyDirectory),
                entry("untracked", Untracked, Directory)
            ],
            "we find all untracked directories, no matter the deletion mode"
        );
    }
    Ok(())
}

#[test]
fn root_that_is_ignored_is_listed_for_files_and_directories() -> crate::Result {
    let root = fixture("ignored-dir");
    for walk_root in ["dir", "dir/file"] {
        let troot = root.join(walk_root);
        for emission in [Matching, CollapseDirectory] {
            let ((out, actual_root), entries) = try_collect_filtered_opts_collect_with_root(
                &root,
                None,
                Some(&troot),
                |keep, ctx| {
                    walk(
                        &root,
                        ctx,
                        walk::Options {
                            emit_ignored: Some(emission),
                            ..options()
                        },
                        keep,
                    )
                },
                None::<&str>,
                Default::default(),
            )?;
            assert_eq!(actual_root, troot);
            assert_eq!(
                out,
                walk::Outcome {
                    read_dir_calls: 0,
                    returned_entries: entries.len(),
                    seen_entries: 1,
                }
            );

            assert_eq!(
                entries,
                [entry("dir", Ignored(Expendable), Directory)],
                "excluded directories or files that walkdir are listed without further recursion"
            );
        }
    }
    Ok(())
}

#[test]
fn nested_bare_repos_in_ignored_directories() -> crate::Result {
    let root = fixture("ignored-dir-with-nested-bare-repository");
    let (_out, entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                for_deletion: Some(Default::default()),
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });
    // NOTE: do not use `_out` as `.git` directory contents can change, it's controlled by Git, causing flakiness.

    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("bare", Untracked, Directory),
            entry("dir", Ignored(Expendable), Directory),
        ],
        "by default, only the directory is listed and recursion is stopped there, as it matches the ignore directives. \
        Note the nested bare repository isn't seen, while the bare repository is just collapsed, and not detected as repository"
    );

    let (_out, entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                for_deletion: Some(ForDeletionMode::FindNonBareRepositoriesInIgnoredDirectories),
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });

    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("bare", Untracked, Directory),
            entry("dir", Ignored(Expendable), Directory),
        ],
        "When looking for non-bare repositories, we won't find bare ones, they just disappear as ignored collapsed directories"
    );

    let (_out, entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                for_deletion: Some(ForDeletionMode::FindRepositoriesInIgnoredDirectories),
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });

    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("bare", Untracked, Directory),
            entry("dir/file", Ignored(Expendable), File),
            entry("dir/subdir/nested-bare", Ignored(Expendable), Repository),
        ],
        "Only in this mode we are able to find them, but it's expensive"
    );
    Ok(())
}

#[test]
fn nested_repos_in_untracked_directories() -> crate::Result {
    let root = fixture("untracked-hidden-bare");
    let (_out, entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });
    // NOTE: do not use `_out` as `.git` directory contents can change, it's controlled by Git, causing flakiness.

    assert_eq!(
        entries,
        [entry("subdir", Untracked, Directory)],
        "by default, the subdir is collapsed and we don't see the contained repository as it doesn't get classified"
    );

    let (_out, entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_untracked: CollapseDirectory,
                classify_untracked_bare_repositories: true,
                ..options()
            },
            keep,
        )
    });

    assert_eq!(
        entries,
        [
            entry("subdir/file", Untracked, File),
            entry("subdir/hidden-bare", Untracked, Repository)
        ],
        "With this flag we are able to find the bare repository"
    );

    Ok(())
}

#[test]
fn nested_repos_in_ignored_directories() -> crate::Result {
    let root = fixture("ignored-dir-with-nested-repository");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(Matching),
                for_deletion: Some(Default::default()),
                emit_untracked: CollapseDirectory,
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 4,
        }
    );

    assert_eq!(
        entries,
        [
            entry("dir", Ignored(Expendable), Directory),
            entry("objs/a.o", Ignored(Expendable), File),
        ],
        "by default, only the directory is listed and recursion is stopped there, as it matches the ignore directives."
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(Matching),
                emit_untracked: CollapseDirectory,
                for_deletion: Some(ForDeletionMode::FindNonBareRepositoriesInIgnoredDirectories),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 4,
            returned_entries: entries.len(),
            seen_entries: 6,
        }
    );

    assert_eq!(
        entries,
        [
            entry("dir/file", Ignored(Expendable), File),
            entry("dir/subdir/a", Ignored(Expendable), File),
            entry("dir/subdir/nested", Ignored(Expendable), Repository),
            entry("objs/a.o", Ignored(Expendable), File)
        ],
        "in this mode, we will list repositories nested in ignored directories separately"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                emit_untracked: CollapseDirectory,
                for_deletion: Some(ForDeletionMode::FindNonBareRepositoriesInIgnoredDirectories),
                ..options()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 4,
            returned_entries: entries.len(),
            seen_entries: 7,
        }
    );

    assert_eq!(
        entries,
        [
            entry("dir/file", Ignored(Expendable), File),
            entry("dir/subdir/a", Ignored(Expendable), File),
            entry("dir/subdir/nested", Ignored(Expendable), Repository),
            entry("objs", Ignored(Expendable), Directory),
        ],
        "finally, we can't fold if there are any nested repositories. Note how the folding isn't affected in unrelated directories"
    );
    Ok(())
}

#[test]
#[cfg_attr(
    not(target_vendor = "apple"),
    ignore = "Needs filesystem that folds unicode composition"
)]
fn decomposed_unicode_in_root_is_returned_precomposed() -> crate::Result {
    let root = gix_testtools::tempfile::TempDir::new()?;

    let decomposed = "a\u{308}";
    let precomposed = "ä";
    std::fs::write(root.path().join(decomposed), [])?;

    let troot = root.path().join(decomposed);
    let ((out, actual_root), entries) = try_collect_filtered_opts_collect_with_root(
        root.path(),
        None,
        Some(&troot),
        |keep, ctx| {
            walk(
                root.path(),
                ctx,
                walk::Options {
                    precompose_unicode: true,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Default::default(),
    )?;

    assert_eq!(actual_root, troot);
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1,
        }
    );
    assert_eq!(
        entries,
        [entry(precomposed, Untracked, File)],
        "even root paths are returned precomposed then"
    );

    let troot = root.path().join(decomposed);
    let ((_out, actual_root), entries) = try_collect_filtered_opts_collect_with_root(
        root.path(),
        None,
        Some(&troot),
        |keep, ctx| {
            walk(
                root.path(),
                ctx,
                walk::Options {
                    precompose_unicode: false,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Default::default(),
    )?;
    assert_eq!(actual_root, troot);
    assert_eq!(
        entries,
        [entry(decomposed, Untracked, File)],
        "if disabled, it stays decomposed as provided"
    );
    Ok(())
}

#[test]
fn untracked_and_ignored_collapse_mix() {
    let root = fixture("untracked-and-ignored-for-collapse");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(CollapseDirectory),
                emit_untracked: Matching,
                ..options_emit_all()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 4,
            returned_entries: entries.len(),
            seen_entries: 7,
        }
    );
    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("ignored", Ignored(Expendable), Directory),
            entry("ignored-inside", Ignored(Expendable), Directory),
            entry("mixed/c", Untracked, File),
            entry("mixed/c.o", Ignored(Expendable), File),
            entry("untracked/a", Untracked, File),
        ],
        "ignored collapses separately from untracked"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(Matching),
                emit_untracked: CollapseDirectory,
                emit_collapsed: Some(OnStatusMismatch),
                ..options_emit_all()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 4,
            returned_entries: entries.len(),
            seen_entries: 8,
        }
    );
    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("ignored", Ignored(Expendable), Directory),
            entry("ignored-inside/d.o", Ignored(Expendable), File),
            entry("mixed", Untracked, Directory),
            entry_dirstat("mixed/c.o", Ignored(Expendable), File, Untracked),
            entry("untracked", Untracked, Directory),
        ],
        "untracked collapses separately from ignored, but note that matching directories are still emitted, i.e. ignored/"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                emit_ignored: Some(Matching),
                emit_untracked: CollapseDirectory,
                emit_collapsed: Some(All),
                ..options_emit_all()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 4,
            returned_entries: entries.len(),
            seen_entries: 8,
        }
    );
    assert_eq!(
        entries,
        [
            entry(".gitignore", Untracked, File),
            entry("ignored", Ignored(Expendable), Directory),
            entry("ignored-inside/d.o", Ignored(Expendable), File),
            entry("mixed", Untracked, Directory),
            entry_dirstat("mixed/c", Untracked, File, Untracked),
            entry_dirstat("mixed/c.o", Ignored(Expendable), File, Untracked),
            entry("untracked", Untracked, Directory),
            entry_dirstat("untracked/a", Untracked, File, Untracked),
        ],
        "we can also emit all collapsed entries"
    );
}

#[test]
fn root_cannot_pass_through_case_altered_capital_dot_git_if_case_insensitive() -> crate::Result {
    let root = fixture("with-nested-capitalized-dot-git");
    for (dir, expected_pathspec) in [("", Some(Verbatim)), ("subdir", None)] {
        let troot = root.join("dir").join(".GIT").join(dir);
        let ((out, _root), entries) = collect(&root, Some(&troot), |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    ignore_case: true,
                    ..options_emit_all()
                },
                keep,
            )
        });
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 0,
                returned_entries: entries.len(),
                seen_entries: 1,
            }
        );
        assert_eq!(
            entries,
            [{
                let mut e = entry("dir/.GIT", Pruned, Directory).with_property(DotGit);
                e.0.pathspec_match = expected_pathspec;
                e
            }],
            "{dir}: no traversal happened as root passes though .git, it compares in a case-insensitive fashion"
        );
    }

    let troot = root.join("dir").join(".GIT").join("config");
    let ((_out, actual_root), entries) = try_collect_filtered_opts_collect_with_root(
        &root,
        None,
        Some(&troot),
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    ignore_case: false,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Default::default(),
    )?;
    assert_eq!(actual_root, troot);
    assert_eq!(
        entries,
        [entry("dir/.GIT/config", Untracked, File)],
        "it passes right through what now seems like any other directory"
    );
    Ok(())
}

#[test]
fn partial_checkout_cone_and_non_one() -> crate::Result {
    for fixture_name in ["partial-checkout-cone-mode", "partial-checkout-non-cone"] {
        let root = fixture(fixture_name);
        let not_in_cone_but_created_locally_by_hand = "d/file-created-manually";
        let troot = root.join(not_in_cone_but_created_locally_by_hand);
        let ((out, actual_root), entries) = try_collect_filtered_opts_collect_with_root(
            &root,
            None,
            Some(&troot),
            |keep, ctx| walk(&root, ctx, options_emit_all(), keep),
            None::<&str>,
            Default::default(),
        )?;
        assert_eq!(actual_root, troot);
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 0,
                returned_entries: entries.len(),
                seen_entries: 1,
            }
        );
        assert_eq!(
            entries,
            [entry("d", Pruned, Directory)
                .with_index_kind(Directory)
                .with_property(TrackedExcluded)],
            "{fixture_name}: we avoid entering excluded sparse-checkout directories even if they are present on disk,\
            no matter with cone or without."
        );
    }
    Ok(())
}

#[test]
fn type_mismatch() {
    let root = fixture("type-mismatch");
    let ((out, _root), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    emit_untracked: Matching,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options {
            fresh_index: false,
            ..Default::default()
        },
    )
    .expect("success");
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3,
        }
    );

    assert_eq!(
        entries,
        [
            entry("dir-is-file", Untracked, File).with_index_kind(Directory),
            entry("file-is-dir/b", Untracked, File)
        ],
        "as long as the index doesn't claim otherwise (i.e. uptodate) it will handle these changes correctly. \
         Also, `dir-is-file` is tracked as directory, but not as file.\
         The typechange is visible only when there is an entry in the index, of course"
    );

    let ((out, _root), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    emit_untracked: CollapseDirectory,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options {
            fresh_index: false,
            ..Default::default()
        },
    )
    .expect("success");
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3 + 1,
        }
    );

    assert_eq!(
        entries,
        [
            entry("dir-is-file", Untracked, File).with_index_kind(Directory),
            entry("file-is-dir", Untracked, Directory).with_index_kind(File)
        ],
        "collapsing works as well, and we allow to see the typechange"
    );
}

#[test]
fn type_mismatch_ignore_case() {
    let root = fixture("type-mismatch-icase");
    let ((out, _root), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    emit_untracked: Matching,
                    ignore_case: true,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options {
            fresh_index: false,
            ..Default::default()
        },
    )
    .expect("success");
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3,
        }
    );
    assert_eq!(
        entries,
        [
            entry("Dir-is-File", Untracked, File).with_index_kind(Directory),
            entry("File-is-Dir/b", Untracked, File)
        ],
        "this is the same as in the non-icase version, which means that icase lookup works"
    );

    let ((out, _root), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    emit_untracked: CollapseDirectory,
                    ignore_case: true,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options {
            fresh_index: false,
            ..Default::default()
        },
    )
    .expect("success");
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3 + 1,
        }
    );
    assert_eq!(
        entries,
        [
            entry("Dir-is-File", Untracked, File).with_index_kind(Directory),
            entry("File-is-Dir", Untracked, Directory).with_index_kind(File)
        ],
        "this is the same as in the non-icase version, which means that icase lookup works"
    );
}

#[test]
fn type_mismatch_ignore_case_clash_dir_is_file() {
    let root = fixture("type-mismatch-icase-clash-dir-is-file");
    let ((out, _root), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    emit_untracked: Matching,
                    ignore_case: true,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options {
            fresh_index: false,
            ..Default::default()
        },
    )
    .expect("success");
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 2,
        }
    );
    assert_eq!(
        entries,
        [entry("d", Tracked, File)],
        "file `d` exists on disk and it is found as well. This is just because we prefer finding files over dirs, coincidence"
    );
}

#[test]
fn type_mismatch_ignore_case_clash_file_is_dir() {
    let root = fixture("type-mismatch-icase-clash-file-is-dir");
    let ((out, _root), entries) = try_collect_filtered_opts_collect(
        &root,
        None,
        |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    emit_tracked: true,
                    emit_untracked: CollapseDirectory,
                    ignore_case: true,
                    ..options()
                },
                keep,
            )
        },
        None::<&str>,
        Options {
            fresh_index: false,
            ..Default::default()
        },
    )
    .expect("success");
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 2,
        }
    );
    assert_eq!(
        entries,
        [entry("D/a", Tracked, File)],
        "`D` exists on disk as directory, and we manage to to find it in in the index, hence no collapsing happens.\
         If there was no special handling for this, it would have found the file (`d` in the index, icase), which would have been wrong."
    );
}

#[test]
fn top_level_slash_with_negations() -> crate::Result {
    for repo_name in ["slash-in-root-and-negated", "star-in-root-and-negated"] {
        let root = fixture(repo_name);
        let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 2,
                returned_entries: entries.len(),
                seen_entries: 5,
            }
        );
        assert_eq!(
            entries,
            &[
                entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
                entry(".github/workflow.yml", Tracked, File),
                entry(".gitignore", Tracked, File),
                entry("file", Untracked, File),
                entry("readme.md", Tracked, File),
            ],
            "the top-level is never considered ignored"
        );

        let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(ForDeletionMode::FindRepositoriesInIgnoredDirectories),
                    emit_tracked: false,
                    ..options_emit_all()
                },
                keep,
            )
        });
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 2,
                returned_entries: entries.len(),
                seen_entries: 5,
            }
        );
        assert_eq!(
            entries,
            &[
                entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
                entry("file", Untracked, File)
            ],
            "And the negated file is correctly detected as untracked"
        );
    }
    Ok(())
}

#[test]
fn subdir_slash_with_negations() -> crate::Result {
    for repo_name in ["slash-in-subdir-and-negated", "star-in-subdir-and-negated"] {
        let root = fixture(repo_name);
        let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 3,
                returned_entries: entries.len(),
                seen_entries: 5,
            }
        );
        assert_eq!(
            entries,
            &[
                entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
                entry("sub/.github/workflow.yml", Tracked, File),
                entry("sub/.gitignore", Tracked, File),
                entry("sub/file", Untracked, File),
                entry("sub/readme.md", Tracked, File),
            ],
            "subdirectory matches work as expected, also with a `/` which has no bearing."
        );

        let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
            walk(
                &root,
                ctx,
                walk::Options {
                    for_deletion: Some(ForDeletionMode::FindRepositoriesInIgnoredDirectories),
                    emit_tracked: false,
                    ..options_emit_all()
                },
                keep,
            )
        });
        assert_eq!(
            out,
            walk::Outcome {
                read_dir_calls: 3,
                returned_entries: entries.len(),
                seen_entries: 5,
            }
        );
        assert_eq!(
            entries,
            &[
                entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
                entry("sub/file", Untracked, File)
            ],
            "This is expected, and the `.git` top-level is pruned."
        );
    }
    Ok(())
}

#[test]
fn one_ignored_submodule() -> crate::Result {
    let root = fixture("one-ignored-submodule");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 5,
        }
    );
    assert_eq!(
        entries,
        &[
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry(".gitignore", Untracked, File),
            entry(".gitmodules", Tracked, File),
            entry("empty", Tracked, File),
            entry("submodule", Tracked, Repository),
        ],
        "when traversing the worktree root, this is correct, the submodule doesn't count as ignored"
    );

    let troot = root.join("submodule");
    let ((out, _root), entries) = collect(&root, Some(&troot), |keep, ctx| {
        walk(&root, ctx, options_emit_all(), keep)
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 0,
            returned_entries: entries.len(),
            seen_entries: 1
        }
    );
    assert_eq!(
        entries,
        &[entryps("submodule", Tracked, Repository, Verbatim)],
        "The submodule is simply tracked, it doesn't count as ignored"
    );
    Ok(())
}

#[test]
fn ignored_sub_repo() -> crate::Result {
    let root = fixture("with-sub-repo");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 1,
            returned_entries: entries.len(),
            seen_entries: 3,
        }
    );
    assert_eq!(
        entries,
        &[
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry(".gitignore", Tracked, File),
            entry("sub-repo", Ignored(Expendable), Directory),
        ],
        "without intent to delete, this looks like just like an untracked directory"
    );

    for ignored_emission_mode in [Matching, CollapseDirectory] {
        for untracked_emission_mode in [Matching, CollapseDirectory] {
            let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
                walk(
                    &root,
                    ctx,
                    walk::Options {
                        for_deletion: Some(ForDeletionMode::IgnoredDirectoriesCanHideNestedRepositories),
                        emit_tracked: false,
                        emit_ignored: Some(ignored_emission_mode),
                        emit_untracked: untracked_emission_mode,
                        ..options_emit_all()
                    },
                    keep,
                )
            });
            assert_eq!(
                out,
                walk::Outcome {
                    read_dir_calls: 1,
                    returned_entries: entries.len(),
                    seen_entries: 3,
                }
            );
            assert_eq!(
                entries,
                &[
                    entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
                    entry("sub-repo", Ignored(Expendable), Repository),
                ],
                "Even when ignored directories can hide repositories, we are able to detect top-level repositories"
            );
        }
    }
    Ok(())
}

#[test]
fn in_repo_worktree() -> crate::Result {
    let root = fixture("in-repo-worktree");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 4,
        }
    );
    assert_eq!(
        entries,
        &[
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry("dir/file", Tracked, File),
            entry("dir/worktree", Untracked, Repository),
            entry("worktree", Untracked, Repository),
        ],
        "without passing worktree information, they count as untracked repositories, making them vulnerable"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                worktree_relative_worktree_dirs: Some(&BTreeSet::from(["worktree".into(), "dir/worktree".into()])),
                ..options_emit_all()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 4,
        }
    );
    assert_eq!(
        entries,
        &[
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry("dir/file", Tracked, File),
            entry("dir/worktree", Tracked, Repository).no_index_kind(),
            entry("worktree", Tracked, Repository).no_index_kind(),
        ],
        "But when worktree information is passed, it is identified as tracked to look similarly to a submodule.\
         What gives it away is that the index-kind is None, which is unusual for a tracked file."
    );
    Ok(())
}

#[test]
fn in_repo_hidden_worktree() -> crate::Result {
    let root = fixture("in-repo-hidden-worktree");
    let ((out, _root), entries) = collect(&root, None, |keep, ctx| walk(&root, ctx, options_emit_all(), keep));
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 4,
        }
    );
    assert_eq!(
        entries,
        &[
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry(".gitignore", Untracked, File),
            entry("dir/file", Tracked, File),
            entry("hidden", Ignored(Expendable), Directory),
        ],
        "if worktree information isn't provided, they would not be discovered in hidden directories"
    );

    let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
        walk(
            &root,
            ctx,
            walk::Options {
                for_deletion: None,
                worktree_relative_worktree_dirs: Some(&BTreeSet::from(["hidden/subdir/worktree".into()])),
                ..options_emit_all()
            },
            keep,
        )
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 4,
        }
    );
    assert_eq!(
        entries,
        &[
            entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
            entry(".gitignore", Untracked, File),
            entry("dir/file", Tracked, File),
            entry("hidden", Ignored(Expendable), Directory),
        ],
        "Without the intend to delete, the worktree remains hidden, which is what we want to see in a `status` for example"
    );

    for ignored_emission_mode in [Matching, CollapseDirectory] {
        for deletion_mode in [
            ForDeletionMode::IgnoredDirectoriesCanHideNestedRepositories,
            ForDeletionMode::FindRepositoriesInIgnoredDirectories,
            ForDeletionMode::FindNonBareRepositoriesInIgnoredDirectories,
        ] {
            let ((out, _root), entries) = collect(&root, None, |keep, ctx| {
                walk(
                    &root,
                    ctx,
                    walk::Options {
                        emit_ignored: Some(ignored_emission_mode),
                        for_deletion: Some(deletion_mode),
                        worktree_relative_worktree_dirs: Some(&BTreeSet::from(["hidden/subdir/worktree".into()])),
                        ..options_emit_all()
                    },
                    keep,
                )
            });
            assert_eq!(
                out,
                walk::Outcome {
                    read_dir_calls: 4,
                    returned_entries: entries.len(),
                    seen_entries: 5,
                }
            );
            assert_eq!(
                entries,
                &[
                    entry_nokind(".git", Pruned).with_property(DotGit).with_match(Always),
                    entry(".gitignore", Untracked, File),
                    entry("dir/file", Tracked, File),
                    entry("hidden/file", Ignored(Expendable), File),
                    entry("hidden/subdir/worktree", Tracked, Repository).no_index_kind(),
                ],
                "Worktrees within hidden directories are also detected and protected by counting them as tracked (like submodules)"
            );
        }
    }
    Ok(())
}
