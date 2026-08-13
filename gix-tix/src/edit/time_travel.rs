use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::ByteSlice,
    refs::{
        Target,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    },
};

use crate::{history, open_repository};

pub(crate) enum Perform {
    Complete(Option<String>),
    Conflict(Conflict),
}

impl Perform {
    #[cfg(test)]
    fn complete(self) -> Result<Option<String>> {
        match self {
            Perform::Complete(notice) => Ok(notice),
            Perform::Conflict(_) => anyhow::bail!("time-travel unexpectedly suspended on a conflict"),
        }
    }
}

pub(crate) struct Conflict {
    rebase: super::rebase::Conflict,
    repository_path: PathBuf,
    bare: bool,
    workdir: PathBuf,
    saved_target: Target,
    head: ObjectId,
}

impl Conflict {
    pub(crate) fn original(&self) -> ObjectId {
        self.rebase.original()
    }

    #[tracing::instrument(skip_all, fields(commit_id = %self.rebase.original()))]
    pub(crate) fn accept(self) -> Result<(String, ObjectId)> {
        let mut conflict = self.rebase.persist()?;
        let head = conflict
            .outcome
            .map(self.head)
            .context("HEAD disappeared while preparing conflict resolution")?;
        let repository = open_repository(&self.repository_path, self.bare, false)
            .context("could not reopen repository before conflict checkout")?;
        let provisional = contains(&repository, conflict.commit, head)
            .then(|| create_or_reuse_pin(&repository, self.saved_target, head))
            .transpose()?;
        drop(repository);
        if let Err(checkout) = checkout_detached(&self.workdir, conflict.commit) {
            if let Some((pin, true)) = &provisional {
                let cleanup = open_repository(&self.repository_path, self.bare, false)
                    .context("could not reopen repository to remove a provisional pin")
                    .and_then(|repository| delete_pin(&repository, pin));
                if let Err(cleanup) = cleanup {
                    return Err(checkout.context(format!(
                        "checkout failed and {} could not be removed: {cleanup:#}",
                        pin_label(pin)
                    )));
                }
            }
            return Err(checkout);
        }
        conflict.write_index()?;
        let mut notice = format!(
            "checked out conflicting {} for resolution",
            conflict.commit.to_hex_with_len(7)
        );
        if let Some((pin, true)) = provisional {
            notice = format!("{notice}; saved {}", pin_label(&pin));
        }
        Ok((notice, conflict.commit))
    }
}

