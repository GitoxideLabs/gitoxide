use anyhow::{Context, Result};
use gix::ObjectId;

use super::{create, rebase};
use crate::{ChangeGroup, ChangeKind, load_worktree_changes_without_lines};

pub(crate) struct Prepared {
    pub editor: std::ffi::OsString,
    pub document: Vec<u8>,
    create: create::Prepared,
    target: ObjectId,
    source: gix::objs::Commit,
    tree: ObjectId,
}

#[tracing::instrument(skip_all)]
pub(crate) fn prepare(mut repo: gix::Repository) -> Result<Prepared> {
    let target = repo
        .head_id()
        .context("splitting requires an existing HEAD commit")?
        .detach();
    let changes = load_worktree_changes_without_lines(&repo)?;
    if changes.paths.iter().any(|change| change.kind == ChangeKind::Unmerged) {
        anyhow::bail!("cannot split with unresolved conflicts");
    }
    if !changes.paths.iter().any(|change| change.group == ChangeGroup::Staged)
        || !changes.paths.iter().any(|change| change.group == ChangeGroup::Unstaged)
    {
        anyhow::bail!("splitting requires both staged and worktree changes");
    }

    let mut source = repo
        .find_commit(target)
        .context("could not find HEAD commit")?
        .decode()
        .context("could not decode HEAD commit")?
        .into_owned()
        .context("could not own HEAD commit")?;
    let mut create = create::prepare(repo.clone(), Some(target))?;
    repo.objects.set_object_memory(std::mem::take(&mut create.objects));

    let head_tree = source.tree;
    let index_tree = create.tree;
    let index = repo
        .find_tree(index_tree)
        .context("could not load the prepared index tree")?;
    let worktree_tree = create::worktree_tree_with_changes(&repo, &index, &changes)?;
    drop(index);
    let source_tree = rebase::cherry_pick_tree(&repo, index_tree, head_tree, worktree_tree)
        .context("worktree changes conflict with the source commit")?;
    let tree = rebase::cherry_pick_tree(&repo, head_tree, source_tree, index_tree)
        .context("staged changes conflict with the rewritten source commit")?;
    source.tree = source_tree;
    create.objects = repo
        .objects
        .take_object_memory()
        .context("candidate object memory was unavailable")?;

    Ok(Prepared {
        editor: create.editor.clone(),
        document: create.document.clone(),
        create,
        target,
        source,
        tree,
    })
}

#[tracing::instrument(skip_all, fields(target = %prepared.target))]
pub(crate) fn apply(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    prepared: Prepared,
    edited: &[u8],
) -> Result<ObjectId> {
    apply_reporting(repo, graph, prepared, edited)?
        .selected
        .context("splitting HEAD did not produce a selection")
}

