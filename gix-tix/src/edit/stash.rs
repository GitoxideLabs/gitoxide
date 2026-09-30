use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    path::Path,
};

use anyhow::{Context, Result};
use gix::{
    ObjectId,
    bstr::{BStr, ByteSlice},
    hash::ChangeId,
    refs::{
        Target,
        transaction::{PreviousValue, RefEdit},
    },
};

use crate::open_repository;

pub(crate) fn reference(repo: &gix::Repository, commit_id: ObjectId) -> Result<gix::refs::FullName> {
    reference_for_change(crate::change_id::for_commit(repo, commit_id)?)
}

fn reference_for_change(change_id: ChangeId) -> Result<gix::refs::FullName> {
    format!("{}{}", String::from_utf8_lossy(crate::history::STASH_PREFIX), change_id)
        .try_into()
        .context("generated an invalid tix stash reference")
}

pub(crate) fn associated_change(name: &BStr) -> Result<Option<ChangeId>> {
    let Some(suffix) = name.strip_prefix(crate::history::STASH_PREFIX) else {
        return Ok(None);
    };
    let id = ChangeId::from_reverse_hex(suffix).context("tix stash reference has an invalid change ID")?;
    if id.to_string().as_bytes() != suffix {
        anyhow::bail!("tix stash reference does not use a canonical full change ID");
    }
    Ok(Some(id))
}

pub(super) struct RewriteEdits {
    pub forward: Vec<RefEdit>,
    pub rollback: Vec<RefEdit>,
}

pub(super) fn rewrite_edits(
    repo: &gix::Repository,
    rewritten: &HashMap<ObjectId, Option<ObjectId>>,
    removed: &HashSet<ObjectId>,
) -> Result<RewriteEdits> {
    let mut stashes = HashMap::new();
    for saved in all(repo)? {
        match associated_change(saved.name.as_bstr()) {
            Ok(Some(change_id)) => {
                stashes.insert(change_id, saved);
            }
            Ok(None) => continue,
            Err(err) => {
                tracing::warn!(name = %saved.name, error = %err, "stash association requires explicit recovery");
            }
        }
    }
    let mut associations = HashMap::new();
    if !stashes.is_empty() {
        for (&old_commit_id, &new_commit_id) in rewritten {
            let old = crate::change_id::for_commit(repo, old_commit_id)?;
            if !stashes.contains_key(&old) {
                continue;
            }
            anyhow::ensure!(
                !removed.contains(&old_commit_id),
                "cannot drop stashed commit {}",
                old_commit_id.to_hex_with_len(7)
            );
            let new_commit_id = new_commit_id.context("a stashed commit cannot disappear during a rewrite")?;
            let new = crate::change_id::for_commit(repo, new_commit_id)?;
            if let Some(other) = associations.insert(old, new) {
                anyhow::ensure!(other == new, "a stashed change has ambiguous rewrite destinations");
            }
        }
    }
    let mut moves = Vec::new();
    let mut destinations = HashMap::new();
    for (old, saved) in stashes {
        let Some(&new) = associations.get(&old) else {
            continue;
        };
        if new == old {
            continue;
        }
        if let Some(other) = destinations.insert(new, old) {
            anyhow::bail!(
                "stashes at {} and {} would converge on {}",
                other.to_reverse_hex_with_len(7),
                old.to_reverse_hex_with_len(7),
                new.to_reverse_hex_with_len(7)
            );
        }
        moves.push((saved.name, saved.target, old, new));
    }

    let mut forward = Vec::with_capacity(moves.len() * 2);
    let mut rollback = Vec::with_capacity(moves.len() * 2);
    for (old_name, target, old, new) in moves {
        let new_name = reference_for_change(new)?;
        if repo.try_find_reference(new_name.as_ref())?.is_some() {
            anyhow::bail!(
                "rewritten change {} already has saved worktree state",
                new.to_reverse_hex_with_len(7)
            );
        }
        forward.push(delete_edit(old_name.clone(), target.clone()));
        forward.push(create_edit(new_name.clone(), target.clone()));
        rollback.push(delete_edit(new_name, target.clone()));
        rollback.push(create_edit(old_name, target));
        tracing::debug!(old = %old, new = %new, "prepared tix stash association rewrite");
    }
    Ok(RewriteEdits { forward, rollback })
}

