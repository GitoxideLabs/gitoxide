use anyhow::{Context, Result};
use gix::{ObjectId, bstr::BStr};

use super::{create, rebase};
use crate::{ChangeKind, PathChange};

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
    selected_path: Option<(&PathChange, Option<ObjectId>)>,
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
        Kind::Spill => match selected_path {
            Some((path, selected_parent)) => {
                spill_path_tree(&repo, old_tree, selected_parent.unwrap_or(parent_tree), path)?
            }
            None => parent_tree,
        },
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
        &repo,
        graph,
        rebase::Edit::Replace { target: head, commit },
        rebase::Signature::InvalidateExisting,
        rebase::Tree::LeaveAsIsAndMark,
    )?
    .complete()?
    .selected)
}

fn spill_path_tree(
    repo: &gix::Repository,
    commit_tree: ObjectId,
    parent_tree: ObjectId,
    change: &PathChange,
) -> Result<ObjectId> {
    let parent = repo.find_tree(parent_tree).context("could not load the parent tree")?;
    let mut editor = repo
        .find_tree(commit_tree)
        .context("could not load the commit tree")?
        .edit()
        .context("could not edit the commit tree")?;
    match change.kind {
        ChangeKind::Added => {
            editor.remove(&change.path).context("could not spill the added path")?;
        }
        ChangeKind::Deleted | ChangeKind::Modified | ChangeKind::TypeChanged => {
            restore_path(&parent, &mut editor, &change.path)?;
        }
        ChangeKind::Renamed | ChangeKind::Copied => {
            editor
                .remove(&change.path)
                .context("could not spill the rewritten destination")?;
            if change.kind == ChangeKind::Renamed {
                restore_path(
                    &parent,
                    &mut editor,
                    change.source.as_ref().context("a rename has no source path")?,
                )?;
            }
        }
        ChangeKind::Unmerged => anyhow::bail!("cannot spill an unmerged path"),
    }
    Ok(editor
        .write()
        .context("could not build the partially spilled tree")?
        .detach())
}

fn restore_path(
    parent: &gix::Tree<'_>,
    editor: &mut gix::object::tree::Editor<'_>,
    path: &gix::bstr::BString,
) -> Result<()> {
    let entry = parent
        .lookup_entry(
            path.split(|byte| *byte == b'/')
                .map(|component| BStr::new(component).to_owned()),
        )
        .context("could not look up the path in the parent tree")?
        .context("the path is absent from the parent tree")?;
    editor
        .upsert(path, entry.mode().kind(), entry.object_id())
        .context("could not restore the path from the parent tree")?;
    Ok(())
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
        let new = perform(repo, &graph, Kind::Amend, None)?.expect("staged changes amend HEAD");
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
        let new = perform(repo, &graph, Kind::Spill, None)?.expect("the tip introduces changes");
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
        assert_eq!(
            perform(repo, &graph, Kind::Spill, None)?,
            None,
            "an empty spill is a no-op"
        );
        Ok(())
    }

    #[test]
    fn spilling_a_root_uses_the_empty_tree() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let new = perform(repo, &graph, Kind::Spill, None)?.expect("the root has a non-empty tree");
        let repo = open(fixture.path())?;
        assert_eq!(repo.find_commit(new)?.tree_id()?.detach(), repo.empty_tree().id);
        assert!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.is_empty(),
            "the root spill resets the index to empty"
        );
        assert_eq!(std::fs::read(fixture.path().join("tracked"))?, b"unstaged\n");
        Ok(())
    }

    #[test]
    fn spilling_one_path_keeps_the_other_commit_changes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        std::fs::write(fixture.path().join("other"), "other\n")?;
        git(fixture.path(), &["add", "other"])?;
        git(fixture.path(), &["commit", "--amend", "--no-edit"])?;
        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let selected = PathChange {
            kind: ChangeKind::Added,
            group: crate::ChangeGroup::Tree,
            source: None,
            path: "tip".into(),
            lines: None,
        };
        let new =
            perform(repo, &graph, Kind::Spill, Some((&selected, None)))?.expect("the selected path can be spilled");
        let repo = open(fixture.path())?;
        let tree = repo.find_commit(new)?.tree()?;
        assert!(
            tree.lookup_entry(["other"])?.is_some(),
            "the unselected addition remains committed"
        );
        assert!(
            tree.lookup_entry(["tip"])?.is_none(),
            "the selected addition is spilled"
        );
        assert_eq!(std::fs::read(fixture.path().join("tip"))?, b"tip\n");
        assert_eq!(git(fixture.path(), &["status", "--short"])?, b"?? tip\n");
        Ok(())
    }
}
