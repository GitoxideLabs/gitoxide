use anyhow::{Context, Result};
use gix::{ObjectId, bstr::ByteSlice};

use crate::{
    ChangeGroup, ChangeKind, ComparedParent, load_tree_changes_without_lines, load_worktree_changes_without_lines,
};

use super::{rebase, reword};

pub(crate) struct Prepared {
    pub editor: Option<gix::command::Prepare>,
    pub document: Vec<u8>,
    pub(super) parent: Option<ObjectId>,
    pub(super) tree: ObjectId,
    pub(super) objects: gix::odb::memory::Storage,
    pub(crate) is_empty: bool,
    pub(super) reset_index: bool,
    below: Option<gix::objs::Commit>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    Normal,
    Empty,
    Below,
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

pub(crate) fn prepare_below(mut repo: gix::Repository, target: ObjectId) -> Result<Prepared> {
    let mut upper = below_head(&repo, target)?;
    let mut prepared = prepare(repo.clone(), Some(target))?;
    repo.objects.set_object_memory(std::mem::take(&mut prepared.objects));
    let parent_tree_id = match upper.parents.first() {
        Some(parent_commit_id) => repo.find_commit(*parent_commit_id)?.tree_id()?.detach(),
        None => repo.empty_tree().id,
    };
    // Move only the local delta, then prove that replaying HEAD preserves the combined tree.
    let lower_tree_id = rebase::cherry_pick_tree(&repo, upper.tree, parent_tree_id, prepared.tree)
        .context("the new changes conflict when moved below HEAD")?;
    let upper_tree_id = rebase::cherry_pick_tree(&repo, parent_tree_id, lower_tree_id, upper.tree)
        .context("HEAD conflicts when replayed above the new commit")?;
    anyhow::ensure!(
        upper_tree_id == prepared.tree,
        "inserting below HEAD would change the combined tree"
    );
    upper.tree = upper_tree_id;
    prepared.tree = lower_tree_id;
    prepared.below = Some(upper);
    prepared.objects = repo
        .objects
        .take_object_memory()
        .context("candidate object memory was unavailable")?;
    Ok(prepared)
}

pub(super) fn below_head(repo: &gix::Repository, target: ObjectId) -> Result<gix::objs::Commit> {
    anyhow::ensure!(repo.head_id()? == target, "new-below requires the current HEAD");
    let commit = repo.find_commit(target)?.decode()?.into_owned()?;
    super::auto_merge::ensure_editable(&commit)?;
    anyhow::ensure!(
        commit.parents.len() <= 1 && !super::review::is_review(&commit),
        "new-below requires an ordinary root or single-parent commit"
    );
    anyhow::ensure!(
        !rebase::is_pending(&commit),
        "finish the pending or conflicting HEAD before creating a commit below it"
    );
    if let Some(parent_commit_id) = commit.parents.first() {
        let parent = repo.find_commit(*parent_commit_id)?.decode()?.into_owned()?;
        anyhow::ensure!(
            super::auto_merge::is_auto_merge(&parent) || !rebase::is_pending(&parent),
            "the new commit's parent has a pending rebase; finish it before creating a commit"
        );
    }
    Ok(commit)
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
    let editor = repo
        .editor_command()
        .context("could not prepare Git editor")?
        .context("no Git editor is available")?;
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
    let changes = load_tree_changes_without_lines(
        &repo,
        parent.map(|_| &baseline),
        &new_tree,
        parent.map(|id| ComparedParent { index: 0, total: 1, id }),
    )?;
    let mut document = Vec::new();
    reword::write_headers(
        &mut document,
        &author,
        None,
        &committer,
        &crate::enrich::Enrichment {
            todo,
            ..Default::default()
        },
    )?;
    document.extend_from_slice(b"\nwhat\n\nwhy\n");
    reword::write_missing_agent_trailers(&mut document, &repo, b"what\n\nwhy\n")?;
    document.extend_from_slice(b"\n; Changes to be committed:\n");
    reword::write_diff_summary(&mut document, &repo, changes)?;
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
        editor: Some(editor),
        document,
        parent,
        tree,
        objects,
        is_empty: tree == baseline_id,
        reset_index: !empty,
        below: None,
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
    (mut commit, enrichment): (gix::objs::Commit, crate::enrich::Headers),
    report: impl FnMut(rebase::Progress),
) -> Result<rebase::Perform> {
    repo.objects.set_object_memory(std::mem::take(&mut prepared.objects));
    let edit = match prepared.below {
        Some(upper) => {
            commit.parents.clone_from(&upper.parents);
            rebase::Edit::InsertBelow {
                target: prepared.parent.context("new-below requires a HEAD commit")?,
                lower: commit,
                upper,
            }
        }
        None => rebase::Edit::Insert {
            anchor: prepared.parent,
            commit,
            reset_index: prepared.reset_index,
        },
    };
    let (performed, _) = rebase::perform_with_enrichment_and_progress(
        &repo,
        graph,
        edit,
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::LeaveAsIsAndMark,
        &enrichment,
        report,
    )?;
    Ok(performed)
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
    use std::path::Path;

    use super::*;

    fn open(path: &Path) -> gix_testtools::Result<gix::Repository> {
        Ok(crate::test_repository::open(path)?)
    }

    fn object_count(path: &Path) -> gix_testtools::Result<Vec<u8>> {
        let output = gix_testtools::git_command(path)
            .args(["count-objects", "-v"])
            .output()?;
        if !output.status.success() {
            return Err(format!("git count-objects failed: {}", output.stderr.to_str_lossy()).into());
        }
        Ok(output.stdout)
    }

    fn pending_child(repo: &gix::Repository) -> gix_testtools::Result<ObjectId> {
        let base_commit_id = repo.head_id()?.detach();
        let mut commit = repo.find_commit(base_commit_id)?.decode()?.into_owned()?;
        commit.parents = [base_commit_id].into_iter().collect();
        commit.message = "pending child\n".into();
        commit
            .extra_headers
            .push(("tix-rebase-parent".into(), base_commit_id.to_string().into()));
        let pending_commit_id = repo.write_object(&commit)?.detach();
        repo.reference(
            "refs/heads/pending",
            pending_commit_id,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "retain the pending ancestor",
        )?;
        Ok(pending_commit_id)
    }

    #[test]
    fn new_and_empty_commits_preserve_older_pending_ancestry() -> gix_testtools::Result {
        for empty in [false, true] {
            for pending_parent_index in [None, Some(0), Some(1)] {
                let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
                let repository = open(fixture.path())?;
                let base_commit_id = repository.head_id()?.detach();
                let pending_commit_id = pending_child(&repository)?;
                let mut parent = repository.find_commit(base_commit_id)?.decode()?.into_owned()?;
                parent.parents = [pending_commit_id].into_iter().collect();
                parent.message = "finalized parent\n".into();
                if let Some(index) = pending_parent_index {
                    let mut side = repository.find_commit(base_commit_id)?.decode()?.into_owned()?;
                    side.parents = [base_commit_id].into_iter().collect();
                    side.message = "finalized side\n".into();
                    let side_commit_id = repository.write_object(&side)?.detach();
                    parent.parents = [side_commit_id].into_iter().collect();
                    parent.parents.insert(index, pending_commit_id);
                }
                let parent_commit_id = repository.write_object(&parent)?.detach();
                repository
                    .find_reference("refs/heads/main")?
                    .set_target_id(parent_commit_id, "check out finalized ancestry")?;
                let graph = super::super::loaded_graph(&repository)?;
                let before = gix_testtools::repository::snapshot(fixture.path())?;
                let index_before = std::fs::read(repository.index_path())?;
                let prepared = if empty {
                    prepare_empty(open(fixture.path())?, Some(parent_commit_id))?
                } else {
                    prepare(open(fixture.path())?, Some(parent_commit_id))?
                };
                let edited = prepared.document.replacen(b"what\n\nwhy", b"new child\n\nreason", 1);
                let outcome = apply_reporting(open(fixture.path())?, &graph, prepared, &edited)?;
                let new_commit_id = outcome.selected.context("creation selects the new child")?;
                let repository = open(fixture.path())?;
                let commit = repository.find_commit(new_commit_id)?;
                assert_eq!(
                    commit.parent_ids().map(gix::Id::detach).collect::<Vec<_>>(),
                    [parent_commit_id],
                    "creation retains the exact selected parent above pending ancestry"
                );
                assert_eq!(
                    repository.find_reference("refs/heads/pending")?.id(),
                    pending_commit_id,
                    "creating a child never moves an older pending ancestor's ref"
                );
                assert!(
                    rebase::is_pending(&repository.find_commit(pending_commit_id)?.decode()?.into_owned()?),
                    "older pending state is preserved instead of finalized"
                );
                let after = gix_testtools::repository::snapshot(fixture.path())?;
                assert_eq!(
                    after.commits.len(),
                    before.commits.len() + 1,
                    "only the new child is added"
                );
                for original in &before.commits {
                    assert!(
                        after.commits.contains(original),
                        "every original ancestor remains reachable with its exact object bytes"
                    );
                }
                assert_eq!(after.index, before.index, "staged content remains intact");
                assert_eq!(
                    after.worktree, before.worktree,
                    "worktree contents remain byte-identical"
                );
                if empty {
                    assert_eq!(
                        commit.tree_id()?,
                        parent.tree,
                        "the empty child keeps its parent's tree"
                    );
                    assert_eq!(
                        std::fs::read(repository.index_path())?,
                        index_before,
                        "empty creation preserves the complete index file"
                    );
                } else {
                    assert_eq!(
                        Some(commit.tree_id()?.detach()),
                        before.index_tree,
                        "normal creation commits the staged tree"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn new_and_empty_commits_reject_the_selected_pending_parent_without_changes() -> gix_testtools::Result {
        for empty in [false, true] {
            for parent_is_head in [false, true] {
                let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
                let repository = open(fixture.path())?;
                let parent_commit_id = pending_child(&repository)?;
                if parent_is_head {
                    repository
                        .find_reference("refs/heads/main")?
                        .set_target_id(parent_commit_id, "check out the pending parent")?;
                }
                let graph = super::super::loaded_graph(&repository)?;
                let before = gix_testtools::repository::snapshot(fixture.path())?;
                let objects_before = object_count(fixture.path())?;
                let index_before = std::fs::read(repository.index_path())?;
                let prepared = if empty {
                    prepare_empty(open(fixture.path())?, Some(parent_commit_id))?
                } else {
                    prepare(open(fixture.path())?, Some(parent_commit_id))?
                };
                let edited = prepared
                    .document
                    .replacen(b"what\n\nwhy", b"rejected child\n\nreason", 1);
                let err = apply(open(fixture.path())?, &graph, prepared, &edited)
                    .expect_err("a pending selected parent cannot gain a new child");
                assert!(
                    format!("{err:#}").contains("the selected parent has a pending rebase"),
                    "the selected parent is checked even when another commit is checked out: {err:#}"
                );
                assert_eq!(
                    gix_testtools::repository::snapshot(fixture.path())?,
                    before,
                    "rejected creation leaves every ref, reachable commit, index entry, and worktree file unchanged"
                );
                assert_eq!(
                    object_count(fixture.path())?,
                    objects_before,
                    "rejection writes no loose objects"
                );
                assert_eq!(
                    std::fs::read(repository.index_path())?,
                    index_before,
                    "rejection preserves the complete index file"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn preparation_is_unobservable_and_staged_changes_win() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let parent = open(fixture.path())?.head_id()?.detach();
        for name in ["refs/patches/create", "refs/tags/keep", "refs/remotes/origin/keep"] {
            assert!(
                gix_testtools::git_command(fixture.path())
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
        let trailers = b";Assisted-by: GPT 5.6\n\
                         ;Co-authored-by: GPT 5.6 <codex@openai.com>\n\
                         ; tix.trailer.assistedBy is unset; using the built-in default.\n\
                         ; tix.trailer.coAuthoredBy is unset; using the built-in default.\n";
        assert!(
            prepared
                .document
                .windows(trailers.len())
                .any(|window| window == trailers),
            "new-commit suggestions stay adjacent and explain their defaults: {}",
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
            gix_testtools::git_command(fixture.path())
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
                repository.find_reference(name)?.id(),
                new_id,
                "{name} advances to the new commit"
            );
        }
        for name in ["refs/tags/keep", "refs/remotes/origin/keep"] {
            assert_eq!(repository.find_reference(name)?.id(), parent, "{name} is not edited");
        }
        Ok(())
    }

    #[test]
    fn worktree_changes_supply_the_tree_when_the_index_is_unchanged() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        assert!(
            gix_testtools::git_command(fixture.path())
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
    fn creates_an_empty_root_commit_for_an_unborn_head() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        assert!(
            gix_testtools::git_command(fixture.path())
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
            commit.tree_id()?,
            ObjectId::empty_tree(repository.object_hash()),
            "no index or worktree changes reuse the empty tree"
        );
        assert_eq!(
            repository.head_name()?.expect("HEAD is attached"),
            "refs/heads/main",
            "the unborn branch is created and remains checked out"
        );
        Ok(())
    }

    #[test]
    fn creates_the_unborn_branch_above_an_existing_base() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = open(fixture.path())?;
        let base = repository.rev_parse_single("main")?.detach();
        let graph = crate::history::HistoryGraph::for_commits(&repository, &[base])?;
        drop(repository);
        assert!(
            gix_testtools::git_command(fixture.path())
                .args(["symbolic-ref", "HEAD", "refs/heads/unborn"])
                .status()?
                .success()
        );

        let prepared = prepare_empty(open(fixture.path())?, Some(base))?;
        let edited = prepared.document.replacen(b"what\n\nwhy", b"first\n\nreason", 1);
        let new_id = apply(open(fixture.path())?, &graph, prepared, &edited)?;
        let repository = open(fixture.path())?;

        assert_eq!(repository.head_id()?, new_id);
        assert_eq!(repository.find_reference("refs/heads/main")?.id(), base);
        assert_eq!(
            repository.find_commit(new_id)?.parent_ids().next().map(gix::Id::detach),
            Some(base),
            "the first commit is based on the selected hidden tip"
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
            commit.tree_id()?,
            repository.find_commit(parent)?.tree_id()?,
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
        crate::test_repository::disable_autocrlf(fixture.path())?;
        assert!(
            gix_testtools::git_command(fixture.path())
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
            gix_testtools::git_command(fixture.path())
                .args(["checkout", "-q", "--orphan", "other"])
                .status()?
                .success()
        );
        assert!(
            gix_testtools::git_command(fixture.path())
                .args(["rm", "-rf", "-q", "."])
                .status()?
                .success()
        );
        std::fs::write(fixture.path().join("other"), b"other\n")?;
        assert!(
            gix_testtools::git_command(fixture.path())
                .args(["add", "other"])
                .status()?
                .success()
        );
        assert!(
            gix_testtools::git_command(fixture.path())
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
            open(fixture.path())?.find_reference("refs/heads/main")?.id(),
            new_id,
            "the selected parent branch advances independently"
        );
        Ok(())
    }

    fn below_git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = gix_testtools::git_command(path).args(args).output()?;
        if !output.status.success() {
            return Err(format!("git {} failed: {}", args.join(" "), output.stderr.to_str_lossy()).into());
        }
        Ok(output.stdout)
    }

    fn below_child(repository: &gix::Repository, parent_commit_id: ObjectId, path: &str) -> Result<ObjectId> {
        let parent = repository.find_commit(parent_commit_id)?;
        let mut tree = parent.tree()?.edit()?;
        tree.upsert(path, gix::objs::tree::EntryKind::Blob, repository.write_blob(path)?)?;
        Ok(repository.new_commit(path, tree.write()?, [parent_commit_id])?.id)
    }

    #[test]
    fn new_below_preserves_head_identity_and_uncommitted_remainder() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_below.sh")?;
        let repository = open(fixture.path())?;
        let head_commit_id = repository.head_id()?.detach();
        let parent_commit_id = repository
            .find_commit(head_commit_id)?
            .parent_ids()
            .next()
            .context("HEAD has a parent")?
            .detach();
        let sibling_commit_id = below_child(&repository, parent_commit_id, "sibling")?;
        let descendant_commit_id = below_child(&repository, head_commit_id, "descendant")?;
        for (name, commit_id) in [("sibling", sibling_commit_id), ("descendant", descendant_commit_id)] {
            repository.reference(
                format!("refs/heads/{name}"),
                commit_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "retain fixture fork",
            )?;
        }
        let head_change_id = crate::change_id::for_commit(&repository, head_commit_id)?;
        let mut notes = repository.notes()?;
        let notes_ref = notes
            .default_ref()
            .context("the fixture has a default notes ref")?
            .to_owned();
        notes.replace_at_ref(notes_ref.as_ref(), head_commit_id, b"HEAD note")?;
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let objects_before = object_count(fixture.path())?;
        let modified_before = ["shared", "keep", "head", "untracked"]
            .map(|path| std::fs::metadata(fixture.path().join(path))?.modified())
            .into_iter()
            .collect::<std::io::Result<Vec<_>>>()?;
        let prepared = prepare_below(open(fixture.path())?, head_commit_id)?;
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "preparation leaves repository state intact"
        );
        assert_eq!(
            object_count(fixture.path())?,
            objects_before,
            "preparation writes no objects"
        );
        let edited = prepared
            .document
            .replacen(b"what\n\nwhy", b"lower\n\nreason", 1)
            .replacen(b";Todo\n;Message:", b"Todo\nMessage: lower enrichment", 1);
        let graph = super::super::loaded_graph(&repository)?;
        let outcome = apply_reporting(open(fixture.path())?, &graph, prepared, &edited)?;
        let lower_commit_id = outcome.selected.context("new-below selects its new commit")?;
        let upper_commit_id = outcome
            .map(head_commit_id)
            .context("HEAD survives above the new commit")?;
        let repository = open(fixture.path())?;
        assert_eq!(
            repository.head_id()?,
            upper_commit_id,
            "HEAD follows its rewritten occurrence"
        );
        assert_eq!(
            repository
                .find_commit(upper_commit_id)?
                .parent_ids()
                .map(gix::Id::detach)
                .collect::<Vec<_>>(),
            [lower_commit_id],
            "HEAD sits immediately above the new commit"
        );
        assert_eq!(
            repository
                .find_commit(lower_commit_id)?
                .parent_ids()
                .map(gix::Id::detach)
                .collect::<Vec<_>>(),
            [parent_commit_id],
            "the new commit keeps the old parent"
        );
        assert_eq!(
            below_git(fixture.path(), &["show", &format!("{lower_commit_id}:shared")])?,
            b"staged\none\ntwo\nthree\nbase unstaged\n",
            "only staged hunks enter the new commit"
        );
        assert!(
            repository
                .find_commit(lower_commit_id)?
                .tree()?
                .find_entry("head")
                .is_none(),
            "HEAD's original addition stays in HEAD"
        );
        assert_eq!(
            crate::change_id::for_commit(&repository, upper_commit_id)?,
            head_change_id,
            "HEAD keeps its change identity"
        );
        let lower_change_id = crate::change_id::for_commit(&repository, lower_commit_id)?;
        assert_ne!(
            lower_change_id, head_change_id,
            "the new commit receives its own change identity"
        );
        assert_eq!(
            repository.find_commit(upper_commit_id)?.message_raw()?,
            b"head\n".as_bstr(),
            "HEAD keeps its original message"
        );
        assert_eq!(
            crate::enrich::load(&mut crate::enrich::open(&repository)?, lower_change_id)?,
            crate::enrich::Enrichment {
                todo: true,
                note: Some("lower enrichment".into())
            },
            "editor enrichment belongs to the new lower commit"
        );
        let mut notes = repository.notes()?.with_refs([notes_ref.as_bstr()])?;
        assert_eq!(
            notes
                .get(upper_commit_id)?
                .first()
                .map(|note| note.blob.data.as_slice()),
            Some(b"HEAD note".as_slice()),
            "HEAD's note follows its rewrite"
        );
        assert!(
            notes.get(lower_commit_id)?.is_empty(),
            "HEAD's note is not copied onto the new commit"
        );
        assert_eq!(
            repository.find_reference("refs/heads/sibling")?.id(),
            sibling_commit_id,
            "the parent's other children stay untouched"
        );
        let descendant_commit_id = outcome
            .map(descendant_commit_id)
            .context("the descendant is retained")?;
        let descendant = repository.find_commit(descendant_commit_id)?.decode()?.into_owned()?;
        assert_eq!(
            descendant.parents.as_slice(),
            [upper_commit_id],
            "descendants follow the rewritten HEAD"
        );
        assert!(
            rebase::is_pending(&descendant),
            "content-changing descendants remain lazy"
        );
        let after = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(
            after.index_tree, before.index_tree,
            "the committed staged tree remains the index tree"
        );
        assert_eq!(
            after.worktree, before.worktree,
            "unstaged and untracked bytes remain untouched"
        );
        let modified_after = ["shared", "keep", "head", "untracked"]
            .map(|path| std::fs::metadata(fixture.path().join(path))?.modified())
            .into_iter()
            .collect::<std::io::Result<Vec<_>>>()?;
        assert_eq!(
            modified_after, modified_before,
            "successful insertion never rewrites worktree files"
        );
        assert_eq!(
            below_git(fixture.path(), &["diff", "--cached", "--name-only"])?,
            b"",
            "the selected staged hunks are consumed"
        );
        assert_eq!(
            below_git(fixture.path(), &["diff", "--name-only"])?,
            b"shared\n",
            "the same-file unstaged remainder survives"
        );
        Ok(())
    }

    #[test]
    fn new_below_uses_tracked_worktree_changes_and_can_insert_a_root() -> gix_testtools::Result {
        for root in [false, true] {
            let fixture =
                gix_testtools::scripted_fixture_writable(if root { "create_commit.sh" } else { "create_below.sh" })?;
            if root {
                below_git(fixture.path(), &["reset", "--hard", "-q", "HEAD", "--"])?;
                std::fs::write(fixture.path().join("new-root"), "new root\n")?;
                below_git(fixture.path(), &["add", "new-root"])?;
            } else {
                below_git(fixture.path(), &["reset", "-q", "HEAD", "--"])?;
            }
            let repository = open(fixture.path())?;
            let head_commit_id = repository.head_id()?.detach();
            let original_parents = repository
                .find_commit(head_commit_id)?
                .parent_ids()
                .map(gix::Id::detach)
                .collect::<Vec<_>>();
            let before = gix_testtools::repository::snapshot(fixture.path())?;
            let prepared = prepare_below(open(fixture.path())?, head_commit_id)?;
            let edited = prepared.document.replacen(b"what\n\nwhy", b"below\n\nreason", 1);
            let graph = super::super::loaded_graph(&repository)?;
            let lower_commit_id = apply(open(fixture.path())?, &graph, prepared, &edited)?;
            let repository = open(fixture.path())?;
            assert_eq!(
                repository
                    .find_commit(lower_commit_id)?
                    .parent_ids()
                    .map(gix::Id::detach)
                    .collect::<Vec<_>>(),
                original_parents,
                "the lower commit inherits all original ancestry, including no parent"
            );
            assert!(
                repository
                    .find_commit(lower_commit_id)?
                    .tree()?
                    .find_entry("untracked")
                    .is_none(),
                "implicit creation excludes untracked files"
            );
            if root {
                assert_eq!(
                    below_git(
                        fixture.path(),
                        &["ls-tree", "--name-only", &lower_commit_id.to_string()]
                    )?,
                    b"new-root\n",
                    "the new root contains only the selected addition"
                );
            } else {
                assert_eq!(
                    below_git(fixture.path(), &["show", &format!("{lower_commit_id}:shared")])?,
                    b"staged\none\ntwo\nthree\nunstaged\n",
                    "an unchanged index selects all tracked worktree hunks"
                );
            }
            assert_eq!(
                gix_testtools::repository::snapshot(fixture.path())?.worktree,
                before.worktree,
                "creation leaves all worktree bytes in place"
            );
            assert_eq!(
                below_git(fixture.path(), &["status", "--short"])?,
                b"?? untracked\n",
                "committed changes leave only untracked files"
            );
        }
        Ok(())
    }

    #[test]
    fn new_below_refuses_conflicting_or_lossy_reordering_without_writes() -> gix_testtools::Result {
        for lossy in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("create_below.sh")?;
            below_git(fixture.path(), &["reset", "--hard", "-q", "HEAD", "--"])?;
            if lossy {
                below_git(fixture.path(), &["rm", "-q", "head"])?;
            } else {
                std::fs::write(fixture.path().join("shared"), "HEAD content\n")?;
                below_git(fixture.path(), &["add", "shared"])?;
                below_git(fixture.path(), &["commit", "--amend", "--no-edit", "-q"])?;
                std::fs::write(fixture.path().join("shared"), "selected content\n")?;
                below_git(fixture.path(), &["add", "shared"])?;
            }
            let repository = open(fixture.path())?;
            let before = gix_testtools::repository::snapshot(fixture.path())?;
            let objects_before = object_count(fixture.path())?;
            let index_before = std::fs::read(repository.index_path())?;
            let err = prepare_below(open(fixture.path())?, repository.head_id()?.detach())
                .err()
                .context("dependent changes cannot be moved below HEAD")?;
            assert!(
                format!("{err:#}").contains(if lossy { "combined tree" } else { "conflict" }),
                "the failure explains the unsafe reorder: {err:#}"
            );
            assert_eq!(
                gix_testtools::repository::snapshot(fixture.path())?,
                before,
                "failed preparation preserves refs, history, index entries, and files"
            );
            assert_eq!(
                object_count(fixture.path())?,
                objects_before,
                "failed preparation publishes no objects"
            );
            assert_eq!(
                std::fs::read(repository.index_path())?,
                index_before,
                "failed preparation preserves the complete index file"
            );
        }
        Ok(())
    }

    #[test]
    fn new_below_checks_only_head_and_its_immediate_parent_for_pending_rebases() -> gix_testtools::Result {
        for depth in 0..3 {
            let fixture = gix_testtools::scripted_fixture_writable("create_below.sh")?;
            let repository = open(fixture.path())?;
            let old_head_commit_id = repository.head_id()?.detach();
            let mut head = repository.find_commit(old_head_commit_id)?.decode()?.into_owned()?;
            let parent_commit_id = head.parents[0];
            let mut pending = repository.find_commit(parent_commit_id)?.decode()?.into_owned()?;
            pending.parents = [parent_commit_id].into_iter().collect();
            pending.message = "pending ancestor\n".into();
            pending
                .extra_headers
                .push(("tix-rebase-parent".into(), parent_commit_id.to_string().into()));
            let pending_commit_id = repository.write_object(&pending)?.detach();
            if depth == 0 {
                head.extra_headers
                    .push(("tix-rebase-parent".into(), parent_commit_id.to_string().into()));
            } else if depth == 1 {
                head.parents = [pending_commit_id].into_iter().collect();
            } else {
                let mut finalized = pending.clone();
                finalized.parents = [pending_commit_id].into_iter().collect();
                finalized.extra_headers.clear();
                finalized.message = "finalized parent\n".into();
                head.parents = [repository.write_object(&finalized)?.detach()].into_iter().collect();
            }
            let head_commit_id = repository.write_object(&head)?.detach();
            repository
                .find_reference("refs/heads/main")?
                .set_target_id(head_commit_id, "prepare pending ancestry")?;
            repository.reference(
                "refs/heads/pending",
                pending_commit_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "retain pending ancestor",
            )?;
            let before = gix_testtools::repository::snapshot(fixture.path())?;
            let objects_before = object_count(fixture.path())?;
            if depth < 2 {
                let err = prepare_below(open(fixture.path())?, head_commit_id)
                    .err()
                    .context("pending HEAD and direct parent must be refused")?;
                assert!(
                    format!("{err:#}").contains("pending"),
                    "the rejection identifies pending history: {err:#}"
                );
                assert_eq!(
                    gix_testtools::repository::snapshot(fixture.path())?,
                    before,
                    "pending rejection leaves the repository intact"
                );
                assert_eq!(
                    object_count(fixture.path())?,
                    objects_before,
                    "pending rejection writes no objects"
                );
            } else {
                let prepared = prepare_below(open(fixture.path())?, head_commit_id)?;
                let edited = prepared.document.replacen(b"what\n\nwhy", b"below\n\nreason", 1);
                let graph = super::super::loaded_graph(&repository)?;
                let outcome = apply_reporting(open(fixture.path())?, &graph, prepared, &edited)?;
                assert_eq!(
                    outcome.map(pending_commit_id),
                    Some(pending_commit_id),
                    "older pending ancestry is never rewritten"
                );
                assert_eq!(
                    repository.find_reference("refs/heads/pending")?.id(),
                    pending_commit_id,
                    "older pending references stay fixed"
                );
                assert!(
                    rebase::is_pending(&repository.find_commit(pending_commit_id)?.decode()?.into_owned()?),
                    "older pending commits remain pending"
                );
                let lower_commit_id = outcome.selected.context("creation selects the lower commit")?;
                assert_eq!(
                    repository
                        .find_commit(lower_commit_id)?
                        .parent_ids()
                        .map(gix::Id::detach)
                        .collect::<Vec<_>>(),
                    head.parents.as_slice(),
                    "the lower commit retains the exact finalized parent"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn new_below_preserves_linked_worktree_staging_and_refuses_conflicting_changes() -> gix_testtools::Result {
        for conflict in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("create_below.sh")?;
            let linked_root = gix_testtools::tempfile::tempdir()?;
            let linked = linked_root.path().join("linked");
            let output = gix_testtools::git_command(fixture.path())
                .args(["worktree", "add", "-q", "-b", "linked"])
                .arg(&linked)
                .arg("HEAD")
                .output()?;
            assert!(
                output.status.success(),
                "the disposable linked worktree is created: {}",
                output.stderr.to_str_lossy()
            );
            let path = if conflict { "shared" } else { "keep" };
            std::fs::write(linked.join(path), "linked staged\n")?;
            below_git(&linked, &["add", path])?;
            std::fs::write(linked.join(path), "linked unstaged\n")?;
            let repository = open(fixture.path())?;
            let head_commit_id = repository.head_id()?.detach();
            let graph = super::super::loaded_graph(&repository)?;
            let before = gix_testtools::repository::snapshot(fixture.path())?;
            let linked_before = gix_testtools::repository::snapshot(&linked)?;
            let index_before = std::fs::read(repository.index_path())?;
            let linked_index_path = open(&linked)?.index_path();
            let linked_index_before = std::fs::read(&linked_index_path)?;
            let prepared = prepare_below(open(fixture.path())?, head_commit_id)?;
            let edited = prepared.document.replacen(b"what\n\nwhy", b"below\n\nreason", 1);
            let result = apply_reporting(open(fixture.path())?, &graph, prepared, &edited);
            if conflict {
                assert!(
                    result.is_err(),
                    "conflicting linked staging refuses the whole insertion"
                );
                assert_eq!(
                    gix_testtools::repository::snapshot(fixture.path())?,
                    before,
                    "linked conflict leaves active refs, index, and worktree unchanged"
                );
                assert_eq!(
                    gix_testtools::repository::snapshot(&linked)?,
                    linked_before,
                    "linked conflict leaves the other worktree unchanged"
                );
                assert_eq!(
                    std::fs::read(repository.index_path())?,
                    index_before,
                    "active index bytes survive linked conflict"
                );
                assert_eq!(
                    std::fs::read(linked_index_path)?,
                    linked_index_before,
                    "linked index bytes survive conflict preflight"
                );
            } else {
                let outcome = result?;
                let upper_commit_id = outcome.map(head_commit_id).context("HEAD is rewritten")?;
                assert_eq!(
                    open(&linked)?.head_id()?,
                    upper_commit_id,
                    "the linked branch follows the rewritten HEAD"
                );
                assert_eq!(
                    below_git(&linked, &["show", ":keep"])?,
                    b"linked staged\n",
                    "unrelated linked staging survives the tree transition"
                );
                assert_eq!(
                    std::fs::read(linked.join("keep"))?,
                    b"linked unstaged\n",
                    "the linked worktree keeps its unstaged remainder"
                );
                assert_eq!(
                    std::fs::read(linked.join("shared"))?,
                    b"staged\none\ntwo\nthree\nbase unstaged\n",
                    "the linked checkout receives the newly committed changes"
                );
                assert_eq!(
                    gix_testtools::repository::snapshot(fixture.path())?.worktree,
                    before.worktree,
                    "updating another checkout never touches active worktree bytes"
                );
            }
        }
        Ok(())
    }
}
