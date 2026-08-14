use std::{io::Write, path::Path, process::Command};

use anyhow::{Context, Result};
use gix::{ObjectId, bstr::ByteSlice};

use super::rebase;

#[tracing::instrument(skip_all, fields(commit_id = %id))]
pub(crate) fn perform(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    id: ObjectId,
) -> Result<Option<ObjectId>> {
    let commit = repo
        .find_commit(id)
        .context("could not find the commit to forget")?
        .decode()?
        .into_owned()?;
    let deletions = super::review::deletions(&repo, &commit)?;
    let result = if deletions.is_empty() {
        rebase::perform(
            &repo,
            graph,
            rebase::Edit::Remove { target: id },
            rebase::Signature::RedoIfNeeded,
            rebase::Tree::LeaveAsIsAndMark,
        )
    } else {
        rebase::perform_deleting_refs(
            &repo,
            graph,
            rebase::Edit::Remove { target: id },
            rebase::Signature::RedoIfNeeded,
            rebase::Tree::LeaveAsIsAndMark,
            deletions,
        )
    }?;
    Ok(result.complete()?.selected)
}

pub(super) fn preflight_tree_transition(
    repo: &gix::Repository,
    workdir: &Path,
    old: ObjectId,
    new: ObjectId,
) -> Result<()> {
    let mut index = gix::tempfile::writable_at(
        std::env::temp_dir().join(format!(
            "tix-forget-index-{}-{old}-{:?}",
            std::process::id(),
            std::thread::current().id()
        )),
        gix::tempfile::ContainingDirectory::Exists,
        gix::tempfile::AutoRemove::Tempfile,
    )
    .context("could not create a temporary index for forget preflight")?;
    index
        .write_all(&std::fs::read(repo.index_path()).context("could not read the index before forgetting")?)
        .context("could not copy the index for forget preflight")?;
    index.flush().context("could not flush the forget preflight index")?;
    let index = index.take().context("the forget preflight index disappeared")?;
    let refresh = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .env("GIT_INDEX_FILE", index.path())
        .args(["update-index", "-q", "--refresh"])
        .output()
        .context("could not refresh the index before forgetting")?;
    if !refresh.status.success() {
        anyhow::bail!("{}", refresh.stderr.to_str_lossy().trim());
    }
    run_read_tree(workdir, Some(index.path()), true, old, new)
        .context("local changes conflict with forgetting this commit")
}

pub(super) fn apply_tree_transition(workdir: &Path, old: ObjectId, new: ObjectId) -> Result<()> {
    let refresh = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["update-index", "-q", "--refresh"])
        .output()
        .context("could not refresh the index before applying forget")?;
    if !refresh.status.success() {
        anyhow::bail!("{}", refresh.stderr.to_str_lossy().trim());
    }
    run_read_tree(workdir, None, false, old, new).context("could not update the index and worktree")
}