fn create_edit(name: gix::refs::FullName, target: Target) -> RefEdit {
    RefEdit::update(name, target, PreviousValue::MustNotExist, "tix commit stash rewrite")
}

fn delete_edit(name: gix::refs::FullName, target: Target) -> RefEdit {
    RefEdit::delete(name, PreviousValue::MustExistAndMatch(target))
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
pub(crate) fn save_manual(repository_path: &Path, bare: bool, id: ObjectId) -> Result<String> {
    let repo = open_repository(repository_path, bare, false).context("could not open repository to stash changes")?;
    let _guard = super::mutation_lock(&repo)?;
    let workdir = repo
        .workdir()
        .context("stashing changes requires a worktree")?
        .to_owned();
    let head = repo
        .head_id()
        .context("stashing changes requires a born HEAD")?
        .detach();
    if head != id {
        anyhow::bail!("changes can only be stashed at the current HEAD");
    }
    if repo
        .index_or_empty()
        .context("could not inspect the index before stashing")?
        .entries()
        .iter()
        .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted)
    {
        anyhow::bail!("cannot stash changes with unresolved index conflicts");
    }
    let name = reference(&repo, id)?;
    if repo.try_find_reference(name.as_ref())?.is_some() {
        anyhow::bail!("{} already has saved worktree state", id.to_hex_with_len(7));
    }
    drop(repo);
    if !super::review::is_dirty(&workdir)? {
        anyhow::bail!("there are no worktree or index changes to stash");
    }
    let saved = save(
        repository_path,
        bare,
        &workdir,
        name,
        format!("tix {}", id.to_hex_with_len(7)),
        "tix commit stash",
        "commit state",
    )?;
    let mut notice = format!("stashed changes at {}", id.to_hex_with_len(7));
    if let Some(warning) = saved.warning {
        write!(notice, "; {warning}").expect("writing to a string cannot fail");
    }
    Ok(notice)
}

#[tracing::instrument(skip_all, fields(commit_id = %id))]
pub(crate) fn restore_manual(repository_path: &Path, bare: bool, id: ObjectId) -> Result<String> {
    let repo = open_repository(repository_path, bare, false).context("could not open repository to unstash changes")?;
    let _guard = super::mutation_lock(&repo)?;
    let workdir = repo
        .workdir()
        .context("unstashing changes requires a worktree")?
        .to_owned();
    if repo
        .head_id()
        .context("unstashing changes requires a born HEAD")?
        .detach()
        != id
    {
        anyhow::bail!("changes can only be unstashed at the current HEAD");
    }
    drop(repo);
    let saved =
        find_for_commit(repository_path, bare, id)?.context("the selected commit has no saved worktree state")?;
    apply(repository_path, bare, &workdir, saved)
}

#[derive(Clone)]
pub(crate) struct SavedStash {
    pub name: gix::refs::FullName,
    pub target: Target,
    pub warning: Option<String>,
}

pub(crate) fn all(repo: &gix::Repository) -> Result<Vec<SavedStash>> {
    let mut saved = Vec::new();
    for reference in repo.references()?.all()? {
        let reference = reference.context("could not enumerate saved Tix stashes")?;
        let name = reference.name();
        if name.as_bstr().starts_with(crate::history::STASH_PREFIX)
            || name.as_bstr().starts_with(crate::history::REVIEW_STASH_PREFIX)
        {
            saved.push(SavedStash {
                name: name.to_owned(),
                target: reference.target().into_owned(),
                warning: None,
            });
        }
    }
    saved.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(saved)
}

