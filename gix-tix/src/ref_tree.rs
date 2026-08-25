use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ffi::OsString,
    sync::atomic::AtomicBool,
};

use anyhow::Context;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use gix::{ObjectId, bstr::ByteSlice};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Clear, Paragraph},
};

use crate::{
    app::{LaneState, Notice, NoticeKind},
    history::{CommitIndex, Decoration, DecorationKind, Decorations, HistoryGraph, RefSnapshot},
    ui::{decoration_style, notice_area, render_notice},
};

#[derive(Clone, Copy, Debug, Default)]
struct Offset {
    x: usize,
    y: usize,
    page_width: usize,
    page_height: usize,
    max_x: usize,
    max_y: usize,
}

#[derive(Clone, Debug)]
struct Node {
    commit: CommitIndex,
    id: ObjectId,
    parent: Option<usize>,
    children: Vec<usize>,
    decorations: Vec<Decoration>,
    is_head: bool,
    is_anchor: bool,
    is_detached_worktree: bool,
    raw_tip: bool,
    sort_key: String,
}

#[derive(Clone, Debug)]
struct Edge {
    child: usize,
    parent: usize,
    hidden: Vec<CommitIndex>,
}

#[derive(Clone, Debug, Default)]
struct Overview {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    roots: Vec<usize>,
    by_commit: HashMap<CommitIndex, usize>,
    commit_count: usize,
}

#[derive(Clone, Debug)]
struct Overlay {
    selected: CommitIndex,
    reachable: Vec<bool>,
    first_parent: Vec<bool>,
    counts: Vec<Option<usize>>,
    boundaries: Vec<Option<ObjectId>>,
    seen: Vec<u32>,
    stamp: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Point {
    x: usize,
    y: usize,
}

#[derive(Default)]
struct Placed {
    nodes: Vec<Point>,
    boundaries: Vec<Option<Point>>,
    rail_rows: Vec<RailRow>,
    rail_width: usize,
    width: usize,
    height: usize,
}

struct RailRow {
    lane: String,
    kind: RailRowKind,
    edge: Option<usize>,
}

#[derive(Clone, Copy)]
enum RailRowKind {
    Node(usize),
    Boundary(usize),
    NodeConnector,
    BoundaryConnector,
}

#[derive(Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Input {
    Handled,
    PinReferences {
        id: ObjectId,
        kinds: Vec<DecorationKind>,
    },
    ResolveRemoteReferences(Vec<gix::refs::FullName>),
    DeleteLocalBranches {
        names: Vec<gix::refs::FullName>,
        fallback: SelectionFallback,
    },
    DeleteRemoteReferences {
        groups: Vec<RemoteDeletion>,
        fallback: SelectionFallback,
    },
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RemoteDeletion {
    pub remote: gix::bstr::BString,
    pub references: Vec<gix::refs::FullName>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SelectionFallback {
    selected: ObjectId,
    candidates: Vec<ObjectId>,
}

#[derive(Default)]
pub(crate) struct Tree {
    active: bool,
    overview: Option<Overview>,
    alternate_overview: Option<Overview>,
    overlay: Option<Overlay>,
    selected: Option<usize>,
    count_anchor: Option<ObjectId>,
    selection_after_reference_deletion: Option<SelectionFallback>,
    topological_choice: Option<usize>,
    offset: Offset,
    placed: Option<Placed>,
    ensure_visible: bool,
    hide_tags: bool,
    edit_expanded: bool,
    remote_deletions: Vec<RemoteDeletion>,
    notice: Option<Notice>,
    history_commits: HashSet<ObjectId>,
}

impl Tree {
    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn toggle(&mut self) -> bool {
        if !self.active && self.overview.as_ref().is_none_or(|overview| overview.nodes.is_empty()) {
            return false;
        }
        self.active = !self.active;
        self.ensure_visible = true;
        true
    }

    pub(crate) fn leave(&mut self) {
        self.active = false;
        self.edit_expanded = false;
        self.topological_choice = None;
    }

    pub(crate) fn leave_attention(&mut self, message: impl Into<String>) {
        self.leave_notice(NoticeKind::Attention, message);
    }

    pub(crate) fn leave_success(&mut self, message: impl Into<String>) {
        self.leave_notice(NoticeKind::Success, message);
    }

    pub(crate) fn leave_error(&mut self, message: impl Into<String>) {
        self.leave_notice(NoticeKind::Error, message);
    }

    fn leave_notice(&mut self, kind: NoticeKind, message: impl Into<String>) {
        self.notice = Some(Notice {
            kind,
            text: message.into(),
        });
    }

    pub(crate) fn set_history_commits(&mut self, commits: impl IntoIterator<Item = ObjectId>) {
        self.history_commits.clear();
        self.history_commits.extend(commits);
    }

    pub(crate) fn rebuild(&mut self, graph: &HistoryGraph, refs: &RefSnapshot, decorations: &Decorations) {
        let selected = self
            .selected
            .and_then(|selected| self.overview.as_ref()?.nodes.get(selected))
            .map(|node| node.id);
        let with_tags = Overview::new(graph, refs, decorations, true);
        let without_tags = Overview::new(graph, refs, decorations, false);
        let (overview, alternate_overview) = if self.hide_tags {
            (without_tags, with_tags)
        } else {
            (with_tags, without_tags)
        };
        let fallback = self
            .selection_after_reference_deletion
            .take()
            .filter(|fallback| Some(fallback.selected) == selected);
        let head = overview
            .nodes
            .iter()
            .position(|node| node.is_head)
            .or_else(|| {
                selected.and_then(|id| {
                    graph
                        .index(id)
                        .and_then(|index| overview.by_commit.get(&index).copied())
                })
            })
            .or_else(|| {
                refs.view_tips.iter().chain(&refs.hidden_tips).find_map(|id| {
                    graph
                        .index(*id)
                        .and_then(|index| overview.by_commit.get(&index).copied())
                })
            })
            .or((!overview.nodes.is_empty()).then_some(0));
        self.selected = selected
            .and_then(|id| {
                graph
                    .index(id)
                    .and_then(|index| overview.by_commit.get(&index).copied())
            })
            .or_else(|| {
                fallback.and_then(|fallback| {
                    fallback
                        .candidates
                        .into_iter()
                        .find_map(|id| overview.nodes.iter().position(|node| node.id == id))
                })
            })
            .or(head);
        if self
            .count_anchor
            .is_some_and(|id| !overview.nodes.iter().any(|node| node.id == id))
        {
            self.count_anchor = None;
        }
        self.topological_choice = None;
        self.overview = Some(overview);
        self.alternate_overview = Some(alternate_overview);
        self.overlay = None;
        self.placed = None;
        self.ensure_visible = true;
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Input {
        self.notice = None;
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
            || key.code == KeyCode::Char('q')
        {
            return Input::Quit;
        }
        if self.topological_choice.is_some() {
            match key.code {
                KeyCode::Left | KeyCode::Char('h' | 'H')
                    if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.cycle_topological_choice(false);
                }
                KeyCode::Right | KeyCode::Char('l' | 'L')
                    if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    self.cycle_topological_choice(true);
                }
                KeyCode::Enter => self.submit_topological_choice(),
                KeyCode::Esc => self.topological_choice = None,
                _ => {}
            }
            return Input::Handled;
        }
        if self.edit_expanded {
            self.edit_expanded = false;
            if key.code == KeyCode::Char('r') && key.modifiers.is_empty() && !self.remote_deletions.is_empty() {
                return Input::DeleteRemoteReferences {
                    groups: std::mem::take(&mut self.remote_deletions),
                    fallback: self
                        .reference_deletion_fallback()
                        .expect("a selected remote reference has a deletion fallback"),
                };
            }
            self.remote_deletions.clear();
            if key.code == KeyCode::Char('d') && key.modifiers.is_empty() {
                let branches = self.selected_local_branches();
                if branches.is_empty() {
                    self.leave_attention("no deletable local branches at the selected node");
                    return Input::Handled;
                }
                return Input::DeleteLocalBranches {
                    names: branches,
                    fallback: self
                        .reference_deletion_fallback()
                        .expect("a selected branch has a deletion fallback"),
                };
            }
            if key.code == KeyCode::Esc {
                return Input::Handled;
            }
        } else if key.code == KeyCode::Char('e') && key.modifiers.is_empty() {
            self.edit_expanded = true;
            self.remote_deletions.clear();
            let references = self.selected_remote_references();
            return if references.is_empty() {
                Input::Handled
            } else {
                Input::ResolveRemoteReferences(references)
            };
        }
        if key.code == KeyCode::Esc {
            self.leave();
            return Input::Handled;
        }
        if key.code == KeyCode::Enter || key.code == KeyCode::Char('p') && key.modifiers.is_empty() {
            return self
                .selected_pin_target()
                .map_or(Input::Handled, |(id, kinds)| Input::PinReferences { id, kinds });
        }
        if key.code == KeyCode::Char(' ') && key.modifiers.is_empty() {
            self.toggle_count_anchor();
            return Input::Handled;
        }
        if key.code == KeyCode::Char('G')
            || key.code == KeyCode::Char('g') && key.modifiers.contains(KeyModifiers::SHIFT)
        {
            self.jump_to_root();
            return Input::Handled;
        }
        if key.code == KeyCode::Char('g') && key.modifiers.is_empty() {
            self.jump_to_top();
            return Input::Handled;
        }
        if key.code == KeyCode::Char('t') && key.modifiers.is_empty() {
            self.leave();
            return Input::Handled;
        }
        if key.code == KeyCode::Char('T')
            || key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::SHIFT)
        {
            self.toggle_tags();
            return Input::Handled;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            let amount = self.offset().page_height.max(1);
            let (direction, amount) = match key.code {
                KeyCode::Char('u' | 'U') => (Direction::Up, (amount / 2).max(1)),
                KeyCode::Char('d' | 'D') => (Direction::Down, (amount / 2).max(1)),
                KeyCode::Char('b' | 'B') => (Direction::Up, amount),
                KeyCode::Char('f' | 'F') => (Direction::Down, amount),
                _ => return Input::Handled,
            };
            if key.modifiers.contains(KeyModifiers::SHIFT) || matches!(key.code, KeyCode::Char('U' | 'D' | 'B' | 'F')) {
                self.pan(direction, amount);
            } else {
                self.page(direction, amount);
            }
            return Input::Handled;
        }
        match key.code {
            KeyCode::PageUp => {
                self.page_or_pan(Direction::Up, key.modifiers);
                return Input::Handled;
            }
            KeyCode::PageDown => {
                self.page_or_pan(Direction::Down, key.modifiers);
                return Input::Handled;
            }
            _ => {}
        }
        let Some(direction) = direction(key.code) else {
            return Input::Handled;
        };
        let topological = matches!(direction, Direction::Up | Direction::Down)
            && (key.modifiers.contains(KeyModifiers::SHIFT) || matches!(key.code, KeyCode::Char('J' | 'K')));
        self.navigate(direction, topological);
        Input::Handled
    }

