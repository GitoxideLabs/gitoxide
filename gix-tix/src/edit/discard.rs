use std::process::Command;

use anyhow::{Context, Result};
use gix::bstr::{BStr, ByteSlice};

use crate::{ChangeGroup, ChangeKind, PathChange};

pub(crate) fn perform(repo: &gix::Repository, change: &PathChange) -> Result<()> {
    let workdir = repo.workdir().context("discard requires a worktree")?;
    let current = crate::load_worktree_changes_without_lines(repo)?;
    anyhow::ensure!(
        change.group != ChangeGroup::Tree
            && current.paths.iter().any(|path| {
                path.path == change.path
                    && path.source == change.source
                    && path.group == change.group
                    && path.kind == change.kind
            }),
        "the selected worktree change is no longer available; refresh and select it again"
    );
    let source = change
        .source
        .as_ref()
        .filter(|_| change.kind == ChangeKind::Renamed)
        .map(|path| path.as_bstr());
    let paths: Vec<&BStr> = source.into_iter().chain([change.path.as_bstr()]).collect();
    let index = repo
        .index_or_empty()
        .context("could not read the index before discard")?;
    anyhow::ensure!(
        !index
            .entries()
            .iter()
            .any(|entry| entry.mode.is_submodule() && paths.contains(&entry.path(&index))),
        "discarding submodule changes is not supported"
    );
    let run = |args: &[&str], paths: &[&BStr]| -> Result<()> {
        let output = Command::new("git")
            .arg("-C")
            .arg(workdir)
            .arg("--literal-pathspecs")
            .args(args)
            .arg("--")
            .args(paths.iter().map(|path| gix::path::from_byte_slice(path.as_bytes())))
            .output()
            .context("could not launch Git to discard the selected change")?;
        anyhow::ensure!(output.status.success(), "{}", output.stderr.trim().to_str_lossy());
        Ok(())
    };

    if change.group == ChangeGroup::Staged || change.kind == ChangeKind::Unmerged {
        let source_tree_id = repo.head_tree_id_or_empty()?;
        run(
            &[
                "restore",
                "--staged",
                "--worktree",
                &format!("--source={source_tree_id}"),
            ],
            &paths,
        )
    } else if matches!(
        change.kind,
        ChangeKind::Added | ChangeKind::Copied | ChangeKind::Renamed
    ) {
        if let Some(source) = source {
            run(&["restore", "--worktree"], &[source])?;
        }
        // Intent-to-add files have an index entry which must be removed as well.
        let command = if index.entry_by_path(change.path.as_bstr()).is_some() {
            "rm"
        } else {
            "clean"
        };
        run(&[command, "--force"], &[change.path.as_bstr()])
    } else {
        run(&["restore", "--worktree"], &paths)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = Command::new("git").arg("-C").arg(path).args(args).output()?;
        if !output.status.success() {
            return Err(format!("git {} failed: {}", args.join(" "), output.stderr.to_str_lossy()).into());
        }
        Ok(output.stdout)
    }

    fn discard(path: &Path, selected: &str, group: ChangeGroup) -> gix_testtools::Result {
        let repo = crate::test_repository::open(path)?;
        let changes = crate::load_worktree_changes_without_lines(&repo)?;
        let change = changes
            .paths
            .iter()
            .find(|change| change.path == selected && change.group == group)
            .expect("the fixture has the selected change");
        perform(&repo, change)?;
        Ok(())
    }

    #[test]
    fn discard_uses_the_selected_group_and_preserves_other_paths() -> gix_testtools::Result {
        for (group, expected) in [
            (ChangeGroup::Unstaged, b"staged\n".as_slice()),
            (ChangeGroup::Staged, b"base\n".as_slice()),
        ] {
            let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
            let path = fixture.path();
            std::fs::write(path.join("other"), "other\n")?;
            git(path, &["add", "other"])?;
            let head = git(path, &["rev-parse", "HEAD"])?;

            discard(path, "tracked", group)?;

            assert_eq!(std::fs::read(path.join("tracked"))?, expected);
            assert_eq!(git(path, &["show", ":tracked"])?, expected);
            assert_eq!(git(path, &["show", ":other"])?, b"other\n", "other staging survives");
            assert_eq!(std::fs::read(path.join("untracked"))?, b"untracked\n");
            assert_eq!(
                git(path, &["rev-parse", "HEAD"])?,
                head,
                "discard never rewrites history"
            );
        }
        Ok(())
    }

    #[test]
    fn discard_removes_only_the_selected_addition() -> gix_testtools::Result {
        for args in [vec![], vec!["add", "added[1]"], vec!["add", "-N", "added[1]"]] {
            let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
            let path = fixture.path();
            std::fs::write(path.join("added[1]"), "added\n")?;
            if !args.is_empty() {
                git(path, &args)?;
            }
            std::fs::write(path.join("added1"), "keep\n")?;
            let group = if args.len() == 2 {
                ChangeGroup::Staged
            } else {
                ChangeGroup::Unstaged
            };

            discard(path, "added[1]", group)?;

            assert!(!path.join("added[1]").exists(), "the selected addition is removed");
            assert_eq!(std::fs::read(path.join("added1"))?, b"keep\n", "paths are literal");
            assert!(git(path, &["ls-files", "--", "added[1]"])?.is_empty());
            assert_eq!(git(path, &["show", ":tracked"])?, b"staged\n");
        }
        Ok(())
    }

    #[test]
    fn discard_restores_both_sides_of_a_staged_rename() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let path = fixture.path();
        git(path, &["restore", "--source=HEAD", "--staged", "--worktree", "tracked"])?;
        git(path, &["mv", "tracked", "renamed"])?;

        discard(path, "renamed", ChangeGroup::Staged)?;

        assert!(!path.join("renamed").exists());
        assert_eq!(std::fs::read(path.join("tracked"))?, b"base\n");
        assert!(git(path, &["diff", "--cached", "--name-only"])?.is_empty());
        assert_eq!(std::fs::read(path.join("untracked"))?, b"untracked\n");
        Ok(())
    }

    #[test]
    fn discard_supports_an_unborn_head() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        let path = fixture.path();
        git(path, &["init", "-q", "-b", "main"])?;
        std::fs::write(path.join("added"), "added\n")?;
        git(path, &["add", "added"])?;

        discard(path, "added", ChangeGroup::Staged)?;

        assert!(!path.join("added").exists());
        assert!(git(path, &["ls-files"])?.is_empty());
        assert!(crate::test_repository::open(path)?.head()?.is_unborn());
        Ok(())
    }

    #[test]
    fn discard_conflicts_restores_head_without_disturbing_other_staging() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        let path = fixture.path();
        git(path, &["checkout", "--detach", "HEAD~2"])?;
        let pick = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["cherry-pick", "main"])
            .output()?;
        assert!(!pick.status.success(), "the tip's edit conflicts with the base");
        assert!(!git(path, &["ls-files", "--unmerged"])?.is_empty());
        std::fs::write(path.join("other"), "other\n")?;
        git(path, &["add", "other"])?;

        discard(path, "file", ChangeGroup::Unstaged)?;

        assert_eq!(std::fs::read(path.join("file"))?, b"base\n");
        assert!(git(path, &["ls-files", "--unmerged"])?.is_empty());
        assert_eq!(git(path, &["show", ":other"])?, b"other\n");
        Ok(())
    }

    #[test]
    fn stale_selections_and_index_locks_leave_files_unchanged() -> gix_testtools::Result {
        for stale in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
            let path = fixture.path();
            let repo = crate::test_repository::open(path)?;
            let changes = crate::load_worktree_changes_without_lines(&repo)?;
            let selected = changes
                .paths
                .iter()
                .find(|change| change.group == ChangeGroup::Staged)
                .expect("tracked is staged");
            if stale {
                git(path, &["restore", "--staged", "--", "tracked"])?;
            } else {
                std::fs::write(repo.index_path().with_extension("lock"), "locked")?;
            }
            let before = std::fs::read(repo.index_path())?;

            let error = perform(&crate::test_repository::open(path)?, selected).expect_err("discard must fail");

            assert!(format!("{error:#}").contains(if stale { "no longer available" } else { "index.lock" }));
            assert_eq!(std::fs::read(repo.index_path())?, before, "the index remains unchanged");
            assert_eq!(std::fs::read(path.join("tracked"))?, b"unstaged\n");
            assert_eq!(std::fs::read(path.join("untracked"))?, b"untracked\n");
        }
        Ok(())
    }

    // macOS filesystems reject ill-formed UTF-8 filenames.
    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn discard_keeps_non_utf8_paths_lossless() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let path = fixture.path();
        let selected = BStr::new(b"file-\xff");
        let other = BStr::new(b"file-\xfe");
        for name in [selected, other] {
            std::fs::write(path.join(gix::path::from_bstr(name)), "contents\n")?;
        }
        let repo = crate::test_repository::open(path)?;
        let changes = crate::load_worktree_changes_without_lines(&repo)?;
        let change = changes
            .paths
            .iter()
            .find(|change| change.path == selected)
            .expect("the raw path is listed");

        perform(&repo, change)?;

        assert!(!path.join(gix::path::from_bstr(selected)).exists());
        assert_eq!(std::fs::read(path.join(gix::path::from_bstr(other)))?, b"contents\n");
        Ok(())
    }
}