pub(crate) fn restore_selected(repo: &gix::Repository, selector: &str) -> Result<String> {
    let _guard = super::mutation_lock(repo)?;
    let workdir = repo.workdir().context("unstashing changes requires a worktree")?;
    repo.head_id().context("unstashing changes requires a born HEAD")?;
    let prefix = gix::hash::Prefix::from_hex(selector).ok();
    let matches = all(repo)?
        .into_iter()
        .filter(|stash| {
            stash.name.as_bstr() == selector.as_bytes()
                || prefix.as_ref().is_some_and(|prefix| {
                    stash.target.try_id().is_some_and(|commit_id| {
                        prefix.hex_len() <= commit_id.kind().len_in_hex() && prefix.cmp_oid(commit_id).is_eq()
                    })
                })
        })
        .collect::<Vec<_>>();
    let saved = match matches.as_slice() {
        [saved] => saved.clone(),
        [] => anyhow::bail!("no Tix stash matches {selector:?}; use `tix stash list` to find saved state"),
        _ => anyhow::bail!("stash {selector:?} is ambiguous; use its full reference from `tix stash list`"),
    };
    let name = saved.name.clone();
    let notice = apply(repo.git_dir(), repo.is_bare(), workdir, saved)?;
    anyhow::ensure!(repo.try_find_reference(name.as_ref())?.is_none(), "{notice}");
    Ok(notice)
}

pub(super) fn save_if_dirty(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    departure_commit_id: ObjectId,
    name: gix::refs::FullName,
) -> Result<Option<SavedStash>> {
    let repo = open_repository(repository_path, bare, false).context("could not verify the stash departure")?;
    let _guard = super::mutation_lock(&repo)?;
    anyhow::ensure!(
        repo.head_id()? == departure_commit_id,
        "changes can only be stashed at the current HEAD"
    );
    drop(repo);
    if !super::review::is_dirty(workdir)? {
        return Ok(None);
    }
    let message = format!("tix travel {}", name.shorten());
    save(
        repository_path,
        bare,
        workdir,
        name,
        message,
        "tix travel auto-stash",
        "departure state",
    )
    .map(Some)
}

pub(super) fn remap(
    repo: &gix::Repository,
    saved: &mut SavedStash,
    departure_commit_id: ObjectId,
    map: impl FnOnce(ObjectId) -> Option<ObjectId>,
) -> Result<()> {
    if associated_change(saved.name.as_bstr())?.is_some() {
        saved.name = reference(
            repo,
            map(departure_commit_id).context("the stashed departure disappeared during replay")?,
        )?;
    }
    Ok(())
}

pub(super) fn restore_after_failure(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    saved: SavedStash,
    cause: anyhow::Error,
) -> anyhow::Error {
    let saved_id = saved.target.try_id().map(|id| id.to_string());
    let restore = (|| -> Result<String> {
        let repo =
            open_repository(repository_path, bare, false).context("could not inspect the rolled-back departure")?;
        let _guard = super::mutation_lock(&repo)?;
        let stash_commit_id = saved.target.try_id().context("a saved stash must point to an object")?;
        let departure_commit_id = repo
            .find_commit(stash_commit_id)?
            .parent_ids()
            .next()
            .context("a saved stash must have a departure parent")?;
        anyhow::ensure!(
            repo.head_id()? == departure_commit_id,
            "HEAD was not restored to the departure"
        );
        anyhow::ensure!(
            !super::review::is_dirty(workdir)?,
            "the departure is not clean after rollback"
        );
        anyhow::ensure!(
            saved.target == repo.find_reference(saved.name.as_ref())?.target(),
            "the departure stash reference changed"
        );
        drop(repo);
        apply(repository_path, bare, workdir, saved)
    })();
    match restore {
        Ok(notice) => cause.context(format!("departure stash restoration: {notice}")),
        Err(restore) => cause.context(format!(
            "departure stash could not be restored: {restore:#}; saved stash object: {}",
            saved_id.as_deref().unwrap_or("unknown")
        )),
    }
}

