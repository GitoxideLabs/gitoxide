use std::ffi::OsString;

use anyhow::{Context, Result};
use gix::{ObjectId, bstr::ByteSlice};

use crate::{
    ChangeGroup, ChangeKind, ComparedParent, add_line_counts, load_tree_changes_without_lines,
    load_worktree_changes_without_lines, ui,
};

use super::{rebase, reword};

pub(crate) struct Prepared {
    pub editor: OsString,
    pub document: Vec<u8>,
    pub(super) parent: Option<ObjectId>,
    pub(super) tree: ObjectId,
    pub(super) objects: gix::odb::memory::Storage,
}

#[tracing::instrument(skip_all, fields(parent = ?parent))]
pub(crate) fn prepare(mut repo: gix::Repository, parent: Option<ObjectId>) -> Result<Prepared> {
    repo.workdir().context("creating a commit requires a worktree")?;
    let head = repo.head().context("could not read HEAD before creating a commit")?;
    let head_id = head.id().map(gix::Id::detach);
    if parent.is_none() && !head.is_unborn() {
        anyhow::bail!("an unborn history is required to create a root commit");
    }
    if let Some(parent) = parent {
        repo.find_commit(parent)
            .context("could not find the selected parent commit")?;
    }
    if parent.is_none() {
        head.referent_name().context("an unborn HEAD must point to a branch")?;
    }
    let editor = repo.editor().context("no Git editor is available")?;
    let author = repo
        .author()
        .context("no Git author is configured")?
        .context("could not resolve the Git author")?
        .to_owned()
        .context("could not own the Git author")?;
    let committer = repo
        .committer()
        .context("no Git committer is configured")?
        .context("could not resolve the Git committer")?
        .to_owned()
        .context("could not own the Git committer")?;
    repo.commit_signing_options_if_enabled()
        .context("could not resolve commit signing configuration")?;

    repo = repo.with_object_memory();
    let baseline = match parent {
        Some(id) => repo
            .find_commit(id)
            .context("could not find the parent commit")?
            .tree()
            .context("could not load the parent tree")?,
        None => repo.empty_tree(),
    };
    let index = repo.index_or_empty().context("could not load the index")?;
    if index
        .entries()
        .iter()
        .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
    {
        anyhow::bail!("cannot create a commit with unresolved index conflicts");
    }
    let index_tree = index_tree(&repo, &index)?;
    let based_on_parent = head_id == parent;
    let tree = if based_on_parent && index_tree != baseline.id {
        index_tree
    } else if based_on_parent {
        worktree_tree(&repo, &baseline)?
    } else {
        baseline.id
    };

    let new_tree = repo.find_tree(tree).context("could not load the candidate tree")?;
    let mut changes = load_tree_changes_without_lines(
        &repo,
        parent.map(|_| &baseline),
        &new_tree,
        parent.map(|id| ComparedParent { index: 0, total: 1, id }),
    )?;
    let line_counts = add_line_counts(&repo, &mut changes)?;
    let mut document = Vec::new();
    reword::write_headers(&mut document, &author, &committer)?;
    document.extend_from_slice(b"\nwhat\n\nwhy\n");
    for trailer in reword::missing_agent_trailers(b"what\n\nwhy\n").into_iter().flatten() {
        document.extend_from_slice(b"\n;");
        document.extend_from_slice(trailer);
    }
    document.extend_from_slice(b"\n\n; Changes to be committed:\n");
    for line in ui::commit_diff_summary(&changes, &line_counts, changes.lines_added, changes.lines_removed) {
        document.extend_from_slice(b"; ");
        for span in line.spans {
            document.extend_from_slice(span.content.as_bytes());
        }
        document.push(b'\n');
    }
    drop(new_tree);
    drop(baseline);
    drop(index);

    let provisional = repo
        .new_commit("what\n\nwhy\n", tree, parent)
        .context("could not prepare the commit object")?
        .id;
    let mut objects = repo
        .objects
        .take_object_memory()
        .context("candidate object memory was unavailable")?;
    objects.remove(&provisional);
    Ok(Prepared {
        editor,
        document,
        parent,
        tree,
        objects,
    })
}

