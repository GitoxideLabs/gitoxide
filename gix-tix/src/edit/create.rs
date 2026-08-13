use std::{ffi::OsString, path::Path};

use anyhow::{Context, Result};
use gix::{ObjectId, bstr::ByteSlice, objs::Write};

use crate::{
    ChangeGroup, ChangeKind, ComparedParent, add_line_counts, load_tree_changes_without_lines,
    load_worktree_changes_without_lines, ui,
};

use super::{refs::MutableRefs, reword, time_travel};

pub(crate) struct Prepared {
    pub editor: OsString,
    pub document: Vec<u8>,
    parent: Option<ObjectId>,
    tree: ObjectId,
    references: MutableRefs,
    checkout: Checkout,
    objects: gix::odb::memory::Storage,
}

enum Checkout {
    None,
    Branch(gix::refs::FullName),
    Detached,
}

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
    let (references, checkout) = match parent {
        Some(parent) => {
            let references = MutableRefs::pointing_to(&repo, parent)?;
            if references.is_empty() {
                anyhow::bail!("no mutable reference points to the selected parent");
            }
            references.ensure_not_checked_out_elsewhere(&repo)?;
            let checkout = if head_id == Some(parent) {
                match head.referent_name() {
                    Some(name) => Checkout::Branch(name.to_owned()),
                    None => Checkout::Detached,
                }
            } else {
                Checkout::None
            };
            (references, checkout)
        }
        None => {
            let name = head
                .referent_name()
                .context("an unborn HEAD must point to a branch")?
                .to_owned();
            (MutableRefs::unborn(&repo)?, Checkout::Branch(name))
        }
    };
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
    let mut index_editor = repo.empty_tree().edit().context("could not prepare the index tree")?;
    for entry in index.entries() {
        let mode = entry
            .mode
            .to_tree_entry_mode()
            .context("an index entry has an invalid mode")?;
        index_editor
            .upsert(entry.path(&index), mode.kind(), entry.id)
            .context("could not add an index entry to the candidate tree")?;
    }
    let index_tree = index_editor
        .write()
        .context("could not build the candidate index tree")?
        .detach();
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
        references,
        checkout,
        objects,
    })
}

fn worktree_tree(repo: &gix::Repository, baseline: &gix::Tree<'_>) -> Result<ObjectId> {
    let changes = load_worktree_changes_without_lines(repo)?;
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

pub(crate) fn apply(mut repo: gix::Repository, mut prepared: Prepared, edited: &[u8]) -> Result<ObjectId> {
    let edit = reword::parse(edited)?;
    if edit.message.is_empty() {
        anyhow::bail!("the edited commit message is empty");
    }
    prepared.references.validate(&repo)?;
    let references = match prepared.parent {
        Some(parent) => MutableRefs::pointing_to(&repo, parent)?,
        None => MutableRefs::unborn(&repo)?,
    };
    if references.is_empty() {
        anyhow::bail!("no mutable reference points to the selected parent anymore");
    }
    references.ensure_not_checked_out_elsewhere(&repo)?;
    let signing = repo
        .commit_signing_options_if_enabled()
        .context("could not resolve commit signing configuration")?;
    repo.objects.set_object_memory(std::mem::take(&mut prepared.objects));
    let mut commit = gix::objs::Commit {
        message: edit.message,
        tree: prepared.tree,
        author: reword::actor(edit.author, edit.author_time, "author")?,
        committer: reword::actor(edit.committer, edit.committer_time, "committer")?,
        encoding: None,
        parents: prepared.parent.into_iter().collect(),
        extra_headers: Vec::new(),
    };
    if let Some(options) = signing {
        commit = commit.sign(options).context("could not sign the new commit")?;
    }
    let new_id = repo
        .write_object(&commit)
        .context("could not prepare the final commit")?
        .detach();
    let objects = repo
        .objects
        .take_object_memory()
        .context("candidate object memory was unavailable")?;
    for (id, (kind, data)) in objects.iter() {
        repo.write_buf_with_known_id(*kind, data, *id)
            .map_err(|err| anyhow::anyhow!("could not persist a prepared commit object: {err}"))?;
    }
    let log_message = gix::reference::log::message("commit", commit.message.as_bstr(), commit.parents.len());
    let mut time_buf = gix::date::parse::TimeBuf::default();
    references.update(
        &repo,
        new_id,
        log_message.as_bstr(),
        Some(commit.committer.to_ref(&mut time_buf)),
    )?;
    let checkout = match &prepared.checkout {
        Checkout::None => Ok(()),
        Checkout::Branch(name) => {
            let workdir = repo.workdir().context("creating a commit requires a worktree")?;
            let branch = name
                .as_bstr()
                .strip_prefix(b"refs/heads/")
                .context("the destination isn't a local branch")?;
            time_travel::checkout(
                workdir,
                [
                    OsString::from("--no-guess"),
                    gix::path::from_bstr(branch.as_bstr()).into_owned().into_os_string(),
                ],
            )
            .and_then(|()| reset_index(workdir, new_id))
        }
        Checkout::Detached => {
            let workdir = repo.workdir().context("creating a commit requires a worktree")?;
            time_travel::checkout_detached(workdir, new_id)
        }
    };
    if let Err(err) = checkout {
        let rollback = references.rollback(&repo, new_id);
        return match rollback {
            Ok(()) => Err(err),
            Err(rollback) => Err(err.context(format!("the destination could not be rolled back: {rollback:#}"))),
        };
    }
    Ok(new_id)
}

fn reset_index(workdir: &Path, id: ObjectId) -> Result<()> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["reset", "--mixed", "--quiet"])
        .arg(id.to_string())
        .output()
        .context("could not update the index after checkout")?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "git reset failed with {}: {}",
            output.status,
            output.stderr.to_str_lossy().trim()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

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
        let new_id = apply(open(fixture.path())?, prepared, &edited)?;
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
        let new_id = apply(open(fixture.path())?, prepared, &edited)?;
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
        let new_id = apply(open(fixture.path())?, prepared, &edited)?;
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
        let new_id = apply(open(fixture.path())?, prepared, &edited)?;
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
