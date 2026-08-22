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
    pub(crate) is_empty: bool,
    pub(super) reset_index: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum Source {
    Default,
    Index,
    Worktree,
    WorktreeUntracked,
}

#[tracing::instrument(skip_all, fields(parent = ?parent))]
pub(crate) fn prepare(repo: gix::Repository, parent: Option<ObjectId>) -> Result<Prepared> {
    prepare_inner(repo, parent, false, Source::Default, None, false)
}

#[tracing::instrument(skip_all, fields(parent = ?parent))]
pub(crate) fn prepare_empty(repo: gix::Repository, parent: Option<ObjectId>) -> Result<Prepared> {
    prepare_inner(repo, parent, true, Source::Default, None, false)
}

pub(crate) fn prepare_from(
    repo: gix::Repository,
    parent: Option<ObjectId>,
    source: Source,
    author: Option<&gix::bstr::BStr>,
    todo: bool,
) -> Result<Prepared> {
    prepare_inner(repo, parent, false, source, author, todo)
}

fn prepare_inner(
    mut repo: gix::Repository,
    parent: Option<ObjectId>,
    empty: bool,
    source: Source,
    author_override: Option<&gix::bstr::BStr>,
    todo: bool,
) -> Result<Prepared> {
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
    let mut author = repo
        .author()
        .context("no Git author is configured")?
        .context("could not resolve the Git author")?
        .to_owned()
        .context("could not own the Git author")?;
    if let Some(value) = author_override {
        author = reword::actor(value, author.time, "author")?;
    }
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
    let baseline_id = baseline.id;
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
    let tree = if empty || !based_on_parent {
        baseline.id
    } else {
        match source {
            Source::Default if index_tree != baseline.id => index_tree,
            Source::Default | Source::Worktree => worktree_tree_tracked(&repo, &baseline, &index)?,
            Source::Index => index_tree,
            Source::WorktreeUntracked => worktree_tree(&repo, &baseline)?,
        }
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
    reword::write_headers(
        &mut document,
        &author,
        &committer,
        &crate::enrich::Enrichment {
            todo,
            ..Default::default()
        },
    )?;
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
        is_empty: tree == baseline_id,
        reset_index: !empty,
    })
}

pub(crate) fn index_tree(repo: &gix::Repository, index: &gix::index::File) -> Result<ObjectId> {
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
    worktree_tree_with_changes_inner(repo, baseline, &changes, None)
}

fn worktree_tree_tracked(
    repo: &gix::Repository,
    baseline: &gix::Tree<'_>,
    index: &gix::index::File,
) -> Result<ObjectId> {
    let changes = load_worktree_changes_without_lines(repo)?;
    worktree_tree_with_changes_inner(repo, baseline, &changes, Some(index))
}

pub(super) fn worktree_tree_with_changes(
    repo: &gix::Repository,
    baseline: &gix::Tree<'_>,
    changes: &crate::Changes,
) -> Result<ObjectId> {
    worktree_tree_with_changes_inner(repo, baseline, changes, None)
}