pub(super) fn index_tree(repo: &gix::Repository, index: &gix::index::File) -> Result<ObjectId> {
    let mut editor = repo.empty_tree().edit().context("could not prepare the index tree")?;
    for entry in index.entries() {
        let mode = entry
            .mode
            .to_tree_entry_mode()
            .context("an index entry has an invalid mode")?;
        editor
            .upsert(entry.path(index), mode.kind(), entry.id)
            .context("could not add an index entry to the candidate tree")?;
    }
    Ok(editor
        .write()
        .context("could not build the candidate index tree")?
        .detach())
}

pub(super) fn worktree_tree(repo: &gix::Repository, baseline: &gix::Tree<'_>) -> Result<ObjectId> {
    let changes = load_worktree_changes_without_lines(repo)?;
    worktree_tree_with_changes(repo, baseline, &changes)
}

pub(super) fn worktree_tree_with_changes(
    repo: &gix::Repository,
    baseline: &gix::Tree<'_>,
    changes: &crate::Changes,
) -> Result<ObjectId> {
    if changes.paths.is_empty() {
        return Ok(baseline.id);
    }
    let (mut pipeline, index) = repo
        .filter_pipeline(None)
        .context("could not initialize worktree filters")?;
    let mut editor = baseline.edit().context("could not edit the parent tree")?;
    for change in changes
        .paths
        .iter()
        .filter(|change| change.group == ChangeGroup::Unstaged)
    {
        if let Some(source) = &change.source {
            editor
                .remove(source)
                .context("could not remove a renamed source path")?;
        }
        if change.kind == ChangeKind::Deleted {
            editor
                .remove(&change.path)
                .context("could not remove a deleted worktree path")?;
            continue;
        }
        match pipeline
            .worktree_file_to_object(change.path.as_bstr(), &index)
            .with_context(|| format!("could not prepare {}", change.path.to_str_lossy()))?
        {
            Some((id, kind, _)) => {
                editor
                    .upsert(&change.path, kind, id)
                    .context("could not add a worktree path to the candidate tree")?;
            }
            None => {
                editor
                    .remove(&change.path)
                    .context("could not remove an unavailable worktree path")?;
            }
        }
    }
    Ok(editor.write().context("could not build the worktree tree")?.detach())
}

#[tracing::instrument(skip_all, fields(parent = ?prepared.parent))]
pub(crate) fn apply(
    mut repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    mut prepared: Prepared,
    edited: &[u8],
) -> Result<ObjectId> {
    repo.objects.set_object_memory(std::mem::take(&mut prepared.objects));
    let commit = commit_from_edit(&prepared, edited)?;
    rebase::perform(
        &repo,
        graph,
        rebase::Edit::Insert {
            anchor: prepared.parent,
            commit,
        },
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::LeaveAsIsAndMark,
    )?
    .complete()?
    .selected
    .context("inserting a commit did not produce a selection")
}

#[tracing::instrument(skip_all, fields(parent = ?prepared.parent))]
pub(crate) fn apply_fork(
    mut repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    mut prepared: Prepared,
    edited: &[u8],
) -> Result<ObjectId> {
    repo.objects.set_object_memory(std::mem::take(&mut prepared.objects));
    let parent = prepared.parent.context("a fork commit requires a parent")?;
    let commit = commit_from_edit(&prepared, edited)?;
    rebase::perform(
        &repo,
        graph,
        rebase::Edit::Fork { anchor: parent, commit },
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::LeaveAsIs,
    )?
    .complete()?
    .selected
    .context("forking a commit did not produce a selection")
}