#[tracing::instrument(skip_all, fields(stash = %name))]
pub(super) fn save(
    repository_path: &Path,
    bare: bool,
    workdir: &Path,
    name: gix::refs::FullName,
    message: String,
    reflog_message: &'static str,
    state_label: &'static str,
) -> Result<SavedStash> {
    let repo = open_repository(repository_path, bare, false)
        .with_context(|| format!("could not open repository to save {state_label}"))?;
    let _guard = super::mutation_lock(&repo)?;
    if repo.try_find_reference(name.as_ref())?.is_some() {
        anyhow::bail!("{state_label} is already saved");
    }
    let previous = repo
        .try_find_reference("refs/stash")?
        .and_then(|mut reference| reference.peel_to_id().ok().map(gix::Id::detach));
    drop(repo);

    let output = crate::git_command(workdir)
        .args(["stash", "push", "--include-untracked", "--quiet", "--message"])
        .arg(message)
        .output()
        .context("could not launch git stash push")?;
    if !output.status.success() {
        anyhow::bail!("git stash push failed: {}", output.stderr.trim().to_str_lossy());
    }

    let repo = open_repository(repository_path, bare, false).context("could not reopen repository after stashing")?;
    let mut stash = repo
        .try_find_reference("refs/stash")?
        .context("git stash push did not create refs/stash")?;
    let id = stash.peel_to_id()?.detach();
    if previous == Some(id) {
        anyhow::bail!("git stash push did not create a new stash");
    }
    let target = Target::Object(id);
    if let Err(err) = repo.edit_references([RefEdit::update(
        name.clone(),
        target.clone(),
        PreviousValue::MustNotExist,
        reflog_message,
    )]) {
        drop(repo);
        let restore = crate::git_command(workdir)
            .args(["stash", "pop", "--index", "--quiet"])
            .output();
        return Err(anyhow::anyhow!(err)).context(match restore {
            Ok(output) if output.status.success() => {
                format!("could not retain {state_label}; original state was restored")
            }
            Ok(output) => format!(
                "could not retain {state_label} and git stash pop failed: {}",
                output.stderr.trim().to_str_lossy()
            ),
            Err(restore) => {
                format!("could not retain {state_label} and could not launch git stash pop: {restore}")
            }
        });
    }
    drop(repo);

    let warning = match current(repository_path, bare) {
        Ok(Some(current)) if current == id => {
            match crate::git_command(workdir)
                .args(["stash", "drop", "--quiet", "stash@{0}"])
                .output()
            {
                Ok(output) => (!output.status.success()).then(|| {
                    format!(
                        "{state_label} was saved, but its ordinary stash entry remains: {}",
                        output.stderr.trim().to_str_lossy()
                    )
                }),
                Err(err) => Some(format!(
                    "{state_label} was saved, but could not launch git stash drop: {err}"
                )),
            }
        }
        Ok(_) => Some(format!(
            "{state_label} was saved, but refs/stash changed before its entry could be dropped"
        )),
        Err(err) => Some(format!(
            "{state_label} was saved, but could not inspect its ordinary stash entry: {err:#}"
        )),
    };
    tracing::info!(stash = %name, %id, "saved worktree state");
    Ok(SavedStash { name, target, warning })
}

fn current(repository_path: &Path, bare: bool) -> Result<Option<ObjectId>> {
    let repo = open_repository(repository_path, bare, false).context("could not inspect refs/stash")?;
    let Some(mut reference) = repo.try_find_reference("refs/stash")? else {
        return Ok(None);
    };
    Ok(Some(reference.peel_to_id()?.detach()))
}

pub(super) fn find(repository_path: &Path, bare: bool, name: gix::refs::FullName) -> Result<Option<SavedStash>> {
    let repo = open_repository(repository_path, bare, false).context("could not inspect saved worktree state")?;
    let Some(reference) = repo.try_find_reference(name.as_ref())? else {
        return Ok(None);
    };
    Ok(Some(SavedStash {
        name,
        target: reference.target().into_owned(),
        warning: None,
    }))
}

pub(super) fn find_for_commit(repository_path: &Path, bare: bool, commit_id: ObjectId) -> Result<Option<SavedStash>> {
    let repo = open_repository(repository_path, bare, false).context("could not locate the stashed change")?;
    let change_id = crate::change_id::for_commit(&repo, commit_id)?;
    let Some(saved) = find(repository_path, bare, reference_for_change(change_id)?)? else {
        return Ok(None);
    };
    let graph = super::loaded_view_graph(&repo)?;
    for candidate in graph.stored_commit_ids() {
        anyhow::ensure!(
            candidate == commit_id || crate::change_id::for_commit(&repo, candidate)? != change_id,
            "stashed change {} is ambiguous in the Tix view; choose its reference with `tix stash restore`",
            change_id.to_reverse_hex_with_len(7)
        );
    }
    Ok(Some(saved))
}