    pub(crate) fn handle_mouse(&mut self, kind: MouseEventKind, modifiers: KeyModifiers, distance: usize) -> bool {
        if self.topological_choice.is_some() {
            return true;
        }
        let direction = match kind {
            MouseEventKind::ScrollUp => Direction::Up,
            MouseEventKind::ScrollDown => Direction::Down,
            MouseEventKind::ScrollLeft => Direction::Left,
            MouseEventKind::ScrollRight => Direction::Right,
            _ => return false,
        };
        if modifiers.contains(KeyModifiers::SHIFT) {
            self.navigate(direction, false);
        } else {
            self.pan(direction, distance.max(1));
        }
        true
    }

    pub(crate) fn draw(&mut self, frame: &mut Frame<'_>, graph: Option<&HistoryGraph>) {
        let overlay_selected = self
            .count_anchor
            .and_then(|id| self.overview.as_ref()?.nodes.iter().position(|node| node.id == id))
            .or(self.selected);
        if self.overlay.as_ref().map(|overlay| overlay.selected)
            != self
                .overview
                .as_ref()
                .and_then(|overview| overlay_selected.and_then(|selected| overview.nodes.get(selected)))
                .map(|node| node.commit)
            && let (Some(graph), Some(selected)) = (graph, overlay_selected)
        {
            self.overlay = self
                .overview
                .as_ref()
                .map(|overview| Overlay::new(graph, overview, selected));
        }
        let [mut body, footer] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
        frame.render_widget(Clear, frame.area());
        let notice = self.notice.clone();
        let notice_area = notice
            .as_ref()
            .and_then(|notice| notice_area(notice, body, body.y, body.bottom()));
        if let Some(notice_area) = notice_area {
            body.height = notice_area.y.saturating_sub(body.y);
        }
        let Some(overview) = self.overview.as_ref() else {
            frame.render_widget(Paragraph::new("ref-tree overview unavailable"), body);
            return;
        };
        if self.placed.is_none() {
            self.placed = Some(place_rail(overview, self.overlay.as_ref()));
        }
        let placed = self.placed.as_ref().expect("rail placement was just populated");
        let selected = self.selected;
        let ensure_visible = self.ensure_visible;
        let selected_point = selected.and_then(|selected| placed.nodes.get(selected)).copied();
        let offset = {
            let offset = &mut self.offset;
            offset.page_width = usize::from(body.width);
            offset.page_height = usize::from(body.height);
            offset.max_x = placed.width.saturating_sub(offset.page_width);
            offset.max_y = placed.height.saturating_sub(offset.page_height);
            offset.x = offset.x.min(offset.max_x);
            offset.y = offset.y.min(offset.max_y);
            if ensure_visible && let Some(point) = selected_point {
                ensure_point_visible(offset, point);
            }
            *offset
        };
        self.ensure_visible = false;
        let overview = self.overview.as_ref().expect("the overview was checked");
        if let (Some(graph), Some(overlay)) = (graph, self.overlay.as_mut()) {
            overlay.compute_visible_counts(
                graph,
                overview,
                placed,
                offset.y..offset.y.saturating_add(usize::from(body.height)),
            );
        }
        draw_rail_edges(frame, body, overview, self.overlay.as_ref(), placed, offset);
        let topological_choice = self
            .topological_choice_status()
            .map(|(choice, _)| choice_marker(choice));
        draw_nodes(
            frame,
            body,
            overview,
            self.overlay.as_ref(),
            placed,
            offset,
            selected,
            topological_choice,
            &self.history_commits,
        );
        let footer_text = if let Some((choice, total)) = self.topological_choice_status() {
            format!("ref-tree · choose child {choice}/{total} · h/l cycle · <enter> move · Esc cancel")
        } else if self.edit_expanded {
            let branches = self.selected_local_branches();
            if branches.is_empty() && self.remote_deletions.is_empty() {
                "ref-tree · e edit (no actions)".into()
            } else {
                let mut actions = Vec::new();
                if !branches.is_empty() {
                    actions.push(format!("d delete {}", short_branch_list(&branches)));
                }
                if !self.remote_deletions.is_empty() {
                    actions.push(format!(
                        "r delete on remote {}",
                        short_remote_deletion_list(&self.remote_deletions)
                    ));
                }
                format!("ref-tree · e edit ({})", actions.join(" · "))
            }
        } else {
            let tags = if self.hide_tags { "off" } else { "on" };
            let counts = self.count_anchor.map_or_else(
                || "Space counts:auto".into(),
                |id| format!("Space counts:{}", self.node_label(id)),
            );
            format!(
                "ref-tree · {counts} · g top · G root · T tags:{tags} · J/K topo · mouse pan · Shift+mouse cursor · pages cursor · Shift+pages pan · p/<enter> pin · e edit · t/Esc history"
            )
        };
        frame.render_widget(
            Paragraph::new(footer_text).style(Style::default().add_modifier(Modifier::DIM)),
            footer,
        );
        if let (Some(area), Some(notice)) = (notice_area, notice.as_ref()) {
            render_notice(frame, area, notice);
        }
    }

    fn selected_local_branches(&self) -> Vec<gix::refs::FullName> {
        self.selected
            .and_then(|selected| self.overview.as_ref()?.nodes.get(selected))
            .into_iter()
            .flat_map(|node| &node.decorations)
            .filter(|decoration| decoration.kind == DecorationKind::Local)
            .map(|decoration| {
                let mut name = b"refs/heads/".to_vec();
                name.extend_from_slice(&decoration.name);
                gix::bstr::BString::from(name)
                    .try_into()
                    .expect("local decorations originate from valid local branch names")
            })
            .collect()
    }

    fn selected_pin_target(&self) -> Option<(ObjectId, Vec<DecorationKind>)> {
        let node = self
            .selected
            .and_then(|selected| self.overview.as_ref()?.nodes.get(selected))?;
        let mut kinds = Vec::new();
        for decoration in &node.decorations {
            let Some(kind) = pinnable_kind(decoration.kind) else {
                continue;
            };
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
        (!kinds.is_empty()).then_some((node.id, kinds))
    }

    fn selected_remote_references(&self) -> Vec<gix::refs::FullName> {
        self.selected
            .and_then(|selected| self.overview.as_ref()?.nodes.get(selected))
            .into_iter()
            .flat_map(|node| &node.decorations)
            .filter(|decoration| decoration.kind == DecorationKind::Remote)
            .map(|decoration| {
                let mut name = b"refs/remotes/".to_vec();
                name.extend_from_slice(&decoration.name);
                gix::bstr::BString::from(name)
                    .try_into()
                    .expect("remote decorations originate from valid remote-tracking names")
            })
            .collect()
    }

    pub(crate) fn set_remote_deletions(&mut self, deletions: Vec<RemoteDeletion>) {
        if self.edit_expanded {
            self.remote_deletions = deletions;
        }
    }

    pub(crate) fn select_after_reference_deletion(&mut self, fallback: SelectionFallback) {
        self.selection_after_reference_deletion = Some(fallback);
    }

    fn selected_id(&self) -> Option<ObjectId> {
        self.selected
            .and_then(|selected| self.overview.as_ref()?.nodes.get(selected))
            .map(|node| node.id)
    }

    fn reference_deletion_fallback(&self) -> Option<SelectionFallback> {
        let overview = self.overview.as_ref()?;
        let selected = self.selected?;
        let temporary;
        let placed = match self.placed.as_ref() {
            Some(placed) => placed,
            None => {
                temporary = place_rail(overview, self.overlay.as_ref());
                &temporary
            }
        };
        let mut rows: Vec<_> = overview
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (placed.nodes[index].y, node.id))
            .collect();
        rows.sort_by_key(|(row, _)| *row);
        let position = rows.iter().position(|(_, id)| *id == overview.nodes[selected].id)?;
        let candidates = rows[position + 1..]
            .iter()
            .map(|(_, id)| *id)
            .chain(rows[..position].iter().rev().map(|(_, id)| *id))
            .collect();
        Some(SelectionFallback {
            selected: overview.nodes[selected].id,
            candidates,
        })
    }

    fn toggle_count_anchor(&mut self) {
        let Some(selected) = self.selected_id() else { return };
        let previous = self.count_anchor.unwrap_or(selected);
        self.count_anchor = (self.count_anchor != Some(selected)).then_some(selected);
        let current = self.count_anchor.unwrap_or(selected);
        if previous != current {
            self.overlay = None;
            self.placed = None;
        }
    }

    fn node_label(&self, id: ObjectId) -> String {
        self.overview
            .as_ref()
            .and_then(|overview| overview.nodes.iter().find(|node| node.id == id))
            .map(node_label)
            .filter(|label| !label.is_empty())
            .unwrap_or_else(|| id.to_hex_with_len(7).to_string())
    }

    fn navigate(&mut self, direction: Direction, topological: bool) {
        if topological {
            self.start_topological_navigation(direction);
            return;
        }
        let Some(selected) = self.selected else { return };
        let next = self
            .placed
            .as_ref()
            .and_then(|placed| nearest(&placed.nodes, selected, direction));
        if let Some(next) = next {
            self.selected = Some(next);
            self.selection_changed();
            self.ensure_visible = true;
        }
    }

    fn start_topological_navigation(&mut self, direction: Direction) {
        let Some(node) = self
            .overview
            .as_ref()
            .and_then(|overview| overview.nodes.get(self.selected?))
        else {
            return;
        };
        let target = match direction {
            Direction::Down => node.parent,
            Direction::Up if node.children.len() > 1 => {
                self.topological_choice = Some(0);
                self.ensure_visible = true;
                return;
            }
            Direction::Up => node.children.first().copied(),
            Direction::Left | Direction::Right => None,
        };
        if let Some(target) = target {
            self.selected = Some(target);
            self.selection_changed();
            self.ensure_visible = true;
        }
    }

    fn cycle_topological_choice(&mut self, right: bool) {
        let Some(choice) = self.topological_choice else {
            return;
        };
        let total = self
            .overview
            .as_ref()
            .and_then(|overview| overview.nodes.get(self.selected?))
            .map_or(0, |node| node.children.len());
        if total == 0 {
            self.topological_choice = None;
            return;
        }
        self.topological_choice = Some(if right {
            (choice + 1) % total
        } else {
            (choice + total - 1) % total
        });
    }

    fn submit_topological_choice(&mut self) {
        let Some(choice) = self.topological_choice.take() else {
            return;
        };
        let Some(target) = self
            .overview
            .as_ref()
            .and_then(|overview| overview.nodes.get(self.selected?))
            .and_then(|node| node.children.get(choice))
            .copied()
        else {
            return;
        };
        self.selected = Some(target);
        self.selection_changed();
        self.ensure_visible = true;
    }

    fn topological_choice_status(&self) -> Option<(usize, usize)> {
        let choice = self.topological_choice?;
        let children = &self.overview.as_ref()?.nodes.get(self.selected?)?.children;
        children.get(choice)?;
        Some((choice + 1, children.len()))
    }

