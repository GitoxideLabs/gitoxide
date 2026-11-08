use std::{io, path::Path};

use gix_dir::{EntryRef, entry, walk};
use gix_testtools::TestResult;

use crate::walk_utils::{entry, options, try_collect_filtered_opts};

const DELETION_MODES: [Option<walk::ForDeletionMode>; 4] = [
    None,
    Some(walk::ForDeletionMode::IgnoredDirectoriesCanHideNestedRepositories),
    Some(walk::ForDeletionMode::FindNonBareRepositoriesInIgnoredDirectories),
    Some(walk::ForDeletionMode::FindRepositoriesInIgnoredDirectories),
];

#[test]
fn directories_removed_or_replaced_before_reading_follow_deletion_policy() -> TestResult {
    for replace_with_file in [false, true] {
        for emit_untracked in [walk::EmissionMode::Matching, walk::EmissionMode::CollapseDirectory] {
            for for_deletion in DELETION_MODES {
                let result = walk_with_directory_change(
                    walk::Options {
                        emit_untracked,
                        for_deletion,
                        emit_empty_directories: true,
                        ..options()
                    },
                    |path| {
                        std::fs::remove_dir_all(path)?;
                        if replace_with_file {
                            std::fs::write(path, "replacement")?;
                        }
                        Ok(())
                    },
                );
                if for_deletion.is_some() {
                    let err = result
                        .err()
                        .expect("deletion walks reject missing or replaced directories");
                    assert!(
                        format!("{err:?}").contains("Failed to read the directory"),
                        "the failed directory read remains visible to deletion callers: {err:?}"
                    );
                    continue;
                }
                let (out, collected) = result?;
                assert_eq!(
                    collected.into_entries_by_path(),
                    [entry("parent/keep", entry::Status::Untracked, entry::Kind::File)],
                    "stale directories are skipped and cannot make their parent a deletion candidate"
                );
                assert_eq!(
                    out.read_dir_calls, 3,
                    "the vanished directory is attempted once without retries"
                );
                assert_eq!(
                    out.read_dir_errors, 1,
                    "the skipped path makes the result observably incomplete"
                );
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn unreadable_directories_are_skipped_only_for_non_deletion_walks() -> TestResult {
    for emit_untracked in [walk::EmissionMode::Matching, walk::EmissionMode::CollapseDirectory] {
        for for_deletion in DELETION_MODES {
            let result = walk_with_directory_change(
                walk::Options {
                    emit_untracked,
                    for_deletion,
                    emit_empty_directories: true,
                    ..options()
                },
                |path| {
                    std::fs::remove_dir_all(path)?;
                    std::os::unix::fs::symlink("gone", path)
                },
            );
            if for_deletion.is_some() {
                let err = result.err().expect("a symlink loop must fail a deletion walk");
                assert!(
                    format!("{err:?}").contains("Failed to read the directory"),
                    "the error identifies the failed directory operation: {err:?}"
                );
                continue;
            }
            let (out, collected) = result?;
            assert_eq!(
                collected.into_entries_by_path(),
                [entry("parent/keep", entry::Status::Untracked, entry::Kind::File)],
                "a non-deletion walk retains readable siblings without collapsing unreadable contents"
            );
            assert_eq!(out.read_dir_calls, 3, "an unreadable directory is attempted once");
            assert_eq!(
                out.read_dir_errors, 1,
                "the unreadable subtree is reported as incomplete"
            );
        }
    }
    Ok(())
}

fn walk_with_directory_change(
    options: walk::Options<'_>,
    change: impl FnMut(&Path) -> io::Result<()>,
) -> TestResult<(walk::Outcome, walk::delegate::Collect)> {
    struct ChangeDirectory<'a, F> {
        root: &'a Path,
        change: F,
        collected: walk::delegate::Collect,
        changed: bool,
    }

    impl<F: FnMut(&Path) -> io::Result<()>> walk::Delegate for ChangeDirectory<'_, F> {
        fn emit(&mut self, entry: EntryRef<'_>, dir_status: Option<entry::Status>) -> walk::Action {
            self.collected.emit(entry, dir_status)
        }

        fn can_recurse(
            &mut self,
            entry: EntryRef<'_>,
            for_deletion: Option<walk::ForDeletionMode>,
            worktree_root_is_repository: bool,
        ) -> bool {
            if *entry.rela_path == "parent/gone" {
                (self.change)(&self.root.join("parent/gone")).expect("the observed directory can be changed");
                self.changed = true;
            }
            entry.status.can_recurse(
                entry.disk_kind,
                entry.pathspec_match,
                for_deletion,
                worktree_root_is_repository,
            )
        }
    }

    let fixture = gix_testtools::scripted_fixture_writable("concurrent-change.sh")?;
    let root = fixture.path();
    let mut delegate = ChangeDirectory {
        root,
        change,
        collected: walk::delegate::Collect::default(),
        changed: false,
    };
    let result = try_collect_filtered_opts(
        root,
        None,
        None,
        None,
        |delegate, ctx| gix_dir::walk(root, ctx, options, delegate),
        None::<&str>,
        &mut delegate,
        Default::default(),
    );
    assert!(delegate.changed, "the directory changed immediately before its read");
    Ok((result?.0, delegate.collected))
}