#[tracing::instrument(skip_all, fields(stash = %stash.name))]
pub(super) fn apply(repository_path: &Path, bare: bool, workdir: &Path, stash: SavedStash) -> Result<String> {
    let repo = open_repository(repository_path, bare, false)
        .context("could not open repository before applying saved worktree state")?;
    let _guard = super::mutation_lock(&repo)?;
    anyhow::ensure!(
        stash.target == repo.find_reference(stash.name.as_ref())?.target(),
        "the saved stash reference changed before restoration"
    );
    let stash_commit_id = stash.target.try_id().context("a saved stash must point to an object")?;
    let output = crate::git_command(workdir)
        .args(["stash", "apply", "--index", "--quiet"])
        .arg(stash_commit_id.to_string())
        .output()
        .context("could not launch git stash apply")?;
    let notice = if output.status.success() {
        let mut notice = format!("restored {}", stash.name.shorten());
        if let Err(err) = repo.edit_references([RefEdit::delete(
            stash.name.clone(),
            PreviousValue::MustExistAndMatch(stash.target),
        )]) {
            write!(notice, "; stash reference remains: {err}").expect("writing to a string cannot fail");
        }
        notice
    } else {
        format!(
            "{} restore needs attention; stash reference remains: {}",
            stash.name.shorten(),
            output.stderr.trim().to_str_lossy()
        )
    };
    tracing::info!(stash = %stash.name, success = output.status.success(), "applied saved worktree state");
    Ok(notice)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(path: &Path, args: &[&str]) -> gix_testtools::Result<Vec<u8>> {
        let output = gix_testtools::git_command(path).args(args).output()?;
        if !output.status.success() {
            return Err(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(output.stdout)
    }

    #[test]
    fn stash_change_names_require_complete_canonical_ids() -> gix_testtools::Result {
        for &kind in gix::hash::Kind::all() {
            let change_id = ChangeId::from(ObjectId::null(kind));
            let name = reference_for_change(change_id)?;
            assert_eq!(
                associated_change(name.as_bstr())?,
                Some(change_id),
                "each supported hash kind round-trips"
            );
            let truncated = &name.as_bstr()[..name.as_bstr().len() - 1];
            assert!(
                associated_change(truncated.as_bstr()).is_err(),
                "abbreviated change IDs cannot own stashes"
            );
        }
        assert!(
            associated_change(b"refs/tix/stash/0123456789012345678901234567890123456789".as_bstr()).is_err(),
            "old commit-hash associations require explicit recovery"
        );
        Ok(())
    }

    #[test]
    fn stash_change_identity_survives_an_ancestor_reword() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let old_commit_id = repo.head_id()?.detach();
        std::fs::write(fixture.path().join("saved"), "work in progress\n")?;
        save_manual(repo.git_dir(), false, old_commit_id)?;
        let name = reference(&repo, old_commit_id)?;
        let stash_commit_id = repo.find_reference(name.as_ref())?.id().detach();
        let graph = super::super::loaded_graph(&repo)?;
        super::super::reword::apply_message_reporting(
            repo.clone(),
            &graph,
            repo.rev_parse_single("HEAD~1")?.detach(),
            b"reworded parent\n",
            None,
        )?;
        let new_commit_id = repo.head_id()?.detach();
        assert_ne!(old_commit_id, new_commit_id, "rewording the parent rewrites its child");
        assert_eq!(
            reference(&repo, new_commit_id)?,
            name,
            "the stash ref depends on stable change identity"
        );
        assert_eq!(
            repo.find_reference(name.as_ref())?.id(),
            stash_commit_id,
            "saved data needs no ref move"
        );
        restore_manual(repo.git_dir(), false, new_commit_id)?;
        assert_eq!(std::fs::read(fixture.path().join("saved"))?, b"work in progress\n");
        Ok(())
    }

    #[test]
    fn stash_change_identity_survives_an_external_reword() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let graph = super::super::loaded_graph(&repo)?;
        super::super::reword::apply_message_reporting(
            repo.clone(),
            &graph,
            repo.head_id()?.detach(),
            b"commit with a stable identity\n",
            None,
        )?;
        let old_commit_id = repo.head_id()?.detach();
        std::fs::write(fixture.path().join("saved"), "work in progress\n")?;
        save_manual(repo.git_dir(), false, old_commit_id)?;
        let name = reference(&repo, old_commit_id)?;
        git(
            fixture.path(),
            &[
                "-c",
                "commit.gpgSign=false",
                "commit",
                "--amend",
                "-qm",
                "external reword",
            ],
        )?;
        let new_commit_id = repo.head_id()?.detach();
        assert_ne!(old_commit_id, new_commit_id, "Git rewrites HEAD outside Tix");
        assert_eq!(
            reference(&repo, new_commit_id)?,
            name,
            "Git preserves the stable change header"
        );
        let decorations = crate::history::decorations(&repo, &[], &[])?;
        assert!(
            decorations.has_stash(new_commit_id, crate::change_id::for_commit(&repo, new_commit_id)?),
            "the saved state remains visible without any association ref update"
        );
        restore_manual(repo.git_dir(), false, new_commit_id)?;
        assert_eq!(std::fs::read(fixture.path().join("saved"))?, b"work in progress\n");
        Ok(())
    }

    #[test]
    fn stash_change_identity_requires_an_unambiguous_visible_version() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let old_commit_id = repo.head_id()?.detach();
        std::fs::write(fixture.path().join("saved"), "work in progress\n")?;
        save_manual(repo.git_dir(), false, old_commit_id)?;
        let graph = super::super::loaded_graph(&repo)?;
        super::super::reword::apply_message_reporting(
            repo.clone(),
            &graph,
            old_commit_id,
            b"a second version\n",
            None,
        )?;
        let new_commit_id = repo.head_id()?.detach();
        repo.reference(
            "refs/worktree/tix/pins/otherversion",
            old_commit_id,
            PreviousValue::MustNotExist,
            "retain another version of the same change",
        )?;
        assert!(
            super::super::loaded_view_graph(&repo)?
                .stored_commit_ids()
                .any(|id| id == old_commit_id),
            "the alternate version is visible to automatic stash lookup"
        );
        let before = gix_testtools::repository::snapshot(fixture.path())?;
        let err = restore_manual(repo.git_dir(), false, new_commit_id)
            .expect_err("automatic lookup cannot choose between two versions of the same change");
        assert!(err.to_string().contains("ambiguous"), "{err:#}");
        assert_eq!(
            gix_testtools::repository::snapshot(fixture.path())?,
            before,
            "ambiguous restoration leaves saved and local state alone"
        );
        restore_selected(&repo, &reference(&repo, new_commit_id)?.to_string())?;
        assert_eq!(
            std::fs::read(fixture.path().join("saved"))?,
            b"work in progress\n",
            "an explicit choice can still recover the stash"
        );
        Ok(())
    }

    #[test]
    fn stash_and_rewrites_respect_the_shared_worktree_lock() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let linked = gix_testtools::tempfile::tempdir()?;
        let linked_path = linked.path().join("linked");
        let output = gix_testtools::git_command(fixture.path())
            .args(["worktree", "add", "--detach"])
            .arg(&linked_path)
            .output()?;
        assert!(output.status.success(), "the disposable linked worktree is created");
        let repo = crate::test_repository::open(fixture.path())?;
        let lock = gix::lock::Marker::acquire_to_hold_resource(
            repo.common_dir().join("tix-mutation"),
            gix::lock::acquire::Fail::Immediately,
            None,
            0,
        )?;

        for path in [fixture.path(), linked_path.as_path()] {
            let repo = crate::test_repository::open(path)?;
            let head_commit_id = repo.head_id()?.detach();
            let graph = super::super::loaded_graph(&repo)?;
            std::fs::write(path.join("local-state"), "saved work\n")?;
            let err = save_manual(repo.git_dir(), false, head_commit_id)
                .expect_err("a competing mutation prevents stashing before Git changes the worktree");
            assert!(err.to_string().contains("another Tix operation"), "{err:#}");
            let err = super::super::reword::apply_message_reporting(
                repo.clone(),
                &graph,
                head_commit_id,
                b"rewritten while stashing\n",
                None,
            )
            .err()
            .expect("a competing mutation prevents publication of the rewrite");
            assert!(err.to_string().contains("another Tix operation"), "{err:#}");
            assert_eq!(
                repo.head_id()?,
                head_commit_id,
                "the competing operation leaves HEAD alone"
            );
            assert_eq!(
                std::fs::read(path.join("local-state"))?,
                b"saved work\n",
                "the competing operation leaves local changes alone"
            );
            assert!(
                repo.try_find_reference(reference(&repo, head_commit_id)?.as_ref())?
                    .is_none(),
                "the rejected save did not publish a stash"
            );
        }

        drop(lock);
        let head_commit_id = repo.head_id()?.detach();
        save_manual(repo.git_dir(), false, head_commit_id)?;
        restore_manual(repo.git_dir(), false, head_commit_id)?;
        assert_eq!(
            std::fs::read(fixture.path().join("local-state"))?,
            b"saved work\n",
            "saving and restoring work once the competing operation releases its lock"
        );
        Ok(())
    }

    #[test]
    fn manual_stashes_preserve_git_stashes_and_restore_index_and_worktree_state() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        git(fixture.path(), &["init", "-q", "-b", "main"])?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
        git(fixture.path(), &["config", "user.name", "user"])?;
        git(fixture.path(), &["config", "user.email", "user@example.com"])?;
        std::fs::write(fixture.path().join("tracked"), "base\n")?;
        std::fs::write(fixture.path().join(".gitignore"), "ignored\n")?;
        git(fixture.path(), &["add", "."])?;
        git(fixture.path(), &["-c", "commit.gpgSign=false", "commit", "-qm", "base"])?;
        let head = ObjectId::from_hex(git(fixture.path(), &["rev-parse", "HEAD"])?.trim())?;

        std::fs::write(fixture.path().join("ordinary"), "ordinary stash\n")?;
        git(
            fixture.path(),
            &["stash", "push", "--include-untracked", "-qm", "ordinary"],
        )?;
        let ordinary = ObjectId::from_hex(git(fixture.path(), &["rev-parse", "refs/stash"])?.trim())?;

        std::fs::write(fixture.path().join("staged"), "staged\n")?;
        git(fixture.path(), &["add", "staged"])?;
        std::fs::write(fixture.path().join("tracked"), "unstaged\n")?;
        std::fs::write(fixture.path().join("untracked"), "untracked\n")?;
        std::fs::write(fixture.path().join("ignored"), "ignored\n")?;

        let repo = crate::test_repository::open(fixture.path())?;
        let repository_path = repo.git_dir().to_owned();
        drop(repo);
        let notice = save_manual(&repository_path, false, head)?;
        assert!(notice.contains("stashed changes"));

        let repo = crate::test_repository::open(fixture.path())?;
        assert_eq!(repo.find_reference("refs/stash")?.id(), ordinary);
        let name = reference(&repo, head)?;
        assert!(repo.try_find_reference(name.as_ref())?.is_some());
        assert!(
            git(fixture.path(), &["status", "--porcelain=v1", "--untracked-files=all"])?.is_empty(),
            "the tracked and untracked changes were stashed"
        );
        assert_eq!(std::fs::read(fixture.path().join("ignored"))?, b"ignored\n");
        drop(repo);

        std::fs::write(fixture.path().join("local"), "existing worktree change\n")?;
        restore_manual(&repository_path, false, head)?;
        assert_eq!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.trim(),
            b"staged"
        );
        assert_eq!(git(fixture.path(), &["diff", "--name-only"])?.trim(), b"tracked");
        assert_eq!(std::fs::read(fixture.path().join("untracked"))?, b"untracked\n");
        assert_eq!(
            std::fs::read(fixture.path().join("local"))?,
            b"existing worktree change\n",
            "unstashing leaves unrelated worktree changes intact"
        );
        let repo = crate::test_repository::open(fixture.path())?;
        assert!(repo.try_find_reference(name.as_ref())?.is_none());
        assert_eq!(repo.find_reference("refs/stash")?.id(), ordinary);
        Ok(())
    }

    #[test]
    fn manual_stash_survives_an_index_conflict_and_can_restore_all_saved_files() -> gix_testtools::Result {
        let fixture = gix_testtools::tempfile::tempdir()?;
        git(fixture.path(), &["init", "-q", "-b", "main"])?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
        git(fixture.path(), &["config", "user.name", "user"])?;
        git(fixture.path(), &["config", "user.email", "user@example.com"])?;
        std::fs::write(fixture.path().join("Cargo.lock"), "base\n")?;
        std::fs::write(fixture.path().join("tracked"), "base\n")?;
        git(fixture.path(), &["add", "."])?;
        git(fixture.path(), &["-c", "commit.gpgSign=false", "commit", "-qm", "base"])?;
        let repo = crate::test_repository::open(fixture.path())?;
        let repository_path = repo.git_dir().to_owned();
        let head_commit_id = repo.head_id()?.detach();
        let name = reference(&repo, head_commit_id)?;
        drop(repo);

        std::fs::write(fixture.path().join("Cargo.lock"), "staged\n")?;
        git(fixture.path(), &["add", "Cargo.lock"])?;
        std::fs::write(fixture.path().join("tracked"), "unstaged\n")?;
        std::fs::write(fixture.path().join("untracked"), "untracked\n")?;
        save_manual(&repository_path, false, head_commit_id)?;
        let stash_commit_id = crate::test_repository::open(fixture.path())?
            .find_reference(name.as_ref())?
            .id()
            .detach();

        // A conflicting staged change makes --index fail before Git restores the other files.
        std::fs::write(fixture.path().join("Cargo.lock"), "local\n")?;
        git(fixture.path(), &["add", "Cargo.lock"])?;
        let notice = restore_manual(&repository_path, false, head_commit_id)?;
        assert!(
            notice.contains("needs attention"),
            "the index conflict is reported: {notice}"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("Cargo.lock"))?,
            b"local\n",
            "the conflicting local change is preserved"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("tracked"))?,
            b"base\n",
            "the early index failure prevents restoring unstaged changes"
        );
        assert!(
            !fixture.path().join("untracked").exists(),
            "the early index failure also prevents restoring untracked files"
        );
        assert_eq!(
            crate::test_repository::open(fixture.path())?
                .find_reference(name.as_ref())?
                .id(),
            stash_commit_id,
            "a failed apply retains the complete original stash"
        );
        assert!(
            notice.contains("stash reference remains"),
            "the notice explains that saved state remains available: {notice}"
        );

        git(
            fixture.path(),
            &["restore", "--source=HEAD", "--staged", "--worktree", "Cargo.lock"],
        )?;
        restore_manual(&repository_path, false, head_commit_id)?;
        assert_eq!(
            git(fixture.path(), &["show", ":Cargo.lock"])?,
            b"staged\n",
            "retry restores the saved index"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("Cargo.lock"))?,
            b"staged\n",
            "retry restores the staged file's worktree contents"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("tracked"))?,
            b"unstaged\n",
            "retry restores the previously skipped tracked change"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("untracked"))?,
            b"untracked\n",
            "retry restores the previously skipped untracked file"
        );
        assert_eq!(
            git(fixture.path(), &["diff", "--cached", "--name-only"])?.trim(),
            b"Cargo.lock",
            "only the original staged path is staged"
        );
        assert_eq!(
            git(fixture.path(), &["diff", "--name-only"])?.trim(),
            b"tracked",
            "the original unstaged change stays unstaged"
        );
        assert!(
            crate::test_repository::open(fixture.path())?
                .try_find_reference(name.as_ref())?
                .is_none(),
            "only a complete restoration consumes the stash"
        );
        Ok(())
    }
}
