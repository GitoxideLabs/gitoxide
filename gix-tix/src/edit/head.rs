use anyhow::{Context, Result};
use gix::{ObjectId, bstr::BStr};

use super::{create, rebase};
use crate::{ChangeKind, PathChange};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Amend,
    Spill,
}

#[cfg(test)]
#[tracing::instrument(skip_all, fields(?kind))]
pub fn perform(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    kind: Kind,
    selected_paths: Option<(&[PathChange], Option<ObjectId>)>,
) -> Result<Option<ObjectId>> {
    Ok(perform_inner(
        repo,
        graph,
        kind,
        selected_paths,
        false,
        rebase::PendingCheckout::Reject,
        |_| {},
    )?
    .and_then(|outcome| outcome.selected))
}

pub(crate) fn perform_with_changes(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    kind: Kind,
    selected_paths: Option<(&[PathChange], Option<ObjectId>)>,
    pending_checkout: rebase::PendingCheckout,
    report: impl FnMut(rebase::Progress),
) -> Result<Option<rebase::Outcome>> {
    perform_inner(repo, graph, kind, selected_paths, false, pending_checkout, report)
}

pub(crate) fn perform_reporting(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    kind: Kind,
) -> Result<Option<rebase::Outcome>> {
    perform_inner(repo, graph, kind, None, false, rebase::PendingCheckout::Reject, |_| {})
}

#[tracing::instrument(skip_all)]
#[cfg(test)]
pub fn amend_index(repo: gix::Repository, graph: &crate::history::HistoryGraph) -> Result<Option<ObjectId>> {
    Ok(perform_inner(
        repo,
        graph,
        Kind::Amend,
        None,
        true,
        rebase::PendingCheckout::FinalizeEditedHead,
        |_| {},
    )?
    .and_then(|outcome| outcome.selected))
}

pub(crate) fn amend_index_reporting(
    repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
) -> Result<Option<rebase::Outcome>> {
    perform_inner(
        repo,
        graph,
        Kind::Amend,
        None,
        true,
        rebase::PendingCheckout::FinalizeEditedHead,
        |_| {},
    )
}