    fn jump_to_root(&mut self) {
        let (Some(overview), Some(mut selected)) = (self.overview.as_ref(), self.selected) else {
            return;
        };
        while let Some(parent) = overview.nodes[selected].parent {
            selected = parent;
        }
        self.selected = Some(selected);
        self.selection_changed();
        self.ensure_visible = true;
    }

    fn jump_to_top(&mut self) {
        let Some(overview) = self.overview.as_ref() else {
            return;
        };
        let Some(mut selected) = overview.roots.first().copied() else {
            return;
        };
        while let Some(child) = overview.nodes[selected].children.first().copied() {
            selected = child;
        }
        self.selected = Some(selected);
        self.selection_changed();
        self.ensure_visible = true;
    }

    fn toggle_tags(&mut self) {
        let selected = self
            .selected
            .and_then(|selected| self.overview.as_ref()?.nodes.get(selected))
            .map(|node| node.id);
        let Some(alternate) = self.alternate_overview.as_mut() else {
            return;
        };
        let overview = self.overview.get_or_insert_with(Overview::default);
        std::mem::swap(overview, alternate);
        self.selected = selected
            .and_then(|id| overview.nodes.iter().position(|node| node.id == id))
            .or_else(|| overview.nodes.iter().position(|node| node.is_head))
            .or_else(|| overview.nodes.iter().position(|node| node.raw_tip))
            .or((!overview.nodes.is_empty()).then_some(0));
        self.hide_tags = !self.hide_tags;
        if self
            .count_anchor
            .is_some_and(|id| !overview.nodes.iter().any(|node| node.id == id))
        {
            self.count_anchor = None;
        }
        self.topological_choice = None;
        self.overlay = None;
        self.placed = None;
        self.ensure_visible = true;
    }

    fn selection_changed(&mut self) {
        self.topological_choice = None;
        if self.count_anchor.is_none() {
            self.overlay = None;
            self.placed = None;
        }
    }

    fn pan(&mut self, direction: Direction, amount: usize) {
        let offset = &mut self.offset;
        match direction {
            Direction::Up => offset.y = offset.y.saturating_sub(amount),
            Direction::Down => offset.y = offset.y.saturating_add(amount).min(offset.max_y),
            Direction::Left => offset.x = offset.x.saturating_sub(amount),
            Direction::Right => offset.x = offset.x.saturating_add(amount).min(offset.max_x),
        }
        self.ensure_visible = false;
    }

    fn page(&mut self, direction: Direction, amount: usize) {
        let (Some(overview), Some(selected)) = (self.overview.as_ref(), self.selected) else {
            return;
        };
        let temporary;
        let placed = match self.placed.as_ref() {
            Some(placed) => placed,
            None => {
                temporary = place_rail(overview, self.overlay.as_ref());
                &temporary
            }
        };
        let source = placed.nodes[selected];
        let target = match direction {
            Direction::Up => source.y.saturating_sub(amount),
            Direction::Down => source.y.saturating_add(amount),
            Direction::Left | Direction::Right => return,
        };
        let next = placed
            .nodes
            .iter()
            .copied()
            .enumerate()
            .filter(|(index, point)| {
                *index != selected
                    && match direction {
                        Direction::Up => point.y < source.y,
                        Direction::Down => point.y > source.y,
                        Direction::Left | Direction::Right => false,
                    }
            })
            .min_by_key(|(index, point)| (point.y.abs_diff(target), point.x.abs_diff(source.x), *index))
            .map(|(index, _)| index);
        if let Some(next) = next {
            self.selected = Some(next);
            self.selection_changed();
            self.ensure_visible = true;
        }
    }

    fn page_or_pan(&mut self, direction: Direction, modifiers: KeyModifiers) {
        let amount = self.offset().page_height.max(1);
        if modifiers.contains(KeyModifiers::SHIFT) {
            self.pan(direction, amount);
        } else {
            self.page(direction, amount);
        }
    }

    fn offset(&self) -> &Offset {
        &self.offset
    }
}

fn pinnable_kind(kind: DecorationKind) -> Option<DecorationKind> {
    match kind {
        DecorationKind::Local
        | DecorationKind::CurrentWorktreeBranch
        | DecorationKind::WorktreeBranch
        | DecorationKind::HeadPinBranch => Some(DecorationKind::Local),
        DecorationKind::Tag | DecorationKind::AnnotatedTag => Some(DecorationKind::Tag),
        DecorationKind::Remote | DecorationKind::Review => Some(kind),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn pin_references(
    repository: &gix::Repository,
    id: ObjectId,
    kinds: &[DecorationKind],
) -> anyhow::Result<Vec<crate::history::Pin>> {
    pin_references_reporting(repository, id, kinds).map(|(pins, _changes)| pins)
}

pub(crate) fn pin_references_reporting(
    repository: &gix::Repository,
    id: ObjectId,
    kinds: &[DecorationKind],
) -> anyhow::Result<(Vec<crate::history::Pin>, Vec<crate::edit::undo::RefChange>)> {
    let mut names = Vec::new();
    for reference in repository
        .references()
        .context("could not open references while pinning the ref-tree selection")?
        .all()
        .context("could not iterate references while pinning the ref-tree selection")?
    {
        let mut reference = match reference {
            Ok(reference) => reference,
            Err(err) if crate::history::is_missing_ref(&*err) => continue,
            Err(err) => return Err(anyhow::anyhow!("could not read reference to pin: {err}")),
        };
        let name = reference.name().to_owned();
        let Some(kind) = pinnable_kind(crate::history::decoration_kind(name.as_bstr())) else {
            continue;
        };
        if !kinds.contains(&kind) {
            continue;
        }
        let Ok(target) = reference.peel_to_id() else {
            continue;
        };
        if target.as_ref() != id {
            continue;
        }
        names.push(name);
    }
    names.sort();
    names.dedup();
    let mut pins = Vec::new();
    let mut changes = Vec::new();
    for name in names {
        let (pin, created) = crate::edit::time_travel::create_or_reuse_pin(
            repository,
            gix::refs::Target::Symbolic(name),
            id,
            "tix ref-tree",
        )?;
        if created {
            changes.push(crate::edit::undo::RefChange {
                name: pin.name.clone(),
                before: crate::edit::undo::State::Missing,
                after: crate::edit::undo::State::Symbolic(
                    pin.target.try_name().expect("a ref-tree pin is symbolic").to_owned(),
                ),
            });
        }
        pins.push(pin);
    }
    Ok((pins, changes))
}

impl Overview {
    fn new(graph: &HistoryGraph, refs: &RefSnapshot, decorations: &Decorations, show_tags: bool) -> Self {
        let mut labels = HashMap::<CommitIndex, Vec<Decoration>>::new();
        let mut heads = HashSet::new();
        let mut anchors = HashSet::new();
        let mut detached_worktrees = HashSet::new();
        for (id, decorations) in decorations {
            let Some(index) = graph.index(*id) else { continue };
            for decoration in decorations {
                if !show_tags && matches!(decoration.kind, DecorationKind::Tag | DecorationKind::AnnotatedTag) {
                    continue;
                }
                if decoration.kind == DecorationKind::Head {
                    heads.insert(index);
                    anchors.insert(index);
                } else if decoration.kind == DecorationKind::Pin {
                    continue;
                } else if decoration.kind != DecorationKind::Special {
                    labels.entry(index).or_default().push(decoration.clone());
                    anchors.insert(index);
                }
            }
        }
        for worktree in refs
            .worktrees
            .iter()
            .filter(|worktree| worktree.is_current && worktree.is_detached)
        {
            let Some(index) = graph.index(worktree.id) else {
                continue;
            };
            detached_worktrees.insert(index);
            anchors.insert(index);
        }
        let pin_tips: HashSet<_> = refs.pins.iter().map(|pin| pin.id).collect();
        let raw: HashSet<_> = refs
            .view_tips
            .iter()
            .chain(&refs.hidden_tips)
            .filter(|id| !pin_tips.contains(*id))
            .filter_map(|id| graph.index(*id))
            .inspect(|index| {
                anchors.insert(*index);
            })
            .collect();
        if anchors.is_empty() {
            return Overview::default();
        }
        for decorations in labels.values_mut() {
            decorations.sort_by(|a, b| a.name.cmp(&b.name));
            decorations.dedup();
        }
        let mut included = HashSet::new();
        for anchor in anchors.iter().copied() {
            let mut current = Some(anchor);
            while let Some(index) = current {
                if !included.insert(index) {
                    break;
                }
                current = graph.parents(index).first().copied();
            }
        }
        let mut children = vec![Vec::new(); graph.commit_count()];
        for child in included.iter().copied() {
            if let Some(parent) = graph
                .parents(child)
                .first()
                .copied()
                .filter(|parent| included.contains(parent))
            {
                children[parent.as_usize()].push(child);
            }
        }
        let structural: HashSet<_> = included
            .iter()
            .copied()
            .filter(|index| {
                anchors.contains(index)
                    || children[index.as_usize()].len() != 1
                    || graph
                        .parents(*index)
                        .first()
                        .is_none_or(|parent| !included.contains(parent))
            })
            .collect();
        let mut structural_order: Vec<_> = structural.iter().copied().collect();
        structural_order.sort_by_key(|index| index.as_usize());
        let by_commit: HashMap<_, _> = structural_order
            .iter()
            .enumerate()
            .map(|(node, commit)| (*commit, node))
            .collect();
        let mut nodes: Vec<_> = structural_order
            .iter()
            .copied()
            .map(|commit| {
                let decorations = labels.remove(&commit).unwrap_or_default();
                let sort_key = decorations.first().map_or_else(
                    || graph.id(commit).to_hex().to_string(),
                    |decoration| decoration.name.to_str_lossy().into_owned(),
                );
                Node {
                    commit,
                    id: graph.id(commit),
                    parent: None,
                    children: Vec::new(),
                    decorations,
                    is_head: heads.contains(&commit),
                    is_anchor: anchors.contains(&commit),
                    is_detached_worktree: detached_worktrees.contains(&commit),
                    raw_tip: raw.contains(&commit),
                    sort_key,
                }
            })
            .collect();
        let mut edges = Vec::new();
        let mut roots = Vec::new();
        for child in 0..nodes.len() {
            let mut hidden = Vec::new();
            let mut parent = graph.parents(nodes[child].commit).first().copied();
            while let Some(index) = parent.filter(|index| included.contains(index)) {
                if let Some(parent_node) = by_commit.get(&index).copied() {
                    nodes[child].parent = Some(parent_node);
                    nodes[parent_node].children.push(child);
                    edges.push(Edge {
                        child,
                        parent: parent_node,
                        hidden,
                    });
                    break;
                }
                hidden.push(index);
                parent = graph.parents(index).first().copied();
            }
            if nodes[child].parent.is_none() {
                roots.push(child);
            }
        }
        fn subtree_key(node: usize, nodes: &mut [Node]) -> String {
            let children = nodes[node].children.clone();
            let mut key = nodes[node].sort_key.clone();
            for child in children {
                key = key.min(subtree_key(child, nodes));
            }
            nodes[node].sort_key.clone_from(&key);
            key
        }
        for root in roots.iter().copied() {
            subtree_key(root, &mut nodes);
        }
        for node in 0..nodes.len() {
            let keys: Vec<_> = nodes[node]
                .children
                .iter()
                .map(|child| (*child, nodes[*child].sort_key.clone(), graph.id(nodes[*child].commit)))
                .collect();
            let mut keys = keys;
            keys.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)));
            nodes[node].children = keys.into_iter().map(|(child, _, _)| child).collect();
        }
        fn contains_head(node: usize, nodes: &[Node]) -> bool {
            nodes[node].is_head || nodes[node].children.iter().any(|child| contains_head(*child, nodes))
        }
        roots.sort_by(|a, b| {
            (!contains_head(*a, &nodes))
                .cmp(&(!contains_head(*b, &nodes)))
                .then(nodes[*a].sort_key.cmp(&nodes[*b].sort_key))
        });
        Overview {
            nodes,
            edges,
            roots,
            by_commit,
            commit_count: graph.commit_count(),
        }
    }
}