pub(super) fn commit_from_edit(prepared: &Prepared, edited: &[u8]) -> Result<gix::objs::Commit> {
    let edit = reword::parse(edited)?;
    if edit.message.is_empty() {
        anyhow::bail!("the edited commit message is empty");
    }
    Ok(gix::objs::Commit {
        message: edit.message,
        tree: prepared.tree,
        author: reword::actor(edit.author, edit.author_time, "author")?,
        committer: reword::actor(edit.committer, edit.committer_time, "committer")?,
        encoding: None,
        parents: prepared.parent.into_iter().collect(),
        extra_headers: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::*;

    fn open(path: &Path) -> gix_testtools::Result<gix::Repository> {
        Ok(gix::open_opts(
            path,
            gix::open::Options::isolated().config_overrides([
                "core.editor=:".to_owned(),
                "user.name=author".to_owned(),
                "user.email=author@example.com".to_owned(),
            ]),
        )?)
    }

    fn object_count(path: &Path) -> gix_testtools::Result<Vec<u8>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(["count-objects", "-v"])
            .output()?;
        if !output.status.success() {
            return Err(format!("git count-objects failed: {}", output.stderr.to_str_lossy()).into());
        }
        Ok(output.stdout)
    }

    #[test]
    fn preparation_is_unobservable_and_staged_changes_win() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let parent = open(fixture.path())?.head_id()?.detach();
        for name in ["refs/patches/create", "refs/tags/keep", "refs/remotes/origin/keep"] {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(fixture.path())
                    .args(["update-ref", name, &parent.to_string()])
                    .status()?
                    .success(),
                "the test reference is created"
            );
        }
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let objects_before = object_count(fixture.path())?;
        let prepared = prepare(open(fixture.path())?, Some(parent))?;
        assert!(
            prepared
                .document
                .windows(b"tracked | 2 +- 0".len())
                .any(|window| window == b"tracked | 2 +- 0"),
            "the editor buffer includes a commented per-file diffstat with net lines: {}",
            prepared.document.as_bstr()
        );
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "preparing the tree and commit leaves the complete repository state unchanged"
        );
        assert_eq!(
            object_count(fixture.path())?,
            objects_before,
            "preparation writes no loose objects"
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["update-ref", "refs/patches/late", &parent.to_string()])
                .status()?
                .success(),
            "a ref may appear while the editor is open"
        );

        let edited = prepared.document.replacen(b"what\n\nwhy", b"title\n\nbody", 1);
        let graph = super::super::loaded_graph(&open(fixture.path())?)?;
        let new_id = apply(open(fixture.path())?, &graph, prepared, &edited)?;
        let after = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(
            after.head,
            gix_testtools::repository::Head::Symbolic {
                name: b"refs/heads/main".into(),
                id: new_id,
            },
            "the checked-out branch advances to the new commit"
        );
        let repository = open(fixture.path())?;
        let commit = repository.find_commit(new_id)?;
        assert_eq!(commit.parent_ids().next().map(gix::Id::detach), Some(parent));
        assert_eq!(commit.message_raw()?, b"title\n\nbody\n".as_bstr());
        assert_eq!(
            Some(commit.tree_id()?.detach()),
            before.index_tree,
            "a changed index supplies the commit tree even when the worktree differs"
        );
        assert_eq!(after.index_tree, before.index_tree, "the committed index stays intact");
        assert_eq!(
            after.worktree, before.worktree,
            "unstaged and untracked files stay intact"
        );
        assert_eq!(
            after.commits.len(),
            before.commits.len() + 1,
            "exactly one reachable commit is added"
        );
        for name in ["refs/heads/main", "refs/patches/create", "refs/patches/late"] {
            assert_eq!(
                repository.find_reference(name)?.id().detach(),
                new_id,
                "{name} advances to the new commit"
            );
        }
        for name in ["refs/tags/keep", "refs/remotes/origin/keep"] {
            assert_eq!(
                repository.find_reference(name)?.id().detach(),
                parent,
                "{name} is not edited"
            );
        }
        Ok(())
    }

    #[test]
    fn worktree_changes_supply_the_tree_when_the_index_is_unchanged() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["reset", "-q", "HEAD"])
                .status()?
                .success()
        );
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let parent = open(fixture.path())?.head_id()?.detach();
        let prepared = prepare(open(fixture.path())?, Some(parent))?;
        let edited = prepared.document.replacen(b"what\n\nwhy", b"worktree\n\nstate", 1);
        let graph = super::super::loaded_graph(&open(fixture.path())?)?;
        let new_id = apply(open(fixture.path())?, &graph, prepared, &edited)?;
        let after = gix_testtools::repository::snapshot(fixture.path())?;
        let repository = open(fixture.path())?;
        let commit = repository.find_commit(new_id)?;
        assert_ne!(
            Some(commit.tree_id()?.detach()),
            before.index_tree,
            "worktree changes produce a new tree"
        );
        assert_eq!(
            after.index_tree,
            Some(commit.tree_id()?.detach()),
            "checking out the commit updates the index to its worktree-derived tree"
        );
        assert_eq!(
            after.worktree, before.worktree,
            "the committed worktree bytes remain exactly as prepared"
        );
        Ok(())
    }

    #[test]
    fn fork_creates_an_independent_pinned_child_then_time_travels_to_it() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let repository = open(fixture.path())?;
        let main = repository.head_id()?.detach();
        let parent = main;
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let prepared = prepare(open(fixture.path())?, Some(parent))?;
        let edited = prepared.document.replacen(b"what\n\nwhy", b"fork\n\nreason", 1);
        let graph = super::super::loaded_graph(&open(fixture.path())?)?;
        let fork = apply_fork(open(fixture.path())?, &graph, prepared, &edited)?;

        let repository = open(fixture.path())?;
        assert_eq!(repository.head_id()?.detach(), main, "forking does not move HEAD");
        assert_eq!(
            repository.find_reference("refs/heads/main")?.id().detach(),
            main,
            "forking does not move the source branch"
        );
        assert_eq!(
            repository.find_commit(fork)?.parent_ids().next().map(gix::Id::detach),
            Some(parent),
            "the fork is a direct child of the selected commit"
        );
        let pins = crate::history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "the new leaf is retained until checkout");
        assert_eq!(pins[0].id, fork);
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?.worktree,
            before.worktree,
            "creating the fork leaves worktree files untouched"
        );

        let repository_path = repository.git_dir().to_owned();
        let graph = super::super::loaded_graph(&repository)?;
        drop(repository);
        super::super::time_travel::perform(&repository_path, false, fork, &graph, &[], false)?.complete()?;
        let repository = open(fixture.path())?;
        assert!(repository.head()?.is_detached(), "automatic fork travel detaches HEAD");
        assert_eq!(repository.head_id()?.detach(), fork);
        assert!(
            crate::history::all_pins(&repository)?.is_empty(),
            "the checkout consumes the fork pin and does not retain a redundant source pin"
        );
        Ok(())
    }

    #[test]
    fn creates_an_empty_root_commit_for_an_unborn_head() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["init", "-q", "-b", "main"])
                .status()?
                .success()
        );
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let prepared = prepare(open(fixture.path())?, None)?;
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "root-commit preflight is unobservable"
        );
        let edited = prepared.document.replacen(b"what\n\nwhy", b"root\n\nreason", 1);
        let graph = super::super::loaded_graph(&open(fixture.path())?)?;
        let new_id = apply(open(fixture.path())?, &graph, prepared, &edited)?;
        let repository = open(fixture.path())?;
        let commit = repository.find_commit(new_id)?;
        assert!(commit.parent_ids().next().is_none(), "the root has no parent");
        assert_eq!(
            commit.tree_id()?.detach(),
            ObjectId::empty_tree(repository.object_hash()),
            "no index or worktree changes reuse the empty tree"
        );
        assert_eq!(
            repository.head_name()?.map(|name| name.as_bstr().to_owned()),
            Some(b"refs/heads/main".into()),
            "the unborn branch is created and remains checked out"
        );
        Ok(())
    }

    #[test]
    fn an_unrelated_worktree_head_is_not_checked_out_or_moved() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let parent = open(fixture.path())?.head_id()?.detach();
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["checkout", "-q", "--orphan", "other"])
                .status()?
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["rm", "-rf", "-q", "."])
                .status()?
                .success()
        );
        std::fs::write(fixture.path().join("other"), b"other\n")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["add", "other"])
                .status()?
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["-c", "commit.gpgSign=false", "commit", "-q", "-m", "other"])
                .status()?
                .success()
        );
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let prepared = prepare(open(fixture.path())?, Some(parent))?;
        let edited = prepared.document.replacen(b"what\n\nwhy", b"child\n\nreason", 1);
        let graph = super::super::loaded_graph(&open(fixture.path())?)?;
        let new_id = apply(open(fixture.path())?, &graph, prepared, &edited)?;
        let after = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(
            after.head, before.head,
            "the unrelated checked-out branch does not move"
        );
        assert_eq!(after.index, before.index, "the unrelated index does not change");
        assert_eq!(
            after.worktree, before.worktree,
            "the unrelated worktree does not change"
        );
        assert_eq!(
            open(fixture.path())?.find_reference("refs/heads/main")?.id().detach(),
            new_id,
            "the selected parent branch advances independently"
        );
        Ok(())
    }
}