fn perform_inner(
    mut repo: gix::Repository,
    graph: &crate::history::HistoryGraph,
    kind: Kind,
    selected_paths: Option<(&[PathChange], Option<ObjectId>)>,
    index_only: bool,
    pending_checkout: rebase::PendingCheckout,
    mut report: impl FnMut(rebase::Progress),
) -> Result<Option<rebase::Outcome>> {
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
    let pending = rebase::is_pending(&commit);
    let review = super::review::is_review(&commit);
    let parent_tree = match commit.parents.first().copied() {
        Some(parent) => repo.find_commit(parent)?.tree_id()?.detach(),
        None => repo.empty_tree().id,
    };
    let selected_amend_path = match (kind, selected_paths) {
        (Kind::Amend, Some(([path], _))) => Some(path),
        (Kind::Amend, Some(_)) => anyhow::bail!("amending requires exactly one selected path"),
        _ => None,
    };
    let tree = match kind {
        Kind::Spill => match selected_paths {
            Some((paths, selected_parent)) => {
                spill_paths_tree(&repo, old_tree, selected_parent.unwrap_or(parent_tree), paths)?
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
            if let Some(path) = selected_amend_path {
                if review && path.group != crate::ChangeGroup::Staged {
                    anyhow::bail!("review commits can amend only staged paths");
                }
                amend_path_tree(&repo, old_tree, path, &index)?
            } else if review || index_only {
                let index_tree = create::index_tree(&repo, &index)?;
                if index_tree == old_tree && !pending {
                    return Ok(None);
                }
                index_tree
            } else {
                let index_tree = create::index_tree(&repo, &index)?;
                if index_tree != old_tree {
                    index_tree
                } else {
                    let baseline = repo.find_tree(old_tree)?;
                    create::worktree_tree(&repo, &baseline)?
                }
            }
        }
    };
    if tree == old_tree && !pending {
        return Ok(None);
    }
    commit.tree = tree;
    let edit = rebase::Edit::Replace { target: head, commit };
    let signature = if review {
        rebase::Signature::Remove
    } else if kind == Kind::Amend || pending {
        rebase::Signature::RedoIfNeeded
    } else {
        rebase::Signature::InvalidateExisting
    };
    let tree_mode = if review {
        rebase::Tree::LeaveAsIsAndMarkDescendants
    } else {
        rebase::Tree::LeaveAsIsAndMark
    };
    let performed = match selected_amend_path {
        Some(path) if pending_checkout == rebase::PendingCheckout::FinalizeEditedHead => {
            let mut paths = vec![path.path.clone()];
            if path.kind == ChangeKind::Renamed
                && let Some(source) = &path.source
            {
                paths.push(source.clone());
            }
            rebase::perform_resetting_index_paths_finalizing_pending_checkout_with_progress(
                &repo,
                graph,
                edit,
                signature,
                tree_mode,
                paths,
                &mut report,
            )?
        }
        Some(path) => {
            let mut paths = vec![path.path.clone()];
            if path.kind == ChangeKind::Renamed
                && let Some(source) = &path.source
            {
                paths.push(source.clone());
            }
            rebase::perform_resetting_index_paths_with_progress(
                &repo,
                graph,
                edit,
                signature,
                tree_mode,
                paths,
                &mut report,
            )?
        }
        _ if kind == Kind::Amend && pending_checkout == rebase::PendingCheckout::FinalizeEditedHead => {
            rebase::perform_finalizing_pending_checkout_with_progress(
                &repo,
                graph,
                edit,
                signature,
                tree_mode,
                &mut report,
            )?
        }
        _ => rebase::perform_with_progress(&repo, graph, edit, signature, tree_mode, &mut report)?,
    };
    Ok(Some(performed.complete()?))
}

fn amend_path_tree(
    repo: &gix::Repository,
    commit_tree: ObjectId,
    change: &PathChange,
    index: &gix::index::File,
) -> Result<ObjectId> {
    match change.group {
        crate::ChangeGroup::Staged => {
            let index_tree = create::index_tree(repo, index)?;
            apply_path_from_tree(repo, commit_tree, index_tree, change)
        }
        crate::ChangeGroup::Unstaged => {
            let baseline = repo.find_tree(commit_tree)?;
            create::worktree_tree_with_changes(
                repo,
                &baseline,
                &crate::Changes {
                    paths: vec![change.clone()],
                    ..crate::Changes::default()
                },
            )
        }
        crate::ChangeGroup::Tree => anyhow::bail!("a tree change cannot be amended from the worktree"),
    }
}

fn apply_path_from_tree(
    repo: &gix::Repository,
    commit_tree: ObjectId,
    source_tree: ObjectId,
    change: &PathChange,
) -> Result<ObjectId> {
    let mut editor = repo.find_tree(commit_tree)?.edit()?;
    if change.kind == ChangeKind::Renamed
        && let Some(source) = &change.source
    {
        editor.remove(source)?;
    }
    if change.kind == ChangeKind::Deleted {
        editor.remove(&change.path)?;
    } else {
        let source = repo.find_tree(source_tree)?;
        let entry = source
            .lookup_entry(
                change
                    .path
                    .split(|byte| *byte == b'/')
                    .map(|component| BStr::new(component).to_owned()),
            )?
            .context("the selected path is absent from its source tree")?;
        editor.upsert(&change.path, entry.mode().kind(), entry.object_id())?;
    }
    Ok(editor.write()?.detach())
}

fn spill_paths_tree(
    repo: &gix::Repository,
    commit_tree: ObjectId,
    parent_tree: ObjectId,
    changes: &[PathChange],
) -> Result<ObjectId> {
    let parent = repo.find_tree(parent_tree).context("could not load the parent tree")?;
    let mut editor = repo
        .find_tree(commit_tree)
        .context("could not load the commit tree")?
        .edit()
        .context("could not edit the commit tree")?;
    for change in changes {
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
    fn amend_prefers_the_index_and_leaves_worktree_files_alone() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let repo = open(fixture.path())?;
        let old = repo.head_id()?.detach();
        let graph = super::super::loaded_graph(&repo)?;
        let new = amend_index(repo, &graph)?.expect("staged changes amend HEAD");
        assert_ne!(new, old);
        assert_eq!(std::fs::read(fixture.path().join("tracked"))?, b"unstaged\n");
        assert_eq!(git(fixture.path(), &["show", "HEAD:tracked"])?, b"staged\n");
        assert!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.is_empty(),
            "the index follows the amended commit"
        );
        let commit = open(fixture.path())?.find_commit(new)?.decode()?.into_owned()?;
        assert!(
            !super::super::rebase::is_pending(&commit),
            "an unsigned amended commit already has its final tree and parent"
        );
        Ok(())
    }

    #[test]
    fn signed_worktree_amend_is_finalized_immediately() -> gix_testtools::Result {
        if !gix_testtools::signature::program_available("ssh-keygen") {
            return Ok(());
        }
        let (_key_home, key) = gix_testtools::signature::ssh_private_key()?;
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        git(fixture.path(), &["reset", "-q", "HEAD", "--", "tracked"])?;
        let repo = crate::test_repository::open_with(
            fixture.path(),
            [
                "user.name=editor".to_owned(),
                "user.email=editor@example.com".to_owned(),
                "commit.gpgSign=true".to_owned(),
                "gpg.format=ssh".to_owned(),
                format!("user.signingKey={}", key.display()),
                format!(
                    "gpg.ssh.allowedSignersFile={}",
                    gix_testtools::signature::fixture("ssh-allowed-signers").display()
                ),
            ],
        )?;
        let old = repo.head_id()?.detach();
        let signed = repo.find_commit(old)?.decode()?.sign(
            repo.commit_signing_options_if_enabled()?
                .expect("commit signing is configured"),
        )?;
        let signed = repo.write_object(&signed)?.detach();
        repo.find_reference("refs/heads/main")?
            .set_target_id(signed, "prepare signed worktree amend")?;
        let graph = super::super::loaded_graph(&repo)?;

        let amended = perform(repo.clone(), &graph, Kind::Amend, None)?.expect("the worktree change amends HEAD");
        assert_eq!(repo.head_id()?, amended, "HEAD follows the amended commit");
        assert!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.is_empty(),
            "the amended index is clean"
        );
        assert!(
            git(fixture.path(), &["diff", "--name-only"])?.is_empty(),
            "the amended worktree is clean"
        );
        let commit = repo.find_commit(amended)?;
        assert!(
            !super::super::rebase::is_pending(&commit.decode()?.into_owned()?),
            "the checked-out amended commit needs no later replay"
        );
        assert!(
            commit
                .verify_signature()?
                .expect("the amended commit is signed")
                .is_valid(),
            "the amended commit receives a valid configured signature"
        );
        Ok(())
    }

    #[test]
    fn index_only_amend_does_not_fall_back_to_worktree_changes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        git(fixture.path(), &["reset", "-q", "HEAD", "--", "tracked"])?;
        let repo = open(fixture.path())?;
        let old = repo.head_id()?.detach();
        let graph = super::super::loaded_graph(&repo)?;

        assert!(amend_index(repo, &graph)?.is_none(), "an unchanged index is a no-op");
        let repo = open(fixture.path())?;
        assert_eq!(repo.head_id()?, old, "HEAD remains unchanged");
        assert!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.is_empty(),
            "the index remains clean"
        );
        assert_eq!(
            git(fixture.path(), &["diff", "--name-only"])?,
            b"tracked\n",
            "worktree-only changes remain uncommitted"
        );
        Ok(())
    }

    #[test]
    fn index_only_amend_finalizes_a_pending_commit_even_when_its_tree_is_unchanged() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let old = repo.head_id()?.detach();
        let mut commit = repo.find_commit(old)?.decode()?.into_owned()?;
        let parent = commit.parents.first().copied().expect("the fixture HEAD has a parent");
        commit
            .extra_headers
            .push(("tix-rebase-parent".into(), parent.to_string().into()));
        let pending = repo.write_object(&commit)?.detach();
        repo.find_reference("refs/heads/main")?
            .set_target_id(pending, "prepare pending amend")?;
        let graph = super::super::loaded_graph(&repo)?;

        let finalized = amend_index(repo, &graph)?.expect("a pending commit must be finalized");
        let repo = open(fixture.path())?;
        assert_eq!(repo.head_id()?, finalized);
        assert!(
            !super::super::rebase::is_pending(&repo.find_commit(finalized)?.decode()?.into_owned()?),
            "an all-ours resolution removes the pending marker"
        );
        Ok(())
    }

    #[test]
    fn ordinary_amend_rejects_a_pending_head_outside_conflict_resolution() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let old = repo.head_id()?.detach();
        let mut commit = repo.find_commit(old)?.decode()?.into_owned()?;
        let parent = commit.parents.first().copied().expect("the fixture HEAD has a parent");
        commit
            .extra_headers
            .push(("tix-rebase-parent".into(), parent.to_string().into()));
        let pending = repo.write_object(&commit)?.detach();
        repo.find_reference("refs/heads/main")?
            .set_target_id(pending, "prepare an externally checked-out pending commit")?;
        let graph = super::super::loaded_graph(&repo)?;

        let err = match perform(repo, &graph, Kind::Amend, None) {
            Ok(_) => return Err("an ordinary amend must not resolve an arbitrary pending HEAD".into()),
            Err(err) => err,
        };
        assert!(err.to_string().contains("time-travel to HEAD"), "{err:#}");
        assert_eq!(open(fixture.path())?.head_id()?, pending);
        Ok(())
    }

    #[test]
    fn amend_rejects_an_unmarked_head_above_pending_ancestry() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let old_tip = repo.head_id()?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let mut pending = repo.find_commit(middle)?.decode()?.into_owned()?;
        pending
            .extra_headers
            .push(("tix-rebase-parent".into(), base.to_string().into()));
        let pending = repo.write_object(&pending)?.detach();
        let mut tip = repo.find_commit(old_tip)?.decode()?.into_owned()?;
        tip.parents = [pending].into_iter().collect();
        let tip = repo.write_object(&tip)?.detach();
        repo.find_reference("refs/heads/main")?
            .set_target_id(tip, "prepare pending checkout ancestry")?;
        std::fs::write(fixture.path().join("tip"), b"amended\n")?;
        git(fixture.path(), &["add", "tip"])?;
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;

        let err = match perform(repo, &graph, Kind::Amend, None) {
            Ok(_) => return Err("amend must not preserve pending checkout ancestry".into()),
            Err(err) => err,
        };
        assert!(err.to_string().contains("time-travel to HEAD"), "{err:#}");
        assert_eq!(gix_testtools::repository::snapshot(fixture.path())?, before);
        Ok(())
    }

    fn assert_review_amend_does_not_cross_pending_base(index_only: bool) -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = open(fixture.path())?;
        let reviewed = repo.head_id()?.detach();
        let middle = repo.rev_parse_single("HEAD~1")?.detach();
        let base = repo.rev_parse_single("HEAD~2")?.detach();
        let mut pending = repo.find_commit(middle)?.decode()?.into_owned()?;
        pending
            .extra_headers
            .push(("tix-rebase-parent".into(), base.to_string().into()));
        let pending = repo.write_object(&pending)?.detach();
        let mut review = repo.find_commit(middle)?.decode()?.into_owned()?;
        review.parents = [pending].into_iter().collect();
        review.message = "review".into();
        review.extra_headers.clear();
        review
            .extra_headers
            .push(("tix-rebase".into(), "onto refs/worktree/tix/review/1".into()));
        let review = repo.write_object(&review)?.detach();
        repo.reference(
            "refs/worktree/tix/review/1",
            reviewed,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "prepare active review",
        )?;
        drop(repo);
        git(fixture.path(), &["checkout", "-q", "--detach", &review.to_string()])?;
        std::fs::write(fixture.path().join("middle"), b"reviewed\n")?;
        git(fixture.path(), &["add", "middle"])?;

        let repo = open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let amended = if index_only {
            amend_index(repo, &graph)?
        } else {
            perform(repo, &graph, Kind::Amend, None)?
        }
        .expect("staged review changes amend HEAD");
        let repo = open(fixture.path())?;
        let amended_commit = repo.find_commit(amended)?.decode()?.into_owned()?;
        assert_eq!(
            amended_commit.parents.first().copied(),
            Some(pending),
            "amending the review does not rewrite its pending base"
        );
        assert!(
            super::super::review::is_review(&amended_commit),
            "amending preserves the review identity"
        );
        assert!(
            super::super::rebase::is_pending(&repo.find_commit(pending)?.decode()?.into_owned()?),
            "the unrelated pending base remains lazy"
        );
        assert_eq!(git(fixture.path(), &["show", "HEAD:middle"])?, b"reviewed\n");
        Ok(())
    }

    #[test]
    fn review_amend_does_not_cross_its_boundary_when_checking_pending_ancestry() -> gix_testtools::Result {
        assert_review_amend_does_not_cross_pending_base(false)
    }

    #[test]
    fn index_only_review_amend_does_not_cross_its_pending_base() -> gix_testtools::Result {
        assert_review_amend_does_not_cross_pending_base(true)
    }

    #[test]
    fn amending_one_worktree_path_preserves_unrelated_staging() -> gix_testtools::Result {
        for (group, expected) in [
            (crate::ChangeGroup::Staged, b"staged\n".as_slice()),
            (crate::ChangeGroup::Unstaged, b"unstaged\n".as_slice()),
        ] {
            let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
            std::fs::write(fixture.path().join("other"), "other\n")?;
            git(fixture.path(), &["add", "other"])?;
            let repo = open(fixture.path())?;
            let graph = super::super::loaded_graph(&repo)?;
            let selected = PathChange {
                kind: ChangeKind::Modified,
                group,
                source: None,
                path: "tracked".into(),
                lines: None,
            };
            let new = perform(repo, &graph, Kind::Amend, Some((std::slice::from_ref(&selected), None)))?
                .expect("the selected path changes HEAD");
            assert_eq!(git(fixture.path(), &["show", &format!("{new}:tracked")])?, expected);
            assert_eq!(
                git(fixture.path(), &["diff", "--cached", "--name-only"])?,
                b"other\n",
                "an unrelated addition remains staged"
            );
            assert_eq!(std::fs::read(fixture.path().join("tracked"))?, b"unstaged\n");
            let unstaged = git(fixture.path(), &["diff", "--name-only"])?;
            if group == crate::ChangeGroup::Staged {
                assert_eq!(unstaged, b"tracked\n", "the worktree-only delta remains");
            } else {
                assert!(unstaged.is_empty(), "the amended worktree version becomes clean");
            }
        }
        Ok(())
    }

    #[test]
    fn scoped_amend_rolls_back_if_the_index_cannot_be_locked() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let repo = open(fixture.path())?;
        let old = repo.head_id()?.detach();
        let index_before = std::fs::read(repo.index_path())?;
        let graph = super::super::loaded_graph(&repo)?;
        let selected = PathChange {
            kind: ChangeKind::Modified,
            group: crate::ChangeGroup::Staged,
            source: None,
            path: "tracked".into(),
            lines: None,
        };
        std::fs::write(fixture.path().join(".git/index.lock"), "locked")?;
        let err = perform(repo, &graph, Kind::Amend, Some((std::slice::from_ref(&selected), None)))
            .expect_err("an index lock prevents the amend");
        assert!(format!("{err:#}").contains("selected index paths"));
        let repo = open(fixture.path())?;
        assert_eq!(repo.head_id()?, old, "the rewritten ref is rolled back");
        assert_eq!(
            std::fs::read(repo.index_path())?,
            index_before,
            "the original index is restored"
        );
        Ok(())
    }

    #[test]
    fn scoped_amend_synchronizes_changed_index_paths() -> gix_testtools::Result {
        for kind in [
            ChangeKind::Added,
            ChangeKind::Deleted,
            ChangeKind::Renamed,
            ChangeKind::Copied,
        ] {
            let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
            git(
                fixture.path(),
                &["restore", "--source=HEAD", "--staged", "--worktree", "tracked"],
            )?;
            std::fs::write(fixture.path().join("other"), "other\n")?;
            git(fixture.path(), &["add", "other"])?;
            let (path, source) = match kind {
                ChangeKind::Added => {
                    std::fs::write(fixture.path().join("added"), "added\n")?;
                    git(fixture.path(), &["add", "added"])?;
                    ("added", None)
                }
                ChangeKind::Deleted => {
                    std::fs::remove_file(fixture.path().join("tracked"))?;
                    git(fixture.path(), &["add", "-u", "tracked"])?;
                    ("tracked", None)
                }
                ChangeKind::Renamed => {
                    git(fixture.path(), &["mv", "tracked", "renamed"])?;
                    ("renamed", Some("tracked"))
                }
                ChangeKind::Copied => {
                    std::fs::copy(fixture.path().join("tracked"), fixture.path().join("copied"))?;
                    git(fixture.path(), &["add", "copied"])?;
                    ("copied", Some("tracked"))
                }
                _ => unreachable!("the test lists only path-shape changes"),
            };
            let repo = open(fixture.path())?;
            let graph = super::super::loaded_graph(&repo)?;
            let selected = PathChange {
                kind,
                group: crate::ChangeGroup::Staged,
                source: source.map(Into::into),
                path: path.into(),
                lines: None,
            };
            let new = perform(repo, &graph, Kind::Amend, Some((std::slice::from_ref(&selected), None)))?
                .expect("the selected path changes HEAD");
            let repo = open(fixture.path())?;
            let tree = repo.find_commit(new)?.tree()?;
            assert_eq!(
                tree.lookup_entry([path])?.is_some(),
                kind != ChangeKind::Deleted,
                "the destination follows the selected change"
            );
            if kind == ChangeKind::Renamed
                && let Some(source) = source
            {
                assert!(tree.lookup_entry([source])?.is_none(), "the renamed source is removed");
            } else if kind == ChangeKind::Copied {
                assert!(
                    tree.lookup_entry(["tracked"])?.is_some(),
                    "the copied source is retained"
                );
            }
            assert_eq!(
                git(fixture.path(), &["diff", "--cached", "--name-only"])?,
                b"other\n",
                "the unrelated addition remains staged"
            );
        }
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
        assert_eq!(repo.find_commit(new)?.tree_id()?, parent_tree);
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
        assert_eq!(repo.find_commit(new)?.tree_id()?, repo.empty_tree().id);
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
        let new = perform(repo, &graph, Kind::Spill, Some((std::slice::from_ref(&selected), None)))?
            .expect("the selected path can be spilled");
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