impl Overlay {
    fn new(graph: &HistoryGraph, overview: &Overview, selected: usize) -> Self {
        let selected_commit = overview.nodes[selected].commit;
        let mut reachable = vec![false; graph.commit_count()];
        let mut pending = vec![selected_commit];
        let mut total = 0;
        while let Some(index) = pending.pop() {
            if std::mem::replace(&mut reachable[index.as_usize()], true) {
                continue;
            }
            total += 1;
            pending.extend_from_slice(graph.parents(index));
        }
        let mut first_parent = vec![false; graph.commit_count()];
        let mut current = Some(selected_commit);
        while let Some(index) = current {
            first_parent[index.as_usize()] = true;
            current = graph.parents(index).first().copied();
        }
        let mut counts = vec![None; overview.nodes.len()];
        counts[selected] = Some(total);
        let boundaries = overview
            .edges
            .iter()
            .map(|edge| {
                (!reachable[overview.nodes[edge.child].commit.as_usize()]).then(|| {
                    edge.hidden
                        .iter()
                        .copied()
                        .find(|index| reachable[index.as_usize()])
                        .map(|index| graph.id(index))
                })?
            })
            .collect();
        Overlay {
            selected: selected_commit,
            reachable,
            first_parent,
            counts,
            boundaries,
            seen: vec![0; graph.commit_count()],
            stamp: 0,
        }
    }

    fn compute_visible_counts(
        &mut self,
        graph: &HistoryGraph,
        overview: &Overview,
        placed: &Placed,
        rows: std::ops::Range<usize>,
    ) {
        for (node, value) in overview.nodes.iter().enumerate() {
            if !value.is_anchor || self.counts[node].is_some() || !rows.contains(&placed.nodes[node].y) {
                continue;
            }
            self.stamp = self.stamp.wrapping_add(1);
            if self.stamp == 0 {
                self.seen.fill(0);
                self.stamp = 1;
            }
            let mut count = 0;
            let mut pending = vec![value.commit];
            while let Some(index) = pending.pop() {
                if self.reachable[index.as_usize()]
                    || std::mem::replace(&mut self.seen[index.as_usize()], self.stamp) == self.stamp
                {
                    continue;
                }
                count += 1;
                pending.extend_from_slice(graph.parents(index));
            }
            self.counts[node] = Some(count);
        }
    }
}

fn place_rail(overview: &Overview, overlay: Option<&Overlay>) -> Placed {
    struct Item {
        id: ObjectId,
        parent: Option<ObjectId>,
        kind: RailRowKind,
        marker: char,
    }

    let mut edge_by_child = vec![None; overview.nodes.len()];
    for (edge, value) in overview.edges.iter().enumerate() {
        edge_by_child[value.child] = Some(edge);
    }
    fn collect(
        node: usize,
        parent: Option<ObjectId>,
        overview: &Overview,
        overlay: Option<&Overlay>,
        edge_by_child: &[Option<usize>],
        out: &mut Vec<Item>,
    ) {
        for child in overview.nodes[node].children.iter().copied() {
            let edge = edge_by_child[child].expect("non-root nodes have an edge");
            let boundary = overlay.and_then(|overlay| overlay.boundaries[edge]);
            collect(
                child,
                Some(boundary.unwrap_or(overview.nodes[node].id)),
                overview,
                overlay,
                edge_by_child,
                out,
            );
            if let Some(id) = boundary {
                out.push(Item {
                    id,
                    parent: Some(overview.nodes[node].id),
                    kind: RailRowKind::Boundary(edge),
                    marker: '●',
                });
            }
        }
        out.push(Item {
            id: overview.nodes[node].id,
            parent,
            kind: RailRowKind::Node(node),
            marker: if overview.nodes[node].is_head { '@' } else { '●' },
        });
    }

    let mut items = Vec::new();
    for root in overview.roots.iter().copied() {
        collect(root, None, overview, overlay, &edge_by_child, &mut items);
    }
    let mut state = LaneState::default();
    let mut placed = Placed {
        nodes: vec![Point::default(); overview.nodes.len()],
        boundaries: vec![None; overview.edges.len()],
        ..Placed::default()
    };
    for item in items {
        let node_lane = state.node_line(item.id, item.marker);
        let mut transition = String::new();
        state.advance_ids(item.id, item.parent, Some(&mut transition), item.marker);
        let rounded_transition = transition.contains('─');
        let lane = if rounded_transition {
            node_lane
        } else {
            std::mem::take(&mut transition)
        };
        let x = lane
            .chars()
            .position(|symbol| symbol == item.marker)
            .expect("a rendered lane contains its node marker");
        let y = placed.rail_rows.len();
        match item.kind {
            RailRowKind::Node(node) => placed.nodes[node] = Point { x, y },
            RailRowKind::Boundary(edge) => placed.boundaries[edge] = Some(Point { x, y }),
            RailRowKind::NodeConnector | RailRowKind::BoundaryConnector => {
                unreachable!("items are always node rows")
            }
        }
        placed.rail_width = placed.rail_width.max(lane.chars().count());
        let edge = match item.kind {
            RailRowKind::Node(node) => edge_by_child[node],
            RailRowKind::Boundary(edge) => Some(edge),
            RailRowKind::NodeConnector | RailRowKind::BoundaryConnector => unreachable!("items are node rows"),
        };
        placed.rail_rows.push(RailRow {
            lane,
            kind: item.kind,
            edge,
        });
        if rounded_transition {
            let mut connector: Vec<_> = transition.chars().collect();
            connector[x] = if x > 0 && connector[x - 1] == '─' {
                '╯'
            } else if connector.get(x + 1) == Some(&'─') {
                '╰'
            } else {
                '│'
            };
            let kind = match item.kind {
                RailRowKind::Node(_) => RailRowKind::NodeConnector,
                RailRowKind::Boundary(_) => RailRowKind::BoundaryConnector,
                RailRowKind::NodeConnector | RailRowKind::BoundaryConnector => unreachable!("items are node rows"),
            };
            let lane: String = connector.into_iter().collect();
            placed.rail_width = placed.rail_width.max(lane.chars().count());
            placed.rail_rows.push(RailRow { lane, kind, edge });
        }
    }
    placed.height = placed.rail_rows.len();
    let label_width = overview
        .nodes
        .iter()
        .map(|node| rail_label(node, Some(overview.commit_count)).chars().count())
        .max()
        .unwrap_or_default();
    placed.width = placed.rail_width.saturating_add(label_width).max(1);
    placed
}

fn draw_rail_edges(
    frame: &mut Frame<'_>,
    area: Rect,
    overview: &Overview,
    overlay: Option<&Overlay>,
    placed: &Placed,
    offset: Offset,
) {
    for (y, row) in placed.rail_rows.iter().enumerate() {
        if y < offset.y || y >= offset.y.saturating_add(usize::from(area.height)) {
            continue;
        }
        let style = match row.kind {
            RailRowKind::Boundary(_) | RailRowKind::BoundaryConnector => Style::default().add_modifier(Modifier::DIM),
            RailRowKind::Node(_) | RailRowKind::NodeConnector => row
                .edge
                .map(|edge| &overview.edges[edge])
                .map_or_else(Style::default, |edge| edge_style(overview, overlay, edge)),
        };
        draw_text(frame, area, offset, Point { x: 0, y }, &row.lane, style);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "drawing receives detached layout and style state"
)]
fn draw_nodes(
    frame: &mut Frame<'_>,
    area: Rect,
    overview: &Overview,
    overlay: Option<&Overlay>,
    placed: &Placed,
    offset: Offset,
    selected: Option<usize>,
    topological_choice: Option<char>,
    history_commits: &HashSet<ObjectId>,
) {
    for (index, node) in overview.nodes.iter().enumerate() {
        let point = placed.nodes[index];
        if point.y < offset.y
            || point.y >= offset.y.saturating_add(usize::from(area.height))
            || point.x >= offset.x.saturating_add(usize::from(area.width))
        {
            continue;
        }
        let count = overlay.and_then(|overlay| overlay.counts[index]);
        let text = rail_label(node, count);
        let mut style = if node.is_anchor && history_commits.contains(&node.id) {
            Style::default().fg(Color::Cyan)
        } else if node.decorations.iter().any(|decoration| {
            matches!(
                decoration.kind,
                DecorationKind::WorktreeBranch | DecorationKind::WorktreeDetached
            )
        }) {
            Style::default().fg(Color::Green)
        } else {
            node.decorations.first().map_or_else(
                || Style::default().fg(Color::LightBlue),
                |decoration| decoration_style(decoration.kind),
            )
        };
        if selected == Some(index) {
            style = style.add_modifier(Modifier::REVERSED | Modifier::BOLD);
        } else if let Some(overlay) = overlay {
            let commit = node.commit.as_usize();
            if overlay.first_parent[commit] {
                style = style.add_modifier(Modifier::BOLD);
            } else if overlay.reachable[commit] {
                style = style.add_modifier(Modifier::DIM);
            }
        }
        let marker = if selected == Some(index) {
            topological_choice.unwrap_or(if node.is_head { '@' } else { '●' })
        } else if node.is_head {
            '@'
        } else {
            '●'
        };
        put(frame, area, offset, point, marker, style);
        draw_text(
            frame,
            area,
            offset,
            Point {
                x: placed.rail_width,
                y: point.y,
            },
            &text,
            style,
        );
        if let Some(pin) = text.chars().position(|symbol| symbol == '📌') {
            put(
                frame,
                area,
                offset,
                Point {
                    x: placed.rail_width + pin,
                    y: point.y,
                },
                '📌',
                decoration_style(DecorationKind::Pin),
            );
        }
    }
}

