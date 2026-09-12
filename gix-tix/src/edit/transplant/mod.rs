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

    fn normalize_leaves(&mut self, parents: &HashMap<ObjectId, ObjectId>) {
        if self.leaves.is_empty() {
            self.leaves.push(self.root);
        }
        self.leaves.sort_unstable();
        self.leaves.dedup();
        let internal: HashSet<_> = parents.values().copied().collect();
        self.leaves.retain(|id| !internal.contains(id));
    }

    pub(crate) fn subtree(repo: &gix::Repository, graph: &HistoryGraph, root: ObjectId) -> Result<Self> {
        eligible_parent(repo, graph, root)?;
        let mut selected = HashSet::from([root]);
        let mut leaves = selected.clone();
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
            let [parent] = parents.as_slice() else { continue };
            if selected.contains(parent)
                && !graph.is_read_only(commit_id)
                && !graph.auto_merges.contains_key(&commit_id)
                && !graph.unavailable_patches.contains(&commit_id)
            {
                eligible_parent(repo, graph, commit_id)?;
                selected.insert(commit_id);
                leaves.remove(parent);
                leaves.insert(commit_id);
            }
        }
        let mut leaves: Vec<_> = leaves.into_iter().collect();
        leaves.sort_unstable();
        Ok(Self { root, leaves })
    }

    fn parents(&self, repo: &gix::Repository, graph: &HistoryGraph) -> Result<HashMap<ObjectId, ObjectId>> {
        let mut parents = HashMap::from([(self.root, eligible_parent(repo, graph, self.root)?)]);
        for &leaf in &self.leaves {
            let mut cursor = leaf;
            let mut seen = HashSet::new();
            loop {
                ensure!(seen.insert(cursor), "the selection ancestry contains a cycle");
                if parents.contains_key(&cursor) {
                    break;
                }
                let parent = eligible_parent(repo, graph, cursor)
                    .context("every selected leaf must have the source root as an ancestor")?;
                parents.insert(cursor, parent);
                cursor = parent;
            }
        }
        Ok(parents)
    }
}

fn eligible_parent(repo: &gix::Repository, graph: &HistoryGraph, commit_id: ObjectId) -> Result<ObjectId> {
    ensure!(
        graph.is_in_edit_scope(commit_id) && !graph.is_read_only(commit_id),
        "a selected commit is outside editable history"
    );
    let parents = graph.parents_of(commit_id).context("a selected commit is incomplete")?;
    let [parent] = parents.as_slice() else {
        anyhow::bail!("every selected commit must have exactly one parent");
    };
    let commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
    auto_merge::ensure_editable(&commit)?;
    ensure!(
        !crate::patch_id::is_unavailable(&commit),
        "resolve and amend the conflicting commit before selecting it"
    );
    Ok(*parent)
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Node {
    Original(ObjectId),
    Copy(ObjectId),
}

impl Node {
    fn commit_id(self) -> ObjectId {
        match self {
            Self::Original(commit_id) | Self::Copy(commit_id) => commit_id,
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
    let root_parent = selected_parents[&selection.root];
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
            destination_parents.len() == 1,
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
                parent = *previous;
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
    let mut cursor = anchor;
    let mut seen = HashSet::new();
    let mut pending = HashSet::new();
    let mut oldest_pending = None;
    loop {
        ensure!(seen.insert(cursor), "the destination ancestry contains a cycle");
        let commit = repo.find_commit(cursor)?.decode()?.into_owned()?;
        let is_pending = rebase::is_pending(&commit);
        if !is_pending && !scope.contains(&cursor) {
            break;
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
            oldest_pending = Some(cursor);
        }
        if auto_merge::is_auto_merge(&commit) {
            break;
        }
        let Some(parent) = commit.parents.first() else { break };
        cursor = cut_parent(*parent);
    }
    if let Some(commit_id) = oldest_pending {
        scope.extend(
            graph
                .descendants_in_parent_order(commit_id)
                .context("pending destination ancestry is unavailable")?,
        );
    }

    let selected_node = |id| match request.mode {
        Mode::Copy => Node::Copy(id),
        Mode::Move => Node::Original(id),
    };
    let leaf = selected_node(selection.leaves[0]);
    let mut parents = HashMap::<Node, Node>::new();
    let mut scope: Vec<_> = scope.into_iter().collect();
    scope.sort_unstable();
    for &commit_id in &scope {
        ensure!(
            !graph.is_read_only(commit_id),
            "transplant cannot rewrite hidden history"
        );
        let old_parents = graph
            .parents_of(commit_id)
            .context("an affected commit is incomplete")?;
        let Some(&old_parent) = old_parents
            .first()
            .filter(|_| old_parents.len() == 1 || graph.auto_merges.contains_key(&commit_id))
        else {
            anyhow::bail!("transplant cannot rewrite root or ordinary merge commits");
        };
        let mut parent = if selected_parents.contains_key(&commit_id) && request.mode == Mode::Move {
            old_parent
        } else {
            cut_parent(old_parent)
        };
        let inserted = request.connection == Connection::Insert
            && !target_is_read_only
            && !(request.mode == Mode::Move && selected_parents.contains_key(&commit_id))
            && match request.placement {
                Placement::Above => parent == destination,
                Placement::Below => commit_id == destination,
            };
        if commit_id == selection.root && request.mode == Mode::Move {
            parent = anchor;
        }
        parents.insert(
            Node::Original(commit_id),
            if inserted { leaf } else { Node::Original(parent) },
        );
    }
    if request.mode == Mode::Copy {
        for (&commit_id, &parent) in &selected_parents {
            parents.insert(
                Node::Copy(commit_id),
                if commit_id == selection.root {
                    Node::Original(anchor)
                } else {
                    Node::Copy(parent)
                },
            );
        }
    }
    ensure!(
        request.mode == Mode::Copy
            || parents.iter().any(|(node, parent)| {
                graph
                    .parents_of(node.commit_id())
                    .and_then(|parents| parents.first().copied())
                    != Some(parent.commit_id())
            }),
        "the selection is already at that destination"
    );
    let mut nodes: Vec<_> = parents.keys().copied().collect();
    nodes.sort_unstable_by_key(|node| (node.commit_id(), matches!(node, Node::Copy(_))));
    let mut positions = HashMap::new();
    let mut steps = Vec::with_capacity(nodes.len());
    for node in nodes {
        let mut path = Vec::new();
        let mut seen = HashSet::new();
        let mut cursor = node;
        while parents.contains_key(&cursor) && !positions.contains_key(&cursor) {
            ensure!(
                seen.insert(cursor),
                "transplanting the selection would create a commit cycle"
            );
            path.push(cursor);
            cursor = parents[&cursor];
        }
        for node in path.into_iter().rev() {
            let parent = parents[&node];
            let parent = positions
                .get(&parent)
                .copied()
                .map_or(PlanParent::Existing(parent.commit_id()), PlanParent::Step);
            positions.insert(node, steps.len());
            steps.push(PlanStep {
                parents: vec![parent],
                commit: match node {
                    Node::Original(id) => PlanCommit::Pick(id),
                    Node::Copy(id) => PlanCommit::Copy(id),
                },
                squash: Vec::new(),
            });
        }
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
