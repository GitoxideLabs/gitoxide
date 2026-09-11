use super::*;
use crate::edit::transplant::{Connection, Mode, Placement, Request, Selection};
use std::{collections::hash_map::Entry, fmt::Write as _};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Role {
    Source,
    Preview,
    Destination,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stage {
    Source,
    Mode,
    Connection,
    Destination,
    Placement,
    Confirm,
}

#[derive(Debug)]
struct Leaf {
    tip: ObjectId,
    endpoint: ObjectId,
    selected: bool,
}

#[derive(Debug)]
pub(super) struct SelectionFlow {
    root: ObjectId,
    parents: HashMap<ObjectId, ObjectId>,
    candidates: Vec<Leaf>,
    candidate_for_commit: HashMap<ObjectId, usize>,
    active: Option<usize>,
    active_path: HashSet<ObjectId>,
    members: HashSet<ObjectId>,
    leaves: Vec<ObjectId>,
    leaf_slots: HashMap<ObjectId, usize>,
    preview: HashSet<ObjectId>,
    stage: Stage,
    mode: Mode,
    connection: Connection,
    placement: Placement,
    destination: Option<ObjectId>,
}

impl SelectionFlow {
    fn path(&self, mut endpoint: ObjectId) -> HashSet<ObjectId> {
        let mut path = HashSet::new();
        while path.insert(endpoint) && endpoint != self.root {
            let Some(parent) = self.parents.get(&endpoint) else {
                break;
            };
            endpoint = *parent;
        }
        path
    }

    fn update_members(&mut self) {
        self.members = HashSet::from([self.root]);
        for leaf in self.candidates.iter().filter(|leaf| leaf.selected) {
            let mut endpoint = leaf.endpoint;
            while self.members.insert(endpoint) && endpoint != self.root {
                let Some(parent) = self.parents.get(&endpoint) else {
                    break;
                };
                endpoint = *parent;
            }
        }
        let non_leaves: HashSet<_> = self
            .members
            .iter()
            .filter_map(|commit_id| self.parents.get(commit_id).copied())
            .collect();
        self.leaves.clear();
        self.leaf_slots.clear();
        for (slot, leaf) in self
            .candidates
            .iter()
            .enumerate()
            .filter(|(_, leaf)| leaf.selected && !non_leaves.contains(&leaf.endpoint))
        {
            if let Entry::Vacant(entry) = self.leaf_slots.entry(leaf.endpoint) {
                entry.insert(slot);
                self.leaves.push(leaf.endpoint);
            }
        }
        if self.leaves.is_empty() {
            self.leaves.push(self.root);
        }
        self.update_preview();
        if self.leaves.len() > 1 {
            self.connection = Connection::Fork;
        }
    }

    fn update_preview(&mut self) {
        self.preview = self
            .active
            .map_or_else(HashSet::new, |active| self.path(self.candidates[active].endpoint));
    }

    pub(super) fn notice(&self, app: &App) -> String {
        let leaves = self
            .leaves
            .iter()
            .take(3)
            .map(|commit_id| commit_id.to_hex_with_len(7).to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let extra = self.leaves.len().saturating_sub(3);
        let leaves = if extra == 0 {
            leaves
        } else {
            format!("{leaves}, +{extra}")
        };
        let mut notice = format!(
            "Source: {} commits · root {} · {} leaves: {leaves}",
            self.members.len(),
            self.root.to_hex_with_len(7),
            self.leaves.len()
        );
        if !matches!(self.stage, Stage::Source | Stage::Mode) {
            notice.push_str(match self.mode {
                Mode::Copy => " · Copy",
                Mode::Move => " · Move",
            });
        }
        if matches!(self.stage, Stage::Destination | Stage::Placement | Stage::Confirm) {
            notice.push_str(match self.connection {
                Connection::Fork => " · Fork",
                Connection::Insert => " · Insert",
            });
        }
        if let Some(destination) = self.destination {
            write!(notice, " · destination {}", destination.to_hex_with_len(7))
                .expect("writing to a string cannot fail");
        }
        if self.stage == Stage::Confirm {
            notice.push_str(match self.placement {
                Placement::Above => " · Above",
                Placement::Below => " · Below",
            });
        }
        notice.push('\n');
        if let Some(navigation) = app.topological_navigation.as_ref() {
            write!(
                notice,
                "choose {} {}/{} · h/l cycle · Enter move · Esc abort",
                match navigation.direction {
                    TopologicalDirection::Parent => "parent",
                    TopologicalDirection::Child => "child",
                },
                navigation.choice + 1,
                navigation.candidates.len()
            )
            .expect("writing to a string cannot fail");
            return notice;
        }
        match self.stage {
            Stage::Source => {
                if let Some(active) = self.active {
                    write!(
                        notice,
                        "leaf {}/{} {} · j/k move · J/K topology · h/l leaves · Space toggle",
                        active + 1,
                        self.candidates.len(),
                        if self.candidates[active].selected {
                            "selected"
                        } else {
                            "preview"
                        }
                    )
                    .expect("writing to a string cannot fail");
                } else {
                    notice.push_str("j/k browse · J/K topology · Space add path · h/l leaves");
                }
                notice.push_str(if app.enhanced_keyboard {
                    " · Shift-Space select subtree"
                } else {
                    " · p: Select subtree"
                });
                notice.push_str(" · Enter continue · Esc abort");
            }
            Stage::Mode => notice.push_str(match self.mode {
                Mode::Copy => "[Copy]  Move · j/k choose · Enter continue · Esc abort",
                Mode::Move => "Copy  [Move] · j/k choose · Enter continue · Esc abort",
            }),
            Stage::Connection => notice.push_str(if self.leaves.len() > 1 {
                "[Fork] · multiple leaves require Fork · Enter continue · Esc abort"
            } else {
                match self.connection {
                    Connection::Fork => "[Fork]  Insert · j/k choose · Enter continue · Esc abort",
                    Connection::Insert => "Fork  [Insert] · j/k choose · Enter continue · Esc abort",
                }
            }),
            Stage::Destination => notice.push_str("destination · j/k browse · J/K topology · Enter choose · Esc abort"),
            Stage::Placement => {
                let index = self
                    .destination
                    .and_then(|destination| app.rows.iter().position(|row| row.id == destination));
                let above = index.is_some_and(|index| app.tree_placement_allowed(self, index, Placement::Above));
                let below = index.is_some_and(|index| app.tree_placement_allowed(self, index, Placement::Below));
                notice.push_str(match (above, below, self.placement) {
                    (true, true, Placement::Above) => "[Above]  Below · j/k choose · Enter preview · Esc abort",
                    (true, true, Placement::Below) => "Above  [Below] · j/k choose · Enter preview · Esc abort",
                    (false, true, _) => "[Below] · Enter preview · Esc abort",
                    _ => "[Above] · Enter preview · Esc abort",
                });
            }
            Stage::Confirm => notice.push_str("Rebase preview · preserve logical HEAD · Enter apply · Esc abort"),
        }
        notice
    }
}

impl App {
    pub(crate) fn set_enhanced_keyboard(&mut self, available: bool) {
        self.enhanced_keyboard = available;
    }

    pub(crate) fn tree_selection_active(&self) -> bool {
        self.tree_selection.is_some()
    }

    pub(crate) fn tree_selection_source_active(&self) -> bool {
        self.tree_selection
            .as_ref()
            .is_some_and(|flow| flow.stage == Stage::Source)
    }

    pub(crate) fn tree_selection_allows(&self, action: &Action) -> bool {
        !self.tree_selection_active()
            || matches!(
                action,
                Action::SelectTree
                    | Action::SelectSubtree
                    | Action::PreviousTreeLeaf
                    | Action::NextTreeLeaf
                    | Action::ConfirmTreeSelection
                    | Action::Cancel
                    | Action::ForceQuit
                    | Action::Quit
                    | Action::MoveUp
                    | Action::MoveDown
                    | Action::MoveUpBy(_)
                    | Action::MoveDownBy(_)
                    | Action::TopologicalUp
                    | Action::TopologicalDown
                    | Action::PreviousChild
                    | Action::NextChild
                    | Action::SubmitTopological
                    | Action::CancelTopological
                    | Action::PanUpBy(_)
                    | Action::PanDownBy(_)
                    | Action::ScrollLeft
                    | Action::ScrollRight
                    | Action::HalfPageUp
                    | Action::HalfPageDown
                    | Action::PageUp
                    | Action::PageDown
                    | Action::First
                    | Action::Last
                    | Action::ToggleDate
                    | Action::CycleIds
                    | Action::ToggleName
                    | Action::ToggleEmail
                    | Action::ToggleTrailers
                    | Action::ToggleMailmap
                    | Action::CycleRefs
                    | Action::ToggleRefs
                    | Action::ToggleHistoryDisplay
                    | Action::ToggleInformation
                    | Action::ToggleActions
                    | Action::ToggleCommit
                    | Action::ToggleChanges
                    | Action::CycleChangesParent
                    | Action::OpenDiff
                    | Action::Copy
                    | Action::CopyAuthor
                    | Action::CopyPath(_)
                    | Action::VerifySignatures
                    | Action::Refresh
            )
    }

    fn tree_source_eligible(&self, row: &CommitRow) -> bool {
        !self.hidden_rows.contains(&row.id)
            && row.parent_ids.len() == 1
            && !self.unavailable_patches.contains(&row.id)
            && !self.auto_merges.contains_key(&row.id)
    }

    pub(crate) fn can_select_tree(&self) -> bool {
        self.tree_selection.is_none()
            && self.state == State::Complete
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.worktree_changes_available
            && !self.rebase_continuation_pending()
            && self.changes_focus.is_none()
            && self.reachable_rows.is_none()
            && !self.has_conflict_marker()
            && self
                .selected
                .and_then(|index| self.rows.get(index))
                .is_some_and(|row| self.tree_source_eligible(row))
    }

    pub(crate) fn can_select_subtree(&self) -> bool {
        self.tree_selection_source_active() || self.can_select_tree()
    }

    pub(crate) fn cancel_tree_selection(&mut self) {
        if self.tree_selection.take().is_some() {
            self.topological_navigation = None;
            self.reachable_rows = None;
            self.restore_compressed_history_around_selection();
        }
    }

    fn begin_tree_selection(&mut self) {
        let root = self.rows[self.selected.expect("tree selection requires a source")].id;
        let mut parents = HashMap::new();
        for row in self.rows.iter().rev() {
            if self.tree_source_eligible(row)
                && (row.id == root
                    || row
                        .parent_ids
                        .first()
                        .is_some_and(|parent| parents.contains_key(parent)))
            {
                parents.insert(row.id, row.parent_ids[0]);
            }
        }
        let non_leaves: HashSet<_> = parents.values().copied().collect();
        let candidates: Vec<_> = self
            .rows
            .iter()
            .filter(|row| parents.contains_key(&row.id) && !non_leaves.contains(&row.id))
            .map(|row| Leaf {
                tip: row.id,
                endpoint: row.id,
                selected: false,
            })
            .collect();
        let mut candidate_for_commit = HashMap::with_capacity(parents.len());
        for (slot, leaf) in candidates.iter().enumerate() {
            let mut commit_id = leaf.tip;
            // Shared stems belong to the first displayed candidate. Each commit is indexed once.
            while let Entry::Vacant(entry) = candidate_for_commit.entry(commit_id) {
                entry.insert(slot);
                if commit_id == root {
                    break;
                }
                commit_id = parents[&commit_id];
            }
        }
        self.materialize_compressed_selection();
        self.tree_selection = Some(SelectionFlow {
            root,
            parents,
            candidates,
            candidate_for_commit,
            active: None,
            active_path: HashSet::new(),
            members: HashSet::from([root]),
            leaves: vec![root],
            leaf_slots: HashMap::new(),
            preview: HashSet::new(),
            stage: Stage::Source,
            mode: Mode::Copy,
            connection: Connection::Fork,
            placement: Placement::Above,
            destination: None,
        });
        self.close_shortcut_groups();
        self.topological_navigation = None;
        self.ensure_visible();
    }

    fn tree_placement_allowed(&self, flow: &SelectionFlow, index: usize, placement: Placement) -> bool {
        let row = &self.rows[index];
        if flow.members.contains(&row.id) {
            return false;
        }
        if flow.mode == Mode::Move && self.is_row_hidden(index) && self.is_known_ancestor(flow.root, row.id) {
            return false;
        }
        if placement == Placement::Below
            && (self.is_row_hidden(index) || row.parent_ids.len() != 1 || self.auto_merges.contains_key(&row.id))
        {
            return false;
        }
        flow.connection == Connection::Fork
            || self.is_row_hidden(index)
            || !self.known_merge_descendants.contains(&row.id)
    }

    pub(super) fn update_tree_selection_mask(&mut self) {
        let Some(flow) = self.tree_selection.as_ref() else {
            return;
        };
        self.reachable_rows = match flow.stage {
            Stage::Source if flow.active.is_some() => {
                Some(self.rows.iter().map(|row| flow.active_path.contains(&row.id)).collect())
            }
            Stage::Destination => Some(
                self.rows
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        self.tree_placement_allowed(flow, index, Placement::Above)
                            || self.tree_placement_allowed(flow, index, Placement::Below)
                    })
                    .collect(),
            ),
            _ => None,
        };
    }

    pub(super) fn update_tree_selection_cursor(&mut self) {
        let Some(commit_id) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) else {
            return;
        };
        let Some(flow) = self.tree_selection.as_mut() else {
            return;
        };
        if flow.stage == Stage::Source {
            if let Some(active) = flow.active
                && flow.active_path.contains(&commit_id)
                && flow.candidates[active].endpoint != commit_id
            {
                flow.candidates[active].endpoint = commit_id;
                if flow.candidates[active].selected {
                    flow.update_members();
                } else {
                    flow.update_preview();
                }
            }
        } else if flow.stage == Stage::Destination {
            flow.destination = Some(commit_id);
        }
    }

    pub(crate) fn tree_selection_marker(&self, commit_id: ObjectId) -> Option<(String, Role)> {
        let flow = self.tree_selection.as_ref()?;
        if flow.destination == Some(commit_id) {
            return Some(("D".into(), Role::Destination));
        }
        if flow.root == commit_id {
            return Some(("R".into(), Role::Source));
        }
        if let Some(slot) = flow.leaf_slots.get(&commit_id) {
            return Some((format!("L{}", slot + 1), Role::Source));
        }
        if let Some(active) = flow.active
            && !flow.candidates[active].selected
            && flow.candidates[active].endpoint == commit_id
        {
            return Some((format!("P{}", active + 1), Role::Preview));
        }
        if flow.members.contains(&commit_id) {
            return Some(("+".into(), Role::Source));
        }
        if flow.preview.contains(&commit_id) {
            return Some(("?".into(), Role::Preview));
        }
        None
    }

    pub(super) fn update_tree_selection(&mut self, action: &Action) -> Option<Vec<Effect>> {
        if matches!(action, Action::SelectTree | Action::SelectSubtree) && self.can_select_tree() {
            self.begin_tree_selection();
            if *action == Action::SelectTree {
                return Some(Vec::new());
            }
        }
        let stage = self.tree_selection.as_ref()?.stage;
        if matches!(action, Action::Cancel | Action::CancelTopological) {
            self.cancel_tree_selection();
            return Some(Vec::new());
        }
        if self.state != State::Complete
            && matches!(
                action,
                Action::SelectTree | Action::SelectSubtree | Action::ConfirmTreeSelection
            )
        {
            return Some(Vec::new());
        }
        match action {
            Action::SelectTree | Action::SelectSubtree if stage == Stage::Source => {
                let commit_id = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id);
                let flow = self.tree_selection.as_mut().expect("the source selection is active");
                if *action == Action::SelectSubtree {
                    for leaf in &mut flow.candidates {
                        leaf.endpoint = leaf.tip;
                        leaf.selected = true;
                    }
                } else if let Some(active) = flow.active {
                    flow.candidates[active].selected = !flow.candidates[active].selected;
                } else if let Some(commit_id) = commit_id
                    && !flow.members.contains(&commit_id)
                    && let Some(index) = flow.candidate_for_commit.get(&commit_id).copied()
                {
                    flow.candidates[index].endpoint = commit_id;
                    flow.candidates[index].selected = true;
                } else {
                    return Some(Vec::new());
                }
                flow.update_members();
                if let Some(active) = flow.active {
                    let endpoint = flow.candidates[active].endpoint;
                    self.select_commit(endpoint);
                }
            }
            Action::PreviousTreeLeaf | Action::NextTreeLeaf if stage == Stage::Source => {
                let flow = self.tree_selection.as_mut().expect("the source selection is active");
                if flow.candidates.is_empty() {
                    return Some(Vec::new());
                }
                let active = match flow.active {
                    None => 0,
                    Some(index) if *action == Action::NextTreeLeaf => (index + 1) % flow.candidates.len(),
                    Some(index) => (index + flow.candidates.len() - 1) % flow.candidates.len(),
                };
                flow.active = Some(active);
                flow.active_path = flow.path(flow.candidates[active].tip);
                flow.update_preview();
                let endpoint = flow.candidates[active].endpoint;
                self.update_tree_selection_mask();
                self.select_commit(endpoint);
            }
            Action::ConfirmTreeSelection => match stage {
                Stage::Source => {
                    self.tree_selection.as_mut().expect("selection exists").stage = Stage::Mode;
                }
                Stage::Mode => self.tree_selection.as_mut().expect("selection exists").stage = Stage::Connection,
                Stage::Connection => {
                    self.tree_selection.as_mut().expect("selection exists").stage = Stage::Destination;
                    self.update_tree_selection_mask();
                    let selected = self.selected.filter(|index| self.is_row_reachable(*index)).or_else(|| {
                        self.rows
                            .iter()
                            .enumerate()
                            .find_map(|(index, _)| self.is_row_reachable(index).then_some(index))
                    });
                    if let Some(selected) = selected {
                        self.select(selected);
                    } else {
                        self.tree_selection.as_mut().expect("selection exists").stage = Stage::Connection;
                        self.leave_attention("this selection has no eligible destination");
                    }
                }
                Stage::Destination => {
                    let Some(index) = self.selected.filter(|index| self.is_row_reachable(*index)) else {
                        return Some(Vec::new());
                    };
                    let placement = if self.tree_placement_allowed(
                        self.tree_selection.as_ref().expect("selection exists"),
                        index,
                        Placement::Above,
                    ) {
                        Placement::Above
                    } else {
                        Placement::Below
                    };
                    let flow = self.tree_selection.as_mut().expect("selection exists");
                    flow.destination = Some(self.rows[index].id);
                    flow.placement = placement;
                    flow.stage = Stage::Placement;
                }
                Stage::Placement => self.tree_selection.as_mut().expect("selection exists").stage = Stage::Confirm,
                Stage::Confirm => {
                    let flow = self.tree_selection.as_ref().expect("selection exists");
                    return Some(vec![Effect::Transplant(Request {
                        selection: Selection {
                            root: flow.root,
                            leaves: flow.leaves.clone(),
                        },
                        mode: flow.mode,
                        connection: flow.connection,
                        placement: flow.placement,
                        destination: flow.destination.expect("confirmation follows destination selection"),
                    })]);
                }
            },
            Action::MoveUp | Action::MoveDown | Action::TopologicalUp | Action::TopologicalDown
                if !matches!(stage, Stage::Source | Stage::Destination) =>
            {
                let flow = self.tree_selection.as_ref().expect("selection exists");
                let other_placement = match flow.placement {
                    Placement::Above => Placement::Below,
                    Placement::Below => Placement::Above,
                };
                let placement_allowed = flow
                    .destination
                    .and_then(|destination| self.rows.iter().position(|row| row.id == destination))
                    .is_some_and(|index| self.tree_placement_allowed(flow, index, other_placement));
                let flow = self.tree_selection.as_mut().expect("selection exists");
                match stage {
                    Stage::Mode => {
                        flow.mode = match flow.mode {
                            Mode::Copy => Mode::Move,
                            Mode::Move => Mode::Copy,
                        }
                    }
                    Stage::Connection if flow.leaves.len() == 1 => {
                        flow.connection = match flow.connection {
                            Connection::Fork => Connection::Insert,
                            Connection::Insert => Connection::Fork,
                        }
                    }
                    Stage::Placement if placement_allowed => flow.placement = other_placement,
                    _ => {}
                }
            }
            Action::First
            | Action::Last
            | Action::MoveUpBy(_)
            | Action::MoveDownBy(_)
            | Action::PageUp
            | Action::PageDown
            | Action::HalfPageUp
            | Action::HalfPageDown
                if !matches!(stage, Stage::Source | Stage::Destination) => {}
            Action::SelectTree | Action::SelectSubtree | Action::PreviousTreeLeaf | Action::NextTreeLeaf => {}
            _ => return None,
        }
        self.update_tree_selection_mask();
        Some(Vec::new())
    }
}
