use anyhow::{Context, Result, ensure};
use gix::{
    ObjectId,
    bstr::{BStr, ByteSlice},
    refs::{FullName, transaction::RefEdit},
};

use super::{Plan, PlanParent};
use crate::edit::{todo, undo};

#[cfg(test)]
pub(crate) mod tests;

pub(crate) const REF: &str = "refs/worktree/tix/rebase";
const TITLE: &str = "tix paused rebase";

/// Detached state only: neither the UI nor a saved continuation keeps a repository open.
#[derive(Clone, Debug)]
pub(crate) struct Session {
    pub commit_id: ObjectId,
    pub operation: String,
    pub conflict_commit_id: ObjectId,
    pub document: Vec<u8>,
    head_commit_id: ObjectId,
    head: undo::State,
    changes: Vec<undo::RefChange>,
    publishing: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Readiness {
    Conflicted,
    Ready,
    Blocked(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Summary {
    pub operation: String,
    pub conflict_commit_id: Option<ObjectId>,
    pub remaining: usize,
    pub readiness: Readiness,
}

pub(crate) fn is_ref(name: &BStr) -> bool {
    name == REF.as_bytes()
}

pub(crate) fn is_commit(repo: &gix::Repository, commit_id: ObjectId) -> Result<bool> {
    let Ok(commit) = repo.find_commit(commit_id) else {
        return Ok(false);
    };
    let message = commit.message_raw()?;
    Ok(message.starts_with(format!("{TITLE}\n\n[rebase]").as_bytes()))
}

pub(crate) fn ensure_idle(repo: &gix::Repository) -> Result<()> {
    ensure!(
        repo.try_find_reference(REF)?.is_none() && !repo.git_dir().join("tix-rebase.lock").try_exists()?,
        "a rebase is paused in this worktree; use `tix rebase status`, `continue`, or `stop` first"
    );
    Ok(())
}

pub(crate) fn load(repo: &gix::Repository) -> Result<Option<Session>> {
    let Some(reference) = repo.try_find_reference(REF)? else {
        return Ok(None);
    };
    let commit_id = reference
        .try_id()
        .context("the rebase state reference must be direct")?
        .detach();
    let commit = repo.find_commit(commit_id).context("could not read the saved rebase")?;
    let message = commit.message_raw()?;
    let body = message
        .strip_prefix(format!("{TITLE}\n\n").as_bytes())
        .context("the rebase state commit has an invalid header")?;
    let config = gix::config::File::try_from(body.as_bstr()).context("could not parse the saved rebase")?;
    let mut sections = config.sections();
    let section = sections.next().context("the saved rebase has no metadata")?;
    ensure!(
        section.header().name() == b"rebase"
            && section.header().subsection_name().is_none()
            && sections.next().is_none(),
        "the saved rebase must contain one [rebase] section"
    );
    ensure!(
        section.value_names().collect::<Vec<_>>()
            == [
                "version",
                "operation",
                "conflict",
                "checkout",
                "head",
                "publishing",
                "todo",
                "undo"
            ],
        "the saved rebase has missing, repeated, or unknown fields"
    );
    let value = |key| {
        section
            .value(key)
            .with_context(|| format!("the saved rebase has no {key}"))
    };
    ensure!(value("version")?.as_slice() == b"1", "unsupported saved rebase version");
    let operation = value("operation")?.to_str()?.to_owned();
    ensure!(
        !operation.is_empty() && !operation.contains(['\n', '\r']),
        "invalid saved operation label"
    );
    let object_id = |key| -> Result<ObjectId> {
        let commit_id = ObjectId::from_hex(value(key)?.as_ref())?;
        ensure!(
            commit_id.kind() == repo.object_hash(),
            "saved rebase object uses the wrong hash kind"
        );
        repo.find_commit(commit_id)
            .with_context(|| format!("the saved {key} commit is missing"))?;
        Ok(commit_id)
    };
    let publishing = match value("publishing")?.as_slice() {
        b"true" => true,
        b"false" => false,
        _ => anyhow::bail!("invalid saved rebase publication state"),
    };
    let session = Session {
        commit_id,
        operation,
        conflict_commit_id: object_id("conflict")?,
        head_commit_id: object_id("checkout")?,
        head: undo::parse_state(repo, value("head")?.as_bstr())?,
        publishing,
        document: value("todo")?.into(),
        changes: undo::parse_config(repo, value("undo")?.as_bstr())?,
    };
    ensure!(
        session.head != undo::State::Missing,
        "the saved rebase has an unborn HEAD"
    );
    ensure!(
        !session.changes.iter().any(|change| is_ref(change.name.as_bstr())),
        "the saved rebase records itself"
    );
    session.parsed(repo)?;
    Ok(Some(session))
}

pub(crate) fn status(repo: &gix::Repository) -> Result<Option<Summary>> {
    let session = match load(repo) {
        Ok(Some(session)) => session,
        Ok(None) => return Ok(None),
        Err(err) => {
            return Ok(Some(Summary {
                operation: "rebase".into(),
                conflict_commit_id: None,
                remaining: 0,
                readiness: Readiness::Blocked(format!("{err:#}")),
            }));
        }
    };
    let parsed = session.parsed(repo)?;
    let readiness = match session.current(repo).and_then(|_| index_conflicted(repo)) {
        Err(err) => Readiness::Blocked(format!("{err:#}")),
        Ok(true) => Readiness::Conflicted,
        Ok(false) => Readiness::Ready,
    };
    Ok(Some(Summary {
        operation: session.operation,
        conflict_commit_id: Some(session.conflict_commit_id),
        remaining: parsed.plan.steps.len() + parsed.plan.steps.iter().map(|step| step.squash.len()).sum::<usize>(),
        readiness,
    }))
}

fn index_conflicted(repo: &gix::Repository) -> Result<bool> {
    Ok(repo
        .index_or_empty()?
        .entries()
        .iter()
        .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted))
}

impl Session {
    pub(crate) fn parsed(&self, repo: &gix::Repository) -> Result<todo::Parsed> {
        let parsed = todo::parse(repo, &self.document)?.context("the saved continuation is empty")?;
        ensure!(
            parsed.resolved == Some(self.conflict_commit_id),
            "the saved continuation belongs to another conflict"
        );
        Ok(parsed)
    }

    /// Accept the captured checkout or an amendment of the same change and parents with a matching, resolved index.
    /// Inspection computes this in memory; refusing a later conflict must not change the saved state.
    pub(crate) fn current(&self, repo: &gix::Repository) -> Result<Self> {
        ensure!(
            !self.publishing,
            "rebase publication is incomplete; wait for the running command, or inspect the checkout and use `tix rebase stop` after an interruption"
        );
        ensure!(
            undo::state(repo, REF.try_into()?)? == undo::State::Object(self.commit_id),
            "the saved rebase changed; retry with its current state"
        );
        let head = repo.head()?;
        let head_commit_id = head.id().context("HEAD became unborn during the rebase")?.detach();
        let attachment = head.referent_name().map(ToOwned::to_owned);
        let expected_attachment = match &self.head {
            undo::State::Symbolic(name) => Some(name.clone()),
            _ => None,
        };
        ensure!(
            attachment == expected_attachment,
            "HEAD attachment changed; return to the paused checkout or stop the rebase"
        );
        let name: FullName = attachment.unwrap_or_else(|| "HEAD".try_into().expect("valid HEAD reference"));
        ensure!(
            undo::state(repo, name.as_ref())? == undo::State::Object(head_commit_id),
            "the paused checkout no longer has a direct target"
        );
        let mut current = self.clone();
        if head_commit_id != self.head_commit_id {
            let previous = repo.find_commit(self.head_commit_id)?.decode()?.into_owned()?;
            let replacement = repo.find_commit(head_commit_id)?.decode()?.into_owned()?;
            ensure!(
                previous.parents == replacement.parents,
                "HEAD moved to an unrelated commit; return to the paused checkout or stop the rebase"
            );
            ensure!(
                crate::change_id::for_commit(repo, self.head_commit_id)?
                    == crate::change_id::for_commit(repo, head_commit_id)?,
                "HEAD moved to a different change; return to the paused checkout or stop the rebase"
            );
            ensure!(
                !index_conflicted(repo)?,
                "the replacement HEAD still has unresolved index conflicts"
            );
            // Computing an index tree for status must not write objects to disk.
            let memory = repo.clone().with_object_memory();
            let index = repo.index_or_empty()?;
            ensure!(
                crate::edit::create::index_tree(&memory, &index)? == replacement.tree,
                "the replacement HEAD does not match the resolved index"
            );
            current.changes.push(undo::RefChange {
                name,
                before: undo::State::Object(self.head_commit_id),
                after: undo::State::Object(head_commit_id),
            });
            current.changes = undo::normalize_changes(current.changes)?;
            current.head_commit_id = head_commit_id;
            current.head = undo::state(repo, "HEAD".try_into()?)?;
        }
        for change in &current.changes {
            ensure!(
                undo::state(repo, change.name.as_ref())? == change.after,
                "saved rebase reference {} changed; restore it or stop the rebase",
                change.name
            );
        }
        let mut parsed = current.parsed(repo)?;
        current.update_expected_refs(&mut parsed.plan);
        for reference in &parsed.plan.expected_refs {
            let expected = reference.old.map_or(undo::State::Missing, undo::State::Object);
            ensure!(
                undo::state(repo, reference.name.as_ref())? == expected,
                "rebase reference {} changed; restore it or stop the rebase",
                reference.name
            );
        }
        Ok(current)
    }

    fn update_expected_refs(&self, plan: &mut Plan) {
        for reference in &mut plan.expected_refs {
            if let Some(change) = self.changes.iter().find(|change| change.name == reference.name)
                && change.before == reference.old.map_or(undo::State::Missing, undo::State::Object)
            {
                reference.old = match change.after {
                    undo::State::Object(commit_id) => Some(commit_id),
                    _ => None,
                };
            }
        }
    }

    fn write(&self, repo: &gix::Repository) -> Result<ObjectId> {
        let parsed = self.parsed(repo)?;
        let mut parents = undo::retention_parents(repo, self.conflict_commit_id, &self.changes)?;
        parents.push(self.head_commit_id);
        parents.push(parsed.plan.base);
        parents.extend(parsed.plan.scope);
        parents.extend(parsed.tips);
        parents.extend(
            parsed
                .plan
                .expected_refs
                .iter()
                .flat_map(|reference| reference.old.into_iter().chain([reference.source])),
        );
        parents.extend(
            parsed
                .plan
                .steps
                .iter()
                .flat_map(|step| &step.parents)
                .filter_map(|parent| match parent {
                    PlanParent::Existing(commit_id) => Some(*commit_id),
                    PlanParent::Step(_) => None,
                }),
        );
        parents.extend(
            parsed
                .plan
                .checkout
                .iter()
                .map(|checkout| checkout.target)
                .chain(parsed.plan.selection)
                .filter_map(|parent| match parent {
                    PlanParent::Existing(commit_id) => Some(commit_id),
                    PlanParent::Step(_) => None,
                }),
        );
        let mut seen = std::collections::HashSet::new();
        parents.retain(|commit_id| seen.insert(*commit_id));
        let mut config = gix::config::File::default();
        let mut section = config.new_section("rebase", None)?;
        section.set("version", "1")?;
        section.set("operation", self.operation.as_str())?;
        section.set("conflict", self.conflict_commit_id.to_string())?;
        section.set("checkout", self.head_commit_id.to_string())?;
        section.set("head", undo::encode_state(&self.head))?;
        section.set("publishing", if self.publishing { "true" } else { "false" })?;
        section.set("todo", self.document.as_bstr())?;
        section.set("undo", undo::serialize_config(&self.changes)?.to_bstring())?;
        undo::write_commit(repo, TITLE, &config, &parents)
    }
}

/// A checked publication is reserved with the history refs, then completed after checkout.
/// Failures roll back that reservation together with the ordinary edit and index changes.
pub(super) struct Publication {
    session: Session,
    previous: Option<ObjectId>,
    reserved: Option<ObjectId>,
    complete: bool,
    lock: Option<gix::lock::Marker>,
}

impl Publication {
    pub(super) fn for_plan(repo: &gix::Repository, plan: &mut Plan) -> Result<Option<Self>> {
        let Some(session) = load(repo)? else {
            ensure_idle(repo)?;
            return Ok(None);
        };
        let session = session.current(repo)?;
        let saved = session.parsed(repo)?;
        ensure!(
            plan.scope == saved.plan.scope && plan.base == saved.plan.base,
            "another rebase is paused; apply its continuation or use `tix rebase stop` first"
        );
        ensure!(
            !index_conflicted(repo)?,
            "the conflict index still has unresolved entries"
        );
        session.update_expected_refs(plan);
        Ok(Some(Self {
            previous: Some(session.commit_id),
            session,
            reserved: None,
            complete: true,
            lock: None,
        }))
    }

    pub(super) fn for_amend(repo: &gix::Repository) -> Result<Option<Self>> {
        let Some(session) = load(repo)? else { return Ok(None) };
        let session = session.current(repo)?;
        Ok(Some(Self {
            previous: Some(session.commit_id),
            session,
            reserved: None,
            complete: false,
            lock: None,
        }))
    }

    pub(super) fn pause(
        previous: Option<Self>,
        repo: &gix::Repository,
        conflict_commit_id: ObjectId,
        document: Vec<u8>,
        operation: &str,
    ) -> Result<Self> {
        let mut publication = match previous {
            Some(publication) => publication,
            None => {
                ensure_idle(repo)?;
                Self {
                    session: Session {
                        commit_id: ObjectId::null(repo.object_hash()),
                        operation: operation.into(),
                        conflict_commit_id,
                        document: Vec::new(),
                        head_commit_id: repo.head_id()?.detach(),
                        head: undo::state(repo, "HEAD".try_into()?)?,
                        changes: Vec::new(),
                        publishing: false,
                    },
                    previous: None,
                    reserved: None,
                    complete: false,
                    lock: None,
                }
            }
        };
        publication.complete = false;
        publication.session.conflict_commit_id = conflict_commit_id;
        publication.session.document = document;
        Ok(publication)
    }

    pub(super) fn reserve(
        &mut self,
        repo: &gix::Repository,
        edits: &mut Vec<RefEdit>,
        rollback: &mut Vec<RefEdit>,
    ) -> Result<()> {
        self.lock = Some(publication_lock(repo)?);
        let mut reserved = self.session.clone();
        reserved.publishing = true;
        reserved.changes.extend(undo::changes_from_edits(edits.clone())?);
        reserved.changes = undo::normalize_changes(reserved.changes)?;
        let commit_id = reserved.write(repo)?;
        let before = self.previous.map_or(undo::State::Missing, undo::State::Object);
        let after = undo::State::Object(commit_id);
        edits.push(undo::checked_edit(&undo::RefChange {
            name: REF.try_into()?,
            before: before.clone(),
            after: after.clone(),
        })?);
        rollback.push(undo::checked_edit(&undo::RefChange {
            name: REF.try_into()?,
            before: after,
            after: before,
        })?);
        // Also guard an unchanged HEAD: two continuations can otherwise replay the same index.
        for (name, state) in
            std::iter::once(("HEAD".try_into()?, self.session.head.clone())).chain(match &self.session.head {
                undo::State::Symbolic(name) => Some((name.clone(), undo::State::Object(self.session.head_commit_id))),
                _ => None,
            })
        {
            if !edits.iter().any(|edit| edit.name == name) {
                edits.push(undo::checked_edit(&undo::RefChange {
                    name,
                    before: state.clone(),
                    after: state,
                })?);
            }
        }
        self.reserved = Some(commit_id);
        Ok(())
    }

    pub(super) fn finish(&mut self, repo: &gix::Repository, changes: &[undo::RefChange]) -> Result<()> {
        self.session
            .changes
            .extend(changes.iter().filter(|change| !is_ref(change.name.as_bstr())).cloned());
        self.session.changes = undo::normalize_changes(std::mem::take(&mut self.session.changes))?;
        let reserved = self.reserved.context("rebase state publication was not reserved")?;
        let (mut edits, after) = if self.complete {
            (
                undo::prepare_record(repo, &undo_title(&self.session.operation), &self.session.changes)?.1,
                undo::State::Missing,
            )
        } else {
            self.session.head_commit_id = repo.head_id()?.detach();
            self.session.head = undo::state(repo, "HEAD".try_into()?)?;
            (Vec::new(), undo::State::Object(self.session.write(repo)?))
        };
        edits.push(undo::checked_edit(&undo::RefChange {
            name: REF.try_into()?,
            before: undo::State::Object(reserved),
            after,
        })?);
        repo.edit_references(edits)
            .context("could not publish the rebase continuation and undo state")?;
        Ok(())
    }
}

fn undo_title(operation: &str) -> String {
    match operation {
        "rebase" => "rebase history".into(),
        "transplant" => "transplant commits".into(),
        "squash" => "squash commits".into(),
        other => other.into(),
    }
}

/// Forget remaining work without moving refs or changing the index/worktree.
/// Even damaged metadata can be stopped; report when its undo payload cannot be recovered.
pub(crate) fn stop(repo: &gix::Repository) -> Result<Option<String>> {
    let Some(reference) = repo.try_find_reference(REF)? else {
        return Ok(None);
    };
    let _lock = publication_lock(repo)?;
    let mut warning = None;
    let mut edits = match load(repo) {
        Ok(Some(session)) => {
            let current = session.current(repo).unwrap_or(session);
            undo::prepare_record(repo, &undo_title(&current.operation), &current.changes)?.1
        }
        Ok(None) => anyhow::bail!("the saved rebase changed while stopping it; retry"),
        Err(err) => {
            warning = Some(format!(
                "stopped unreadable rebase state; could not recover its undo history: {err:#}"
            ));
            Vec::new()
        }
    };
    edits.push(RefEdit::delete(
        reference.name().to_owned(),
        gix::refs::transaction::PreviousValue::MustExistAndMatch(reference.target().into_owned()),
    ));
    repo.edit_references(edits).context("could not stop the saved rebase")?;
    Ok(warning)
}

fn publication_lock(repo: &gix::Repository) -> Result<gix::lock::Marker> {
    gix::lock::Marker::acquire_to_hold_resource(
        repo.git_dir().join("tix-rebase"),
        gix::lock::acquire::Fail::Immediately,
        None,
    )
    .context("another Tix command is publishing this rebase; retry after it finishes")
}
