use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail, ensure};
use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice, ByteVec},
    hash::ChangeId,
    refs::FullName,
};

use super::{rebase, undo};

const HEADER: &str = "tix-auto-merge";

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum InputSource {
    Reference(FullName),
    Change(ChangeId),
}

impl InputSource {
    pub(crate) fn reference(&self) -> Option<&FullName> {
        match self {
            Self::Reference(name) => Some(name),
            Self::Change(_) => None,
        }
    }

    fn is_static(&self) -> bool {
        self.reference().is_some_and(|name| {
            matches!(
                name.category(),
                Some(gix::refs::Category::Tag | gix::refs::Category::RemoteBranch)
            )
        })
    }

    fn label(&self) -> BString {
        match self {
            Self::Change(change_id) => change_id.to_reverse_hex_with_len(7).to_string().into(),
            Self::Reference(name) if name.as_bstr().starts_with(crate::history::PIN_PREFIX) => "📌".into(),
            Self::Reference(name) if name.category() == Some(gix::refs::Category::Tag) => {
                format!("tag: {}", name.shorten()).into()
            }
            Self::Reference(name) => name.shorten().to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Input {
    pub source: InputSource,
    pub commit_id: ObjectId,
    pub muted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Definition {
    pub inputs: Vec<Input>,
}

impl Definition {
    pub(crate) fn from_commit(commit: &gix::objs::Commit) -> Result<Option<Self>> {
        Self::from_headers(
            commit
                .extra_headers
                .iter()
                .map(|(name, value)| (name.as_bstr(), value.as_bstr())),
        )
    }

    pub(crate) fn from_headers<'a>(headers: impl IntoIterator<Item = (&'a BStr, &'a BStr)>) -> Result<Option<Self>> {
        let mut inputs = Vec::new();
        let mut seen = HashSet::new();
        for (name, value) in headers {
            if name != HEADER {
                continue;
            }
            let mut fields = value.splitn(4, |byte| *byte == b' ');
            ensure!(
                fields.next() == Some(b"1".as_slice()),
                "unsupported AutoMerge metadata version"
            );
            let commit_id = ObjectId::from_hex(fields.next().context("AutoMerge input has no commit ID")?)
                .context("AutoMerge input has an invalid commit ID")?;
            let muted = match fields.next() {
                Some(b"included") => false,
                Some(b"muted") => true,
                _ => bail!("AutoMerge input has an invalid merge state"),
            };
            let source = fields.next().context("AutoMerge input has no identity")?;
            let source = if let Some(change_id) = source.strip_prefix(b"change-id ") {
                InputSource::Change(
                    ChangeId::from_reverse_hex(change_id).context("AutoMerge input has an invalid change ID")?,
                )
            } else {
                let reference: FullName = source
                    .as_bstr()
                    .try_into()
                    .context("AutoMerge input has an invalid reference")?;
                ensure!(
                    allowed_ref(reference.as_bstr()),
                    "AutoMerge input names an internal or unsupported reference"
                );
                InputSource::Reference(reference)
            };
            ensure!(seen.insert(source.clone()), "AutoMerge input is listed more than once");
            inputs.push(Input {
                source,
                commit_id,
                muted,
            });
        }
        Ok((!inputs.is_empty()).then_some(Definition { inputs }))
    }

    pub(crate) fn store(&self, commit: &mut gix::objs::Commit) {
        commit.extra_headers.retain(|(name, _)| name != HEADER);
        for input in &self.inputs {
            let mut value = BString::from(format!(
                "1 {} {} ",
                input.commit_id,
                if input.muted { "muted" } else { "included" }
            ));
            match &input.source {
                InputSource::Reference(name) => value.push_str(name.as_bstr()),
                InputSource::Change(change_id) => value.push_str(format!("change-id {change_id}")),
            }
            commit.extra_headers.push((HEADER.into(), value));
        }
    }

    pub(crate) fn title(&self) -> BString {
        let mut title = BString::default();
        for input in &self.inputs {
            if !title.is_empty() {
                title.push(b' ');
            }
            title.push_str(if input.muted { "💥 " } else { "✔️ " });
            title.push_str(input.source.label());
        }
        title
    }
}

pub(crate) fn is_auto_merge(commit: &gix::objs::Commit) -> bool {
    commit.extra_headers.iter().any(|(name, _)| name == HEADER)
}

pub(super) fn ensure_editable(commit: &gix::objs::Commit) -> Result<()> {
    ensure!(
        !is_auto_merge(commit),
        "AutoMerge trees and titles are generated; edit an input or change the AutoMerge inputs instead"
    );
    Ok(())
}

pub(crate) fn allowed_ref(name: &BStr) -> bool {
    name.starts_with(b"refs/heads/")
        || name.starts_with(b"refs/remotes/")
        || name.starts_with(b"refs/tags/")
        || name.starts_with(crate::history::PIN_PREFIX)
            && name != crate::history::HEAD_PIN_NAME
            && !name.starts_with(crate::history::REVIEW_PIN_PREFIX)
}

pub(crate) fn mapped(mut commit_id: ObjectId, rewritten: &HashMap<ObjectId, Option<ObjectId>>) -> Option<ObjectId> {
    let mut seen = HashSet::new();
    while let Some(next) = rewritten.get(&commit_id) {
        let next = (*next)?;
        if next == commit_id || !seen.insert(commit_id) {
            break;
        }
        commit_id = next;
    }
    Some(commit_id)
}

/// ponytail: snapshot input refs per operation; a later remerge picks up concurrent changes.
#[derive(Default)]
pub(crate) struct References {
    observed: HashMap<FullName, undo::State>,
    candidates: Option<Vec<ObjectId>>,
    change_ids: Option<HashMap<ChangeId, Vec<ObjectId>>>,
    placements: HashMap<ObjectId, rebase::RefDestination>,
    ambiguous: HashMap<ObjectId, ChangeId>,
}

impl References {
    pub(crate) fn for_graph(graph: &crate::history::HistoryGraph) -> Self {
        Self {
            candidates: graph.bounded_history.clone(),
            ..Self::default()
        }
    }

    /// Record logical rewrites, excluding ref-only moves such as inserting a child.
    pub(super) fn rewritten(&mut self, old_commit_id: ObjectId, new_commit_id: Option<ObjectId>) {
        self.placements.insert(
            old_commit_id,
            new_commit_id.map_or(rebase::RefDestination::Delete, rebase::RefDestination::Existing),
        );
    }

    pub(super) fn notice(&self) -> Option<String> {
        let mut changes: Vec<_> = self.ambiguous.values().map(ToString::to_string).collect();
        changes.sort();
        changes.dedup();
        (!changes.is_empty()).then(|| {
            format!(
                "ambiguous AutoMerge change IDs {}; kept their current inputs",
                changes.join(", ")
            )
        })
    }

    pub(crate) fn resolve_input(
        &mut self,
        repo: &gix::Repository,
        input: &Input,
        rewritten: &HashMap<ObjectId, Option<ObjectId>>,
        planned: Option<(&[rebase::PlanRef], &[ObjectId])>,
    ) -> Result<Option<ObjectId>> {
        let change_id = match &input.source {
            InputSource::Reference(name) => return self.resolve(repo, name, rewritten, planned),
            InputSource::Change(change_id) => *change_id,
        };
        let mut commit_id = input.commit_id;
        if !self.placements.contains_key(&commit_id) {
            if self.change_ids.is_none()
                && let Some(candidates) = &self.candidates
            {
                let mut changes = HashMap::<_, Vec<_>>::new();
                for &candidate_commit_id in candidates {
                    let matches = changes
                        .entry(crate::change_id::for_commit(repo, candidate_commit_id)?)
                        .or_default();
                    if !matches.contains(&candidate_commit_id) {
                        matches.push(candidate_commit_id);
                    }
                }
                self.change_ids = Some(changes);
            }
            if let Some(matches) = self.change_ids.as_ref().and_then(|changes| changes.get(&change_id)) {
                match matches.as_slice() {
                    [unique] => commit_id = *unique,
                    _ => {
                        self.ambiguous.insert(input.commit_id, change_id);
                    }
                }
            }
        }
        let mut seen = HashSet::new();
        while let Some(destination) = self.placements.get(&commit_id) {
            ensure!(seen.insert(commit_id), "AutoMerge input rewrites contain a cycle");
            self.ambiguous.remove(&input.commit_id);
            let Some(next_commit_id) = destination.resolve(planned.map_or(&[], |(_, produced)| produced))? else {
                return Ok(None);
            };
            if crate::change_id::for_commit(repo, next_commit_id)? != change_id {
                return Ok(None);
            }
            if next_commit_id == commit_id || matches!(destination, rebase::RefDestination::Step(_)) {
                return Ok(Some(next_commit_id));
            }
            commit_id = next_commit_id;
        }
        Ok(Some(commit_id))
    }

    pub(crate) fn resolve(
        &mut self,
        repo: &gix::Repository,
        name: &FullName,
        rewritten: &HashMap<ObjectId, Option<ObjectId>>,
        planned: Option<(&[rebase::PlanRef], &[ObjectId])>,
    ) -> Result<Option<ObjectId>> {
        let mut name = name.clone();
        let mut seen = HashSet::new();
        loop {
            ensure!(
                seen.insert(name.clone()),
                "AutoMerge input has a symbolic reference cycle"
            );
            if let Some((planned, produced)) = planned
                && let Some(expected) = planned.iter().find(|expected| expected.name == name)
            {
                // Existing destinations are literal, never remapped through another rewrite.
                return expected.destination.resolve(produced);
            }
            let state = match self.observed.get(&name) {
                Some(state) => state.clone(),
                None => {
                    let state = undo::state(repo, name.as_ref())?;
                    self.observed.insert(name.clone(), state.clone());
                    state
                }
            };
            match state {
                undo::State::Missing => return Ok(None),
                undo::State::Symbolic(target) => name = target,
                undo::State::Object(commit_id) => {
                    let commit_id = if matches!(
                        name.category(),
                        Some(gix::refs::Category::Tag | gix::refs::Category::RemoteBranch)
                    ) {
                        commit_id
                    } else {
                        let Some(commit_id) = mapped(commit_id, rewritten) else {
                            return Ok(None);
                        };
                        commit_id
                    };
                    let object = repo
                        .find_object(commit_id)?
                        .peel_to_kind(gix::object::Kind::Commit)
                        .context("AutoMerge input does not point to a commit")?;
                    return Ok(Some(object.id));
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Rebuilt {
    Commit,
    Collapse(ObjectId),
    Empty,
}

/// Rebuild only the derived commit. Replaying pending inputs is the shared rebase engine's job.
pub(crate) fn rebuild(
    repo: &gix::Repository,
    commit: &mut gix::objs::Commit,
    refs: &mut References,
    rewritten: &HashMap<ObjectId, Option<ObjectId>>,
    planned: Option<(&[rebase::PlanRef], &[ObjectId])>,
    eager: bool,
) -> Result<Rebuilt> {
    let mut definition = Definition::from_commit(commit)?.context("the commit is not an AutoMerge")?;
    let mut inputs = Vec::with_capacity(definition.inputs.len());
    for mut input in definition.inputs {
        if let Some(commit_id) = refs.resolve_input(repo, &input, rewritten, planned)? {
            input.commit_id = commit_id;
            inputs.push(input);
        }
    }
    definition.inputs = inputs;
    match definition.inputs.as_slice() {
        [] => return Ok(Rebuilt::Empty),
        [only] => return Ok(Rebuilt::Collapse(only.commit_id)),
        _ => {}
    }
    if eager {
        let mut accumulated = None;
        for input in &mut definition.inputs {
            let input_commit = repo.find_commit(input.commit_id)?.decode()?.into_owned()?;
            input.muted = rebase::is_pending(&input_commit);
            if input.muted {
                continue;
            }
            let Some(our_commit_id) = accumulated else {
                accumulated = Some(input.commit_id);
                continue;
            };
            if our_commit_id == input.commit_id {
                continue;
            }
            let mut outcome = repo
                .merge_commits(
                    our_commit_id,
                    input.commit_id,
                    gix::merge::blob::builtin_driver::text::Labels::default(),
                    gix::merge::commit::Options::from(repo.tree_merge_options()?).with_allow_missing_merge_base(true),
                )
                .context("could not merge an AutoMerge input")?;
            input.muted = outcome
                .tree_merge
                .has_unresolved_conflicts(gix::merge::tree::TreatAsUnresolved::git());
            if input.muted {
                continue;
            }
            let tree_id = outcome.tree_merge.tree.write()?.detach();
            let mut intermediate = commit.clone();
            intermediate.tree = tree_id;
            intermediate.parents = [our_commit_id, input.commit_id].into_iter().collect();
            intermediate.extra_headers.clear();
            intermediate.message = "AutoMerge intermediate\n".into();
            accumulated = Some(repo.write_object(&intermediate)?.detach());
        }
        commit.tree = match accumulated {
            Some(commit_id) => repo.find_commit(commit_id)?.tree_id()?.detach(),
            None => match repo.merge_base_octopus(definition.inputs.iter().map(|input| input.commit_id)) {
                Ok(base) => repo.find_commit(base)?.tree_id()?.detach(),
                Err(gix::repository::merge_base_octopus::Error::MergeBaseOctopus(
                    gix::repository::merge_base_octopus_with_graph::Error::NoMergeBase,
                )) => repo.empty_tree().id,
                Err(err) => return Err(err).context("could not find the common base of muted inputs"),
            },
        };
        let mut message = definition.title();
        if let Some(newline) = commit.message.find_byte(b'\n') {
            message.push_str(&commit.message[newline..]);
        } else {
            message.push(b'\n');
        }
        commit.message = message;
    }
    let mut seen = HashSet::new();
    commit.parents = definition
        .inputs
        .iter()
        .map(|input| input.commit_id)
        .filter(|commit_id| seen.insert(*commit_id))
        .collect();
    definition.store(commit);
    Ok(Rebuilt::Commit)
}

pub(crate) struct Preparation {
    pub refs: References,
    pub optional: HashSet<ObjectId>,
    pub eager: HashSet<ObjectId>,
}

/// Only the checkout's ordinary first-parent path requires conflict materialization.
/// Inputs beneath an AutoMerge can instead remain pending and be muted.
pub(crate) fn checkout_path(
    repo: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    checkout: Option<ObjectId>,
) -> Result<HashSet<ObjectId>> {
    let mut path = HashSet::new();
    let mut cursor = checkout;
    while let Some(commit_id) = cursor {
        if !graph.is_in_edit_scope(commit_id) || !path.insert(commit_id) {
            break;
        }
        let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
        if is_auto_merge(&commit) {
            break;
        }
        cursor = commit.parents.first().copied();
    }
    Ok(path)
}

pub(crate) fn prepare(
    repo: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    affected: &mut Vec<ObjectId>,
    checkout: Option<ObjectId>,
    force: Option<ObjectId>,
) -> Result<Preparation> {
    let mut refs = References::for_graph(graph);
    let mut included: HashSet<_> = affected.iter().copied().collect();
    // Exact edits take precedence during dependency discovery as well as replay.
    for &commit_id in &included {
        refs.rewritten(commit_id, Some(commit_id));
    }
    let required = checkout_path(repo, graph, checkout)?;
    let mut eager = HashSet::new();
    let mut queue = Vec::new();
    loop {
        let before = included.len();
        for (&merge_commit_id, definition) in &graph.auto_merges {
            if !graph.is_in_edit_scope(merge_commit_id) {
                continue;
            }
            let mut affected_input = false;
            for input in &definition.inputs {
                if !input.source.is_static()
                    && refs
                        .resolve_input(repo, input, &HashMap::new(), None)?
                        .is_some_and(|commit_id| included.contains(&commit_id))
                {
                    affected_input = true;
                }
            }
            if included.contains(&merge_commit_id)
                || affected_input
                || force == Some(merge_commit_id)
                || required.contains(&merge_commit_id)
                    && rebase::is_pending(&repo.find_commit(merge_commit_id)?.decode()?.into_owned()?)
            {
                for commit_id in graph.descendants_in_parent_order(merge_commit_id).into_iter().flatten() {
                    if included.insert(commit_id) {
                        refs.rewritten(commit_id, Some(commit_id));
                        affected.push(commit_id);
                    }
                }
                if required.contains(&merge_commit_id) || force == Some(merge_commit_id) {
                    queue.push(merge_commit_id);
                }
            }
        }
        if before == included.len() {
            break;
        }
    }
    let mut optional = HashSet::new();
    while let Some(merge_commit_id) = queue.pop() {
        if !eager.insert(merge_commit_id) {
            continue;
        }
        if included.insert(merge_commit_id) {
            refs.rewritten(merge_commit_id, Some(merge_commit_id));
            affected.push(merge_commit_id);
        }
        let definition = match graph.auto_merges.get(&merge_commit_id) {
            Some(definition) => definition.clone(),
            None => Definition::from_commit(&repo.find_commit(merge_commit_id)?.decode()?.into_owned()?)?
                .context("an AutoMerge dependency lost its recipe")?,
        };
        for input in definition.inputs {
            let mut cursor = refs.resolve_input(repo, &input, &HashMap::new(), None)?;
            if input.source.is_static() {
                continue;
            }
            let mut seen = HashSet::new();
            while let Some(commit_id) = cursor {
                ensure!(seen.insert(commit_id), "AutoMerge input ancestry contains a cycle");
                let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
                if is_auto_merge(&commit) {
                    queue.push(commit_id);
                    break;
                }
                if !included.contains(&commit_id) && !rebase::is_pending(&commit) {
                    break;
                }
                optional.insert(commit_id);
                if included.insert(commit_id) {
                    refs.rewritten(commit_id, Some(commit_id));
                    affected.push(commit_id);
                }
                cursor = commit.parents.first().copied();
            }
        }
    }
    *affected = ordered(repo, graph, affected, &mut refs)?;
    // Only report ambiguities encountered while rebuilding the affected merges.
    refs.ambiguous.clear();
    Ok(Preparation { refs, optional, eager })
}

fn ordered(
    repo: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    ids: &[ObjectId],
    refs: &mut References,
) -> Result<Vec<ObjectId>> {
    let included: HashSet<_> = ids.iter().copied().collect();
    let mut dependencies = HashMap::new();
    for &commit_id in ids {
        let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
        let parents = if let Some(definition) = graph
            .auto_merges
            .get(&commit_id)
            .cloned()
            .or(Definition::from_commit(&commit)?)
        {
            let mut parents = Vec::new();
            for input in definition.inputs {
                if let Some(parent) = refs.resolve_input(repo, &input, &HashMap::new(), None)? {
                    ensure!(
                        !contains(repo, commit_id, parent)?,
                        "an AutoMerge cannot track itself or its descendants"
                    );
                    parents.push(parent);
                }
            }
            parents
        } else {
            commit.parents.into_iter().collect()
        };
        dependencies.insert(commit_id, parents);
    }
    let mut out = Vec::with_capacity(ids.len());
    let mut done = HashSet::new();
    while out.len() < included.len() {
        let before = out.len();
        for &commit_id in ids {
            if done.contains(&commit_id) {
                continue;
            }
            if dependencies[&commit_id]
                .iter()
                .all(|parent| !included.contains(parent) || done.contains(parent))
            {
                done.insert(commit_id);
                out.push(commit_id);
            }
        }
        ensure!(before != out.len(), "AutoMerge dependencies contain a cycle");
    }
    Ok(out)
}

pub(crate) fn contains(
    repo: &gix::Repository,
    ancestor_commit_id: ObjectId,
    descendant_commit_id: ObjectId,
) -> Result<bool> {
    if ancestor_commit_id == descendant_commit_id {
        return Ok(true);
    }
    match repo.merge_base(ancestor_commit_id, descendant_commit_id) {
        Ok(base) => Ok(base == ancestor_commit_id),
        Err(gix::repository::merge_base::Error::NotFound { .. }) => Ok(false),
        Err(err) => Err(err).context("could not inspect AutoMerge input ancestry"),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Choice {
    pub reference: FullName,
    pub commit_id: ObjectId,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Selection {
    pub merge_commit_id: ObjectId,
    pub change: Change,
    pub label: String,
}

pub(crate) fn removals(
    repo: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    selected_commit_id: ObjectId,
    from_merge: bool,
) -> Result<Vec<Selection>> {
    let mut choices = Vec::new();
    let mut refs = References::for_graph(graph);
    for (&merge_commit_id, definition) in &graph.auto_merges {
        if !graph.is_in_edit_scope(merge_commit_id) || from_merge && merge_commit_id != selected_commit_id {
            continue;
        }
        for input in &definition.inputs {
            if !(from_merge || matches!(input.source, InputSource::Change(_)) && input.commit_id == selected_commit_id)
                && refs.resolve_input(repo, input, &HashMap::new(), None)? != Some(selected_commit_id)
            {
                continue;
            }
            let label = match &input.source {
                InputSource::Reference(name) if name.as_bstr().starts_with(crate::history::PIN_PREFIX) => {
                    format!("📌 {}", name.shorten())
                }
                source => source.label().to_string(),
            };
            choices.push(Selection {
                merge_commit_id,
                change: Change::Remove(input.source.clone()),
                label: if from_merge {
                    label
                } else {
                    format!(
                        "{label} · {} · {}",
                        definition.title(),
                        merge_commit_id.to_hex_with_len(7)
                    )
                },
            });
        }
    }
    choices.sort_by_key(|choice| choice.merge_commit_id);
    Ok(choices)
}

pub(crate) fn decorated_tip(
    reference: &FullName,
    decorations: &crate::history::Decorations,
    pins: &[crate::history::Pin],
) -> Option<ObjectId> {
    use crate::history::DecorationKind as Kind;
    if reference.as_bstr().starts_with(crate::history::PIN_PREFIX) {
        return pins.iter().find(|pin| pin.name == *reference).map(|pin| pin.id);
    }
    decorations.iter().find_map(|(&commit_id, names)| {
        names
            .iter()
            .any(|decoration| match reference.category() {
                Some(gix::refs::Category::LocalBranch) => {
                    matches!(
                        decoration.kind,
                        Kind::Local | Kind::CurrentWorktreeBranch | Kind::WorktreeBranch | Kind::HeadPinBranch
                    ) && decoration.name == reference.shorten()
                }
                Some(gix::refs::Category::RemoteBranch) => {
                    decoration.kind == Kind::Remote && decoration.name == reference.shorten()
                }
                Some(gix::refs::Category::Tag) => {
                    matches!(decoration.kind, Kind::Tag | Kind::AnnotatedTag)
                        && decoration.name.strip_prefix(b"tag: ") == Some(reference.shorten().as_bytes())
                }
                _ => false,
            })
            .then_some(commit_id)
    })
}

pub(super) fn input_pins(repo: &gix::Repository, revisions: &[std::ffi::OsString]) -> Result<HashSet<FullName>> {
    // ponytail: reload only for pin-consuming checkouts; pass their cached graph if this becomes a measured cost.
    let hidden = crate::history::available_hidden_revisions(repo, &[], true)?.0;
    let graph = super::loaded_explicit_view_graph(repo, revisions, &hidden)?;
    Ok(graph
        .auto_merges
        .into_values()
        .flat_map(|definition| definition.inputs)
        .filter_map(|input| match input.source {
            InputSource::Reference(name) if name.as_bstr().starts_with(crate::history::PIN_PREFIX) => Some(name),
            _ => None,
        })
        .collect())
}

pub(super) fn follows_reference(repo: &gix::Repository, input: &FullName, reference: &FullName) -> Result<bool> {
    let mut name = input.clone();
    let mut seen = HashSet::new();
    loop {
        if name == *reference {
            return Ok(true);
        }
        ensure!(
            seen.insert(name.clone()),
            "AutoMerge input has a symbolic reference cycle"
        );
        match undo::state(repo, name.as_ref())? {
            undo::State::Symbolic(target) => name = target,
            _ => return Ok(false),
        }
    }
}

pub(crate) fn choices(repo: &gix::Repository) -> Result<Vec<Choice>> {
    let mut choices = Vec::new();
    for reference in repo.references()?.all()? {
        let mut reference = reference.map_err(|err| anyhow::anyhow!("could not read merge input: {err}"))?;
        if !allowed_ref(reference.name().as_bstr()) {
            continue;
        }
        let name = reference.name().to_owned();
        let Ok(commit) = reference.peel_to_commit() else {
            continue;
        };
        let label = if name.as_bstr().starts_with(crate::history::PIN_PREFIX) {
            format!(
                "📌 {} · {}",
                name.as_bstr()
                    .strip_prefix(crate::history::PIN_PREFIX)
                    .expect("the prefix was checked")
                    .as_bstr(),
                commit.id.to_hex_with_len(7)
            )
        } else if name.category() == Some(gix::refs::Category::Tag) {
            format!("tag: {}", name.shorten())
        } else {
            name.shorten().to_str_lossy().into_owned()
        };
        choices.push(Choice {
            reference: name,
            commit_id: commit.id,
            label,
        });
    }
    let rank = |name: &FullName| match name.category() {
        Some(gix::refs::Category::LocalBranch) => 0,
        Some(gix::refs::Category::RemoteBranch) => 2,
        Some(gix::refs::Category::Tag) => 3,
        _ => 1,
    };
    choices.sort_by(|a, b| {
        rank(&a.reference)
            .cmp(&rank(&b.reference))
            .then(a.reference.cmp(&b.reference))
    });
    Ok(choices)
}

fn sources(repo: &gix::Repository, commit_id: ObjectId) -> Result<Vec<Choice>> {
    let mut inputs = Vec::new();
    let mut seen = HashSet::new();
    for mut choice in choices(repo)? {
        if choice.commit_id != commit_id {
            continue;
        }
        if choice.reference.as_bstr().starts_with(crate::history::PIN_PREFIX) {
            let reference = repo.find_reference(choice.reference.as_ref())?;
            if let Some(name) = reference.target().try_name()
                && name.category() == Some(gix::refs::Category::LocalBranch)
            {
                choice.reference = name.to_owned();
                choice.label = name.shorten().to_string();
            }
        } else if choice.reference.category() != Some(gix::refs::Category::LocalBranch) {
            continue;
        }
        if seen.insert(choice.reference.clone()) {
            inputs.push(choice);
        }
    }
    Ok(inputs)
}

pub(crate) fn additions(repo: &gix::Repository, selected_commit_id: ObjectId) -> Result<Vec<Selection>> {
    let head_commit_id = repo.head_id()?.detach();
    let inputs = if selected_commit_id == head_commit_id {
        choices(repo)?
    } else {
        ensure!(
            !contains(repo, selected_commit_id, head_commit_id)?,
            "the selected commit is already an ancestor of HEAD"
        );
        if is_auto_merge(&repo.head_commit()?.decode()?.into_owned()?) {
            ensure!(
                !contains(repo, head_commit_id, selected_commit_id)?,
                "an AutoMerge cannot track itself or its descendants"
            );
        }
        let sources = sources(repo, selected_commit_id)?;
        if sources.is_empty() {
            return Ok(vec![Selection {
                merge_commit_id: head_commit_id,
                change: Change::AddCommit(selected_commit_id),
                label: crate::change_id::for_commit(repo, selected_commit_id)?
                    .to_reverse_hex_with_len(7)
                    .to_string(),
            }]);
        }
        sources
    };
    Ok(inputs
        .into_iter()
        .map(|choice| Selection {
            merge_commit_id: head_commit_id,
            change: Change::Add(choice.reference),
            label: choice.label,
        })
        .collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Change {
    Add(FullName),
    AddCommit(ObjectId),
    Remove(InputSource),
    Remerge,
}

pub(crate) struct Operation {
    pub result: Option<rebase::Perform>,
    pub notice: String,
}

pub(crate) fn perform(
    repo: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    selected_commit_id: ObjectId,
    change: Change,
    checkout: rebase::CheckoutOptions<'_>,
    report: impl FnMut(rebase::Progress),
) -> Result<Operation> {
    repo.workdir().context("AutoMerge requires a worktree")?;
    ensure!(
        !repo
            .index_or_empty()?
            .entries()
            .iter()
            .any(|entry| entry.stage() != gix::index::entry::Stage::Unconflicted),
        "cannot change AutoMerge with unresolved index conflicts"
    );
    let head_commit_id = repo.head_id()?.detach();
    if !matches!(change, Change::Remove(_)) {
        ensure!(
            head_commit_id == selected_commit_id,
            "AutoMerge and Remerge require the selected HEAD entry"
        );
    }
    let original = repo.find_commit(selected_commit_id)?.decode()?.into_owned()?;
    let previous = Definition::from_commit(&original)?;
    let created = previous.is_none();
    let mut definition = match previous {
        Some(definition) => definition,
        None => {
            ensure!(
                matches!(change, Change::Add(_) | Change::AddCommit(_)),
                "the selection is not an AutoMerge"
            );
            let source = match sources(repo, head_commit_id)?.as_slice() {
                [only] => InputSource::Reference(only.reference.clone()),
                _ => InputSource::Change(crate::change_id::for_commit(repo, head_commit_id)?),
            };
            Definition {
                inputs: vec![Input {
                    source,
                    commit_id: head_commit_id,
                    muted: false,
                }],
            }
        }
    };
    let mut references = References::for_graph(graph);
    let notice = match change {
        adding @ (Change::Add(_) | Change::AddCommit(_)) => {
            let (source, commit_id) = match adding {
                Change::Add(reference) => {
                    ensure!(allowed_ref(reference.as_bstr()), "unsupported AutoMerge input");
                    let commit_id = references
                        .resolve(repo, &reference, &HashMap::new(), None)?
                        .context("the selected input reference disappeared")?;
                    (InputSource::Reference(reference), commit_id)
                }
                Change::AddCommit(commit_id) => (
                    InputSource::Change(crate::change_id::for_commit(repo, commit_id)?),
                    commit_id,
                ),
                _ => unreachable!("only additions are matched"),
            };
            if contains(repo, commit_id, head_commit_id)? {
                return Ok(Operation {
                    result: None,
                    notice: format!("{} is already an ancestor of HEAD; no input added", source.label()),
                });
            }
            if !created {
                ensure!(
                    !contains(repo, selected_commit_id, commit_id)?,
                    "an AutoMerge cannot track itself or its descendants"
                );
            }
            if let Some(input) = definition.inputs.iter_mut().find(|input| input.source == source) {
                input.commit_id = commit_id;
            } else {
                definition.inputs.push(Input {
                    source,
                    commit_id,
                    muted: false,
                });
            }
            "AutoMerge updated"
        }
        Change::Remove(source) => {
            let before = definition.inputs.len();
            definition.inputs.retain(|input| input.source != source);
            ensure!(
                before != definition.inputs.len(),
                "the selection is no longer an input of this AutoMerge"
            );
            "input removed from AutoMerge"
        }
        Change::Remerge => "AutoMerge updated",
    };
    let repo = repo.clone().with_object_memory();
    let mut commit = if created {
        gix::objs::Commit {
            author: repo.author().context("no Git author is configured")??.to_owned()?,
            committer: repo
                .committer()
                .context("no Git committer is configured")??
                .to_owned()?,
            tree: original.tree,
            parents: definition.inputs.iter().map(|input| input.commit_id).collect(),
            encoding: None,
            message: BString::default(),
            extra_headers: Vec::new(),
        }
    } else {
        original
    };
    definition.store(&mut commit);
    let mut ids = graph.edit_commit_ids();
    let target = if created {
        let commit_id = repo.write_object(&commit)?.detach();
        ids.push(commit_id);
        commit_id
    } else {
        selected_commit_id
    };
    let mut expanded = crate::history::HistoryGraph::for_commits(&repo, &ids)?;
    expanded.bounded_history.clone_from(&graph.bounded_history);
    expanded.auto_merges.insert(target, definition.clone());
    let eager = created || head_commit_id == selected_commit_id;
    let result = rebase::perform_with_progress(
        &repo,
        &expanded,
        rebase::Edit::Replace { target, commit },
        rebase::Signature::RedoIfNeeded,
        if eager {
            rebase::Tree::CherryPick
        } else {
            rebase::Tree::LeaveAsIsAndMark
        },
        created.then_some(checkout),
        report,
    )?;
    let notice = match &result {
        rebase::Perform::Complete(outcome) if !created && outcome.ref_changes.is_empty() => {
            let mut any_live = false;
            for input in &definition.inputs {
                any_live |= references.resolve_input(&repo, input, &HashMap::new(), None)?.is_some();
            }
            if !any_live {
                "no inputs remain; AutoMerge unchanged"
            } else {
                "AutoMerge is already up to date"
            }
        }
        _ => notice,
    };
    let notice = match &result {
        rebase::Perform::Complete(outcome) => outcome
            .notice
            .as_ref()
            .map_or_else(|| notice.into(), |checkout| format!("{notice}; {checkout}")),
        _ => notice.into(),
    };
    Ok(Operation {
        result: Some(result),
        notice,
    })
}

/// Add derived dependents and pending inputs outside the explicitly edited todo.
/// Omitted picks inside its original scope remain deliberate deletions.
pub(crate) fn expand_plan(
    repo: &gix::Repository,
    graph: &crate::history::HistoryGraph,
    plan: &mut rebase::Plan,
) -> Result<References> {
    if graph.auto_merges.is_empty() {
        return Ok(References::default());
    }
    let original: HashSet<_> = plan.scope.iter().copied().collect();
    let checkout = plan
        .checkout
        .as_ref()
        .and_then(|checkout| match checkout.target {
            rebase::PlanParent::Existing(commit_id) => Some(commit_id),
            rebase::PlanParent::Step(index) => plan.steps.get(index).and_then(|step| match step.commit {
                rebase::PlanCommit::Pick(commit_id) | rebase::PlanCommit::Resolved(commit_id) => Some(commit_id),
                _ => None,
            }),
        })
        .or(repo.head()?.id().map(gix::Id::detach));
    let mut affected = plan.scope.clone();
    for (&merge_commit_id, definition) in &graph.auto_merges {
        if !graph.is_in_edit_scope(merge_commit_id) {
            continue;
        }
        for input in &definition.inputs {
            let Some(reference) = input.source.reference() else {
                continue;
            };
            let mut affected_ref = false;
            for expected in &plan.expected_refs {
                affected_ref |= follows_reference(repo, reference, &expected.name)?;
            }
            if affected_ref {
                for commit_id in graph.descendants_in_parent_order(merge_commit_id).into_iter().flatten() {
                    if !affected.contains(&commit_id) {
                        affected.push(commit_id);
                    }
                }
                break;
            }
        }
    }
    let preparation = prepare(repo, graph, &mut affected, checkout, None)?;
    let mut positions = HashMap::new();
    for (index, step) in plan.steps.iter().enumerate() {
        if let rebase::PlanCommit::Pick(commit_id) | rebase::PlanCommit::Resolved(commit_id) = step.commit {
            positions.insert(commit_id, index);
        }
        for &commit_id in &step.squash {
            positions.insert(commit_id, index);
        }
    }
    let extra: Vec<_> = affected.into_iter().filter(|id| !original.contains(id)).collect();
    for &commit_id in &extra {
        let parent_commit_id = graph
            .parents_or_load(repo, commit_id)?
            .first()
            .copied()
            .context("a pending input has no parent")?;
        let parent = positions
            .get(&parent_commit_id)
            .copied()
            .map_or(rebase::PlanParent::Existing(parent_commit_id), rebase::PlanParent::Step);
        positions.insert(commit_id, plan.steps.len());
        plan.steps.push(rebase::PlanStep {
            parent,
            commit: rebase::PlanCommit::Pick(commit_id),
            squash: Vec::new(),
        });
    }
    for reference in rebase::capture_refs(repo, &extra, &[])? {
        if !plan
            .expected_refs
            .iter()
            .any(|expected| expected.name == reference.name)
        {
            plan.expected_refs.push(reference);
        }
    }
    if plan.checkout.is_none() {
        let head = repo.head()?;
        if let Some(commit_id) = head.id().map(gix::Id::detach).filter(|id| extra.contains(id)) {
            plan.checkout = Some(rebase::PlanCheckout {
                target: rebase::PlanParent::Step(positions[&commit_id]),
                reference: head.referent_name().map(ToOwned::to_owned),
            });
        }
    }
    plan.scope.extend(extra);
    Ok(preparation.refs)
}

/// Resolve the complete todo before execution: inputs may be in a later fork section.
pub(crate) fn order_plan(
    repo: &gix::Repository,
    plan: &mut rebase::Plan,
    refs: &mut References,
) -> Result<Vec<Vec<usize>>> {
    for parent in plan
        .steps
        .iter()
        .map(|step| step.parent)
        .chain(plan.checkout.iter().map(|checkout| checkout.target))
        .chain(
            plan.expected_refs
                .iter()
                .filter_map(|expected| expected.destination.placement()),
        )
    {
        if let rebase::PlanParent::Step(index) = parent {
            ensure!(index < plan.steps.len(), "a rebase plan points to a missing step");
        }
    }
    let mut positions = HashMap::new();
    for (index, step) in plan.steps.iter().enumerate() {
        if let rebase::PlanCommit::Pick(commit_id) | rebase::PlanCommit::Resolved(commit_id) = step.commit {
            positions.insert(commit_id, index);
        }
        for &commit_id in &step.squash {
            positions.insert(commit_id, index);
        }
    }
    let automatic: HashSet<_> = plan
        .steps
        .iter()
        .enumerate()
        .filter_map(|(index, step)| match step.commit {
            rebase::PlanCommit::Pick(commit_id) => Some((index, commit_id)),
            _ => None,
        })
        .map(|(index, commit_id)| -> Result<_> {
            Ok((
                index,
                is_auto_merge(&repo.find_commit(commit_id)?.decode()?.into_owned()?),
            ))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|(index, automatic)| automatic.then_some(index))
        .collect();
    for expected in &mut plan.expected_refs {
        let rebase::RefDestination::Follow { tip } = expected.destination else {
            continue;
        };
        let mut commit_id = expected.source;
        let mut seen = HashSet::new();
        let mut target = loop {
            ensure!(seen.insert(commit_id), "a dropped input has cyclic ancestry");
            if let Some(&index) = positions.get(&commit_id) {
                break rebase::PlanParent::Step(index);
            }
            if !plan.scope.contains(&commit_id) {
                break rebase::PlanParent::Existing(commit_id);
            }
            let Some(parent) = repo.find_commit(commit_id)?.parent_ids().next() else {
                break rebase::PlanParent::Existing(plan.base);
            };
            commit_id = parent.detach();
        };
        if tip {
            let mut followed = HashSet::new();
            while let Some(index) = plan
                .steps
                .iter()
                .enumerate()
                .position(|(index, step)| step.parent == target && !automatic.contains(&index))
            {
                ensure!(followed.insert(index), "a reference follows a cycle in the todo");
                target = rebase::PlanParent::Step(index);
            }
        }
        expected.destination = target.into();
    }
    let mut dependencies = Vec::with_capacity(plan.steps.len());
    let scope: HashSet<_> = plan.scope.iter().copied().collect();
    let mut placements = HashMap::new();
    for step in &plan.steps {
        let definition = match step.commit {
            rebase::PlanCommit::Pick(commit_id) => {
                Definition::from_commit(&repo.find_commit(commit_id)?.decode()?.into_owned()?)?
            }
            _ => None,
        };
        let mut parents = Vec::new();
        if let Some(definition) = definition {
            ensure!(step.squash.is_empty(), "an AutoMerge cannot be squashed into");
            for input in definition.inputs {
                let mut name = match &input.source {
                    InputSource::Reference(name) => name.clone(),
                    InputSource::Change(change_id) => {
                        let commit_id = if scope.contains(&input.commit_id) {
                            input.commit_id
                        } else {
                            refs.resolve_input(repo, &input, &HashMap::new(), None)?
                                .expect("change inputs remain until a rewrite removes them")
                        };
                        if scope.contains(&commit_id) {
                            let destination = match positions.get(&commit_id).copied() {
                                Some(index)
                                    if matches!(plan.steps[index].commit,
                                        rebase::PlanCommit::Pick(retained) | rebase::PlanCommit::Resolved(retained)
                                        if crate::change_id::for_commit(repo, retained)? == *change_id
                                    ) =>
                                {
                                    parents.push(index);
                                    rebase::RefDestination::Step(index)
                                }
                                _ => rebase::RefDestination::Delete,
                            };
                            placements.insert(input.commit_id, destination);
                        }
                        continue;
                    }
                };
                let mut seen = HashSet::new();
                loop {
                    ensure!(
                        seen.insert(name.clone()),
                        "AutoMerge input has a symbolic reference cycle"
                    );
                    if let Some(expected) = plan.expected_refs.iter().find(|expected| expected.name == name) {
                        if let rebase::RefDestination::Step(index) = expected.destination {
                            parents.push(index);
                        }
                        break;
                    }
                    match undo::state(repo, name.as_ref())? {
                        undo::State::Symbolic(target) => name = target,
                        _ => {
                            if let Some(commit_id) = refs.resolve(repo, &name, &HashMap::new(), None)?
                                && !matches!(
                                    name.category(),
                                    Some(gix::refs::Category::Tag | gix::refs::Category::RemoteBranch)
                                )
                                && let Some(index) = positions.get(&commit_id)
                            {
                                parents.push(*index);
                            }
                            break;
                        }
                    }
                }
            }
        } else if let rebase::PlanParent::Step(parent) = step.parent {
            parents.push(parent);
        }
        dependencies.push(parents);
    }
    let mut order = Vec::new();
    let mut done = HashSet::new();
    while order.len() < plan.steps.len() {
        let before = order.len();
        for (index, parents) in dependencies.iter().enumerate() {
            if !done.contains(&index) && parents.iter().all(|parent| done.contains(parent)) {
                done.insert(index);
                order.push(index);
            }
        }
        ensure!(
            order.len() != before,
            "AutoMerge references create a cycle in the rebase todo"
        );
    }
    let mut remap = vec![0; order.len()];
    for (new, old) in order.iter().copied().enumerate() {
        remap[old] = new;
    }
    let parent = |parent| match parent {
        rebase::PlanParent::Existing(commit_id) => rebase::PlanParent::Existing(commit_id),
        rebase::PlanParent::Step(index) => rebase::PlanParent::Step(remap[index]),
    };
    let previous = std::mem::take(&mut plan.steps);
    plan.steps = order
        .iter()
        .map(|index| {
            let mut step = previous[*index].clone();
            step.parent = parent(step.parent);
            step
        })
        .collect();
    if let Some(checkout) = &mut plan.checkout {
        checkout.target = parent(checkout.target);
    }
    for reference in &mut plan.expected_refs {
        if let Some(placement) = reference.destination.placement() {
            reference.destination = parent(placement).into();
        }
    }
    for (commit_id, destination) in placements {
        refs.placements.insert(
            commit_id,
            match destination {
                rebase::RefDestination::Step(index) => rebase::RefDestination::Step(remap[index]),
                destination => destination,
            },
        );
    }
    Ok(order
        .iter()
        .map(|index| dependencies[*index].iter().map(|parent| remap[*parent]).collect())
        .collect())
}

#[cfg(test)]
mod tests;
