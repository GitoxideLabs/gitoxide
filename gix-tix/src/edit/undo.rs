use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

use anyhow::{Context, Result, bail, ensure};
use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice},
    config::File,
    refs::{
        FullName, FullNameRef, Target,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    },
};

pub(crate) const TIP_REF: &str = "refs/worktree/tix/undo";
pub(crate) const CURSOR_REF: &str = "refs/worktree/tix/undo-cursor";

const VERSION: &str = "1";
const START_TITLE: &str = "start of undo history";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum State {
    Missing,
    Object(ObjectId),
    Symbolic(FullName),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RefChange {
    pub name: FullName,
    pub before: State,
    pub after: State,
}

impl RefChange {
    pub(crate) fn from_edit(edit: &RefEdit) -> Result<Self> {
        let (before, after) = match &edit.change {
            Change::Update { expected, new, .. } => (state_from_expected(expected)?, state_from_target(new)),
            Change::Delete { expected, .. } => (state_from_expected(expected)?, State::Missing),
        };
        Ok(RefChange {
            name: edit.name.clone(),
            before,
            after,
        })
    }

    fn reversed(&self) -> Self {
        RefChange {
            name: self.name.clone(),
            before: self.after.clone(),
            after: self.before.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Position {
    pub title: String,
    pub undo: usize,
    pub redo: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Plan {
    /// The operation crossed by this step. `position.title` is the title at the destination.
    pub title: String,
    /// Changes oriented from the current queue position to the destination.
    pub changes: Vec<RefChange>,
    /// Checked, non-dereferencing edits for `changes`, followed by the cursor edit.
    pub edits: Vec<RefEdit>,
    pub position: Position,
}

impl Plan {
    pub(crate) fn apply(self, repo: &gix::Repository) -> Result<()> {
        self.apply_with_worktrees(repo)
    }

    pub(crate) fn apply_with_worktrees(self, repo: &gix::Repository) -> Result<()> {
        let transitions = worktree_transitions(repo, &self.changes)?;
        for transition in &transitions {
            super::forget::preflight_tree_transition(
                &transition.repo,
                &transition.workdir,
                transition.old,
                transition.new,
            )
            .context("local changes prevent undo/redo; stash them manually and retry")?;
        }
        for (applied, transition) in transitions.iter().enumerate() {
            if let Err(err) = super::forget::apply_tree_transition(&transition.workdir, transition.old, transition.new)
            {
                return Err(rollback_transitions(
                    &transitions[..=applied],
                    err.context("could not align a worktree with the undo queue"),
                ));
            }
        }
        if let Err(err) = repo.edit_references(self.edits) {
            return Err(rollback_transitions(
                &transitions,
                anyhow::Error::new(err).context("could not atomically move references and the undo cursor"),
            ));
        }
        Ok(())
    }
}

pub(crate) fn is_queue_ref(name: &BStr) -> bool {
    name.as_bytes() == TIP_REF.as_bytes() || name.as_bytes() == CURSOR_REF.as_bytes()
}

pub(crate) fn ref_chain_reaches_queue(repo: &gix::Repository, name: &FullNameRef) -> Result<bool> {
    let mut name = name.to_owned();
    let mut seen = HashSet::new();
    loop {
        if is_queue_ref(name.as_bstr()) {
            return Ok(true);
        }
        ensure!(seen.insert(name.clone()), "a symbolic reference chain contains a cycle");
        let Some(reference) = repo.try_find_reference(name.as_ref())? else {
            return Ok(false);
        };
        let target = reference.target();
        let Some(next) = target.try_name() else {
            return Ok(false);
        };
        name = next.to_owned();
    }
}

pub(crate) fn is_queue_commit(repo: &gix::Repository, needle: ObjectId) -> Result<bool> {
    let Some(mut id) = read_queue_ref(repo, TIP_REF)?.or(read_queue_ref(repo, CURSOR_REF)?) else {
        return Ok(false);
    };
    let mut seen = HashSet::new();
    loop {
        ensure!(seen.insert(id), "the undo queue first-parent chain contains a cycle");
        let Ok(stored) = parse_commit(repo, id) else {
            return Ok(false);
        };
        if id == needle {
            return Ok(true);
        }
        let Some(parent) = stored.parent else {
            return Ok(false);
        };
        id = parent;
    }
}

pub(crate) fn review_blocks_undo(repo: &gix::Repository) -> Result<bool> {
    let references = repo.references().context("could not open review references")?;
    for reference in references
        .prefixed(crate::history::REVIEW_PREFIX.as_bstr())
        .context("could not iterate review references")?
    {
        let reference = match reference {
            Ok(reference) => reference,
            Err(err) if crate::history::is_missing_ref(&err) => continue,
            Err(err) => return Err(anyhow::anyhow!("could not read review reference: {err}")),
        };
        if crate::history::review_number(reference.name().as_bstr()).is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn clear(repo: &gix::Repository) -> Result<()> {
    let mut edits = Vec::new();
    for name in [TIP_REF, CURSOR_REF] {
        let Some(reference) = repo
            .try_find_reference(name)
            .with_context(|| format!("could not read {name}"))?
        else {
            continue;
        };
        edits.push(RefEdit::delete(
            reference.name().to_owned(),
            PreviousValue::MustExistAndMatch(reference.target().into_owned()),
        ));
    }
    if edits.is_empty() {
        return Ok(());
    }
    repo.edit_references(edits)
        .context("could not clear undo history")
        .map(|_| ())
}

pub(crate) fn apply_reversed_changes(repo: &gix::Repository, changes: &[RefChange]) -> Result<()> {
    let changes = normalize_changes(changes.iter().cloned())?;
    if changes.is_empty() {
        return Ok(());
    }
    let edits = changes
        .iter()
        .map(RefChange::reversed)
        .map(|change| checked_edit(&change))
        .collect::<Result<Vec<_>>>()?;
    repo.edit_references(edits)
        .context("could not roll back provisional reference changes")
        .map(|_| ())
}

pub(crate) fn state(repo: &gix::Repository, name: &FullNameRef) -> Result<State> {
    Ok(match repo.try_find_reference(name)? {
        Some(reference) => state_from_target_ref(reference.target()),
        None => State::Missing,
    })
}

/// Convert checked transaction edits into one change per reference.
///
/// Repeated edits must form a continuous sequence; the first before-state and final after-state
/// are retained, and resulting no-ops are omitted.
pub(crate) fn changes_from_edits(edits: impl IntoIterator<Item = RefEdit>) -> Result<Vec<RefChange>> {
    normalize_changes(
        edits
            .into_iter()
            .map(|edit| RefChange::from_edit(&edit))
            .collect::<Result<Vec<_>>>()?,
    )
}

/// Record an already-successful operation and publish the new tip and cursor atomically.
///
/// Returns `None` when all supplied changes cancel each other out.
pub(crate) fn record(repo: &gix::Repository, title: &str, changes: &[RefChange]) -> Result<Option<ObjectId>> {
    validate_title(title)?;
    if review_blocks_undo(repo)? {
        clear(repo)?;
        return Ok(None);
    }
    let changes = normalize_changes(changes.iter().cloned())?;
    if changes.is_empty() {
        return Ok(None);
    }

    let queue = load(repo)?;
    let (predecessor, old_tip, old_cursor) = match queue {
        Some(queue) => (queue.cursor, Some(queue.tip), Some(queue.cursor)),
        None => {
            let config = serialize_config(&[])?;
            let sentinel = write_commit(repo, START_TITLE, &config, &[])?;
            (sentinel, None, None)
        }
    };
    let config = serialize_config(&changes)?;
    let parents = retention_parents(repo, predecessor, &changes)?;
    let entry = write_commit(repo, title, &config, &parents)?;

    repo.edit_references([
        queue_update(TIP_REF, old_tip, entry)?,
        queue_update(CURSOR_REF, old_cursor, entry)?,
    ])
    .context("could not publish the undo entry")?;
    Ok(Some(entry))
}

pub(crate) fn position(repo: &gix::Repository) -> Result<Position> {
    if review_blocks_undo(repo)? {
        return Ok(empty_position());
    }
    Ok(load(repo)?.map_or_else(empty_position, |queue| queue.position(queue.cursor_index)))
}

pub(crate) fn plan_undo(repo: &gix::Repository) -> Result<Option<Plan>> {
    if review_blocks_undo(repo)? {
        return Ok(None);
    }
    let Some(queue) = load(repo)? else { return Ok(None) };
    if queue.cursor_index == 0 {
        return Ok(None);
    }
    let entry = &queue.entries[queue.cursor_index - 1];
    let changes: Vec<_> = entry.changes.iter().map(RefChange::reversed).collect();
    let cursor = if queue.cursor_index == 1 {
        queue.sentinel
    } else {
        queue.entries[queue.cursor_index - 2].id
    };
    Ok(Some(make_plan(
        &queue,
        entry.title.clone(),
        changes,
        cursor,
        queue.cursor_index - 1,
    )?))
}

pub(crate) fn plan_redo(repo: &gix::Repository) -> Result<Option<Plan>> {
    if review_blocks_undo(repo)? {
        return Ok(None);
    }
    let Some(queue) = load(repo)? else { return Ok(None) };
    let Some(entry) = queue.entries.get(queue.cursor_index) else {
        return Ok(None);
    };
    Ok(Some(make_plan(
        &queue,
        entry.title.clone(),
        entry.changes.clone(),
        entry.id,
        queue.cursor_index + 1,
    )?))
}

fn make_plan(
    queue: &Queue,
    title: String,
    changes: Vec<RefChange>,
    cursor: ObjectId,
    cursor_index: usize,
) -> Result<Plan> {
    let mut edits = changes.iter().map(checked_edit).collect::<Result<Vec<_>>>()?;
    edits.push(queue_update(CURSOR_REF, Some(queue.cursor), cursor)?);
    Ok(Plan {
        title,
        changes,
        edits,
        position: queue.position(cursor_index),
    })
}

fn empty_position() -> Position {
    Position {
        title: START_TITLE.into(),
        undo: 0,
        redo: 0,
    }
}

fn normalize_changes(changes: impl IntoIterator<Item = RefChange>) -> Result<Vec<RefChange>> {
    let mut by_name = BTreeMap::<FullName, RefChange>::new();
    for change in changes {
        ensure!(
            !is_queue_ref(change.name.as_bstr()),
            "the undo queue cannot record itself"
        );
        match by_name.get_mut(&change.name) {
            Some(existing) => {
                ensure!(
                    existing.after == change.before,
                    "successive changes to {} are not continuous",
                    change.name
                );
                existing.after = change.after;
            }
            None => {
                by_name.insert(change.name.clone(), change);
            }
        }
    }
    Ok(by_name
        .into_values()
        .filter(|change| change.before != change.after)
        .collect())
}

fn state_from_expected(expected: &PreviousValue) -> Result<State> {
    match expected {
        PreviousValue::MustNotExist => Ok(State::Missing),
        PreviousValue::MustExistAndMatch(target) => Ok(state_from_target(target)),
        PreviousValue::Any | PreviousValue::MustExist | PreviousValue::ExistingMustMatch(_) => {
            bail!("an undo entry requires the exact previous reference value")
        }
    }
}

fn state_from_target(target: &Target) -> State {
    match target {
        Target::Object(id) => State::Object(*id),
        Target::Symbolic(name) => State::Symbolic(name.clone()),
    }
}

fn state_from_target_ref(target: gix::refs::TargetRef<'_>) -> State {
    match target {
        gix::refs::TargetRef::Object(id) => State::Object(id.to_owned()),
        gix::refs::TargetRef::Symbolic(name) => State::Symbolic(name.to_owned()),
    }
}

struct WorktreeTransition {
    repo: gix::Repository,
    workdir: PathBuf,
    old: ObjectId,
    new: ObjectId,
}

fn worktree_transitions(repo: &gix::Repository, changes: &[RefChange]) -> Result<Vec<WorktreeTransition>> {
    let current_git_dir = gix::path::realpath(repo.git_dir())
        .map_err(gix::Exn::into_error)
        .context("could not resolve the current Git directory")?;
    let mut repos = vec![
        repo.main_repo()
            .context("could not open the main worktree repository")?,
    ];
    for proxy in repo.worktrees().context("could not enumerate linked worktrees")? {
        repos.push(
            proxy
                .into_repo_with_possibly_inaccessible_worktree()
                .context("could not inspect a linked worktree")?,
        );
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for worktree_repo in repos {
        if !seen.insert(worktree_repo.git_dir().to_owned()) {
            continue;
        }
        let head = worktree_repo.head().context("could not inspect a worktree HEAD")?;
        let old_id = head.id().map(gix::Id::detach);
        let raw_head = worktree_repo
            .find_reference("HEAD")
            .context("could not read a worktree HEAD reference")?;
        let raw_head = state_from_target_ref(raw_head.target());
        let current = gix::path::realpath(worktree_repo.git_dir())
            .map_err(gix::Exn::into_error)
            .context("could not resolve an affected worktree Git directory")?
            == current_git_dir;
        let projected_head = projected_ref_state(&worktree_repo, b"HEAD".as_bstr(), &raw_head, changes, current)?;
        ensure!(
            projected_head != State::Missing,
            "undo/redo cannot delete a worktree HEAD"
        );
        let new_id = resolve_state(&worktree_repo, &projected_head, changes, current, &mut HashSet::new())?;
        let old = tree_id(&worktree_repo, old_id).context("could not inspect the current worktree tree")?;
        let new = tree_id(&worktree_repo, new_id).context("could not inspect the destination worktree tree")?;
        if old == new || (worktree_repo.workdir().is_none() && worktree_repo.is_bare()) {
            continue;
        }
        let workdir = worktree_repo
            .workdir()
            .filter(|path| path.is_dir())
            .context("an affected worktree is inaccessible")?
            .to_owned();
        out.push(WorktreeTransition {
            repo: worktree_repo,
            workdir,
            old,
            new,
        });
    }
    Ok(out)
}

fn projected_ref_state(
    repo: &gix::Repository,
    name: &BStr,
    fallback: &State,
    changes: &[RefChange],
    include_head: bool,
) -> Result<State> {
    if (include_head || name != b"HEAD")
        && let Some(change) = changes.iter().find(|change| change.name.as_bstr() == name)
    {
        return Ok(change.after.clone());
    }
    if name == b"HEAD" {
        return Ok(fallback.clone());
    }
    let name = FullName::try_from(name).context("a projected worktree reference name is invalid")?;
    state(repo, name.as_ref())
}

fn resolve_state(
    repo: &gix::Repository,
    state: &State,
    changes: &[RefChange],
    include_head: bool,
    seen: &mut HashSet<FullName>,
) -> Result<Option<ObjectId>> {
    match state {
        State::Missing => Ok(None),
        State::Object(id) => Ok(Some(*id)),
        State::Symbolic(name) => {
            ensure!(
                seen.insert(name.clone()),
                "a projected worktree reference contains a symbolic cycle"
            );
            let next = projected_ref_state(repo, name.as_bstr(), &State::Missing, changes, include_head)?;
            resolve_state(repo, &next, changes, include_head, seen)
        }
    }
}

fn tree_id(repo: &gix::Repository, commit: Option<ObjectId>) -> Result<ObjectId> {
    commit.map_or_else(
        || Ok(ObjectId::empty_tree(repo.object_hash())),
        |commit| {
            repo.find_commit(commit)
                .context("a worktree HEAD target is not a commit")?
                .tree_id()
                .context("could not decode a worktree HEAD commit")
                .map(gix::Id::detach)
        },
    )
}

fn rollback_transitions(transitions: &[WorktreeTransition], mut cause: anyhow::Error) -> anyhow::Error {
    for transition in transitions.iter().rev() {
        if let Err(err) = super::forget::apply_tree_transition(&transition.workdir, transition.new, transition.old) {
            cause = cause.context(format!("worktree rollback failed: {err:#}"));
        }
    }
    cause
}

fn checked_edit(change: &RefChange) -> Result<RefEdit> {
    let expected = match &change.before {
        State::Missing => PreviousValue::MustNotExist,
        State::Object(id) => PreviousValue::MustExistAndMatch(Target::Object(*id)),
        State::Symbolic(name) => PreviousValue::MustExistAndMatch(Target::Symbolic(name.clone())),
    };
    let tx_change = match &change.after {
        State::Missing => {
            ensure!(
                change.before != State::Missing,
                "cannot delete an already-missing reference"
            );
            Change::Delete {
                expected,
                log: RefLog::AndReference,
            }
        }
        State::Object(id) => Change::Update {
            expected,
            new: Target::Object(*id),
            log: log_change(),
        },
        State::Symbolic(name) => Change::Update {
            expected,
            new: Target::Symbolic(name.clone()),
            log: log_change(),
        },
    };
    Ok(RefEdit::new(change.name.clone(), tx_change))
}

fn queue_update(name: &str, old: Option<ObjectId>, new: ObjectId) -> Result<RefEdit> {
    Ok(RefEdit::update_with_log(
        name.try_into().context("the undo queue reference name is invalid")?,
        new,
        old.map_or(PreviousValue::MustNotExist, |id| {
            PreviousValue::MustExistAndMatch(Target::Object(id))
        }),
        log_change(),
    ))
}

fn log_change() -> LogChange {
    LogChange {
        mode: RefLog::AndReference,
        force_create_reflog: false,
        message: "tix undo queue".into(),
    }
}

fn serialize_config(changes: &[RefChange]) -> Result<File> {
    let mut config = File::default();
    config
        .new_section("undo", None)?
        .set("version", VERSION)
        .context("could not serialize the undo version")?;
    for change in changes {
        let mut section = config
            .new_section("ref", change.name.as_bstr())
            .context("could not serialize an undo reference")?;
        section
            .set("before", encode_state(&change.before))
            .context("could not serialize an undo before-state")?;
        section
            .set("after", encode_state(&change.after))
            .context("could not serialize an undo after-state")?;
    }
    Ok(config)
}

fn encode_state(state: &State) -> BString {
    match state {
        State::Missing => "missing".into(),
        State::Object(id) => format!("object:{id}").into(),
        State::Symbolic(name) => {
            let mut value = BString::from("symbolic:");
            value.extend_from_slice(name.as_bstr());
            value
        }
    }
}

fn parse_config(repo: &gix::Repository, body: &BStr) -> Result<Vec<RefChange>> {
    let config = File::try_from(body).context("could not parse undo metadata as Git config")?;
    let mut sections = config.sections();
    let undo = sections.next().context("undo metadata has no version section")?;
    ensure!(
        undo.header().name() == b"undo" && undo.header().subsection_name().is_none(),
        "undo metadata must start with [undo]"
    );
    ensure_exact_keys(&undo, &["version"])?;
    ensure!(
        undo.value("version").as_ref().map(|value| value.as_slice()) == Some(VERSION.as_bytes()),
        "unsupported undo metadata version"
    );

    let mut changes = Vec::new();
    let mut previous_name: Option<FullName> = None;
    for section in sections {
        ensure!(
            section.header().name() == b"ref",
            "undo metadata contains an unknown section"
        );
        let subsection = section
            .header()
            .subsection_name()
            .context("an undo ref section has no reference name")?;
        let name = FullName::try_from(subsection).context("an undo entry contains an invalid reference name")?;
        ensure!(!is_queue_ref(name.as_bstr()), "the undo queue records itself");
        if let Some(previous) = &previous_name {
            ensure!(
                previous < &name,
                "undo reference sections are duplicated or out of order"
            );
        }
        previous_name = Some(name.clone());
        ensure_exact_keys(&section, &["before", "after"])?;
        let before = section.value("before").context("an undo ref has no before-state")?;
        let before = parse_state(repo, before.as_bstr())?;
        let after = section.value("after").context("an undo ref has no after-state")?;
        let after = parse_state(repo, after.as_bstr())?;
        ensure!(before != after, "an undo ref does not change");
        changes.push(RefChange { name, before, after });
    }
    Ok(changes)
}

fn ensure_exact_keys(section: &gix::config::file::SectionRef<'_>, expected: &[&str]) -> Result<()> {
    let actual: Vec<_> = section.value_names().collect();
    ensure!(
        actual == expected,
        "undo metadata has missing, repeated, or unknown keys"
    );
    Ok(())
}

fn parse_state(repo: &gix::Repository, value: &BStr) -> Result<State> {
    if value == b"missing" {
        return Ok(State::Missing);
    }
    if let Some(hex) = value.strip_prefix(b"object:") {
        let id = ObjectId::from_hex(hex).context("an undo object ID is invalid")?;
        ensure!(
            id.kind() == repo.object_hash(),
            "an undo object ID uses the wrong hash kind"
        );
        return Ok(State::Object(id));
    }
    if let Some(name) = value.strip_prefix(b"symbolic:") {
        return FullName::try_from(name.as_bstr())
            .map(State::Symbolic)
            .context("an undo symbolic target is invalid");
    }
    bail!("an undo reference state has an unknown encoding")
}

fn validate_title(title: &str) -> Result<()> {
    ensure!(!title.is_empty(), "an undo operation title cannot be empty");
    ensure!(
        !title.as_bytes().iter().any(|byte| matches!(byte, b'\n' | b'\r')),
        "an undo operation title must be one line"
    );
    Ok(())
}

fn retention_parents(repo: &gix::Repository, predecessor: ObjectId, changes: &[RefChange]) -> Result<Vec<ObjectId>> {
    let mut parents = vec![predecessor];
    let mut seen = HashSet::from([predecessor]);
    for id in changes
        .iter()
        .flat_map(|change| [&change.before, &change.after])
        .filter_map(|state| match state {
            State::Object(id) => Some(*id),
            State::Missing | State::Symbolic(_) => None,
        })
    {
        if seen.contains(&id) {
            continue;
        }
        if repo
            .find_header(id)
            .with_context(|| format!("could not inspect retained undo object {id}"))?
            .kind()
            == gix::object::Kind::Commit
        {
            seen.insert(id);
            parents.push(id);
        }
    }
    Ok(parents)
}

fn write_commit(repo: &gix::Repository, title: &str, config: &File, parents: &[ObjectId]) -> Result<ObjectId> {
    validate_title(title)?;
    let tree = repo
        .write_object(gix::objs::Tree::empty())
        .context("could not write the undo queue's empty tree")?
        .detach();
    let committer = repo
        .committer()
        .context("no Git committer is configured")?
        .context("could not resolve the Git committer")?
        .to_owned()?;
    let mut message = BString::from(title);
    message.extend_from_slice(b"\n\n");
    message.extend_from_slice(&config.to_bstring());
    let commit = gix::objs::Commit {
        tree,
        parents: parents.iter().copied().collect(),
        author: committer.clone(),
        committer,
        encoding: None,
        message,
        extra_headers: Vec::new(),
    };
    repo.write_object(&commit)
        .context("could not write an undo queue commit")
        .map(gix::Id::detach)
}

#[derive(Debug)]
struct StoredEntry {
    id: ObjectId,
    title: String,
    changes: Vec<RefChange>,
}

#[derive(Debug)]
struct Queue {
    tip: ObjectId,
    cursor: ObjectId,
    sentinel: ObjectId,
    entries: Vec<StoredEntry>,
    cursor_index: usize,
}

impl Queue {
    fn position(&self, cursor_index: usize) -> Position {
        Position {
            title: cursor_index
                .checked_sub(1)
                .and_then(|index| self.entries.get(index))
                .map_or_else(|| START_TITLE.into(), |entry| entry.title.clone()),
            undo: cursor_index,
            redo: self.entries.len() - cursor_index,
        }
    }
}

fn load(repo: &gix::Repository) -> Result<Option<Queue>> {
    let tip = read_queue_ref(repo, TIP_REF)?;
    let cursor = read_queue_ref(repo, CURSOR_REF)?;
    let (tip, cursor) = match (tip, cursor) {
        (None, None) => return Ok(None),
        (Some(tip), Some(cursor)) => (tip, cursor),
        _ => bail!("the undo queue has only one of its tip and cursor references"),
    };

    let mut id = tip;
    let mut seen = HashSet::new();
    let mut entries = Vec::new();
    let sentinel;
    loop {
        ensure!(seen.insert(id), "the undo queue first-parent chain contains a cycle");
        let stored = parse_commit(repo, id)?;
        match stored.parent {
            Some(parent) => {
                entries.push(StoredEntry {
                    id,
                    title: stored.title,
                    changes: stored.changes,
                });
                id = parent;
            }
            None => {
                sentinel = id;
                break;
            }
        }
    }
    entries.reverse();
    let cursor_index = if cursor == sentinel {
        0
    } else {
        entries
            .iter()
            .position(|entry| entry.id == cursor)
            .map(|index| index + 1)
            .context("the undo cursor is not on the tip's first-parent chain")?
    };
    Ok(Some(Queue {
        tip,
        cursor,
        sentinel,
        entries,
        cursor_index,
    }))
}

fn read_queue_ref(repo: &gix::Repository, name: &str) -> Result<Option<ObjectId>> {
    let Some(reference) = repo
        .try_find_reference(name)
        .with_context(|| format!("could not read {name}"))?
    else {
        return Ok(None);
    };
    match reference.target() {
        gix::refs::TargetRef::Object(id) => Ok(Some(id.to_owned())),
        gix::refs::TargetRef::Symbolic(_) => bail!("{name} must be a direct reference"),
    }
}

struct ParsedCommit {
    parent: Option<ObjectId>,
    title: String,
    changes: Vec<RefChange>,
}

fn parse_commit(repo: &gix::Repository, id: ObjectId) -> Result<ParsedCommit> {
    let commit = repo
        .find_commit(id)
        .with_context(|| format!("could not find undo queue commit {id}"))?
        .decode()
        .context("could not decode an undo queue commit")?
        .into_owned()
        .context("could not own an undo queue commit")?;
    ensure!(
        commit.tree == ObjectId::empty_tree(repo.object_hash()),
        "an undo queue commit does not use the empty tree"
    );
    let message = gix::objs::commit::MessageRef::from_bytes(&commit.message);
    let title = message
        .title
        .to_str()
        .context("an undo operation title is not UTF-8")?
        .to_owned();
    validate_title(&title)?;
    let body = message.body.context("an undo queue commit has no metadata body")?;
    let changes = parse_config(repo, body)?;
    let parent = commit.parents.first().copied();
    match parent {
        None => {
            ensure!(title == START_TITLE, "the undo sentinel has the wrong title");
            ensure!(changes.is_empty(), "the undo sentinel contains reference changes");
        }
        Some(parent) => {
            ensure!(!changes.is_empty(), "an undo operation contains no reference changes");
            let expected = retention_parents(repo, parent, &changes)?;
            ensure!(
                commit.parents.as_slice() == expected,
                "an undo commit has invalid retention parents"
            );
        }
    }
    Ok(ParsedCommit { parent, title, changes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> gix_testtools::Result<(gix_testtools::tempfile::TempDir, gix::Repository)> {
        let fixture = gix_testtools::scripted_fixture_writable("create_commit.sh")?;
        let repository = crate::test_repository::open(fixture.path())?;
        Ok((fixture, repository))
    }

    fn name(value: &str) -> gix_testtools::Result<FullName> {
        Ok(value.try_into()?)
    }

    fn set(repo: &gix::Repository, name: FullName, before: State, after: State) -> Result<()> {
        checked_edit(&RefChange { name, before, after }).and_then(|edit| {
            repo.edit_references([edit])?;
            Ok(())
        })
    }

    fn child(repo: &gix::Repository, parent: ObjectId, title: &str) -> gix_testtools::Result<ObjectId> {
        let mut commit = repo.find_commit(parent)?.decode()?.into_owned()?;
        commit.parents = [parent].into_iter().collect();
        commit.message = title.into();
        Ok(repo.write_object(&commit)?.detach())
    }

    #[test]
    fn config_round_trip_is_lossless_and_strict() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let id = repo.head_id()?.detach();
        let changes = vec![
            RefChange {
                name: name("HEAD")?,
                before: State::Symbolic(name("refs/heads/main")?),
                after: State::Object(id),
            },
            RefChange {
                name: name("refs/heads/new")?,
                before: State::Missing,
                after: State::Object(id),
            },
        ];
        let config = serialize_config(&changes)?;
        assert_eq!(
            parse_config(&repo, config.to_bstring().as_bstr())?,
            changes,
            "all supported reference states round-trip"
        );
        assert!(
            parse_config(&repo, b"[undo]\nversion = 1\nextra = nope\n".as_bstr()).is_err(),
            "unknown metadata keys are rejected"
        );
        assert!(
            parse_config(
                &repo,
                b"[undo]\nversion = 1\n[ref \"refs/heads/a\"]\nbefore = missing\nafter = missing\n".as_bstr()
            )
            .is_err(),
            "no-op entries are rejected"
        );
        Ok(())
    }

    #[test]
    fn undo_and_redo_apply_checked_ref_and_cursor_edits() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let branch = name("refs/heads/undo-test")?;
        let head = repo.head_id()?.detach();
        set(&repo, branch.clone(), State::Missing, State::Object(head))?;
        let entry = record(
            &repo,
            "create branch",
            &[RefChange {
                name: branch.clone(),
                before: State::Missing,
                after: State::Object(head),
            }],
        )?
        .expect("a change creates an entry");

        let parents: Vec<_> = repo.find_commit(entry)?.parent_ids().map(gix::Id::detach).collect();
        assert_eq!(
            parents.len(),
            2,
            "the prior queue entry and changed commit are retained"
        );
        assert_eq!(parents[1], head, "the changed commit is a retention parent");
        assert_eq!(position(&repo)?.undo, 1, "the operation is initially applied");

        let undo = plan_undo(&repo)?.expect("one operation can be undone");
        assert_eq!(undo.title, "create branch");
        assert_eq!(undo.position.title, START_TITLE);
        undo.apply(&repo)?;
        assert!(
            repo.try_find_reference(branch.as_ref())?.is_none(),
            "undo deletes the created branch"
        );
        assert_eq!(position(&repo)?.redo, 1, "the operation can be redone");

        let redo = plan_redo(&repo)?.expect("one operation can be redone");
        assert_eq!(redo.position.title, "create branch");
        redo.apply(&repo)?;
        assert_eq!(repo.find_reference(branch.as_ref())?.id(), head);
        assert!(plan_redo(&repo)?.is_none(), "the tip has no redo operation");
        Ok(())
    }

    #[test]
    fn recording_behind_tip_truncates_redo_and_merges_changes() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let branch = name("refs/heads/undo-test")?;
        let first = repo.head_id()?.detach();
        let discarded = child(&repo, first, "discarded")?;
        let replacement = child(&repo, first, "replacement")?;

        set(&repo, branch.clone(), State::Missing, State::Object(first))?;
        record(
            &repo,
            "first",
            &[RefChange {
                name: branch.clone(),
                before: State::Missing,
                after: State::Object(first),
            }],
        )?;
        set(&repo, branch.clone(), State::Object(first), State::Object(discarded))?;
        record(
            &repo,
            "discarded",
            &[RefChange {
                name: branch.clone(),
                before: State::Object(first),
                after: State::Object(discarded),
            }],
        )?;
        plan_undo(&repo)?.expect("second entry exists").apply(&repo)?;

        set(&repo, branch.clone(), State::Object(first), State::Object(replacement))?;
        record(
            &repo,
            "replacement",
            &[
                RefChange {
                    name: branch.clone(),
                    before: State::Object(first),
                    after: State::Object(discarded),
                },
                RefChange {
                    name: branch,
                    before: State::Object(discarded),
                    after: State::Object(replacement),
                },
            ],
        )?;
        let at_tip = position(&repo)?;
        assert_eq!((at_tip.title.as_str(), at_tip.undo, at_tip.redo), ("replacement", 2, 0));
        assert!(plan_redo(&repo)?.is_none(), "the old redo tail is no longer reachable");
        Ok(())
    }

    #[test]
    fn divergent_refs_fail_without_moving_the_cursor() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let branch = name("refs/heads/undo-test")?;
        let expected = repo.head_id()?.detach();
        let external = child(&repo, expected, "external")?;
        set(&repo, branch.clone(), State::Missing, State::Object(expected))?;
        record(
            &repo,
            "create branch",
            &[RefChange {
                name: branch.clone(),
                before: State::Missing,
                after: State::Object(expected),
            }],
        )?;
        let plan = plan_undo(&repo)?.expect("the operation can be planned");
        set(&repo, branch, State::Object(expected), State::Object(external))?;
        assert!(plan.apply(&repo).is_err(), "the stale reference CAS is rejected");
        assert_eq!(position(&repo)?.undo, 1, "the cursor remains at the applied operation");
        Ok(())
    }

    #[test]
    fn recording_accepts_a_ref_that_the_operation_deleted() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let branch = name("refs/heads/undo-test")?;
        let head = repo.head_id()?.detach();
        set(&repo, branch.clone(), State::Missing, State::Object(head))?;
        set(&repo, branch.clone(), State::Object(head), State::Missing)?;

        record(
            &repo,
            "delete branch",
            &[RefChange {
                name: branch.clone(),
                before: State::Object(head),
                after: State::Missing,
            }],
        )?
        .expect("the deletion creates an entry");
        plan_undo(&repo)?.expect("the deletion can be undone").apply(&repo)?;
        assert_eq!(
            repo.find_reference(branch.as_ref())?.id(),
            head,
            "undo recreates the deleted reference"
        );
        Ok(())
    }

    #[test]
    fn an_active_review_blocks_and_discards_ref_only_undo_history() -> gix_testtools::Result {
        let (_fixture, repo) = repo()?;
        let head = repo.head_id()?.detach();
        let before_review = name("refs/heads/before-review")?;
        set(&repo, before_review.clone(), State::Missing, State::Object(head))?;
        record(
            &repo,
            "before review",
            &[RefChange {
                name: before_review,
                before: State::Missing,
                after: State::Object(head),
            }],
        )?;

        let review = name("refs/worktree/tix/review/1")?;
        set(&repo, review, State::Missing, State::Object(head))?;
        assert!(
            review_blocks_undo(&repo)?,
            "a valid review reference blocks the ref-only queue"
        );
        assert!(
            plan_undo(&repo)?.is_none(),
            "review checkout state cannot be undone as refs alone"
        );
        assert_eq!(
            position(&repo)?,
            empty_position(),
            "the blocked queue is not presented as available"
        );

        let during_review = name("refs/heads/during-review")?;
        set(&repo, during_review.clone(), State::Missing, State::Object(head))?;
        assert!(
            record(
                &repo,
                "during review",
                &[RefChange {
                    name: during_review,
                    before: State::Missing,
                    after: State::Object(head),
                }],
            )?
            .is_none(),
            "operations during a review are deliberately not journalled"
        );
        assert!(
            repo.try_find_reference(TIP_REF)?.is_none(),
            "the old queue tip is discarded"
        );
        assert!(
            repo.try_find_reference(CURSOR_REF)?.is_none(),
            "the old queue cursor is discarded atomically"
        );
        Ok(())
    }

    #[test]
    fn attached_branch_checkout_follows_undo_and_redo() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("forget_commit.sh")?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
        let repo = crate::test_repository::open(fixture.path())?;
        let top = repo.head_id()?.detach();
        let parent = repo
            .find_commit(top)?
            .parent_ids()
            .next()
            .expect("the fixture tip has a parent")
            .detach();
        let branch = repo
            .head()?
            .referent_name()
            .expect("the fixture HEAD is attached")
            .to_owned();
        let top_state = gix_testtools::repository::snapshot(fixture.path())?;
        record(
            &repo,
            "restore tip",
            &[RefChange {
                name: branch,
                before: State::Object(parent),
                after: State::Object(top),
            }],
        )?;

        plan_undo(&repo)?
            .expect("the tip operation can be undone")
            .apply_with_worktrees(&repo)?;
        let undone = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(
            undone.head,
            gix_testtools::repository::Head::Symbolic {
                name: b"refs/heads/main".into(),
                id: parent,
            }
        );
        assert_eq!(
            undone.index_tree,
            Some(repo.find_commit(parent)?.tree_id()?.detach()),
            "the index follows the undo destination"
        );
        assert_eq!(std::fs::read(fixture.path().join("tracked"))?, b"base\n");
        assert!(!fixture.path().join("added").exists());

        plan_redo(&repo)?
            .expect("the tip operation can be redone")
            .apply_with_worktrees(&repo)?;
        let redone = gix_testtools::repository::snapshot(fixture.path())?;
        assert_eq!(redone.head, top_state.head);
        assert_eq!(redone.index_tree, top_state.index_tree);
        assert_eq!(redone.worktree, top_state.worktree);
        Ok(())
    }

    #[test]
    fn linked_worktree_head_checkout_follows_undo_and_redo() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("forget_commit.sh")?;
        crate::test_repository::disable_autocrlf(fixture.path())?;
        let linked = fixture.path().join("linked");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["worktree", "add", "-q", "-b", "linked"])
            .arg(&linked)
            .arg("HEAD")
            .status()?;
        assert!(status.success(), "git creates the linked worktree");

        let repo = crate::test_repository::open(&linked)?;
        let top = repo.head_id()?.detach();
        let parent = repo
            .find_commit(top)?
            .parent_ids()
            .next()
            .expect("the fixture tip has a parent")
            .detach();
        let branch = repo
            .head()?
            .referent_name()
            .expect("the linked worktree HEAD is attached")
            .to_owned();
        let top_state = gix_testtools::repository::snapshot(&linked)?;
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&linked)
            .args(["checkout", "-q", "--detach"])
            .arg(parent.to_string())
            .status()?;
        assert!(status.success(), "git detaches the linked worktree at the parent");
        let detached_state = gix_testtools::repository::snapshot(&linked)?;
        record(
            &repo,
            "detach linked worktree",
            &[RefChange {
                name: name("HEAD")?,
                before: State::Symbolic(branch),
                after: State::Object(parent),
            }],
        )?;

        plan_undo(&repo)?
            .expect("the linked checkout can be undone")
            .apply_with_worktrees(&repo)?;
        let undone = gix_testtools::repository::snapshot(&linked)?;
        assert_eq!(undone.head, top_state.head, "undo restores the attached HEAD");
        assert_eq!(
            undone.index_tree, top_state.index_tree,
            "the linked index follows the undo destination"
        );
        assert_eq!(
            undone.worktree, top_state.worktree,
            "the linked worktree follows the undo destination"
        );

        plan_redo(&repo)?
            .expect("the linked checkout can be redone")
            .apply_with_worktrees(&repo)?;
        let redone = gix_testtools::repository::snapshot(&linked)?;
        assert_eq!(redone.head, detached_state.head, "redo restores the detached HEAD");
        assert_eq!(
            redone.index_tree, detached_state.index_tree,
            "the linked index follows the redo destination"
        );
        assert_eq!(
            redone.worktree, detached_state.worktree,
            "the linked worktree follows the redo destination"
        );
        Ok(())
    }
}