fn run_read_tree(workdir: &Path, index: Option<&Path>, dry_run: bool, old: ObjectId, new: ObjectId) -> Result<()> {
    let mut command = Command::new("git");
    command.arg("-C").arg(workdir).arg("read-tree");
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    if dry_run {
        command.arg("-n");
    }
    let output = command
        .args(["-m", "-u"])
        .arg(old.to_string())
        .arg(new.to_string())
        .output()
        .context("could not run git read-tree")?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!("{}", output.stderr.to_str_lossy().trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(path: &Path) -> gix_testtools::Result<gix::Repository> {
        Ok(gix::open_opts(
            path,
            gix::open::Options::isolated().config_overrides([
                "user.name=author".to_owned(),
                "user.email=author@example.com".to_owned(),
            ]),
        )?)
    }

    #[test]
    fn forgets_a_tip_atomically_and_preserves_untracked_files() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("forget_commit.sh")?;
        let repository = open(fixture.path())?;
        let top = repository.head_id()?.detach();
        let parent = repository
            .find_commit(top)?
            .parent_ids()
            .next()
            .expect("top has a parent")
            .detach();

        let graph = super::super::loaded_graph(&repository)?;
        assert_eq!(perform(repository.clone(), &graph, top)?, Some(parent));
        let state = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(
            state.head,
            gix_testtools::repository::Head::Symbolic {
                name: b"refs/heads/main".into(),
                id: parent,
            },
            "the attached branch is retargeted to the parent"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("tracked"))?,
            b"base\n",
            "the selected commit's tracked change is discarded"
        );
        assert!(
            !fixture.path().join("added").exists(),
            "the selected commit's added file is removed"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("untracked"))?,
            b"untracked\n",
            "unrelated untracked files survive"
        );
        assert_eq!(
            state.index_tree,
            Some(repository.find_commit(parent)?.tree_id()?.detach()),
            "the index matches the parent tree"
        );
        for name in ["refs/heads/main", "refs/patches/forget"] {
            assert_eq!(
                repository.find_reference(name)?.id().detach(),
                parent,
                "{name} follows the forget"
            );
        }
        for name in ["refs/tags/keep", "refs/remotes/origin/keep"] {
            assert_eq!(
                repository.find_reference(name)?.id().detach(),
                top,
                "{name} remains immutable"
            );
        }
        Ok(())
    }

    #[test]
    fn refuses_conflicting_local_changes_without_mutating_repository_state() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("forget_commit.sh")?;
        std::fs::write(fixture.path().join("tracked"), b"local\n")?;
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let repository = open(fixture.path())?;
        let top = repository.head_id()?.detach();
        assert!(
            perform(repository.clone(), &super::super::loaded_graph(&repository)?, top).is_err(),
            "overlapping local changes are rejected"
        );
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "a failed preflight leaves refs, index, commits, and worktree unchanged"
        );
        Ok(())
    }

    #[test]
    fn forgetting_the_checked_out_root_leaves_an_unborn_branch() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        let git = |args: &[&str]| -> std::io::Result<std::process::ExitStatus> {
            Command::new("git").arg("-C").arg(fixture.path()).args(args).status()
        };
        assert!(git(&["init", "-q", "-b", "main"])?.success());
        assert!(git(&["config", "user.name", "author"])?.success());
        assert!(git(&["config", "user.email", "author@example.com"])?.success());
        std::fs::write(fixture.path().join("tracked"), b"root\n")?;
        assert!(git(&["add", "tracked"])?.success());
        assert!(git(&["-c", "commit.gpgSign=false", "commit", "-q", "-m", "root"])?.success());
        std::fs::write(fixture.path().join("untracked"), b"keep\n")?;
        let repository = open(fixture.path())?;
        let root = repository.head_id()?.detach();

        let graph = super::super::loaded_graph(&repository)?;
        assert_eq!(perform(repository.clone(), &graph, root)?, None);
        let state = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(
            state.head,
            gix_testtools::repository::Head::Unborn(b"refs/heads/main".into()),
            "deleting the root branch leaves symbolic HEAD unborn"
        );
        assert!(state.index.is_empty(), "the index becomes empty");
        assert!(
            !fixture.path().join("tracked").exists(),
            "tracked root content is removed"
        );
        assert_eq!(std::fs::read(fixture.path().join("untracked"))?, b"keep\n");
        Ok(())
    }

    #[test]
    fn forgetting_without_a_worktree_only_retargets_references() -> gix_testtools::Result {
        let source = gix_testtools::scripted_fixture_read_only("forget_commit.sh")?;
        let fixture = gix_testtools::tempfile::tempdir()?;
        assert!(
            Command::new("git")
                .args(["clone", "-q", "--bare"])
                .arg(source)
                .arg(fixture.path())
                .status()?
                .success()
        );
        let repository = open(fixture.path())?;
        assert!(repository.is_bare(), "the scenario has no worktree");
        let top = repository.head_id()?.detach();
        let parent = repository
            .find_commit(top)?
            .parent_ids()
            .next()
            .expect("top has a parent")
            .detach();
        let graph = super::super::loaded_graph(&repository)?;
        assert_eq!(perform(repository.clone(), &graph, top)?, Some(parent));
        assert_eq!(
            repository.head_id()?.detach(),
            parent,
            "HEAD's branch moves without checkout"
        );
        Ok(())
    }

    #[test]
    fn refuses_to_forget_a_checked_out_detached_root() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        let git = |args: &[&str]| -> std::io::Result<std::process::ExitStatus> {
            Command::new("git").arg("-C").arg(fixture.path()).args(args).status()
        };
        assert!(git(&["init", "-q", "-b", "main"])?.success());
        assert!(git(&["config", "user.name", "author"])?.success());
        assert!(git(&["config", "user.email", "author@example.com"])?.success());
        assert!(
            git(&[
                "-c",
                "commit.gpgSign=false",
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "root"
            ])?
            .success()
        );
        assert!(git(&["checkout", "-q", "--detach"])?.success());
        assert!(git(&["branch", "-D", "main"])?.success());
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let repository = open(fixture.path())?;
        let root = repository.head_id()?.detach();
        assert!(
            perform(repository.clone(), &super::super::loaded_graph(&repository)?, root).is_err(),
            "detached HEAD cannot become unborn"
        );
        assert_eq!(gix_testtools::repository::snapshot(fixture.path())?, before);
        Ok(())
    }
}