pub(crate) fn apply_reporting(
    mut repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    mut prepared: Prepared,
    edited: &[u8],
) -> Result<rebase::Outcome> {
    let repository_path = repo.git_dir().to_owned();
    let bare = repo.is_bare();
    repo.objects
        .set_object_memory(std::mem::take(&mut prepared.create.objects));
    let (mut upper, enrichment) = create::commit_from_edit(&prepared.create, edited)?;
    upper.tree = prepared.tree;
    let outcome = rebase::perform(
        &repo,
        graph,
        rebase::Edit::Split {
            target: prepared.target,
            source: prepared.source,
            upper,
        },
        rebase::Signature::InvalidateExisting,
        rebase::Tree::LeaveAsIsAndMark,
    )?
    .complete()?;
    let id = outcome.selected.context("splitting HEAD did not produce a selection")?;
    drop(repo);
    crate::enrich::apply_headers(&crate::open_repository(&repository_path, bare, false)?, id, &enrichment)
        .context("the commit was split, but its enrichment could not be saved")?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use gix::bstr::ByteSlice;

    use super::*;

    fn open(path: &Path) -> gix_testtools::Result<gix::Repository> {
        Ok(crate::test_repository::open_with(
            path,
            ["user.name=editor", "user.email=editor@example.com"],
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
    fn splits_staged_and_worktree_changes_without_touching_files_during_preparation() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("split_commit.sh")?;
        let repository = open(fixture.path())?;
        let old = repository.head_id()?.detach();
        let old_parent = repository.find_commit(old)?.parent_ids().next().map(gix::Id::detach);
        for name in ["refs/patches/split", "refs/tags/keep", "refs/remotes/origin/keep"] {
            git(fixture.path(), &["update-ref", name, &old.to_string()])?;
        }
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let prepared = prepare(open(fixture.path())?)?;
        assert!(
            prepared
                .document
                .windows(b";Author: \xf0\x9f\x9a\xa7WIP\xf0\x9f\x9a\xa7 <wip@invalid>".len())
                .any(|window| window == b";Author: \xf0\x9f\x9a\xa7WIP\xf0\x9f\x9a\xa7 <wip@invalid>"),
            "split editors offer the provisional author"
        );
        assert!(
            prepared
                .document
                .windows(b"staged".len())
                .any(|window| window == b"staged"),
            "the upper commit editor describes staged paths"
        );
        assert!(
            !prepared
                .document
                .windows(b"unstaged |".len())
                .any(|window| window == b"unstaged |"),
            "the upper commit editor excludes worktree-only paths"
        );
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "preparing both cherry-picks leaves the repository unchanged"
        );

        let edited = prepared
            .document
            .replacen(b"what\n\nwhy", b"upper\n\nreason", 1)
            .replacen(b";Todo\n;Message:", b"Todo\nMessage: split enrichment", 1);
        let graph = super::super::loaded_graph(&open(fixture.path())?)?;
        let upper = apply(open(fixture.path())?, &graph, prepared, &edited)?;
        let repository = open(fixture.path())?;
        assert_eq!(
            crate::enrich::load(
                &mut crate::enrich::open(&repository)?,
                crate::change_id::for_commit(&repository, upper)?
            )?,
            crate::enrich::Enrichment {
                todo: true,
                note: Some("split enrichment".into()),
            }
        );
        let source = repository
            .find_commit(upper)?
            .parent_ids()
            .next()
            .expect("the upper commit has the rewritten source")
            .detach();
        assert_eq!(
            repository.find_commit(source)?.parent_ids().next().map(gix::Id::detach),
            old_parent,
            "the source commit retains its original ancestry"
        );
        assert_eq!(
            git(fixture.path(), &["show", &format!("{source}:both")])?,
            b"one\nbase staged\nmiddle\nworktree\nend\n"
        );
        assert_eq!(
            git(fixture.path(), &["show", &format!("{upper}:both")])?,
            b"one\nstaged\nmiddle\nworktree\nend\n"
        );
        assert_eq!(
            git(fixture.path(), &["show", &format!("{source}:unstaged")])?,
            b"worktree\n"
        );
        assert_eq!(
            git(fixture.path(), &["show", &format!("{source}:untracked")])?,
            b"untracked\n"
        );
        assert_eq!(git(fixture.path(), &["show", &format!("{upper}:staged")])?, b"staged\n");
        assert_eq!(repository.find_commit(source)?.message_raw()?, b"base\n".as_bstr());
        assert_eq!(
            repository.find_commit(upper)?.message_raw()?,
            b"upper\n\nreason\n".as_bstr()
        );
        let source_commit = repository.find_commit(source)?.decode()?.into_owned()?;
        assert_eq!(
            crate::change_id::effective(source, source_commit.extra_headers().find_all(crate::change_id::HEADER)),
            old.into(),
            "the rewritten lower commit inherits the original identity"
        );
        assert_eq!(
            repository
                .find_commit(upper)?
                .decode()?
                .extra_headers()
                .find(crate::change_id::HEADER),
            None,
            "the newly created upper commit remains headerless"
        );
        assert!(
            !super::super::rebase::is_pending(&source_commit),
            "the unsigned source already has its final tree and parent"
        );
        assert!(
            !super::super::rebase::is_pending(&repository.find_commit(upper)?.decode()?.into_owned()?),
            "the new upper commit already has its final tree and parent"
        );
        assert_eq!(git(fixture.path(), &["status", "--short"])?, b"");
        for name in ["refs/heads/main", "refs/patches/split"] {
            assert_eq!(
                repository.find_reference(name)?.id().detach(),
                upper,
                "{name} follows the split"
            );
        }
        for name in ["refs/tags/keep", "refs/remotes/origin/keep"] {
            assert_eq!(
                repository.find_reference(name)?.id().detach(),
                old,
                "{name} is not edited"
            );
        }
        Ok(())
    }

    #[test]
    fn conflicting_staged_and_worktree_hunks_abort_before_the_editor() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("split_commit.sh")?;
        std::fs::write(
            fixture.path().join("both"),
            b"one\nworktree conflict\nmiddle\nbase worktree\nend\n",
        )?;
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let err = match prepare(open(fixture.path())?) {
            Ok(_) => return Err("overlapping staged and worktree hunks should conflict".into()),
            Err(err) => err,
        };
        assert!(
            format!("{err:#}").contains("worktree changes conflict with the source commit"),
            "the error identifies the conflicting half of the split: {err:#}"
        );
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "a conflicting split writes no observable state"
        );
        Ok(())
    }
}