fn worktree_tree_with_changes_inner(
    repo: &gix::Repository,
    baseline: &gix::Tree<'_>,
    changes: &crate::Changes,
    tracked_by: Option<&gix::index::File>,
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
        if tracked_by.is_some_and(|index| {
            index.entry_by_path(change.path.as_bstr()).is_none()
                && change
                    .source
                    .as_ref()
                    .is_none_or(|source| index.entry_by_path(source.as_bstr()).is_none())
        }) {
            continue;
        }
        if change.kind == ChangeKind::Renamed
            && let Some(source) = &change.source
        {
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
#[cfg(test)]
pub(crate) fn apply(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    prepared: Prepared,
    edited: &[u8],
) -> Result<ObjectId> {
    apply_reporting(repo, graph, prepared, edited)?
        .selected
        .context("inserting a commit did not produce a selection")
}

pub(crate) fn apply_reporting(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    prepared: Prepared,
    edited: &[u8],
) -> Result<rebase::Outcome> {
    apply_conflict_reporting(repo, graph, prepared, edited, |_| {})?.complete()
}

pub(crate) fn apply_conflict_reporting(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    prepared: Prepared,
    edited: &[u8],
    report: impl FnMut(rebase::Progress),
) -> Result<rebase::Perform> {
    let edit = commit_from_edit(&prepared, edited)?;
    apply_commit_conflict(repo, graph, prepared, edit, report)
}

pub(crate) fn apply_message_reporting(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    prepared: Prepared,
    message: &[u8],
) -> Result<rebase::Outcome> {
    let mut edit = reword::parse(&prepared.document)?;
    edit.message = reword::cleanup_message(message, None);
    let commit = commit_from_parsed_edit(&prepared, edit)?;
    apply_commit(repo, graph, prepared, commit)
}

fn apply_commit(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    prepared: Prepared,
    edit: (gix::objs::Commit, crate::enrich::Headers),
) -> Result<rebase::Outcome> {
    apply_commit_conflict(repo, graph, prepared, edit, |_| {})?.complete()
}

fn apply_commit_conflict(
    mut repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    mut prepared: Prepared,
    (commit, enrichment): (gix::objs::Commit, crate::enrich::Headers),
    report: impl FnMut(rebase::Progress),
) -> Result<rebase::Perform> {
    repo.objects.set_object_memory(std::mem::take(&mut prepared.objects));
    let (performed, _) = rebase::perform_with_enrichment_and_progress(
        &repo,
        graph,
        rebase::Edit::Insert {
            anchor: prepared.parent,
            commit,
            reset_index: prepared.reset_index,
        },
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::LeaveAsIsAndMark,
        &enrichment,
        report,
    )?;
    Ok(performed)
}

#[tracing::instrument(skip_all, fields(parent = ?prepared.parent))]
#[cfg(test)]
pub(crate) fn apply_fork(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    prepared: Prepared,
    edited: &[u8],
) -> Result<ObjectId> {
    apply_fork_reporting(repo, graph, prepared, edited)?
        .selected
        .context("forking a commit did not produce a selection")
}

pub(crate) fn apply_fork_reporting(
    mut repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    mut prepared: Prepared,
    edited: &[u8],
) -> Result<rebase::Outcome> {
    let repository_path = repo.git_dir().to_owned();
    let bare = repo.is_bare();
    repo.objects.set_object_memory(std::mem::take(&mut prepared.objects));
    let parent = prepared.parent.context("a fork commit requires a parent")?;
    let (commit, enrichment) = commit_from_edit(&prepared, edited)?;
    let mut outcome = rebase::perform(
        &repo,
        graph,
        rebase::Edit::Fork { anchor: parent, commit },
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::LeaveAsIs,
    )?
    .complete()?;
    let id = outcome
        .selected
        .context("forking a commit did not produce a selection")?;
    drop(repo);
    let repo = crate::open_repository(&repository_path, bare, false)?;
    let name: gix::refs::FullName = crate::enrich::REF_NAME.try_into().expect("valid enrich ref");
    let before = super::undo::state(&repo, name.as_ref())?;
    crate::enrich::apply_headers(&repo, id, &enrichment)
        .context("the fork was created, but its enrichment could not be saved")?;
    let after = super::undo::state(&repo, name.as_ref())?;
    if before != after {
        outcome.ref_changes.push(super::undo::RefChange { name, before, after });
    }
    Ok(outcome)
}

pub(super) fn commit_from_edit(
    prepared: &Prepared,
    edited: &[u8],
) -> Result<(gix::objs::Commit, crate::enrich::Headers)> {
    let edit = reword::parse(edited)?;
    commit_from_parsed_edit(prepared, edit)
}

fn commit_from_parsed_edit(
    prepared: &Prepared,
    edit: reword::Edit<'_>,
) -> Result<(gix::objs::Commit, crate::enrich::Headers)> {
    if edit.message.is_empty() {
        anyhow::bail!("the edited commit message is empty");
    }
    Ok((
        gix::objs::Commit {
            message: edit.message,
            tree: prepared.tree,
            author: reword::actor(edit.author, edit.author_time, "author")?,
            committer: reword::actor(edit.committer, edit.committer_time, "committer")?,
            encoding: None,
            parents: prepared.parent.into_iter().collect(),
            extra_headers: Vec::new(),
        },
        edit.enrichment,
    ))
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::*;

    fn open(path: &Path) -> gix_testtools::Result<gix::Repository> {
        Ok(crate::test_repository::open(path)?)
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
        assert_eq!(
            prepared
                .document
                .split(|byte| *byte == b'\n')
                .filter(|line| line.strip_prefix(b";").unwrap_or(line).starts_with(b"Author: "))
                .count(),
            1,
            "new-commit editors contain only the configured author"
        );
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

        let edited = prepared
            .document
            .replacen(b"what\n\nwhy", b"title\n\nbody", 1)
            .replacen(b";Todo\n;Message:", b"Todo\nMessage: created enrichment", 1);
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
        assert_eq!(
            crate::enrich::load(
                &mut crate::enrich::open(&repository)?,
                crate::change_id::for_commit(&repository, new_id)?
            )?,
            crate::enrich::Enrichment {
                todo: true,
                note: Some("created enrichment".into()),
            }
        );
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
            before.commits.len() + 2,
            "the history commit and its enrichment note commit are added"
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
        for args in [
            ["reset", "--hard", "-q", "HEAD"].as_slice(),
            ["clean", "-fdq"].as_slice(),
            ["commit", "--allow-empty", "-q", "-m", "descendant"].as_slice(),
        ] {
            assert!(
                Command::new("git")
                    .arg("-C")
                    .arg(fixture.path())
                    .args(args)
                    .status()?
                    .success(),
                "the fork fixture has a clean descendant"
            );
        }
        let repository = open(fixture.path())?;
        let main = repository.head_id()?.detach();
        let parent = repository.rev_parse_single("HEAD~1")?.detach();
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let prepared = prepare(open(fixture.path())?, Some(parent))?;
        let edited = prepared
            .document
            .replacen(b"what\n\nwhy", b"fork\n\nreason", 1)
            .replacen(b";Todo\n;Message:", b"Todo\nMessage: fork enrichment", 1);
        let graph = super::super::loaded_graph(&open(fixture.path())?)?;
        let fork = apply_fork(open(fixture.path())?, &graph, prepared, &edited)?;

        let repository = open(fixture.path())?;
        assert_eq!(
            crate::enrich::load(
                &mut crate::enrich::open(&repository)?,
                crate::change_id::for_commit(&repository, fork)?
            )?,
            crate::enrich::Enrichment {
                todo: true,
                note: Some("fork enrichment".into()),
            }
        );
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
        let after = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(
            after.index, before.index,
            "creating the fork leaves the index untouched"
        );
        assert_eq!(after.index_tree, before.index_tree, "the index tree remains unchanged");
        assert_eq!(
            after.worktree, before.worktree,
            "creating the fork leaves worktree files untouched"
        );

        let repository_path = repository.git_dir().to_owned();
        let graph = super::super::loaded_graph(&repository)?;
        drop(repository);
        super::super::time_travel::perform(&repository_path, false, fork, &graph, &[], &[], false)?.complete()?;
        let repository = open(fixture.path())?;
        assert!(repository.head()?.is_detached(), "automatic fork travel detaches HEAD");
        assert_eq!(repository.head_id()?.detach(), fork);
        let pins = crate::history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "the checkout consumes the temporary fork pin");
        assert!(pins[0].is_head(), "the singleton remembers the departed branch");
        assert_eq!(pins[0].id, main);
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
        let prepared = prepare_empty(open(fixture.path())?, None)?;
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
    fn explicit_empty_commit_preserves_index_and_worktree_changes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let parent = open(fixture.path())?.head_id()?.detach();
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let prepared = prepare_empty(open(fixture.path())?, Some(parent))?;
        assert!(prepared.is_empty, "the explicit commit reuses its parent's tree");
        let edited = prepared.document.replacen(b"what\n\nwhy", b"empty\n\nreason", 1);
        let graph = super::super::loaded_graph(&open(fixture.path())?)?;
        let new_id = apply(open(fixture.path())?, &graph, prepared, &edited)?;
        let repository = open(fixture.path())?;
        let commit = repository.find_commit(new_id)?;
        assert_eq!(
            commit.tree_id()?.detach(),
            repository.find_commit(parent)?.tree_id()?.detach(),
            "the explicit empty commit keeps the parent tree"
        );
        let after = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(after.index, before.index, "staged changes remain staged");
        assert_eq!(
            after.worktree, before.worktree,
            "worktree changes remain byte-identical"
        );
        Ok(())
    }

    #[test]
    fn implicit_new_commit_ignores_untracked_files() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["reset", "--hard", "-q", "HEAD"])
                .status()?
                .success(),
            "tracked files are restored while the untracked fixture remains"
        );
        let parent = open(fixture.path())?.head_id()?.detach();
        let prepared = prepare(open(fixture.path())?, Some(parent))?;
        assert!(prepared.is_empty, "untracked files do not enter an implicit new commit");
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
