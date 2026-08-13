use anyhow::{Context, Result};
use gix::ObjectId;

use super::{create, rebase};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Amend,
    Spill,
}

#[tracing::instrument(skip_all, fields(?kind))]
pub fn perform(
    mut repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    kind: Kind,
) -> Result<Option<ObjectId>> {
    let head = repo
        .head_id()
        .context("editing requires an existing HEAD commit")?
        .detach();
    let mut commit = repo
        .find_commit(head)
        .context("could not find HEAD commit")?
        .decode()
        .context("could not decode HEAD commit")?
        .into_owned()
        .context("could not own HEAD commit")?;
    repo.workdir().context("editing HEAD requires a worktree")?;
    repo.commit_signing_options_if_enabled()
        .context("could not resolve commit signing configuration")?;
    repo = repo.with_object_memory();
    let old_tree = commit.tree;
    let parent_tree = match commit.parents.first().copied() {
        Some(parent) => repo.find_commit(parent)?.tree_id()?.detach(),
        None => repo.empty_tree().id,
    };
    let tree = match kind {
        Kind::Spill => parent_tree,
        Kind::Amend => {
            let index = repo.index_or_empty().context("could not load the index")?;
            if index
                .entries()
                .iter()
                .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
            {
                anyhow::bail!("cannot amend with unresolved index conflicts");
            }
            let index_tree = create::index_tree(&repo, &index)?;
            drop(index);
            if index_tree != old_tree {
                index_tree
            } else {
                let baseline = repo.find_tree(old_tree)?;
                create::worktree_tree(&repo, &baseline)?
            }
        }
    };
    if tree == old_tree {
        return Ok(None);
    }
    commit.tree = tree;
    Ok(rebase::perform(
        repo,
        graph,
        rebase::Edit::Replace { target: head, commit },
        rebase::Signature::InvalidateExisting,
        rebase::Tree::LeaveAsIsAndMark,
    )?
    .selected)
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use gix::bstr::ByteSlice;

    use super::*;

    fn open(path: &Path) -> gix_testtools::Result<gix::Repository> {
        Ok(gix::open_opts(
            path,
            gix::open::Options::isolated().config_overrides([
                "user.name=editor".to_owned(),
                "user.email=editor@example.com".to_owned(),
                "commit.gpgSign=false".to_owned(),
            ]),
        )?)
    }

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = Command::new("git").arg("-C").arg(path).args(args).output()?;
        if !output.status.success() {
            return Err(format!("git {} failed: {}", args.join(" "), output.stderr.to_str_lossy()).into());
        }
        Ok(output.stdout)
    }

    #[test]
    fn amend_prefers_the_index_and_leaves_worktree_files_alone() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let repo = open(fixture.path())?;
        let old = repo.head_id()?.detach();
        let graph = super::super::loaded_graph(&repo)?;
        let new = perform(repo, &graph, Kind::Amend)?.expect("staged changes amend HEAD");
        assert_ne!(new, old);
        assert_eq!(std::fs::read(fixture.path().join("tracked"))?, b"unstaged\n");
        assert_eq!(git(fixture.path(), &["show", "HEAD:tracked"])?, b"staged\n");
        assert!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.is_empty(),
            "the index follows the amended commit"
        );
        let commit = open(fixture.path())?.find_commit(new)?.decode()?.into_owned()?;
        assert!(super::super::rebase::has_marker(&commit), "lazy descendants are marked");
        Ok(())
    }

    #[test]
    fn spill_moves_the_tip_tree_change_to_the_worktree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let old = repo.head_id()?.detach();
        let parent_tree = repo
            .find_commit(old)?
            .parent_ids()
            .next()
            .expect("tip has parent")
            .object()?
            .peel_to_tree()?
            .id;
        let graph = super::super::loaded_graph(&repo)?;
        let new = perform(repo, &graph, Kind::Spill)?.expect("the tip introduces changes");
        let repo = open(fixture.path())?;
        assert_eq!(repo.find_commit(new)?.tree_id()?.detach(), parent_tree);
        assert_eq!(
            std::fs::read(fixture.path().join("tip"))?,
            b"tip\n",
            "worktree content survives"
        );
        assert!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.is_empty(),
            "the index follows the spilled commit"
        );
        assert_eq!(git(fixture.path(), &["status", "--short"])?, b"?? tip\n");
        let graph = super::super::loaded_graph(&repo)?;
        assert_eq!(perform(repo, &graph, Kind::Spill)?, None, "an empty spill is a no-op");
        Ok(())
    }

    #[test]
    fn spilling_a_root_uses_the_empty_tree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let new = perform(repo, &graph, Kind::Spill)?.expect("the root has a non-empty tree");
        let repo = open(fixture.path())?;
        assert_eq!(repo.find_commit(new)?.tree_id()?.detach(), repo.empty_tree().id);
        assert!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.is_empty(),
            "the root spill resets the index to empty"
        );
        assert_eq!(std::fs::read(fixture.path().join("tracked"))?, b"unstaged\n");
        Ok(())
    }
}