#[tracing::instrument(skip_all, fields(commit_id = %selected))]
pub(crate) fn perform(
    repository_path: &Path,
    bare: bool,
    mut selected: ObjectId,
    graph: &history::HistoryGraph,
    revisions: &[OsString],
    include_worktrees: bool,
) -> Result<Perform> {
    let mut repository =
        open_repository(repository_path, bare, false).context("could not open repository for time-travel")?;
    let workdir = repository
        .workdir()
        .context("time-travel requires a worktree")?
        .to_owned();
    let head = repository.head().context("could not read HEAD before time-travel")?;
    let Some(mut head_id) = head.id().map(gix::Id::detach) else {
        anyhow::bail!("cannot time-travel from an unborn HEAD");
    };
    let head_referent = head.referent_name().map(ToOwned::to_owned);
    drop(head);
    if repository
        .index_or_empty()
        .context("could not inspect the index before time-travel")?
        .entries()
        .iter()
        .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
    {
        anyhow::bail!("cannot time-travel with unresolved index conflicts");
    }
    let mut completed_graph = None;
    while let Some(base) = pending_base(&repository, head_id, selected)? {
        let outcome = super::rebase::perform(
            &repository,
            completed_graph.as_ref().unwrap_or(graph),
            super::rebase::Edit::Repeat { base },
            super::rebase::Signature::RedoIfNeeded,
            super::rebase::Tree::CherryPick,
        )?;
        let outcome = match outcome {
            super::rebase::Perform::Complete(outcome) => outcome,
            super::rebase::Perform::Conflict(rebase) => {
                return Ok(Perform::Conflict(Conflict {
                    rebase,
                    repository_path: repository_path.to_owned(),
                    bare,
                    workdir,
                    saved_target: head_referent.map(Target::Symbolic).unwrap_or(Target::Object(head_id)),
                    head: head_id,
                }));
            }
        };
        selected = outcome
            .map(selected)
            .context("the time-travel destination disappeared while completing its rebase")?;
        head_id = outcome
            .map(head_id)
            .context("HEAD disappeared while completing its rebase")?;
        repository = open_repository(repository_path, bare, false)
            .context("could not reopen repository after completing a pending rebase")?;
        completed_graph = Some(super::loaded_graph(&repository)?);
    }
    let graph = completed_graph.as_ref().unwrap_or(graph);
    if selected == head_id {
        return Ok(Perform::Complete(None));
    }
    if let Some(pin) = selected_pin(&repository, selected)? {
        drop(repository);
        checkout_pin(&workdir, &pin)?;
        let repository = open_repository(repository_path, bare, false)
            .context("could not reopen repository after returning from time-travel")?;
        return Ok(Perform::Complete(Some(match delete_pin(&repository, &pin) {
            Ok(()) => format!("returned from {}", pin_label(&pin)),
            Err(err) => format!("returned from {}; pin remains: {err:#}", pin_label(&pin)),
        })));
    }
    let saved_target = head_referent.map(Target::Symbolic).unwrap_or(Target::Object(head_id));
    let provisional = graph
        .is_ancestor(selected, head_id)
        .then(|| create_or_reuse_pin(&repository, saved_target, head_id))
        .transpose()?;
    drop(repository);

    if let Err(checkout) = checkout_detached(&workdir, selected) {
        if let Some((pin, true)) = &provisional {
            let cleanup = open_repository(repository_path, bare, false)
                .context("could not reopen repository to remove a provisional pin")
                .and_then(|repository| delete_pin(&repository, pin));
            if let Err(cleanup) = cleanup {
                return Err(checkout.context(format!(
                    "checkout failed and {} could not be removed: {cleanup:#}",
                    pin_label(pin)
                )));
            }
        }
        return Err(checkout);
    }

    let mut notice = format!("time-travelled to {}", selected.to_hex_with_len(7));
    if let Some((pin, true)) = provisional {
        let repository =
            open_repository(repository_path, bare, false).context("could not reopen repository after time-travel")?;
        let snapshot =
            history::snapshot_ignoring_pin(&repository, revisions, &[], include_worktrees, Some(pin.name.as_bstr()))?;
        if snapshot
            .view_tips
            .iter()
            .copied()
            .any(|tip| contains(&repository, head_id, tip))
        {
            if let Err(err) = delete_pin(&repository, &pin) {
                notice = format!("{notice}; redundant {} remains: {err:#}", pin_label(&pin));
            }
        } else {
            notice = format!("{notice}; saved {}", pin_label(&pin));
        }
    }
    Ok(Perform::Complete(Some(notice)))
}

fn pending_base(repository: &gix::Repository, head: ObjectId, selected: ObjectId) -> Result<Option<ObjectId>> {
    for endpoint in [head, selected] {
        let mut current = endpoint;
        let mut base = None;
        loop {
            let commit = repository
                .find_commit(current)
                .context("could not inspect a time-travel endpoint for a pending rebase")?
                .decode()?
                .into_owned()?;
            if !super::rebase::has_marker(&commit) {
                break;
            }
            base = Some(current);
            let Some(parent) = commit.parents.first().copied() else {
                break;
            };
            current = parent;
        }
        if base.is_some() {
            return Ok(base);
        }
    }
    Ok(None)
}

