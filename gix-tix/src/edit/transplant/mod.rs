use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, ensure};
use gix::ObjectId;

use super::{auto_merge, rebase};
use crate::history::HistoryGraph;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Selection {
    pub root: ObjectId,
    pub leaves: Vec<ObjectId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Mode {
    Copy,
    Move,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Connection {
    Fork,
    Insert,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Placement {
    Above,
    Below,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Request {
    pub selection: Selection,
    pub mode: Mode,
    pub connection: Connection,
    pub placement: Placement,
    pub destination: ObjectId,
}

impl Selection {
    pub(crate) fn normalize(
        repo: &gix::Repository,
        graph: &HistoryGraph,
        root: ObjectId,
        leaves: &[ObjectId],
    ) -> Result<Self> {
        let mut selection = Self {
            root,
            leaves: leaves.to_vec(),
        };
        let parents = selection.parents(repo, graph)?;
        selection.normalize_leaves(&parents);
        Ok(selection)
    }

    fn normalize_leaves(&mut self, parents: &HashMap<ObjectId, Vec<ObjectId>>) {
        if self.leaves.is_empty() {
            self.leaves.push(self.root);
        }
        self.leaves.sort_unstable();
        self.leaves.dedup();
        let internal: HashSet<_> = parents.values().flatten().copied().collect();
        self.leaves.retain(|id| !internal.contains(id));
    }

    pub(crate) fn subtree(repo: &gix::Repository, graph: &HistoryGraph, root: ObjectId) -> Result<Self> {
        let parents = Self::reachable_parents(repo, graph, root)?;
        let mut selection = Self {
            root,
            leaves: parents.keys().copied().collect(),
        };
        selection.normalize_leaves(&parents);
        Ok(selection)
    }

    fn reachable_parents(
        repo: &gix::Repository,
        graph: &HistoryGraph,
        root: ObjectId,
    ) -> Result<HashMap<ObjectId, Vec<ObjectId>>> {
        let mut selected = HashMap::from([(root, vec![eligible_root_parent(repo, graph, root)?])]);
        for commit_id in graph
            .descendants_in_parent_order(root)
            .context("the selection root is outside editable history")?
        {
            if commit_id == root {
                continue;
            }
            let Some(parents) = graph.parents_of(commit_id) else {
                continue;
            };
            if !parents.iter().any(|parent| selected.contains_key(parent)) || graph.is_read_only(commit_id) {
                continue;
            }
            let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
            if crate::patch_id::is_unavailable(&commit)
                || auto_merge::is_auto_merge(&commit) && auto_merge::ensure_freezable(&commit).is_err()
            {
                continue;
            }
            selected.insert(
                commit_id,
                rebase::replay_parents(&commit)?.unwrap_or_else(|| commit.parents.to_vec()),
            );
        }
        Ok(selected)
    }

    fn parents(&self, repo: &gix::Repository, graph: &HistoryGraph) -> Result<HashMap<ObjectId, Vec<ObjectId>>> {
        let reachable = Self::reachable_parents(repo, graph, self.root)?;
        let mut pending = self.leaves.clone();
        pending.push(self.root);
        for &leaf in &pending {
            if !reachable.contains_key(&leaf) {
                eligible_parents(repo, graph, leaf)?;
                anyhow::bail!("every selected leaf must have an eligible path from the source root");
            }
        }
        let mut parents = HashMap::new();
        while let Some(commit_id) = pending.pop() {
            if parents.contains_key(&commit_id) {
                continue;
            }
            let edges = &reachable[&commit_id];
            // Side histories outside the root-connected selection stay fixed.
            pending.extend(edges.iter().filter(|parent| reachable.contains_key(*parent)).copied());
            parents.insert(commit_id, edges.clone());
        }
        Ok(parents)
    }
}

fn eligible_root_parent(repo: &gix::Repository, graph: &HistoryGraph, commit_id: ObjectId) -> Result<ObjectId> {
    let parents = eligible_parents(repo, graph, commit_id)?;
    let [parent] = parents.as_slice() else {
        anyhow::bail!("the selection root must have exactly one parent");
    };
    auto_merge::ensure_editable(&repo.find_commit(commit_id)?.decode()?.into_owned()?)?;
    Ok(*parent)
}

fn eligible_parents(repo: &gix::Repository, graph: &HistoryGraph, commit_id: ObjectId) -> Result<Vec<ObjectId>> {
    ensure!(
        graph.is_in_edit_scope(commit_id) && !graph.is_read_only(commit_id),
        "a selected commit is outside editable history"
    );
    graph.parents_of(commit_id).context("a selected commit is incomplete")?;
    let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
    if auto_merge::is_auto_merge(&commit) {
        auto_merge::ensure_freezable(&commit)?;
    }
    ensure!(
        !crate::patch_id::is_unavailable(&commit),
        "resolve and amend the conflicting commit before selecting it"
    );
    let parents = rebase::replay_parents(&commit)?.unwrap_or_else(|| commit.parents.to_vec());
    ensure!(!parents.is_empty(), "a selected commit must have a parent");
    Ok(parents)
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Node {
    Original(ObjectId),
    Copy(ObjectId),
    Fixed(ObjectId),
}

impl Node {
    fn commit_id(self) -> ObjectId {
        match self {
            Self::Original(commit_id) | Self::Copy(commit_id) | Self::Fixed(commit_id) => commit_id,
        }
    }
}

pub(crate) fn plan(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    request: &Request,
    target_is_read_only: bool,
) -> Result<rebase::Plan> {
    build_plan(repo, graph, request, target_is_read_only, false)
}

pub(crate) fn paste_plan(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    source: ObjectId,
    target: ObjectId,
    target_is_read_only: bool,
) -> Result<rebase::Plan> {
    build_plan(
        repo,
        graph,
        &Request {
            selection: Selection {
                root: source,
                leaves: vec![source],
            },
            mode: Mode::Copy,
            connection: Connection::Insert,
            placement: Placement::Above,
            destination: target,
        },
        target_is_read_only,
        true,
    )
}

fn build_plan(
    repo: &gix::Repository,
    graph: &HistoryGraph,
    request: &Request,
    target_is_read_only: bool,
    pasted: bool,
) -> Result<rebase::Plan> {
    use rebase::{PlanCheckout, PlanCommit, PlanParent, PlanRef, PlanStep};

    repo.workdir().context("transplant requires a worktree")?;
    let mut selection = request.selection.clone();
    let selected_parents = selection.parents(repo, graph)?;
    selection.normalize_leaves(&selected_parents);
    ensure!(
        request.connection != Connection::Insert || selection.leaves.len() == 1,
        "inserting requires one selected leaf; fork a branching selection instead"
    );
    let root_parent = selected_parents[&selection.root][0];
    let mut frozen = HashSet::new();
    for &commit_id in selected_parents.keys() {
        if request.mode == Mode::Copy
            && auto_merge::is_auto_merge(&repo.find_commit(commit_id)?.decode()?.into_owned()?)
        {
            frozen.insert(commit_id);
        }
    }
    let destination = request.destination;
    ensure!(
        selection.root != destination,
        "the selection root and destination must differ"
    );
    ensure!(
        pasted || !selected_parents.contains_key(&destination),
        "the destination must not be part of the selected tree"
    );
    let destination_parents = graph
        .parents_of(destination)
        .context("the destination is not in the loaded history")?;
    let target_is_read_only = target_is_read_only || graph.is_read_only(destination);
    if request.placement == Placement::Below {
        ensure!(!target_is_read_only, "cannot transplant below a hidden boundary");
        ensure!(
            destination_parents.len() == 1
                && !rebase::has_merge_replay(&repo.find_commit(destination)?.decode()?.into_owned()?),
            "transplanting below requires a single-parent destination"
        );
        ensure!(
            !graph.auto_merges.contains_key(&destination),
            "cannot transplant below an AutoMerge"
        );
    }
    if request.mode == Mode::Move && target_is_read_only {
        ensure!(
            !graph.is_ancestor(selection.root, destination),
            "a read-only destination cannot descend from the moved selection"
        );
    }
    let cut_parent = |mut parent: ObjectId| {
        if request.mode == Mode::Move {
            while let Some(previous) = selected_parents.get(&parent) {
                parent = previous[0];
            }
        }
        parent
    };
    let anchor = match request.placement {
        Placement::Above => destination,
        Placement::Below => cut_parent(destination_parents[0]),
    };
    let mut scope = HashSet::new();
    if request.mode == Mode::Move {
        scope.extend(
            graph
                .descendants_in_parent_order(selection.root)
                .context("the selection root is unavailable")?,
        );
    }
    if request.connection == Connection::Insert && !target_is_read_only {
        scope.extend(
            graph
                .descendants_in_parent_order(destination)
                .context("the destination is outside editable history")?
                .into_iter()
                .filter(|id| request.placement == Placement::Below || *id != destination),
        );
    }

    // Required paths may leave the planned edits at a pending destination. Complete
    // that ancestry in the same transaction; its other descendants remain lazy.
    let mut ancestry = vec![anchor];
    let mut seen = HashSet::new();
    let mut pending = HashSet::new();
    while let Some(cursor) = ancestry.pop() {
        if !seen.insert(cursor) {
            continue;
        }
        let commit = repo.find_commit(cursor)?.decode()?.into_owned()?;
        let is_pending = rebase::is_pending(&commit);
        if !is_pending && !scope.contains(&cursor) {
            continue;
        }
        if is_pending {
            ensure!(
                graph.is_in_edit_scope(cursor)
                    && !graph.is_read_only(cursor)
                    && !(target_is_read_only && cursor == destination),
                "a pending read-only destination must be rebased before transplanting"
            );
            ensure!(
                !crate::patch_id::is_unavailable(&commit),
                "resolve the conflicting destination before transplanting"
            );
            pending.insert(cursor);
            scope.extend(
                graph
                    .descendants_in_parent_order(cursor)
                    .context("pending destination ancestry is unavailable")?,
            );
        }
        if auto_merge::is_auto_merge(&commit) {
            continue;
        }
        ancestry.extend(commit.parents.into_iter().map(cut_parent));
    }

    let selected_node = |id| match request.mode {
        Mode::Copy => Node::Copy(id),
        Mode::Move => Node::Original(id),
    };
    let leaf = selected_node(selection.leaves[0]);
    let selected_edges = |commit_id: ObjectId, edges: &[ObjectId]| {
        if commit_id == selection.root {
            vec![Node::Original(anchor)]
        } else {
            edges
                .iter()
                .map(|parent| {
                    if selected_parents.contains_key(parent) {
                        selected_node(*parent)
                    } else {
                        Node::Fixed(*parent)
                    }
                })
                .collect()
        }
    };
    let mut parents = HashMap::<Node, Vec<Node>>::new();
    let mut scope: Vec<_> = scope.into_iter().collect();
    scope.sort_unstable();
    for &commit_id in &scope {
        ensure!(
            !graph.is_read_only(commit_id),
            "transplant cannot rewrite hidden history"
        );
        let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
        let old_parents = rebase::replay_parents(&commit)?.unwrap_or_else(|| commit.parents.to_vec());
        ensure!(!old_parents.is_empty(), "transplant cannot rewrite root commits");
        let edges = if selected_parents.contains_key(&commit_id) && request.mode == Mode::Move {
            selected_edges(commit_id, &selected_parents[&commit_id])
        } else {
            old_parents
                .into_iter()
                .map(|parent| {
                    let parent = cut_parent(parent);
                    let inserted = request.connection == Connection::Insert
                        && !target_is_read_only
                        && match request.placement {
                            Placement::Above => parent == destination,
                            Placement::Below => commit_id == destination,
                        };
                    if inserted { leaf } else { Node::Original(parent) }
                })
                .collect()
        };
        parents.insert(Node::Original(commit_id), edges);
    }
    if request.mode == Mode::Copy {
        for (&commit_id, edges) in &selected_parents {
            parents.insert(Node::Copy(commit_id), selected_edges(commit_id, edges));
        }
    }
    ensure!(
        request.mode == Mode::Copy
            || parents.iter().any(|(node, edges)| {
                graph.parents_of(node.commit_id()) != Some(edges.iter().map(|parent| parent.commit_id()).collect())
            }),
        "the selection is already at that destination"
    );
    let mut nodes: Vec<_> = parents.keys().copied().collect();
    nodes.sort_unstable_by_key(|node| (node.commit_id(), matches!(node, Node::Copy(_))));
    let positions: HashMap<_, _> = nodes.iter().enumerate().map(|(index, node)| (*node, index)).collect();
    let mut steps = Vec::with_capacity(nodes.len());
    for node in nodes {
        let edges = parents[&node]
            .iter()
            .map(|parent| {
                positions
                    .get(parent)
                    .copied()
                    .map_or(PlanParent::Existing(parent.commit_id()), PlanParent::Step)
            })
            .collect();
        steps.push(PlanStep {
            parents: edges,
            commit: match node {
                Node::Copy(id) if frozen.contains(&id) => PlanCommit::FrozenCopy(id),
                Node::Original(id) => PlanCommit::Pick(id),
                Node::Copy(id) => PlanCommit::Copy(id),
                Node::Fixed(_) => unreachable!("fixed parents are never rewritten"),
            },
            squash: Vec::new(),
        });
    }
    let original_destination = |commit_id| {
        positions
            .get(&Node::Original(commit_id))
            .copied()
            .map_or(PlanParent::Existing(commit_id), PlanParent::Step)
    };
    let head = repo.head()?;
    let head_commit_id = head.id().context("transplant requires a born HEAD")?.detach();
    let head_reference = head.referent_name().map(ToOwned::to_owned);
    let mut ref_scope = scope.clone();
    ref_scope.extend([destination, head_commit_id]);
    let mut expected_refs = rebase::capture_refs(repo, &ref_scope, &[])?;
    let destination_has_children = graph.edit_commit_ids().into_iter().any(|id| {
        graph
            .parents_of(id)
            .is_some_and(|parents| parents.contains(&destination))
    });
    let selected_root = PlanParent::Step(positions[&selected_node(selection.root)]);
    let inserted_leaf = PlanParent::Step(positions[&leaf]);
    for PlanRef {
        source,
        destination: placement,
        name,
        ..
    } in &mut expected_refs
    {
        *placement = original_destination(*source).into();
        if request.connection == Connection::Insert
            && request.placement == Placement::Above
            && *source == destination
            && !target_is_read_only
            && !destination_has_children
            && !(request.mode == Mode::Move && selected_parents.contains_key(source))
        {
            *placement = inserted_leaf.into();
        }
        if pasted && *source == destination && (target_is_read_only || head_commit_id == destination) {
            *placement = if head_commit_id == destination && head_reference.as_ref() == Some(name) {
                inserted_leaf.into()
            } else {
                original_destination(*source).into()
            };
        }
    }
    let checkout_target = if pasted {
        selected_root
    } else {
        original_destination(head_commit_id)
    };
    let checkout_reference = head_reference.filter(|name| {
        (!pasted || head_commit_id == destination)
            && expected_refs
                .iter()
                .any(|expected| expected.name == *name && expected.destination.placement() == Some(checkout_target))
    });
    let mut eager: Vec<_> = selected_parents
        .keys()
        .map(|id| positions[&selected_node(*id)])
        .collect();
    eager.extend(
        pending
            .into_iter()
            .filter_map(|id| positions.get(&Node::Original(id)).copied()),
    );
    eager.sort_unstable();
    eager.dedup();
    let mut base = root_parent;
    while positions.contains_key(&Node::Original(base)) {
        base = graph
            .parents_of(base)
            .and_then(|parents| parents.first().copied())
            .context("the transplant has no unchanged base")?;
    }
    Ok(rebase::Plan {
        base,
        scope,
        steps,
        checkout: Some(PlanCheckout {
            target: checkout_target,
            reference: checkout_reference,
        }),
        expected_refs,
        eager,
        selection: Some(selected_root),
    })
}

#[cfg(test)]
mod tests {
    use std::{path::Path, process::Command};

    use super::*;
    use gix::{bstr::ByteSlice, refs::transaction::PreviousValue};

    fn git(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
        let output = Command::new("git").arg("-C").arg(path).args(args).output()?;
        ensure!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(output.stdout)
    }

    fn child(repo: &gix::Repository, parent: ObjectId, path: &str) -> Result<ObjectId> {
        let mut commit = repo.find_commit(parent)?.decode()?.into_owned()?;
        let mut tree = repo.find_tree(commit.tree)?.edit()?;
        tree.upsert(
            path,
            gix::objs::tree::EntryKind::Blob,
            repo.write_blob(format!("{path}\n"))?,
        )?;
        commit.tree = tree.write()?.detach();
        commit.parents = [parent].into_iter().collect();
        commit.message = format!("add {path}\n").into();
        commit.extra_headers.clear();
        Ok(repo.write_object(&commit)?.detach())
    }

    fn merge(repo: &gix::Repository, parents: &[ObjectId], path: &str) -> Result<ObjectId> {
        let mut commit = repo.find_commit(parents[0])?.decode()?.into_owned()?;
        let mut tree = repo.find_tree(commit.tree)?.edit()?;
        // These fixtures use distinct flat paths, so their recorded merge also
        // has an explicit merge-only addition whose survival can be checked.
        for parent_commit_id in &parents[1..] {
            for entry in repo.find_commit(*parent_commit_id)?.tree()?.iter() {
                let entry = entry?;
                tree.upsert(entry.filename(), entry.mode().kind(), entry.object_id())?;
            }
        }
        tree.upsert(
            path,
            gix::objs::tree::EntryKind::Blob,
            repo.write_blob(format!("{path}\n"))?,
        )?;
        commit.tree = tree.write()?.detach();
        commit.parents = parents.iter().copied().collect();
        commit.message = format!("merge {path}\n").into();
        commit.extra_headers.clear();
        Ok(repo.write_object(&commit)?.detach())
    }

    fn reference(repo: &gix::Repository, name: &str, commit_id: ObjectId) -> Result<()> {
        repo.reference(format!("refs/heads/{name}"), commit_id, PreviousValue::Any, "fixture")?;
        Ok(())
    }

    fn parent(repo: &gix::Repository, commit_id: ObjectId) -> Result<ObjectId> {
        repo.find_commit(commit_id)?
            .parent_ids()
            .next()
            .map(gix::Id::detach)
            .context("the test commit has a parent")
    }

    fn request(
        root: ObjectId,
        leaves: Vec<ObjectId>,
        mode: Mode,
        connection: Connection,
        placement: Placement,
        destination: ObjectId,
    ) -> Request {
        Request {
            selection: Selection { root, leaves },
            mode,
            connection,
            placement,
            destination,
        }
    }

    #[test]
    fn merges_cannot_be_roots_or_below_destinations_even_when_pending_parent_slots_coincide() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let left_commit_id = repo.head_id()?.detach();
        let root_commit_id = parent(&repo, left_commit_id)?;
        let base_commit_id = parent(&repo, root_commit_id)?;
        let right_commit_id = child(&repo, root_commit_id, "right")?;
        let merge_commit_id = merge(&repo, &[left_commit_id, right_commit_id], "merge-only")?;
        let destination_commit_id = child(&repo, base_commit_id, "destination")?;
        reference(&repo, "merge", merge_commit_id)?;
        reference(&repo, "destination", destination_commit_id)?;
        let graph = super::super::loaded_graph(&repo)?;
        assert!(
            Selection::normalize(&repo, &graph, merge_commit_id, &[]).is_err(),
            "ordinary merges cannot be selection roots"
        );
        let below = request(
            destination_commit_id,
            vec![],
            Mode::Copy,
            Connection::Fork,
            Placement::Below,
            merge_commit_id,
        );
        assert!(
            plan(&repo, &graph, &below, false)
                .expect_err("an ordinary merge is not a Below destination")
                .to_string()
                .contains("single-parent destination"),
            "Below rejects the destination's merge topology"
        );
        let outcome = rebase::perform_plan(
            &repo,
            &graph,
            plan(
                &repo,
                &graph,
                &request(
                    root_commit_id,
                    vec![left_commit_id, right_commit_id],
                    Mode::Move,
                    Connection::Fork,
                    Placement::Above,
                    destination_commit_id,
                ),
                false,
            )?,
        )?
        .complete()?;
        let pending_commit_id = outcome.map(merge_commit_id).context("the excluded merge survives")?;
        let pending = repo.find_commit(pending_commit_id)?.decode()?.into_owned()?;
        assert_eq!(
            pending.parents.as_slice(),
            [base_commit_id],
            "identical Git parent IDs are deduplicated while replay remains pending"
        );
        assert_eq!(
            rebase::replay_parents(&pending)?,
            Some(vec![base_commit_id, base_commit_id]),
            "the excluded merge retains both replay slots"
        );
        let graph = super::super::loaded_graph(&repo)?;
        assert!(
            Selection::normalize(&repo, &graph, pending_commit_id, &[]).is_err(),
            "a pending merge remains ineligible as a root after its Git parents deduplicate"
        );
        assert!(
            plan(
                &repo,
                &graph,
                &Request {
                    destination: pending_commit_id,
                    ..below
                },
                false
            )
            .expect_err("a pending merge is not a Below destination")
            .to_string()
            .contains("single-parent destination"),
            "Below rejects durable merge slots even when only one Git parent remains"
        );
        Ok(())
    }

    #[test]
    fn octopus_transplants_keep_all_selected_paths_external_parents_and_redundant_edges() -> gix_testtools::Result {
        for mode in [Mode::Copy, Mode::Move] {
            for connection in [Connection::Fork, Connection::Insert] {
                for placement in [Placement::Above, Placement::Below] {
                    let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
                    let repo = crate::test_repository::open(fixture.path())?;
                    let left_commit_id = repo.head_id()?.detach();
                    let root_commit_id = parent(&repo, left_commit_id)?;
                    let base_commit_id = parent(&repo, root_commit_id)?;
                    let right_commit_id = child(&repo, root_commit_id, "right")?;
                    let outside_commit_id = child(&repo, root_commit_id, "outside")?;
                    let external_commit_id = child(&repo, base_commit_id, "external")?;
                    let merge_commit_id = merge(
                        &repo,
                        &[left_commit_id, right_commit_id, external_commit_id, root_commit_id],
                        "merge-only",
                    )?;
                    let destination_base_commit_id = child(&repo, base_commit_id, "advanced-base")?;
                    let destination_commit_id = child(&repo, destination_base_commit_id, "destination")?;
                    for (name, commit_id) in [
                        ("main", merge_commit_id),
                        ("outside", outside_commit_id),
                        ("destination", destination_commit_id),
                    ] {
                        reference(&repo, name, commit_id)?;
                    }
                    git(fixture.path(), &["reset", "--hard", "main"])?;
                    let graph = super::super::loaded_graph(&repo)?;
                    let selection = Selection::normalize(
                        &repo,
                        &graph,
                        root_commit_id,
                        &[root_commit_id, left_commit_id, right_commit_id, merge_commit_id],
                    )?;
                    assert_eq!(
                        selection.leaves,
                        [merge_commit_id],
                        "the merge is the common endpoint of every selected path"
                    );
                    let selected = selection.parents(&repo, &graph)?;
                    assert_eq!(
                        selected.keys().copied().collect::<HashSet<_>>(),
                        HashSet::from([root_commit_id, left_commit_id, right_commit_id, merge_commit_id]),
                        "selection follows every root-to-endpoint path without importing side histories"
                    );
                    let outcome = rebase::perform_plan(
                        &repo,
                        &graph,
                        plan(
                            &repo,
                            &graph,
                            &Request {
                                selection,
                                mode,
                                connection,
                                placement,
                                destination: destination_commit_id,
                            },
                            false,
                        )?,
                    )?
                    .complete()?;
                    let new_root_commit_id = outcome.selected.context("the transplanted root is selected")?;
                    assert_eq!(
                        parent(&repo, new_root_commit_id)?,
                        if placement == Placement::Above {
                            destination_commit_id
                        } else {
                            destination_base_commit_id
                        },
                        "{mode:?} {connection:?} {placement:?} uses the requested destination edge"
                    );
                    let new_merge_commit_id = match mode {
                        Mode::Move => outcome.map(merge_commit_id).context("the moved merge survives")?,
                        Mode::Copy => super::super::loaded_graph(&repo)?
                            .edit_commit_ids()
                            .into_iter()
                            .find_map(|commit_id| {
                                repo.find_commit(commit_id)
                                    .ok()?
                                    .parent_ids()
                                    .last()
                                    .is_some_and(|parent| parent == new_root_commit_id)
                                    .then_some(commit_id)
                            })
                            .context("the copied merge tip remains visible")?,
                    };
                    let new_merge = repo.find_commit(new_merge_commit_id)?.decode()?.into_owned()?;
                    assert_eq!(
                        new_merge.parents.len(),
                        4,
                        "ancestry-redundant edges retain their original slots"
                    );
                    assert_eq!(
                        new_merge.parents[2], external_commit_id,
                        "the external parent remains fixed"
                    );
                    assert_eq!(
                        new_merge.parents[3], new_root_commit_id,
                        "the redundant selected root maps to its successor"
                    );
                    for (&side_commit_id, path) in new_merge.parents[..2].iter().zip(["tip", "right"]) {
                        assert_eq!(
                            parent(&repo, side_commit_id)?,
                            new_root_commit_id,
                            "both selected branches map to the same root"
                        );
                        assert!(
                            repo.find_commit(side_commit_id)?.tree()?.find_entry(path).is_some(),
                            "selected branch order preserves the {path} parent slot"
                        );
                    }
                    let tree = repo.find_tree(new_merge.tree)?;
                    for path in ["tip", "right", "external", "merge-only", "advanced-base"] {
                        assert!(
                            tree.find_entry(path).is_some(),
                            "{mode:?} {connection:?} {placement:?} retains {path}"
                        );
                    }
                    assert!(
                        !rebase::is_pending(&new_merge),
                        "every selected merge is replayed eagerly"
                    );
                    assert_eq!(
                        repo.head_id()?.detach(),
                        outcome.map(merge_commit_id).context("logical HEAD survives")?,
                        "transplant preserves logical HEAD"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn copying_keeps_an_external_parent_fixed_even_when_insertion_rewrites_that_parent() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let left_commit_id = repo.head_id()?.detach();
        let root_commit_id = parent(&repo, left_commit_id)?;
        let base_commit_id = parent(&repo, root_commit_id)?;
        let external_commit_id = child(&repo, base_commit_id, "external")?;
        let merge_commit_id = merge(&repo, &[left_commit_id, external_commit_id], "merge-only")?;
        reference(&repo, "main", merge_commit_id)?;
        reference(&repo, "external", external_commit_id)?;
        git(fixture.path(), &["reset", "--hard", "main"])?;
        let graph = super::super::loaded_graph(&repo)?;
        let plan = plan(
            &repo,
            &graph,
            &request(
                root_commit_id,
                vec![merge_commit_id],
                Mode::Copy,
                Connection::Insert,
                Placement::Above,
                base_commit_id,
            ),
            false,
        )?;
        let copied_merge_step = plan
            .steps
            .iter()
            .position(|step| step.commit == rebase::PlanCommit::Copy(merge_commit_id))
            .context("the merge has a copied result")?;
        let external_step = plan
            .steps
            .iter()
            .find(|step| step.commit == rebase::PlanCommit::Pick(external_commit_id))
            .context("insertion also rewrites the external source parent")?;
        assert_eq!(
            plan.steps[copied_merge_step].parents[1],
            rebase::PlanParent::Existing(external_commit_id),
            "a fixed source edge is not redirected through an unrelated rewrite"
        );
        assert_eq!(
            external_step.parents,
            [rebase::PlanParent::Step(copied_merge_step)],
            "the external source parent follows the inserted tree separately"
        );
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let new_external_commit_id = outcome
            .map(external_commit_id)
            .context("the external source parent survives")?;
        let copied_merge_commit_id = parent(&repo, new_external_commit_id)?;
        let copied_merge = repo.find_commit(copied_merge_commit_id)?.decode()?.into_owned()?;
        assert_eq!(
            copied_merge.parents[1], external_commit_id,
            "copying preserves the original external parent exactly"
        );
        assert_ne!(
            new_external_commit_id, external_commit_id,
            "the original branch still receives the insertion"
        );
        Ok(())
    }

    #[test]
    fn a_fixed_pending_side_does_not_prevent_replaying_the_changed_selected_parents() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let left_commit_id = repo.head_id()?.detach();
        let root_commit_id = parent(&repo, left_commit_id)?;
        let base_commit_id = parent(&repo, root_commit_id)?;
        let side_commit_id = child(&repo, root_commit_id, "side")?;
        let mut side = repo.find_commit(side_commit_id)?.decode()?.into_owned()?;
        crate::patch_id::mark_unavailable(&mut side);
        let pending_side_commit_id = repo.write_object(&side)?.detach();
        let merge_commit_id = merge(&repo, &[left_commit_id, pending_side_commit_id], "manual-resolution")?;
        let destination_commit_id = child(&repo, base_commit_id, "destination")?;
        reference(&repo, "main", merge_commit_id)?;
        reference(&repo, "destination", destination_commit_id)?;
        git(fixture.path(), &["reset", "--hard", "main"])?;
        let graph = super::super::loaded_graph(&repo)?;
        let selection = Selection::normalize(&repo, &graph, root_commit_id, &[merge_commit_id])?;
        let selected = selection.parents(&repo, &graph)?;
        assert!(
            !selected.contains_key(&pending_side_commit_id),
            "an unavailable side is outside the eligible selection"
        );
        assert!(
            selected.contains_key(&merge_commit_id),
            "the merge remains reachable through its other parent"
        );
        let outcome = rebase::perform_plan(
            &repo,
            &graph,
            plan(
                &repo,
                &graph,
                &Request {
                    selection,
                    mode: Mode::Copy,
                    connection: Connection::Fork,
                    placement: Placement::Above,
                    destination: destination_commit_id,
                },
                false,
            )?,
        )?
        .complete()?;
        let copied_root_commit_id = outcome.selected.context("the copied root is selected")?;
        let copied_merge_commit_id = crate::history::all_pins(&repo)?
            .into_iter()
            .find_map(|pin| {
                let commit = repo.find_commit(pin.id).ok()?.decode().ok()?.into_owned().ok()?;
                (commit.parents.len() == 2
                    && commit.parents[1] == pending_side_commit_id
                    && parent(&repo, commit.parents[0]).ok() == Some(copied_root_commit_id))
                .then_some(pin.id)
            })
            .context("the copied merge tip is retained")?;
        let copied_merge = repo.find_commit(copied_merge_commit_id)?.decode()?.into_owned()?;
        assert!(
            !rebase::is_pending(&copied_merge),
            "the unchanged external parent needs no replay contribution"
        );
        assert!(
            repo.find_tree(copied_merge.tree)?.find_entry("destination").is_some(),
            "the selected parent update was replayed eagerly"
        );
        assert!(
            rebase::is_pending(&repo.find_commit(pending_side_commit_id)?.decode()?.into_owned()?),
            "the fixed side's pending state is untouched"
        );
        Ok(())
    }

    #[test]
    fn moving_bypasses_each_excluded_merge_edge_and_pending_copies_keep_duplicate_parent_slots() -> gix_testtools::Result
    {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let left_commit_id = repo.head_id()?.detach();
        let root_commit_id = parent(&repo, left_commit_id)?;
        let base_commit_id = parent(&repo, root_commit_id)?;
        let external_commit_id = child(&repo, base_commit_id, "external")?;
        let other_commit_id = child(&repo, base_commit_id, "other")?;
        let merge_commit_id = merge(&repo, &[left_commit_id, external_commit_id], "merge-only")?;
        let excluded_commit_id = merge(
            &repo,
            &[merge_commit_id, left_commit_id, other_commit_id],
            "excluded-only",
        )?;
        let destination_commit_id = child(&repo, base_commit_id, "destination")?;
        for (name, commit_id) in [
            ("main", merge_commit_id),
            ("excluded", excluded_commit_id),
            ("destination", destination_commit_id),
        ] {
            reference(&repo, name, commit_id)?;
        }
        git(fixture.path(), &["reset", "--hard", "main"])?;
        let graph = super::super::loaded_graph(&repo)?;
        let moved = plan(
            &repo,
            &graph,
            &request(
                root_commit_id,
                vec![merge_commit_id],
                Mode::Move,
                Connection::Fork,
                Placement::Above,
                destination_commit_id,
            ),
            false,
        )?;
        let excluded = moved
            .steps
            .iter()
            .find(|step| step.commit == rebase::PlanCommit::Pick(excluded_commit_id))
            .context("the excluded descendant must follow the move")?;
        assert_eq!(
            excluded.parents,
            [
                rebase::PlanParent::Existing(base_commit_id),
                rebase::PlanParent::Existing(base_commit_id),
                rebase::PlanParent::Existing(other_commit_id)
            ],
            "each removed edge follows the selected first-parent chain without deduplicating slots"
        );
        let outcome = rebase::perform_plan(&repo, &graph, moved)?.complete()?;
        let pending_commit_id = outcome.map(excluded_commit_id).context("the excluded merge survives")?;
        let pending = repo.find_commit(pending_commit_id)?.decode()?.into_owned()?;
        assert_eq!(
            pending.parents.as_slice(),
            [base_commit_id, other_commit_id],
            "written Git parents deduplicate identical IDs"
        );
        assert_eq!(
            rebase::replay_parents(&pending)?,
            Some(vec![base_commit_id, base_commit_id, other_commit_id]),
            "durable replay retains every corresponding parent slot"
        );
        let graph = super::super::loaded_graph(&repo)?;
        let copied = plan(
            &repo,
            &graph,
            &request(
                other_commit_id,
                vec![pending_commit_id],
                Mode::Copy,
                Connection::Fork,
                Placement::Above,
                destination_commit_id,
            ),
            false,
        )?;
        let copied_merge_step = copied
            .steps
            .iter()
            .find(|step| step.commit == rebase::PlanCommit::Copy(pending_commit_id))
            .context("the pending ordinary merge can be copied")?;
        assert_eq!(
            copied_merge_step.parents.len(),
            3,
            "selection reads the pending merge's full intended slots"
        );
        let outcome = rebase::perform_plan(&repo, &graph, copied)?.complete()?;
        let copied_other_commit_id = outcome.selected.context("the copied source root is selected")?;
        let copied_merge_commit_id = crate::history::all_pins(&repo)?
            .into_iter()
            .find_map(|pin| {
                repo.find_commit(pin.id)
                    .ok()?
                    .parent_ids()
                    .last()
                    .is_some_and(|parent| parent == copied_other_commit_id)
                    .then_some(pin.id)
            })
            .context("the copied merge is retained")?;
        let copied_merge = repo.find_commit(copied_merge_commit_id)?.decode()?.into_owned()?;
        assert!(
            !rebase::is_pending(&copied_merge),
            "copying finishes all pending merge contributions"
        );
        assert_eq!(
            copied_merge.parents.as_slice(),
            [base_commit_id, copied_other_commit_id],
            "final parent order is preserved after deduplication"
        );
        assert!(
            repo.find_tree(copied_merge.tree)?.find_entry("excluded-only").is_some(),
            "the excluded merge's own edit survives replay"
        );
        assert!(
            rebase::is_pending(&repo.find_commit(pending_commit_id)?.decode()?.into_owned()?),
            "copying leaves the original pending occurrence intact"
        );
        Ok(())
    }

    #[test]
    fn transplant_freezes_only_the_copied_occurrence_and_preserves_the_live_original() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let left_commit_id = repo.head_id()?.detach();
        let root_commit_id = parent(&repo, left_commit_id)?;
        let base_commit_id = parent(&repo, root_commit_id)?;
        let right_commit_id = child(&repo, root_commit_id, "right")?;
        reference(&repo, "left", left_commit_id)?;
        reference(&repo, "right", right_commit_id)?;
        let merge_commit_id = merge(&repo, &[left_commit_id, right_commit_id], "merge-only")?;
        let mut automatic = repo.find_commit(merge_commit_id)?.decode()?.into_owned()?;
        let definition = auto_merge::Definition {
            inputs: vec![
                auto_merge::Input {
                    source: auto_merge::InputSource::Reference("refs/heads/left".try_into()?),
                    commit_id: left_commit_id,
                    muted: false,
                },
                auto_merge::Input {
                    source: auto_merge::InputSource::Reference("refs/heads/right".try_into()?),
                    commit_id: right_commit_id,
                    muted: false,
                },
            ],
        };
        definition.store(&mut automatic);
        automatic.message = definition.title();
        automatic.message.extend_from_slice(b"\n\nrecorded body\n");
        let automatic_commit_id = repo.write_object(&automatic)?.detach();
        reference(&repo, "main", automatic_commit_id)?;
        git(fixture.path(), &["reset", "--hard", "main"])?;
        let graph = super::super::loaded_graph(&repo)?;
        assert!(
            Selection::normalize(&repo, &graph, automatic_commit_id, &[]).is_err(),
            "AutoMerges cannot be selection roots"
        );
        let plan = plan(
            &repo,
            &graph,
            &request(
                root_commit_id,
                vec![automatic_commit_id],
                Mode::Copy,
                Connection::Insert,
                Placement::Above,
                base_commit_id,
            ),
            false,
        )?;
        assert!(
            plan.steps
                .iter()
                .any(|step| step.commit == rebase::PlanCommit::FrozenCopy(automatic_commit_id)),
            "the selected result occurrence is explicitly frozen before dependency expansion"
        );
        assert!(
            plan.steps
                .iter()
                .any(|step| step.commit == rebase::PlanCommit::Pick(automatic_commit_id)),
            "the same plan also maintains the live original occurrence"
        );
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let rewritten_root_commit_id = outcome
            .map(root_commit_id)
            .context("insertion rewrites the original source root")?;
        let live_commit_id = outcome.map(automatic_commit_id).context("the live original survives")?;
        assert!(
            auto_merge::is_auto_merge(&repo.find_commit(live_commit_id)?.decode()?.into_owned()?),
            "copying leaves the original occurrence live during the same transaction"
        );
        let frozen_commit_id = parent(&repo, rewritten_root_commit_id)?;
        let frozen = repo.find_commit(frozen_commit_id)?.decode()?.into_owned()?;
        assert!(
            !auto_merge::is_auto_merge(&frozen),
            "the transplanted occurrence becomes an ordinary merge"
        );
        assert_eq!(
            frozen.message.as_slice(),
            b"Merge left and right\n\nrecorded body\n",
            "freeze uses prose and preserves the recorded body"
        );
        assert!(
            !rebase::is_pending(&frozen),
            "the frozen selected occurrence is finalized eagerly"
        );
        assert!(
            repo.find_tree(frozen.tree)?.find_entry("merge-only").is_some(),
            "freeze preserves the recorded merge-only edit"
        );
        Ok(())
    }

    #[test]
    fn branching_copies_replay_every_selected_path_and_preserve_the_checkout() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head = repo.head_id()?.detach();
        let root = parent(&repo, head)?;
        let base = parent(&repo, root)?;
        let other = child(&repo, root, "other")?;
        let destination = child(&repo, base, "destination")?;
        reference(&repo, "other", other)?;
        reference(&repo, "destination", destination)?;
        let graph = super::super::loaded_graph(&repo)?;
        assert_eq!(
            Selection::normalize(&repo, &graph, root, &[])?.leaves,
            [root],
            "an omitted endpoint selects only the validated root"
        );
        let selection = Selection::normalize(&repo, &graph, root, &[root, head, other, head])?;
        assert_eq!(selection.leaves.len(), 2, "ancestor and repeated leaves are normalized");
        assert_eq!(Selection::subtree(&repo, &graph, root)?, selection);
        let plan = plan(
            &repo,
            &graph,
            &Request {
                selection,
                mode: Mode::Copy,
                connection: Connection::Fork,
                placement: Placement::Above,
                destination,
            },
            false,
        )?;
        assert_eq!(
            plan.eager.len(),
            3,
            "every copied commit is required even away from HEAD"
        );
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let copied_root = outcome.selected.context("the copied root is selected")?;
        assert_eq!(parent(&repo, copied_root)?, destination);
        assert_eq!(repo.head_id()?, head, "copying preserves logical HEAD");
        assert_eq!(
            repo.head()?.referent_name().map(gix::refs::FullNameRef::as_bstr),
            Some("refs/heads/main".as_bytes().as_bstr())
        );
        assert_eq!(
            repo.find_reference("refs/heads/other")?.id(),
            other,
            "copies inherit no source refs"
        );
        assert_eq!(
            repo.find_reference("refs/heads/destination")?.id(),
            destination,
            "forking preserves destination refs"
        );
        let copied_leaves: Vec<_> = crate::history::all_pins(&repo)?
            .into_iter()
            .filter(|pin| parent(&repo, pin.id).is_ok_and(|id| id == copied_root))
            .collect();
        assert_eq!(
            copied_leaves.len(),
            2,
            "both unreferenced copied leaves remain reachable"
        );
        for commit_id in std::iter::once(copied_root).chain(copied_leaves.into_iter().map(|pin| pin.id)) {
            let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
            assert!(!rebase::is_pending(&commit), "every selected copy is fully replayed");
            assert!(
                repo.find_tree(commit.tree)?.find_entry("destination").is_some(),
                "every selected tree includes its new base"
            );
        }
        Ok(())
    }

    #[test]
    fn moving_a_branching_selection_cuts_out_all_selected_ancestors() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head = repo.head_id()?.detach();
        let root = parent(&repo, head)?;
        let base = parent(&repo, root)?;
        let other = child(&repo, root, "other")?;
        let excluded = child(&repo, head, "excluded")?;
        let destination = child(&repo, base, "destination")?;
        for (name, id) in [("other", other), ("excluded", excluded), ("destination", destination)] {
            reference(&repo, name, id)?;
        }
        let graph = super::super::loaded_graph(&repo)?;
        let plan = plan(
            &repo,
            &graph,
            &request(
                root,
                vec![head, other],
                Mode::Move,
                Connection::Fork,
                Placement::Above,
                destination,
            ),
            false,
        )?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let moved_root = outcome.map(root).context("root survives")?;
        assert_eq!(
            outcome.selected,
            Some(moved_root),
            "selection is independent of the leaf checkout"
        );
        assert_eq!(parent(&repo, moved_root)?, destination);
        for leaf in [head, other] {
            let moved = outcome.map(leaf).context("selected leaf survives")?;
            assert_eq!(parent(&repo, moved)?, moved_root);
            assert!(
                !rebase::is_pending(&repo.find_commit(moved)?.decode()?.into_owned()?),
                "all selected branches replay eagerly"
            );
        }
        let excluded = outcome.map(excluded).context("excluded child survives")?;
        assert_eq!(
            parent(&repo, excluded)?,
            base,
            "excluded children bypass the entire selected path"
        );
        assert!(
            rebase::is_pending(&repo.find_commit(excluded)?.decode()?.into_owned()?),
            "off-selection descendants remain lazy"
        );
        assert_eq!(repo.head_id()?.detach(), outcome.map(head).context("HEAD survives")?);
        Ok(())
    }

    #[test]
    fn transplanting_below_changes_only_the_requested_destination_edge() -> gix_testtools::Result {
        for mode in [Mode::Copy, Mode::Move] {
            for connection in [Connection::Fork, Connection::Insert] {
                let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
                let repo = crate::test_repository::open(fixture.path())?;
                let source_commit_id = repo.head_id()?.detach();
                let destination_commit_id = parent(&repo, source_commit_id)?;
                let base_commit_id = parent(&repo, destination_commit_id)?;
                let sibling_commit_id = child(&repo, base_commit_id, "sibling")?;
                reference(&repo, "sibling", sibling_commit_id)?;
                let graph = super::super::loaded_graph(&repo)?;
                let plan = plan(
                    &repo,
                    &graph,
                    &request(
                        source_commit_id,
                        vec![source_commit_id],
                        mode,
                        connection,
                        Placement::Below,
                        destination_commit_id,
                    ),
                    false,
                )?;
                let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
                let transplanted_commit_id = outcome.selected.context("the transplanted root is selected")?;
                assert_eq!(
                    parent(&repo, transplanted_commit_id)?,
                    base_commit_id,
                    "{mode:?} {connection:?} Below uses the destination's parent"
                );
                assert_eq!(
                    parent(
                        &repo,
                        outcome.map(destination_commit_id).context("the destination survives")?
                    )?,
                    match connection {
                        Connection::Fork => base_commit_id,
                        Connection::Insert => transplanted_commit_id,
                    },
                    "{mode:?} {connection:?} reparents the destination only for insertion"
                );
                assert_eq!(
                    repo.find_reference("refs/heads/sibling")?.id(),
                    sibling_commit_id,
                    "{mode:?} {connection:?} preserves destination siblings"
                );
                let source_commit_id = outcome.map(source_commit_id).context("the source survives")?;
                assert_eq!(repo.head_id()?, source_commit_id, "logical HEAD follows the source");
                match mode {
                    Mode::Copy => assert_ne!(
                        source_commit_id, transplanted_commit_id,
                        "the original remains distinct from its copy"
                    ),
                    Mode::Move => assert_eq!(
                        source_commit_id, transplanted_commit_id,
                        "the source follows its moved occurrence"
                    ),
                }
                let commit = repo.find_commit(transplanted_commit_id)?.decode()?.into_owned()?;
                assert!(
                    !rebase::is_pending(&commit),
                    "{mode:?} {connection:?} replays the selected commit eagerly"
                );
                assert!(
                    repo.find_tree(commit.tree)?.find_entry("middle").is_none(),
                    "{mode:?} {connection:?} does not inherit the destination's changes"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn inserting_below_a_rewritten_parent_preserves_linked_worktree_staging() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let source_commit_id = repo.head_id()?.detach();
        let destination_commit_id = parent(&repo, source_commit_id)?;
        let linked_root = gix_testtools::tempfile::tempdir()?;
        let linked = linked_root.path().join("linked");
        let output = Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["worktree", "add", "-q", "-b", "linked"])
            .arg(&linked)
            .arg(destination_commit_id.to_string())
            .output()?;
        assert!(output.status.success(), "the linked worktree can be created");
        std::fs::write(linked.join("base"), "staged base\n")?;
        git(&linked, &["add", "base"])?;
        std::fs::write(linked.join("base"), "unstaged base\n")?;
        let staged_tree = git(&linked, &["write-tree"])?;
        let graph = super::super::loaded_graph(&repo)?;
        let plan = plan(
            &repo,
            &graph,
            &request(
                source_commit_id,
                vec![source_commit_id],
                Mode::Move,
                Connection::Insert,
                Placement::Below,
                destination_commit_id,
            ),
            false,
        )?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let destination_commit_id = outcome.map(destination_commit_id).context("the destination survives")?;
        assert_eq!(
            repo.find_reference("refs/heads/linked")?.id(),
            destination_commit_id,
            "the checked-out linked branch follows its rewritten commit"
        );
        assert!(
            rebase::is_pending(&repo.find_commit(destination_commit_id)?.decode()?.into_owned()?),
            "the excluded destination remains lazy"
        );
        assert_eq!(
            git(&linked, &["write-tree"])?,
            staged_tree,
            "a lazy parent rewrite leaves the linked index intact"
        );
        assert_eq!(
            std::fs::read(linked.join("base"))?,
            b"unstaged base\n",
            "the linked worktree retains its separate unstaged changes"
        );
        Ok(())
    }

    #[test]
    fn advancing_the_destination_branch_preserves_its_checkout_and_local_changes() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let source_commit_id = repo.head_id()?.detach();
        let base_commit_id = parent(&repo, parent(&repo, source_commit_id)?)?;
        let destination_commit_id = child(&repo, base_commit_id, "destination")?;
        reference(&repo, "destination", destination_commit_id)?;
        git(fixture.path(), &["checkout", "-q", "destination"])?;
        std::fs::write(fixture.path().join("base"), "staged base\n")?;
        git(fixture.path(), &["add", "base"])?;
        std::fs::write(fixture.path().join("base"), "unstaged base\n")?;
        let staged_tree = git(fixture.path(), &["write-tree"])?;
        let graph = super::super::loaded_graph(&repo)?;
        let plan = plan(
            &repo,
            &graph,
            &request(
                source_commit_id,
                vec![source_commit_id],
                Mode::Copy,
                Connection::Insert,
                Placement::Above,
                destination_commit_id,
            ),
            false,
        )?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let copied_commit_id = outcome.selected.context("the copy is selected")?;
        assert_eq!(
            repo.find_reference("refs/heads/destination")?.id(),
            copied_commit_id,
            "the destination tip branch advances to the inserted copy"
        );
        assert_eq!(repo.head_id()?, destination_commit_id, "logical HEAD stays put");
        assert!(repo.head()?.referent_name().is_none(), "the old tip is detached");
        assert_eq!(
            git(fixture.path(), &["write-tree"])?,
            staged_tree,
            "the current index does not follow the advanced branch"
        );
        assert_eq!(
            std::fs::read(fixture.path().join("base"))?,
            b"unstaged base\n",
            "the preserved checkout retains its unstaged changes"
        );
        assert!(
            !fixture.path().join("tip").exists(),
            "the copied tree is never checked out in the current worktree"
        );
        assert!(
            crate::history::all_pins(&repo)?
                .iter()
                .any(|pin| pin.is_head() && pin.id == copied_commit_id),
            "the existing HEAD pin retains the advanced branch"
        );
        Ok(())
    }

    #[test]
    fn pending_destination_is_replayed_in_the_same_plan() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_edit.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let head = repo.head_id()?.detach();
        let source = parent(&repo, head)?;
        let base = parent(&repo, source)?;
        let advanced_base = child(&repo, base, "advanced-base")?;
        let old_destination = child(&repo, base, "destination")?;
        let mut destination = repo.find_commit(old_destination)?.decode()?.into_owned()?;
        destination.parents = [advanced_base].into_iter().collect();
        destination
            .extra_headers
            .push(("tix-rebase-parent".into(), base.to_string().into()));
        let destination = repo.write_object(&destination)?.detach();
        reference(&repo, "destination", destination)?;
        let graph = super::super::loaded_graph(&repo)?;
        let plan = plan(
            &repo,
            &graph,
            &request(
                source,
                vec![source],
                Mode::Copy,
                Connection::Fork,
                Placement::Above,
                destination,
            ),
            false,
        )?;
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let copied = outcome.selected.context("copy is selected")?;
        let destination = outcome.map(destination).context("destination survives")?;
        assert_eq!(parent(&repo, copied)?, destination);
        assert_eq!(repo.find_reference("refs/heads/destination")?.id(), destination);
        for commit_id in [destination, copied] {
            let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
            assert!(
                !rebase::is_pending(&commit),
                "required destination ancestry and selection are final"
            );
            assert!(
                repo.find_tree(commit.tree)?.find_entry("advanced-base").is_some(),
                "the pending base was replayed before the copy"
            );
        }
        assert_eq!(
            repo.head_id()?,
            head,
            "replaying a destination elsewhere preserves HEAD"
        );
        Ok(())
    }
}