fn choice_marker(choice: usize) -> char {
    u32::try_from(choice)
        .ok()
        .and_then(|digit| char::from_digit(digit, 10))
        .unwrap_or('+')
}

fn rail_label(node: &Node, count: Option<usize>) -> String {
    let mut out = count.map_or_else(String::new, |count| format!("{count}•"));
    let suffix = node_label(node);
    if !out.is_empty() && !suffix.is_empty() {
        out.push(' ');
    }
    out.push_str(&suffix);
    if node.is_detached_worktree {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push('📌');
    }
    out
}

fn node_label(node: &Node) -> String {
    let labels: Vec<_> = node
        .decorations
        .iter()
        .map(|decoration| {
            let name = decoration.name.to_str_lossy();
            match decoration.kind {
                DecorationKind::CurrentWorktreeBranch | DecorationKind::CurrentWorktreeDetached => {
                    format!("@{name}")
                }
                DecorationKind::WorktreeBranch | DecorationKind::WorktreeDetached => format!("{name}@"),
                DecorationKind::HeadPinBranch => format!("★{name}"),
                _ => name.into_owned(),
            }
        })
        .collect();
    if !labels.is_empty() {
        labels.join(", ")
    } else if node.raw_tip {
        node.id.to_hex_with_len(7).to_string()
    } else if node.parent.is_none() {
        "<root>".into()
    } else {
        String::new()
    }
}