fn selected_pin(repository: &gix::Repository, selected: ObjectId) -> Result<Option<history::Pin>> {
    let mut pins: Vec<_> = history::applicable_pins(repository)?
        .into_iter()
        .filter(|pin| pin.id == selected)
        .collect();
    pins.sort_by(|a, b| {
        a.target
            .try_name()
            .is_none()
            .cmp(&b.target.try_name().is_none())
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(pins.into_iter().next())
}

fn create_or_reuse_pin(repository: &gix::Repository, target: Target, id: ObjectId) -> Result<(history::Pin, bool)> {
    let pins = history::all_pins(repository)?;
    if let Some(pin) = pins.iter().find(|pin| pin.target == target) {
        return Ok((pin.clone(), false));
    }
    let hex = id.to_hex().to_string();
    let mut suffix_len = 8.min(hex.len());
    let mut number = 2;
    let name = loop {
        let suffix = if suffix_len <= hex.len() {
            hex[..suffix_len].to_owned()
        } else {
            let suffix = format!("{hex}{number}");
            number += 1;
            suffix
        };
        let name: gix::refs::FullName = format!("{}{}", String::from_utf8_lossy(history::PIN_PREFIX), suffix)
            .try_into()
            .context("generated an invalid tix pin name")?;
        if repository
            .try_find_reference(name.as_ref())
            .context("could not check for a colliding tix pin")?
            .is_none()
        {
            break name;
        }
        if suffix_len < hex.len() {
            suffix_len += 1;
        } else {
            suffix_len = hex.len() + 1;
        }
    };
    repository
        .edit_references_as(
            [RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: "tix time-travel".into(),
                    },
                    expected: PreviousValue::MustNotExist,
                    new: target.clone(),
                },
                name: name.clone(),
                deref: false,
            }],
            None,
        )
        .context("could not create tix pin")?;
    Ok((history::Pin { name, target, id }, true))
}

fn delete_pin(repository: &gix::Repository, pin: &history::Pin) -> Result<()> {
    repository
        .edit_references_as(
            [RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExistAndMatch(pin.target.clone()),
                    log: RefLog::AndReference,
                },
                name: pin.name.clone(),
                deref: false,
            }],
            None,
        )
        .context("could not remove tix pin")?;
    Ok(())
}

fn checkout_pin(workdir: &Path, pin: &history::Pin) -> Result<()> {
    match pin.target.try_name() {
        Some(name) => {
            let branch = name
                .as_bstr()
                .strip_prefix(b"refs/heads/")
                .context("a symbolic tix pin does not point to a local branch")?;
            checkout(
                workdir,
                [
                    OsString::from("--no-guess"),
                    gix::path::from_bstr(branch.as_bstr()).into_owned().into_os_string(),
                ],
            )
        }
        None => checkout_detached(workdir, pin.id),
    }
}

pub(super) fn checkout_detached(workdir: &Path, id: ObjectId) -> Result<()> {
    checkout(
        workdir,
        [OsString::from("--detach"), OsString::from(id.to_hex().to_string())],
    )
}

pub(super) fn checkout(workdir: &Path, args: impl IntoIterator<Item = OsString>) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workdir)
        .arg("checkout")
        .args(args)
        .output()
        .context("could not launch git checkout")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = output.stderr.trim().to_str_lossy();
    if stderr.is_empty() {
        anyhow::bail!("git checkout failed with {}", output.status)
    }
    anyhow::bail!("git checkout failed with {}: {}", output.status, stderr)
}

fn contains(repository: &gix::Repository, ancestor: ObjectId, descendant: ObjectId) -> bool {
    ancestor == descendant
        || repository
            .merge_base(ancestor, descendant)
            .is_ok_and(|base| base.as_ref() == ancestor)
}

fn pin_label(pin: &history::Pin) -> String {
    format!(
        "pin:{}",
        pin.name
            .as_bstr()
            .strip_prefix(history::PIN_PREFIX)
            .unwrap_or(pin.name.as_bstr())
            .to_str_lossy()
    )
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use super::*;

    fn loaded_graph(repository: &gix::Repository, revisions: &[OsString]) -> Result<history::HistoryGraph> {
        let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(
            history::Authors::default(),
        ));
        let mut graph = None;
        history::load(
            repository,
            revisions,
            &[],
            false,
            &authors,
            &AtomicBool::new(false),
            |event| {
                if let history::Event::Complete(value) = event {
                    graph = Some(value);
                }
                true
            },
        )?;
        graph.context("history traversal did not produce a graph")
    }

    #[test]
    fn travels_with_symbolic_and_direct_pins_and_returns() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let root = repository.rev_parse_single("main~2")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let topic = repository.rev_parse_single("topic")?.detach();
        let graph = loaded_graph(&repository, &[])?;
        assert!(graph.is_ancestor(root, main), "the selected root is known ancestry");
        assert!(history::all_pins(&repository)?.is_empty());
        assert_eq!(
            open_repository(&repository_path, false, false)?.head_id()?.detach(),
            main
        );
        assert!(!contains(&repository, main, root));
        drop(repository);

        let notice = perform(&repository_path, false, root, &graph, &[], false)?
            .complete()?
            .context("time-travel changed HEAD")?;
        assert!(notice.contains("saved pin:"), "{notice}");
        let repository = crate::open_test_repository(fixture.path())?;
        assert!(repository.head()?.is_detached(), "travel detaches HEAD");
        assert_eq!(repository.head_id()?.detach(), root);
        let pins = history::all_pins(&repository)?;
        assert_eq!(pins.len(), 1, "the lost branch tip gets one pin");
        assert_eq!(
            pins[0].target.try_name().map(gix::refs::FullNameRef::as_bstr),
            Some(b"refs/heads/main".as_bstr())
        );
        assert_eq!(pins[0].id, main);
        assert!(
            history::snapshot(&repository, &[], &[], false)?
                .view_tips
                .contains(&main)
        );

        repository
            .find_reference("refs/heads/main")?
            .set_target_id(topic, "advance pinned branch")?;
        let advanced = history::snapshot(&repository, &[], &[], false)?;
        assert!(advanced.view_tips.contains(&topic), "a symbolic pin follows its branch");
        drop(repository);

        perform(&repository_path, false, topic, &graph, &[], false)?.complete()?;
        let repository = crate::open_test_repository(fixture.path())?;
        assert_eq!(
            repository.head_name()?.map(|name| name.as_bstr().to_owned()),
            Some(b"refs/heads/main".into()),
            "returning through a symbolic pin reattaches HEAD"
        );
        assert!(history::all_pins(&repository)?.is_empty(), "the used pin is removed");

        let detach = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["checkout", "--detach", &main.to_hex().to_string()])
            .status()?;
        assert!(detach.success());
        let graph = loaded_graph(&crate::open_test_repository(fixture.path())?, &[])?;
        perform(&repository_path, false, root, &graph, &[], false)?.complete()?;
        let pin = history::all_pins(&crate::open_test_repository(fixture.path())?)?
            .pop()
            .context("direct pin is present")?;
        assert_eq!(pin.target.try_id().map(ToOwned::to_owned), Some(main));
        perform(&repository_path, false, main, &graph, &[], false)?.complete()?;
        let repository = crate::open_test_repository(fixture.path())?;
        assert!(
            repository.head()?.is_detached(),
            "a direct pin returns to detached HEAD"
        );
        assert_eq!(repository.head_id()?.detach(), main);
        assert!(history::all_pins(&repository)?.is_empty());
        Ok(())
    }

    #[test]
    fn explicit_tips_avoid_redundant_pins_and_failed_checkouts_clean_up() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let root = repository.rev_parse_single("main~2")?.detach();
        let main = repository.rev_parse_single("main")?.detach();
        let revisions = [OsString::from("main")];
        let graph = loaded_graph(&repository, &revisions)?;
        drop(repository);

        perform(&repository_path, false, root, &graph, &revisions, false)?.complete()?;
        assert!(
            history::all_pins(&crate::open_test_repository(fixture.path())?)?.is_empty(),
            "an explicit tip already retains the former HEAD"
        );

        let checkout = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["checkout", "--no-guess", "main"])
            .status()?;
        assert!(checkout.success());
        std::fs::write(fixture.path().join("main"), "dirty\n")?;
        let err = perform(&repository_path, false, root, &graph, &[], false)
            .and_then(Perform::complete)
            .expect_err("Git rejects a conflicting checkout");
        assert!(format!("{err:#}").contains("git checkout failed"));
        let repository = crate::open_test_repository(fixture.path())?;
        assert_eq!(repository.head_id()?.detach(), main, "failed checkout retains HEAD");
        assert!(
            history::all_pins(&repository)?.is_empty(),
            "the provisional pin is removed"
        );
        Ok(())
    }

    #[test]
    fn conflicting_pending_rebases_are_unobservable_until_accepted() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_conflict.sh")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["config", "gitoxide.commit.committerDate", "2001-01-01T00:00:00 +0000",])
                .status()?
                .success()
        );
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        std::fs::write(fixture.path().join("after"), "after\n")?;
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["add", "after"])
                .status()?
                .success()
        );
        assert!(
            Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(["commit", "-q", "-m", "after"])
                .env("GIT_AUTHOR_DATE", "2000-01-04T00:00:00 +0000")
                .env("GIT_COMMITTER_DATE", "2000-01-04T00:00:00 +0000")
                .status()?
                .success()
        );
        let graph = super::super::loaded_graph(&repository)?;
        super::super::rebase::perform(
            &repository,
            &graph,
            super::super::rebase::Edit::Remove { target: middle },
            super::super::rebase::Signature::RedoIfNeeded,
            super::super::rebase::Tree::LeaveAsIsAndMark,
        )?
        .complete()?;
        let graph = super::super::loaded_graph(&repository)?;
        let root = repository.rev_parse_single("HEAD~2")?.detach();
        let before = gix_testtools::repository::snapshot(fixture.path())?;

        let Perform::Conflict(conflict) = perform(&repository_path, false, root, &graph, &[], false)? else {
            return Err("the pending rebase should suspend at its conflicting cherry-pick".into());
        };
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "preparing the exact merge result changes no repository state"
        );

        let (_notice, conflict_id) = conflict.accept()?;
        let repository = crate::open_test_repository(fixture.path())?;
        assert_eq!(
            repository.head_id()?.detach(),
            conflict_id,
            "the conflicting commit is checked out"
        );
        assert!(repository.head()?.is_detached(), "conflict resolution detaches HEAD");
        let branch = repository.find_reference("refs/heads/main")?.id().detach();
        assert_ne!(
            branch, conflict_id,
            "the remaining descendant stays on the saved branch"
        );
        assert!(
            super::super::rebase::has_marker(&repository.find_commit(branch)?.decode()?.into_owned()?),
            "remaining descendants stay as lazy rewrites"
        );
        let index = repository.index_or_empty()?;
        assert!(
            index
                .entries()
                .iter()
                .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted),
            "the retained merge outcome supplies unresolved index stages"
        );
        assert!(
            std::fs::read(fixture.path().join("file"))?
                .as_bstr()
                .contains_str("<<<<<<<"),
            "the checked-out merge tree contains conflict markers"
        );
        insta::assert_snapshot!(
            "accepted-pending-rebase-conflict",
            gix_testtools::repository::snapshot(fixture.path())?
                .to_string()
                .replace("\n  \n", "\n\n")
        );
        let err = perform(&repository_path, false, root, &graph, &[], false)
            .and_then(Perform::complete)
            .expect_err("time-travel is disabled until the index conflict is resolved");
        assert!(format!("{err:#}").contains("unresolved index conflicts"));
        Ok(())
    }

    #[test]
    fn travelling_completes_the_whole_pending_rebase() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repository = crate::open_test_repository(fixture.path())?;
        let repository_path = repository.git_dir().to_owned();
        let middle = repository.rev_parse_single("HEAD~1")?.detach();
        let root = repository.rev_parse_single("HEAD~2")?.detach();
        let graph = loaded_graph(&repository, &[])?;
        let commit = repository.find_commit(middle)?.decode()?.into_owned()?;
        super::super::rebase::perform(
            &repository,
            &graph,
            super::super::rebase::Edit::Replace { target: middle, commit },
            super::super::rebase::Signature::InvalidateExisting,
            super::super::rebase::Tree::LeaveAsIsAndMark,
        )?;
        let graph = loaded_graph(&repository, &[])?;
        perform(&repository_path, false, root, &graph, &[], false)?.complete()?;

        let repository = crate::open_test_repository(fixture.path())?;
        let mut current = Some(repository.find_reference("refs/heads/main")?.id().detach());
        while let Some(id) = current {
            let commit = repository.find_commit(id)?.decode()?.into_owned()?;
            assert!(
                !super::super::rebase::has_marker(&commit),
                "time travel clears the complete pending region"
            );
            current = commit.parents.first().copied();
        }
        Ok(())
    }
}