fn short_branch_list(names: &[gix::refs::FullName]) -> String {
    names
        .iter()
        .map(|name| name.shorten().to_str_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(", ")
}

fn short_remote_deletion_list(groups: &[RemoteDeletion]) -> String {
    groups
        .iter()
        .flat_map(|group| {
            group
                .references
                .iter()
                .map(|reference| format!("{}/{}", group.remote.to_str_lossy(), reference.shorten().to_str_lossy()))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn resolve_remote_deletions(
    repository: &gix::Repository,
    tracking_references: Vec<gix::refs::FullName>,
) -> Vec<RemoteDeletion> {
    let mut groups = BTreeMap::<gix::bstr::BString, Vec<gix::refs::FullName>>::new();
    for tracking in tracking_references {
        match repository.upstream_branch_and_remote_for_tracking_branch(tracking.as_ref()) {
            Ok(Some((upstream, remote))) => {
                let Some(name) = remote.name() else { continue };
                groups.entry(name.as_bstr().to_owned()).or_default().push(upstream);
            }
            Ok(None) => {}
            Err(err) => tracing::warn!(name = %tracking, error = %err, "remote reference is not editable"),
        }
    }
    groups
        .into_iter()
        .map(|(remote, mut references)| {
            references.sort();
            references.dedup();
            RemoteDeletion { remote, references }
        })
        .collect()
}

fn edge_style(overview: &Overview, overlay: Option<&Overlay>, edge: &Edge) -> Style {
    let Some(overlay) = overlay else {
        return Style::default();
    };
    let child = overview.nodes[edge.child].commit.as_usize();
    let parent = overview.nodes[edge.parent].commit.as_usize();
    if overlay.first_parent[child] && overlay.first_parent[parent] {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else if overlay.reachable[child] {
        Style::default().add_modifier(Modifier::DIM)
    } else {
        Style::default()
    }
}

pub(crate) fn render_full(
    repository: &gix::Repository,
    revisions: &[OsString],
    hidden: &[OsString],
    show_tags: bool,
    unicode: bool,
) -> anyhow::Result<String> {
    let hidden_refs = if hidden.is_empty() {
        HashMap::new()
    } else {
        crate::history::referenced_refs(repository, hidden)?
    };
    let mut visible_revisions = Vec::with_capacity(revisions.len());
    for revision in revisions {
        let referenced = crate::history::referenced_refs(repository, std::slice::from_ref(revision))?;
        if referenced.keys().all(|name| !hidden_refs.contains_key(name)) {
            visible_revisions.push(revision.clone());
        }
    }
    let mut refs = crate::history::snapshot(repository, &visible_revisions, hidden, true)?;
    let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(
        crate::history::Authors::default(),
    ));
    let mut graph = None;
    crate::history::load(
        repository,
        &visible_revisions,
        hidden,
        true,
        &authors,
        &AtomicBool::new(false),
        |event| {
            if let crate::history::Event::Complete(value) = event {
                graph = Some(value);
            }
            true
        },
    )?;
    let graph = graph.ok_or_else(|| anyhow::anyhow!("history traversal did not produce a graph"))?;
    let hidden_refs = hidden_refs.into_keys().collect();
    let decorations = crate::history::decorations_excluding(repository, &refs.pins, &refs.worktrees, &hidden_refs)?;
    refs.hidden_tips.clear();
    let overview = Overview::new(&graph, &refs, &decorations, show_tags);
    let labels = overview
        .nodes
        .iter()
        .filter(|node| node.raw_tip && node.decorations.is_empty())
        .map(|node| Ok((node.id, crate::change_id::display(repository, node.id, 7)?)))
        .collect::<anyhow::Result<HashMap<_, _>>>()?;
    Ok(render_overview(&overview, unicode, &labels))
}

fn render_overview(overview: &Overview, unicode: bool, commit_labels: &HashMap<ObjectId, String>) -> String {
    if overview.nodes.is_empty() {
        return String::new();
    }
    let placed = place_rail(overview, None);
    let mut out = String::new();
    for row in &placed.rail_rows {
        let node = match row.kind {
            RailRowKind::Node(node) => Some(node),
            RailRowKind::NodeConnector => None,
            RailRowKind::Boundary(_) | RailRowKind::BoundaryConnector => continue,
        };
        let lane = if unicode {
            row.lane.clone()
        } else {
            row.lane
                .chars()
                .map(|symbol| match symbol {
                    '●' => 'o',
                    '│' => '|',
                    '─' => '-',
                    ' ' | '@' => symbol,
                    _ => '+',
                })
                .collect()
        };
        let width = lane.chars().count();
        out.push_str(&lane);
        out.extend(std::iter::repeat_n(' ', placed.rail_width.saturating_sub(width)));
        if let Some(node) = node {
            let node = &overview.nodes[node];
            let label = commit_labels
                .get(&node.id)
                .cloned()
                .unwrap_or_else(|| rail_label(node, None));
            if unicode {
                out.push_str(&label);
            } else {
                out.push_str(&label.replace('📌', "[pin]"));
            }
        }
        while out.ends_with(' ') {
            out.pop();
        }
        out.push('\n');
    }
    out
}

fn draw_text(frame: &mut Frame<'_>, area: Rect, offset: Offset, point: Point, text: &str, style: Style) {
    for (index, symbol) in text.chars().enumerate() {
        put(
            frame,
            area,
            offset,
            Point {
                x: point.x + index,
                y: point.y,
            },
            symbol,
            style,
        );
    }
}

fn put(frame: &mut Frame<'_>, area: Rect, offset: Offset, point: Point, symbol: char, style: Style) {
    let Some(x) = point.x.checked_sub(offset.x) else { return };
    let Some(y) = point.y.checked_sub(offset.y) else { return };
    if x >= usize::from(area.width) || y >= usize::from(area.height) {
        return;
    }
    frame.buffer_mut()[(area.x + x as u16, area.y + y as u16)]
        .set_char(symbol)
        .set_style(style);
}

fn ensure_point_visible(offset: &mut Offset, point: Point) {
    if point.x < offset.x {
        offset.x = point.x;
    } else if point.x >= offset.x.saturating_add(offset.page_width) {
        offset.x = point.x.saturating_sub(offset.page_width.saturating_sub(1));
    }
    if point.y < offset.y {
        offset.y = point.y;
    } else if point.y >= offset.y.saturating_add(offset.page_height) {
        offset.y = point.y.saturating_sub(offset.page_height.saturating_sub(1));
    }
    offset.x = offset.x.min(offset.max_x);
    offset.y = offset.y.min(offset.max_y);
}

fn nearest(points: &[Point], selected: usize, direction: Direction) -> Option<usize> {
    let source = *points.get(selected)?;
    points
        .iter()
        .copied()
        .enumerate()
        .filter(|(index, point)| {
            *index != selected
                && match direction {
                    Direction::Up => point.y < source.y,
                    Direction::Down => point.y > source.y,
                    Direction::Left => point.x < source.x,
                    Direction::Right => point.x > source.x,
                }
        })
        .min_by_key(|(index, point)| {
            let dx = point.x.abs_diff(source.x);
            let dy = point.y.abs_diff(source.y);
            let perpendicular = match direction {
                Direction::Up | Direction::Down => dx,
                Direction::Left | Direction::Right => dy,
            };
            (dx * dx + dy * dy, perpendicular, *index)
        })
        .map(|(index, _)| index)
}

fn direction(code: KeyCode) -> Option<Direction> {
    match code {
        KeyCode::Up | KeyCode::Char('k' | 'K') => Some(Direction::Up),
        KeyCode::Down | KeyCode::Char('j' | 'J') => Some(Direction::Down),
        KeyCode::Left | KeyCode::Char('h' | 'H') => Some(Direction::Left),
        KeyCode::Right | KeyCode::Char('l' | 'L') => Some(Direction::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn id(n: u8) -> ObjectId {
        let mut bytes = [0; 20];
        bytes[19] = n;
        ObjectId::Sha1(bytes)
    }

    fn fixture() -> (HistoryGraph, RefSnapshot, Decorations) {
        let graph = HistoryGraph::from_test_commits(&[
            (id(1), vec![]),
            (id(2), vec![id(1)]),
            (id(3), vec![id(2)]),
            (id(4), vec![id(2)]),
            (id(5), vec![id(4)]),
            (id(6), vec![id(3), id(4)]),
        ]);
        let refs = RefSnapshot {
            view: HashMap::new(),
            hidden: HashMap::new(),
            view_tips: vec![id(6), id(5)],
            hidden_tips: Vec::new(),
            pins: Vec::new(),
            head_pin_branch: None,
            worktrees: Vec::new(),
        };
        let decorations = Decorations::from([
            (
                id(6),
                vec![
                    Decoration {
                        name: "main".into(),
                        kind: DecorationKind::Local,
                    },
                    Decoration {
                        name: "HEAD".into(),
                        kind: DecorationKind::Head,
                    },
                ],
            ),
            (
                id(5),
                vec![Decoration {
                    name: "topic".into(),
                    kind: DecorationKind::Local,
                }],
            ),
        ]);
        (graph, refs, decorations)
    }

    #[test]
    fn e_arms_immediate_deletion_of_all_ordinary_local_branches() {
        let (graph, refs, mut decorations) = fixture();
        decorations.get_mut(&id(6)).expect("main is decorated").extend([
            Decoration {
                name: "also-main".into(),
                kind: DecorationKind::Local,
            },
            Decoration {
                name: "checked-out".into(),
                kind: DecorationKind::WorktreeBranch,
            },
            Decoration {
                name: "remembered".into(),
                kind: DecorationKind::HeadPinBranch,
            },
            Decoration {
                name: "origin/main".into(),
                kind: DecorationKind::Remote,
            },
        ]);
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.toggle(), "the ref-tree opens");

        let Input::ResolveRemoteReferences(remote) =
            tree.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE))
        else {
            panic!("e should request remote resolution")
        };
        assert_eq!(remote, ["refs/remotes/origin/main"]);
        assert!(tree.edit_expanded);
        let Input::DeleteLocalBranches { names, .. } =
            tree.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE))
        else {
            panic!("d should submit branch deletion")
        };
        assert_eq!(
            names,
            ["refs/heads/also-main", "refs/heads/main"],
            "d immediately submits every ordinary local branch and no protected decoration"
        );
        assert!(!tree.edit_expanded);
    }

    #[test]
    fn e_r_deletes_every_resolved_remote_reference_without_confirmation() {
        let (graph, refs, mut decorations) = fixture();
        decorations
            .get_mut(&id(6))
            .expect("main is decorated")
            .push(Decoration {
                name: "origin/main".into(),
                kind: DecorationKind::Remote,
            });
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.toggle());
        assert!(matches!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
            Input::ResolveRemoteReferences(_)
        ));
        let groups = vec![RemoteDeletion {
            remote: "origin".into(),
            references: vec![
                "refs/heads/main".try_into().expect("valid"),
                "refs/heads/other".try_into().expect("valid"),
            ],
        }];
        tree.set_remote_deletions(groups.clone());

        assert!(matches!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
            Input::DeleteRemoteReferences { groups: submitted, .. } if submitted == groups
        ));
        assert!(!tree.edit_expanded);
    }

    #[test]
    fn resolves_only_remote_tracking_refs_with_an_upstream_mapping() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        for args in [
            &["config", "remote.origin.url", "."][..],
            &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"][..],
        ] {
            assert!(
                std::process::Command::new("git")
                    .arg("-C")
                    .arg(fixture.path())
                    .args(args)
                    .status()?
                    .success()
            );
        }
        let repository = crate::test_repository::open(fixture.path())?;
        let groups = resolve_remote_deletions(
            &repository,
            vec![
                "refs/remotes/origin/topic".try_into()?,
                "refs/remotes/missing/topic".try_into()?,
            ],
        );
        assert_eq!(
            groups,
            [RemoteDeletion {
                remote: "origin".into(),
                references: vec!["refs/heads/topic".try_into()?],
            }],
            "only uniquely reverse-mapped remote references are editable"
        );
        Ok(())
    }

    #[test]
    fn remote_deletion_continues_after_a_failed_remote() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let git = |args: &[&str]| -> gix_testtools::Result<()> {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(fixture.path())
                .args(args)
                .status()?;
            assert!(status.success(), "git {} succeeds", args.join(" "));
            Ok(())
        };
        git(&["remote", "add", "z-working", "."])?;
        git(&["fetch", "z-working", "topic"])?;
        git(&["remote", "add", "a-broken", "./missing"])?;

        let outcome = crate::push_remote_deletions(
            fixture.path(),
            &[
                RemoteDeletion {
                    remote: "a-broken".into(),
                    references: vec!["refs/heads/topic".try_into()?],
                },
                RemoteDeletion {
                    remote: "z-working".into(),
                    references: vec!["refs/heads/topic".try_into()?],
                },
            ],
        );
        assert_eq!(outcome.deleted, 1, "the later working remote is still attempted");
        assert_eq!(outcome.failures.len(), 1, "the failed remote is reported");
        let repository = crate::test_repository::open(fixture.path())?;
        assert!(repository.try_find_reference("refs/heads/topic")?.is_none());
        assert!(
            repository.try_find_reference("refs/remotes/z-working/topic")?.is_none(),
            "git push removes the corresponding tracking reference"
        );
        Ok(())
    }

    #[test]
    fn escape_cancels_the_edit_prefix_without_leaving_the_ref_tree() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.toggle(), "the ref-tree opens");

        tree.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        tree.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(tree.is_active(), "Escape cancels the armed prefix first");
        assert!(!tree.edit_expanded);
    }

    #[test]
    fn protected_branches_leave_no_delete_action() {
        let (graph, refs, mut decorations) = fixture();
        decorations.get_mut(&id(5)).expect("topic is decorated")[0].kind = DecorationKind::HeadPinBranch;
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        tree.selected = tree.overview.as_ref().and_then(|overview| {
            graph
                .index(id(5))
                .and_then(|index| overview.by_commit.get(&index).copied())
        });

        tree.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE)),
            Input::Handled
        );
        assert_eq!(
            tree.notice.as_ref().map(|notice| notice.text.as_str()),
            Some("no deletable local branches at the selected node")
        );
    }

    #[test]
    fn first_parent_shape_uses_all_parent_reachability_for_counts_and_boundaries() {
        let (graph, refs, decorations) = fixture();
        let overview = Overview::new(&graph, &refs, &decorations, true);
        let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
        let topic = overview.by_commit[&graph.index(id(5)).expect("topic exists")];
        let fork = overview.by_commit[&graph.index(id(2)).expect("fork exists")];
        let mut overlay = Overlay::new(&graph, &overview, main);

        assert_eq!(overview.nodes.len(), 4, "only refs, the fork, and root remain");
        assert_eq!(overview.nodes[main].parent, Some(fork));
        assert_eq!(overview.nodes[topic].parent, Some(fork));
        assert_eq!(overlay.counts[main], Some(5), "selected count follows every parent");
        assert_eq!(overlay.counts[topic], None, "off-screen reference counts start lazy");
        let topic_edge = overview
            .edges
            .iter()
            .position(|edge| edge.child == topic)
            .expect("topic has a contracted edge");
        assert_eq!(
            overlay.boundaries[topic_edge],
            Some(id(4)),
            "the hidden merged topic commit becomes the visual reachability boundary"
        );
        let rail = place_rail(&overview, Some(&overlay));
        let boundary = rail.boundaries[topic_edge].expect("rail inserts the boundary row");
        assert!(
            matches!(rail.rail_rows[boundary.y].kind, RailRowKind::Boundary(edge) if edge == topic_edge),
            "the inserted rail row retains its source edge"
        );
        let topic_row = rail.nodes[topic].y;
        overlay.compute_visible_counts(&graph, &overview, &rail, topic_row..topic_row + 1);
        assert_eq!(
            overlay.counts[topic],
            Some(1),
            "a visible reference gets its exact exclusive count"
        );
        let stamp = overlay.stamp;
        overlay.compute_visible_counts(&graph, &overview, &rail, topic_row..topic_row + 1);
        assert_eq!(overlay.stamp, stamp, "a cached visible count is not traversed again");
    }

    #[test]
    fn space_keeps_counts_anchored_without_rebuilding_while_navigating() -> gix_testtools::Result {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        let mut terminal = Terminal::new(TestBackend::new(100, 18))?;
        terminal.draw(|frame| tree.draw(frame, Some(&graph)))?;
        let main = tree
            .selected
            .and_then(|selected| tree.overview.as_ref()?.nodes.get(selected))
            .map(|node| node.id)
            .expect("the initial selection exists");

        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(tree.count_anchor, Some(main), "Space anchors counts to the cursor");
        tree.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT));
        assert!(tree.overlay.is_some(), "anchored navigation retains reachability");
        assert!(tree.placed.is_some(), "anchored navigation retains tree placement");
        assert_eq!(
            tree.overlay.as_ref().map(|overlay| overlay.selected),
            graph.index(main),
            "the retained overlay still uses the anchored commit"
        );
        terminal.draw(|frame| tree.draw(frame, Some(&graph)))?;
        let footer: String = terminal
            .backend()
            .buffer()
            .content
            .chunks(100)
            .last()
            .expect("the terminal has a footer")
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            footer.contains("Space counts:main"),
            "the fixed anchor remains identifiable"
        );

        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert!(
            tree.overlay.is_none(),
            "moving the anchor invalidates reachability once"
        );
        terminal.draw(|frame| tree.draw(frame, Some(&graph)))?;
        let moved = tree
            .selected
            .and_then(|selected| tree.overview.as_ref()?.nodes.get(selected))
            .map(|node| node.id)
            .expect("the moved selection exists");
        assert_eq!(tree.count_anchor, Some(moved));

        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert!(
            tree.count_anchor.is_none(),
            "Space on the anchor restores automatic counts"
        );
        assert!(tree.overlay.is_some(), "clearing at the cursor reuses the same overlay");
        tree.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT));
        assert!(
            tree.overlay.is_none(),
            "automatic counts invalidate when the cursor moves"
        );
        Ok(())
    }

    #[test]
    fn only_the_current_detached_worktree_is_a_pin_marker() {
        let (graph, mut refs, mut decorations) = fixture();
        *decorations.get_mut(&id(5)).expect("topic is decorated") = vec![
            Decoration {
                name: "pin:first".into(),
                kind: DecorationKind::Pin,
            },
            Decoration {
                name: "pin:second".into(),
                kind: DecorationKind::Pin,
            },
        ];
        refs.pins.push(crate::history::Pin {
            name: "refs/worktree/tix/pins/first".try_into().expect("valid"),
            target: gix::refs::Target::Object(id(5)),
            id: id(5),
        });
        refs.worktrees.push(crate::history::WorktreeCheckout {
            id: id(3),
            label_id: id(6),
            checkout_name: "main-wt".into(),
            reference: Some("refs/heads/main".try_into().expect("valid")),
            is_current: true,
            is_detached: true,
        });
        refs.worktrees.push(crate::history::WorktreeCheckout {
            id: id(4),
            label_id: id(6),
            checkout_name: "foreign-wt".into(),
            reference: Some("refs/heads/main".try_into().expect("valid")),
            is_current: false,
            is_detached: true,
        });
        decorations.get_mut(&id(6)).expect("main is decorated")[0].kind = DecorationKind::HeadPinBranch;
        decorations.entry(id(4)).or_default().push(Decoration {
            name: "foreign-wt".into(),
            kind: DecorationKind::WorktreeDetached,
        });
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        let overview = tree.overview.as_ref().expect("overview exists");
        assert!(
            !overview.nodes.iter().any(|node| node.id == id(5)),
            "ordinary pin-only tips are absent"
        );
        let unicode = render_overview(overview, true, &HashMap::new());
        let ascii = render_overview(overview, false, &HashMap::new());
        assert_eq!(unicode.matches('📌').count(), 1, "detached HEAD has one marker");
        assert!(
            unicode.contains("★main"),
            "the remembered branch stays at its actual tip"
        );
        assert!(
            unicode.contains("foreign-wt@"),
            "the foreign checkout identifies its actual HEAD"
        );
        assert_eq!(
            ascii.matches("[pin]").count(),
            1,
            "only current detached state is marked"
        );
        assert!(!unicode.contains("pin:"), "ordinary pin names remain hidden");
    }

    #[test]
    fn count_anchor_survives_refresh_and_clears_with_a_hidden_tag() {
        let (graph, refs, mut decorations) = fixture();
        decorations.entry(id(3)).or_default().push(Decoration {
            name: "v1".into(),
            kind: DecorationKind::Tag,
        });
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        let main = tree.count_anchor.expect("main is anchored");
        tree.rebuild(&graph, &refs, &decorations);
        assert_eq!(tree.count_anchor, Some(main), "refresh preserves a visible anchor");

        tree.selected = tree.overview.as_ref().and_then(|overview| {
            graph
                .index(id(3))
                .and_then(|commit| overview.by_commit.get(&commit).copied())
        });
        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(tree.count_anchor, Some(id(3)), "Space moves the anchor to the tag");
        tree.toggle_tags();
        assert!(
            tree.count_anchor.is_none(),
            "hiding the anchor tag restores automatic counts"
        );
    }

    #[test]
    fn deleted_reference_selection_uses_the_next_surviving_row_then_the_previous() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        tree.placed = tree.overview.as_ref().map(|overview| place_rail(overview, None));
        let overview = tree.overview.as_ref().expect("overview exists");
        let mut rows: Vec<_> = overview
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (tree.placed.as_ref().expect("placed").nodes[index].y, node.id))
            .collect();
        rows.sort_by_key(|(row, _)| *row);
        let (_, selected) = rows[1];
        tree.selected = overview.nodes.iter().position(|node| node.id == selected);
        let fallback = tree.reference_deletion_fallback().expect("selection has neighbours");
        tree.select_after_reference_deletion(fallback);

        let mut without_refs = refs.clone();
        without_refs.view_tips.retain(|id| *id != selected);
        let mut without_selected = decorations.clone();
        without_selected.remove(&selected);
        tree.rebuild(&graph, &without_refs, &without_selected);
        assert_eq!(
            tree.selected_id(),
            Some(id(1)),
            "the first surviving row below is preferred"
        );

        let graph = HistoryGraph::from_test_commits(&[(id(1), vec![]), (id(2), vec![]), (id(3), vec![])]);
        let refs = RefSnapshot {
            view: HashMap::new(),
            hidden: HashMap::new(),
            view_tips: vec![id(1), id(2), id(3)],
            hidden_tips: Vec::new(),
            pins: Vec::new(),
            head_pin_branch: None,
            worktrees: Vec::new(),
        };
        let decorations = HashMap::from([
            (
                id(1),
                vec![
                    Decoration {
                        name: "a".into(),
                        kind: DecorationKind::Local,
                    },
                    Decoration {
                        name: "HEAD".into(),
                        kind: DecorationKind::Head,
                    },
                ],
            ),
            (
                id(2),
                vec![Decoration {
                    name: "b".into(),
                    kind: DecorationKind::Local,
                }],
            ),
            (
                id(3),
                vec![Decoration {
                    name: "c".into(),
                    kind: DecorationKind::Local,
                }],
            ),
        ]);
        tree.rebuild(&graph, &refs, &decorations);
        tree.selected = tree
            .overview
            .as_ref()
            .and_then(|overview| overview.nodes.iter().position(|node| node.id == id(3)));
        tree.placed = tree.overview.as_ref().map(|overview| place_rail(overview, None));
        let fallback = tree.reference_deletion_fallback().expect("selection has neighbours");
        tree.select_after_reference_deletion(fallback);
        let mut remaining_refs = refs.clone();
        remaining_refs.view_tips.retain(|candidate| *candidate != id(3));
        let mut remaining_decorations = decorations.clone();
        remaining_decorations.remove(&id(3));
        tree.rebuild(&graph, &remaining_refs, &remaining_decorations);
        assert_eq!(
            tree.selected_id(),
            Some(id(2)),
            "the previous row is used when nothing follows"
        );

        tree.rebuild(&graph, &refs, &decorations);
        tree.selected = tree
            .overview
            .as_ref()
            .and_then(|overview| overview.nodes.iter().position(|node| node.id == id(3)));
        tree.placed = tree.overview.as_ref().map(|overview| place_rail(overview, None));
        let fallback = tree.reference_deletion_fallback().expect("selection has neighbours");
        tree.select_after_reference_deletion(fallback);
        tree.selected = tree
            .overview
            .as_ref()
            .and_then(|overview| overview.nodes.iter().position(|node| node.id == id(1)));
        tree.rebuild(&graph, &remaining_refs, &remaining_decorations);
        assert_eq!(
            tree.selected_id(),
            Some(id(1)),
            "moving the cursor before refresh cancels deletion fallback"
        );
    }

    #[test]
    fn ambiguous_topological_navigation_requires_a_choice() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        let overview = tree.overview.as_ref().expect("overview exists");
        let root = overview.by_commit[&graph.index(id(1)).expect("root exists")];
        let fork = overview.by_commit[&graph.index(id(2)).expect("fork exists")];
        let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
        tree.selected = Some(root);

        tree.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT));
        assert_eq!(tree.selected, Some(fork), "Shift-Up moves toward a leaf");
        tree.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT));
        assert_eq!(tree.selected, Some(root), "Shift-Down moves toward the root");

        tree.placed = tree.overview.as_ref().map(|overview| place_rail(overview, None));
        let nearest = nearest(
            &tree.placed.as_ref().expect("the ref-tree is placed").nodes,
            root,
            Direction::Up,
        );
        tree.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(tree.selected, nearest, "plain Up immediately returns to nearest motion");

        tree.selected = Some(fork);
        tree.ensure_visible = false;
        tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE));
        assert_eq!(
            tree.selected,
            Some(fork),
            "an ambiguous child does not move immediately"
        );
        assert!(tree.topological_choice.is_some(), "the child choice remains pending");
        assert!(tree.ensure_visible, "starting a choice reveals its source marker");

        tree.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
        assert_eq!(tree.topological_choice, Some(0), "modified choice keys are ignored");
        tree.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        assert_eq!(tree.topological_choice, Some(1));
        tree.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
        assert_eq!(tree.topological_choice, Some(0));
        tree.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        let chosen = tree.overview.as_ref().expect("overview exists").nodes[fork].children[1];
        tree.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(
            tree.selected,
            Some(chosen),
            "h wraps to the last child and Enter follows it"
        );
        assert!(tree.topological_choice.is_none(), "submission leaves choice mode");

        tree.handle_key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE));
        assert_eq!(
            tree.selected,
            Some(fork),
            "J follows the unique visible parent immediately"
        );
        tree.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT));
        assert!(tree.topological_choice.is_some());
        tree.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert!(!tree.edit_expanded, "unrelated keys are consumed while choosing");
        assert_eq!(
            tree.topological_choice,
            Some(0),
            "unrelated keys keep the choice active"
        );
        tree.handle_mouse(MouseEventKind::ScrollUp, KeyModifiers::NONE, 1);
        assert_eq!(tree.selected, Some(fork), "mouse input is consumed while choosing");
        assert_eq!(tree.topological_choice, Some(0), "mouse input keeps the choice active");
        tree.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(tree.topological_choice.is_none(), "Escape cancels the choice");

        tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE));
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Input::Quit,
            "quit remains available while choosing"
        );
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.topological_choice.is_none(), "rebuilding cancels a stale choice");

        tree.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));
        assert_eq!(tree.selected, Some(main), "plain g reaches the top selectable node");
        tree.selected = Some(main);
        tree.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
        assert_eq!(
            tree.selected,
            Some(root),
            "uppercase G reaches the current component root"
        );
        tree.selected = Some(main);
        tree.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::SHIFT));
        assert_eq!(tree.selected, Some(root), "shift-modified lowercase g reaches the root");
    }

    #[test]
    fn plain_pages_move_the_cursor_while_shift_pages_and_plain_mouse_pan() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        tree.placed = tree.overview.as_ref().map(|overview| place_rail(overview, None));
        tree.offset.page_height = 4;
        let points = tree.placed.as_ref().expect("the ref-tree is placed").nodes.clone();
        let first = points
            .iter()
            .enumerate()
            .min_by_key(|(_, point)| point.y)
            .map(|(index, _)| index)
            .expect("the fixture has nodes");
        tree.selected = Some(first);
        let first_y = points[first].y;

        tree.offset.max_y = tree
            .placed
            .as_ref()
            .expect("the ref-tree is placed")
            .height
            .saturating_sub(tree.offset.page_height);
        tree.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        let full_page = tree.selected.expect("PageDown retains a selection");
        let full_page_y = points[full_page].y;
        assert!(full_page_y > first_y, "plain PageDown advances the ref-tree cursor");
        assert!(
            tree.ensure_visible,
            "plain page navigation keeps the new cursor visible"
        );

        tree.selected = Some(first);
        tree.offset.y = 0;
        tree.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::SHIFT));
        assert_eq!(tree.selected, Some(first), "Shift-PageDown leaves the cursor alone");
        assert!(tree.offset.y > 0, "Shift-PageDown pans the viewport");

        tree.selected = Some(first);
        tree.offset.y = 0;
        tree.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        let half_page_y = points[tree.selected.expect("Ctrl-d retains a selection")].y;
        assert!(
            half_page_y > first_y && half_page_y <= full_page_y,
            "plain half-page navigation advances no farther than a full page"
        );

        tree.selected = Some(first);
        tree.offset.y = 0;
        tree.handle_key(KeyEvent::new(
            KeyCode::Char('d'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ));
        assert_eq!(tree.selected, Some(first), "Shift-Ctrl-d leaves the cursor alone");
        assert!(tree.offset.y > 0, "Shift-Ctrl-d pans by half a viewport");

        tree.selected = Some(first);
        tree.placed = tree.overview.as_ref().map(|overview| place_rail(overview, None));
        tree.handle_mouse(MouseEventKind::ScrollDown, KeyModifiers::NONE, 2);
        assert_eq!(
            tree.selected,
            Some(first),
            "plain mouse scrolling remains viewport-only"
        );
        tree.handle_mouse(MouseEventKind::ScrollDown, KeyModifiers::SHIFT, 1);
        assert_ne!(
            tree.selected,
            Some(first),
            "Shift-mouse scrolling moves the nearest cursor"
        );
    }

    #[test]
    fn enter_pins_every_visible_reference_kind_but_not_synthetic_nodes() {
        let (graph, refs, mut decorations) = fixture();
        decorations.get_mut(&id(6)).expect("main is decorated").extend([
            Decoration {
                name: "tag: release".into(),
                kind: DecorationKind::Tag,
            },
            Decoration {
                name: "origin/main".into(),
                kind: DecorationKind::Remote,
            },
        ]);
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);

        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Input::PinReferences {
                id: id(6),
                kinds: vec![DecorationKind::Local, DecorationKind::Remote, DecorationKind::Tag],
            },
            "Enter retains every displayed reference namespace"
        );
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE)),
            Input::PinReferences {
                id: id(6),
                kinds: vec![DecorationKind::Local, DecorationKind::Remote, DecorationKind::Tag],
            },
            "p retains every displayed reference namespace"
        );

        tree.selected = tree
            .overview
            .as_ref()
            .and_then(|overview| overview.by_commit.get(&graph.index(id(2))?).copied());
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Input::Handled,
            "fork-only nodes have no pin action"
        );
    }

    #[test]
    fn pin_references_uses_all_requested_refs_and_makes_symbolic_pins_visible() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let old = repo.rev_parse_single("main~2")?.detach();
        for name in ["refs/heads/old", "refs/tags/old", "refs/remotes/origin/old"] {
            repo.reference(
                name,
                old,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "prepare ref-tree pin test",
            )?;
        }
        assert!(
            std::process::Command::new("git")
                .current_dir(fixture.path())
                .args([
                    "symbolic-ref",
                    "refs/remotes/origin/old-head",
                    "refs/remotes/origin/old"
                ])
                .status()?
                .success(),
            "git creates a symbolic remote alias"
        );

        let pins = pin_references(&repo, old, &[DecorationKind::Local, DecorationKind::Remote])?;
        let targets: Vec<_> = pins
            .iter()
            .filter_map(|pin| pin.target.try_name().map(|name| name.as_bstr().to_owned()))
            .collect();
        assert!(targets.contains(&b"refs/heads/old".as_bstr().to_owned()));
        assert!(targets.contains(&b"refs/remotes/origin/old".as_bstr().to_owned()));
        assert!(
            targets.contains(&b"refs/remotes/origin/old-head".as_bstr().to_owned()),
            "the pin retains the selected symbolic reference instead of its referent"
        );
        assert!(!targets.contains(&b"refs/tags/old".as_bstr().to_owned()));
        for pin in &pins {
            assert_eq!(
                repo.find_reference(pin.name.as_ref())?.target().into_owned(),
                pin.target,
                "the stored pin itself remains symbolic"
            );
        }
        assert!(
            crate::history::snapshot(&repo, &[], &[], false)?
                .view_tips
                .contains(&old),
            "a symbolic reference pin augments attached history even below HEAD"
        );
        Ok(())
    }

    #[test]
    fn rebuild_preserves_selection_by_object_id() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        tree.selected = tree.overview.as_ref().and_then(|overview| {
            graph
                .index(id(5))
                .and_then(|index| overview.by_commit.get(&index).copied())
        });

        let reordered = HistoryGraph::from_test_commits(&[
            (id(6), vec![id(3), id(4)]),
            (id(5), vec![id(4)]),
            (id(4), vec![id(2)]),
            (id(3), vec![id(2)]),
            (id(2), vec![id(1)]),
            (id(1), vec![]),
        ]);
        tree.rebuild(&reordered, &refs, &decorations);

        assert_eq!(
            tree.selected
                .and_then(|selected| tree.overview.as_ref()?.nodes.get(selected))
                .map(|node| node.id),
            Some(id(5)),
            "refresh keeps the selected commit when graph indices change"
        );
    }

    #[test]
    fn toggling_tags_removes_their_labels_and_topology() {
        let (graph, refs, mut decorations) = fixture();
        decorations.entry(id(3)).or_default().extend([
            Decoration {
                name: "v1".into(),
                kind: DecorationKind::Tag,
            },
            Decoration {
                name: "release".into(),
                kind: DecorationKind::AnnotatedTag,
            },
        ]);
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.toggle(), "the available ref-tree opens");
        let tagged = graph.index(id(3)).expect("tagged commit exists");
        assert!(
            tree.overview
                .as_ref()
                .is_some_and(|overview| overview.by_commit.contains_key(&tagged)),
            "tags retain otherwise linear commits"
        );

        tree.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
        assert!(tree.hide_tags, "uppercase T hides tags");
        assert!(
            tree.overview
                .as_ref()
                .is_some_and(|overview| !overview.by_commit.contains_key(&tagged)),
            "a tag-only linear node disappears from the projection"
        );

        tree.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::SHIFT));
        assert!(
            tree.overview
                .as_ref()
                .is_some_and(|overview| overview.by_commit.contains_key(&tagged)),
            "shift-modified lowercase t restores tag nodes"
        );
        tree.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
        assert!(!tree.is_active(), "plain t returns to history");
    }

    #[test]
    fn full_rendering_is_unstyled_ascii() {
        let (graph, refs, decorations) = fixture();
        let overview = Overview::new(&graph, &refs, &decorations, true);
        let rendered = render_overview(&overview, false, &HashMap::new());

        assert!(rendered.contains("main"), "HEAD is rendered without selection state");
        assert!(rendered.contains("topic"), "ordinary nodes retain their label");
        assert!(rendered.contains("<root>"), "the full projection reaches its root");
        assert!(rendered.contains('o'), "ordinary nodes use an ASCII marker");
        assert!(!rendered.contains('●'), "ASCII output has no Unicode node glyphs");
        assert!(!rendered.contains('\u{1b}'), "plain output has no terminal styles");
    }

    #[test]
    fn full_rendering_pairs_raw_commit_tips_with_change_ids() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let mut commit = repo.head_commit()?.decode()?.into_owned()?;
        commit.message = "unreferenced raw tip".into();
        let id = repo.write_object(&commit)?.detach();
        let rendered = render_full(&repo, &[id.to_string().into()], &[], true, false)?;

        assert!(
            rendered.contains(&crate::change_id::display(&repo, id, 7)?),
            "a displayed raw hash is immediately followed by its change ID: {rendered:?}"
        );
        Ok(())
    }

    #[test]
    fn rail_layout_uses_rounded_unicode_connections_and_ascii_fallbacks() {
        let (graph, refs, decorations) = fixture();
        let overview = Overview::new(&graph, &refs, &decorations, true);
        let unicode = render_overview(&overview, true, &HashMap::new());
        let ascii = render_overview(&overview, false, &HashMap::new());

        assert!(
            unicode.contains("│ ● topic"),
            "branch nodes stay in their lane: {unicode:?}"
        );
        assert!(
            unicode.contains("├─╯"),
            "forks terminate with a rounded corner: {unicode:?}"
        );
        assert!(!unicode.contains('┌'), "rail corners are not square");
        assert!(ascii.contains("| o topic"), "ASCII output retains the branch lane");
        assert!(ascii.contains("+-+"), "ASCII output retains the join row");
    }

    #[test]
    fn interactive_ref_tree_renders_exact_visible_counts() -> gix_testtools::Result {
        let (graph, refs, mut decorations) = fixture();
        decorations.get_mut(&id(5)).expect("topic is decorated")[0].kind = DecorationKind::WorktreeBranch;
        let mut tree = Tree::default();
        tree.set_history_commits([id(6)]);
        tree.rebuild(&graph, &refs, &decorations);
        let mut terminal = Terminal::new(TestBackend::new(100, 18))?;

        terminal.draw(|frame| tree.draw(frame, Some(&graph)))?;
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .chunks(100)
            .map(|row| row.iter().map(ratatui::buffer::Cell::symbol).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("main"), "the rounded ref-tree shows main");
        assert!(rendered.contains("topic"), "the rounded ref-tree shows topic");
        assert!(rendered.contains("ref-tree ·"), "the footer identifies the ref-tree");
        assert!(
            rendered.contains("5• main"),
            "the selected reference shows its exact count"
        );
        assert!(
            rendered.contains("1• topic"),
            "visible references show relative exact counts"
        );
        assert!(
            !rendered.contains("5●"),
            "the rail marker is not repeated after a count"
        );
        let selected = tree.selected.expect("the ref-tree selects a node");
        let placed = tree.placed.as_ref().expect("drawing places the ref-tree");
        let point = placed.nodes[selected];
        let rail_width = placed.rail_width;
        assert!(
            terminal.backend().buffer()[(point.x as u16, point.y as u16)]
                .modifier
                .contains(Modifier::REVERSED),
            "the selected disk is inverted with its label"
        );
        assert!(
            terminal.backend().buffer()[(rail_width as u16, point.y as u16)]
                .modifier
                .contains(Modifier::REVERSED),
            "the selected node label remains inverted"
        );
        let topic =
            tree.overview.as_ref().expect("the ref-tree exists").by_commit[&graph.index(id(5)).expect("topic exists")];
        let point = tree.placed.as_ref().expect("drawing places the ref-tree").nodes[topic];
        assert_eq!(
            terminal.backend().buffer()[(rail_width as u16, point.y as u16)].fg,
            Color::Green,
            "an out-of-history linked worktree uses dark green"
        );
        tree.set_history_commits([id(6), id(5)]);
        terminal.draw(|frame| tree.draw(frame, Some(&graph)))?;
        assert_eq!(
            terminal.backend().buffer()[(rail_width as u16, point.y as u16)].fg,
            Color::Cyan,
            "history visibility takes precedence with the bright current-history color"
        );

        let fork = tree.overview.as_ref().expect("the ref-tree exists").by_commit
            [&graph.index(id(2)).expect("the fork exists")];
        tree.selected = Some(fork);
        tree.selection_changed();
        tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE));
        terminal.draw(|frame| tree.draw(frame, Some(&graph)))?;
        let point = tree.placed.as_ref().expect("drawing places the ref-tree").nodes[fork];
        assert_eq!(
            terminal.backend().buffer()[(point.x as u16, point.y as u16)].symbol(),
            "1",
            "the pending choice replaces the selected disk"
        );
        assert!(
            terminal.backend().buffer()[(point.x as u16, point.y as u16)]
                .modifier
                .contains(Modifier::REVERSED),
            "the choice marker keeps the selected disk inverted"
        );
        let footer = terminal
            .backend()
            .buffer()
            .content
            .chunks(100)
            .nth(17)
            .expect("the terminal has a footer")
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(
            footer.contains("choose child 1/2 · h/l cycle · <enter> move · Esc cancel"),
            "the footer shows the exact pending choice"
        );
        assert!(
            (0..17).all(|y| {
                (0..rail_width).all(|x| terminal.backend().buffer()[(x as u16, y as u16)].fg != Color::Yellow)
            }),
            "navigation choices do not recolor graph lanes"
        );
        assert_eq!(choice_marker(9), '9');
        assert_eq!(choice_marker(10), '+', "large ordinals use the overflow marker");
        tree.leave_error("remote deletion failed");
        terminal.draw(|frame| tree.draw(frame, Some(&graph)))?;
        assert_eq!(terminal.backend().buffer()[(2, 16)].bg, Color::LightRed);
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .chunks(100)
                .last()
                .expect("the terminal has a footer")
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
                .contains("ref-tree"),
            "ref-tree notices leave its footer visible"
        );
        Ok(())
    }

    #[test]
    fn ref_tree_toggle_opens_and_closes_the_single_layout() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);

        assert!(tree.toggle());
        assert!(tree.is_active());
        assert!(tree.toggle());
        assert!(!tree.is_active());
    }
}
