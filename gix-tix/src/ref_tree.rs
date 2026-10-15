use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ffi::OsString,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use gix::{
    ObjectId, Result,
    bstr::ByteSlice,
    error::{ErrorExt, OptionExt, ResultExt, message},
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use crate::{
    app::{Action, App, LaneState, Notice, NoticeKind, ViewportPosition},
    command_menu::{self, Command, CommandGroup, CommandId},
    graph_tiles::{Hit, Tile},
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
    highlighted_edges: Vec<usize>,
}

struct RailRow {
    lane: String,
    tiles: Vec<Tile>,
    routes: Vec<RailRoutes>,
    kind: RailRowKind,
    edge: Option<usize>,
}

#[derive(Default)]
struct RailRoutes {
    edges: Vec<Option<usize>>,
    paths: [Option<usize>; 10],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Hover {
    edge: usize,
    target: usize,
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
    OpenCommandMenu(Vec<gix::refs::FullName>),
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
    viewport_position: Option<ViewportPosition>,
    placed: Option<Placed>,
    node_hits: Vec<(Rect, usize)>,
    mouse_overlay: Option<Rect>,
    mouse_body: Rect,
    mouse_offset: Offset,
    mouse_graphics: bool,
    hover_position: Option<Position>,
    hover_alt: bool,
    hovered: Option<Hover>,
    ensure_visible: bool,
    hide_tags: bool,
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
        self.viewport_position = None;
        self.node_hits.clear();
        self.update_hover(None, false);
        self.ensure_visible = true;
        true
    }

    pub(crate) fn leave(&mut self) {
        self.active = false;
        self.viewport_position = None;
        self.node_hits.clear();
        self.update_hover(None, false);
        self.remote_deletions.clear();
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
        self.remote_deletions.clear();
        self.viewport_position = None;
        self.overview = Some(overview);
        self.alternate_overview = Some(alternate_overview);
        self.overlay = None;
        self.placed = None;
        self.node_hits.clear();
        self.hovered = None;
        self.ensure_visible = true;
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent, app: &mut App) -> Input {
        self.notice = None;
        if key.kind == crossterm::event::KeyEventKind::Release
            || key.kind == crossterm::event::KeyEventKind::Repeat
                && matches!(key.code, KeyCode::Char('a' | 'v' | 'z' | '?' | '/' | 'q' | 't'))
        {
            return Input::Handled;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Input::Quit;
        }
        if key.code == KeyCode::Char('q') {
            return self.handle_action(Action::Quit, app);
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
        if let Some(action) = self.action(key, app) {
            return self.handle_action(action, app);
        }
        if key.code == KeyCode::Char('p')
            && key.modifiers.is_empty()
            && key.kind == crossterm::event::KeyEventKind::Press
        {
            app.close_shortcut_groups();
            self.remote_deletions.clear();
            return Input::OpenCommandMenu(self.selected_remote_references());
        }
        app.close_shortcut_groups();
        self.remote_deletions.clear();
        if key.code == KeyCode::Esc {
            self.leave();
            return Input::Handled;
        }
        if key.code == KeyCode::Enter {
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

    pub(crate) fn topological_navigation_active(&self) -> bool {
        self.topological_choice.is_some()
    }

    pub(crate) fn commands(&self) -> Vec<Command> {
        let mut commands = Vec::new();
        if self.selected_pin_target().is_some() {
            commands.push(command_menu::command(
                CommandId::PinReferences,
                0,
                "pin references",
                true,
            ));
        }
        if !self.selected_local_branches().is_empty() {
            commands.push(command_menu::command(
                CommandId::DeleteLocalBranches,
                0,
                "delete local branches",
                true,
            ));
        }
        if !self.remote_deletions.is_empty() {
            commands.push(command_menu::command(
                CommandId::DeleteRemoteReferences,
                0,
                "delete on remote",
                true,
            ));
        }
        commands.extend([
            command_menu::command(CommandId::Tags, 0, "tags", !self.hide_tags),
            command_menu::command(CommandId::CountAnchor, 0, "counts", self.count_anchor.is_some()),
            command_menu::command(CommandId::ViewportCenter, 0, "z center", true),
            command_menu::command(CommandId::ViewportTop, 0, "top", true),
            command_menu::command(CommandId::ViewportBottom, 0, "bottom", true),
            command_menu::command(CommandId::RefTreeTop, 0, "g top", true),
            command_menu::command(CommandId::RefTreeRoot, 0, "G root", true),
            command_menu::command(CommandId::History, 0, "history", true),
        ]);
        command_menu::balance(commands)
    }

    pub(crate) fn action(&self, key: KeyEvent, app: &App) -> Option<Action> {
        if key.kind == crossterm::event::KeyEventKind::Release
            || key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return None;
        }
        let prefix = match key.code {
            KeyCode::Char('a') if !key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::ToggleActions),
            KeyCode::Char('v') if !key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::ToggleHistoryDisplay),
            KeyCode::Char('z') if !key.modifiers.contains(KeyModifiers::SHIFT) => Some(if app.viewport_expanded {
                Action::PositionSelection(ViewportPosition::Center)
            } else {
                Action::ToggleViewport
            }),
            KeyCode::Char('?') => Some(Action::ToggleInformation),
            KeyCode::Char('/') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(Action::ToggleInformation),
            _ => None,
        };
        if let Some(prefix) = prefix {
            return (key.kind == crossterm::event::KeyEventKind::Press).then_some(prefix);
        }
        let group = self.prefix_group(app)?;
        if key.code == KeyCode::Esc {
            return Some(Action::Cancel);
        }
        let KeyCode::Char(mut letter) = key.code else {
            return None;
        };
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            letter = letter.to_ascii_uppercase();
        }
        self.commands()
            .into_iter()
            .find(|command| command.group == group && command.key() == Some(letter))
            .map(|command| command.action)
    }

    pub(crate) fn handle_action(&mut self, action: Action, app: &mut App) -> Input {
        self.notice = None;
        if self.topological_choice.is_some() && matches!(action, Action::ToggleViewport | Action::PositionSelection(_))
        {
            return Input::Handled;
        }
        if matches!(
            action,
            Action::ToggleActions | Action::ToggleHistoryDisplay | Action::ToggleInformation | Action::ToggleViewport
        ) {
            let expanded = match action {
                Action::ToggleActions => !app.actions_expanded,
                Action::ToggleHistoryDisplay => !app.history_display_expanded,
                Action::ToggleViewport => !app.viewport_expanded,
                _ => !app.information_expanded,
            };
            app.close_shortcut_groups();
            match action {
                Action::ToggleActions => app.actions_expanded = expanded,
                Action::ToggleHistoryDisplay => app.history_display_expanded = expanded,
                Action::ToggleViewport => app.viewport_expanded = expanded,
                _ => app.information_expanded = expanded,
            }
            self.remote_deletions.clear();
            let references = self.selected_remote_references();
            return if action == Action::ToggleActions && expanded && !references.is_empty() {
                Input::ResolveRemoteReferences(references)
            } else {
                Input::Handled
            };
        }
        if action == Action::Cancel {
            app.close_shortcut_groups();
        }
        match action {
            Action::PositionSelection(position) => {
                app.close_shortcut_groups();
                self.viewport_position = Some(position);
                self.ensure_visible = false;
                Input::Handled
            }
            Action::PinReferences => self
                .selected_pin_target()
                .map_or(Input::Handled, |(id, kinds)| Input::PinReferences { id, kinds }),
            Action::DeleteLocalBranches => Input::DeleteLocalBranches {
                names: self.selected_local_branches(),
                fallback: self
                    .reference_deletion_fallback()
                    .expect("a selected branch has a deletion fallback"),
            },
            Action::DeleteRemoteReferences => Input::DeleteRemoteReferences {
                groups: std::mem::take(&mut self.remote_deletions),
                fallback: self
                    .reference_deletion_fallback()
                    .expect("a selected remote reference has a deletion fallback"),
            },
            Action::ToggleTags => {
                self.toggle_tags();
                Input::Handled
            }
            Action::ToggleCountAnchor => {
                self.toggle_count_anchor();
                Input::Handled
            }
            Action::First | Action::Last => {
                app.close_shortcut_groups();
                if action == Action::First {
                    self.jump_to_top();
                } else {
                    self.jump_to_root();
                }
                Input::Handled
            }
            Action::ToggleRefTree | Action::Quit => {
                app.close_shortcut_groups();
                self.leave();
                Input::Handled
            }
            _ => Input::Handled,
        }
    }

    fn prefix_group(&self, app: &App) -> Option<CommandGroup> {
        if app.history_display_expanded {
            Some(CommandGroup::View)
        } else if app.actions_expanded {
            Some(CommandGroup::Actions)
        } else if app.viewport_expanded {
            Some(CommandGroup::Viewport)
        } else if app.information_expanded {
            Some(CommandGroup::Information)
        } else {
            None
        }
    }

    fn footer(&self, app: &App) -> (Line<'static>, Option<(CommandGroup, usize)>) {
        if let Some((choice, total)) = self.topological_choice_status() {
            return (
                Line::raw(format!(
                    "ref-tree · child {choice}/{total} · h/l cycle · <enter> move · Esc cancel"
                )),
                None,
            );
        }
        let mut spans = vec![Span::raw("ref-tree · ")];
        spans.extend(crate::ui::shortcut("p command", 'p', true));
        let mut active = None;
        for (group, label) in [
            (CommandGroup::View, "view"),
            (CommandGroup::Viewport, "z position"),
            (CommandGroup::Actions, "actions"),
            (CommandGroup::Information, "? help"),
        ] {
            spans.push(Span::raw(" · "));
            let anchor = spans.iter().map(Span::width).sum();
            let mut label = crate::ui::shortcut(label, group.prefix(), true);
            if self.prefix_group(app) == Some(group) {
                crate::ui::emphasize_prefix(&mut label);
                active = Some((group, anchor));
            }
            spans.extend(label);
        }
        spans.push(Span::raw(" · "));
        spans.extend(crate::ui::shortcut("q history", 'q', true));
        (Line::from(spans), active)
    }

    fn prefix_hints(&self, group: CommandGroup) -> Vec<Vec<Span<'static>>> {
        match group {
            CommandGroup::View => vec![vec![Span::raw(format!(
                "tags:{} · Space counts:{}",
                if self.hide_tags { "off" } else { "on" },
                self.count_anchor
                    .map_or_else(|| "auto".into(), |id| self.node_label(id))
            ))]],
            CommandGroup::Information => [
                "↑↓/jk move",
                "h/l cursor",
                "J/K topo",
                "mouse pan",
                "Shift+mouse cursor",
                "Alt/Option+hover preview line",
                "Alt/Option+click jump to commit",
                "Ctrl-u/d half-page",
                "Ctrl-b/f full-page",
                "pages cursor",
                "Shift+pages pan",
                "zz center · zt top · zb bottom",
                "<enter> pin",
                "t/Esc history",
            ]
            .into_iter()
            .map(|hint| vec![Span::raw(hint)])
            .collect(),
            _ => Vec::new(),
        }
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
        self.remote_deletions.clear();
        if modifiers.contains(KeyModifiers::SHIFT) {
            self.navigate(direction, false);
        } else {
            self.pan(direction, distance.max(1));
        }
        true
    }

    pub(crate) fn select_at(&mut self, column: u16, row: u16) -> bool {
        let position = (column, row).into();
        if !self.active
            || self.topological_choice.is_some()
            || self.mouse_overlay.is_some_and(|area| area.contains(position))
        {
            return false;
        }
        let Some((selected, reveal)) = self
            .node_hits
            .iter()
            .find_map(|(area, selected)| area.contains(position).then_some((*selected, false)))
            .or_else(|| {
                self.hover_alt
                    .then(|| self.route_at(position))?
                    .map(|hover| (hover.target, true))
            })
        else {
            return false;
        };
        self.notice = None;
        self.remote_deletions.clear();
        if self.selected != Some(selected) {
            self.selected = Some(selected);
            self.selection_changed();
        }
        self.ensure_visible = reveal;
        true
    }

    pub(crate) fn update_hover(&mut self, position: Option<Position>, alt: bool) -> bool {
        self.hover_position = position;
        self.hover_alt = alt;
        let hovered = position.filter(|_| alt).and_then(|position| self.route_at(position));
        std::mem::replace(&mut self.hovered, hovered).map(|hover| hover.edge) != hovered.map(|hover| hover.edge)
    }

    fn route_at(&self, position: Position) -> Option<Hover> {
        if !self.active
            || self.topological_choice.is_some()
            || !self.mouse_body.contains(position)
            || self.mouse_overlay.is_some_and(|area| area.contains(position))
        {
            return None;
        }
        let placed = self.placed.as_ref()?;
        let point = Point {
            x: usize::from(position.x - self.mouse_body.x) + self.mouse_offset.x,
            y: usize::from(position.y - self.mouse_body.y) + self.mouse_offset.y,
        };
        if point.x >= placed.rail_width {
            return None;
        }
        let row = placed.rail_rows.get(point.y)?;
        let tile = row.tiles.get(point.x)?;
        let routes = row.routes.get(point.x)?;
        let edge = match tile.hit_test(self.mouse_graphics)? {
            Hit::Edge(index) => routes.edges.get(index).copied().flatten(),
            Hit::Path(path) => routes.paths[path.ilog2() as usize],
        }?;
        let endpoints = self.overview.as_ref()?.edges.get(edge)?;
        let target = [endpoints.child, endpoints.parent]
            .into_iter()
            .min_by_key(|&node| placed.nodes[node].y.abs_diff(point.y))?;
        Some(Hover { edge, target })
    }

    pub(crate) fn draw(&mut self, frame: &mut Frame<'_>, area: Rect, graph: Option<&HistoryGraph>, app: &mut App) {
        self.draw_once(frame, area, graph, app);
        if self.update_hover(self.hover_position, self.hover_alt) {
            self.draw_once(frame, area, graph, app);
        }
    }

    fn draw_once(&mut self, frame: &mut Frame<'_>, area: Rect, graph: Option<&HistoryGraph>, app: &mut App) {
        self.node_hits.clear();
        self.mouse_overlay = None;
        self.mouse_body = Rect::default();
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
        let [mut body, footer] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
        frame.render_widget(Clear, area);
        let commands = self.commands();
        let (_, active_prefix) = self.footer(app);
        let anchor = active_prefix.map_or(0, |(_, anchor)| anchor);
        let mut popup = active_prefix.map(|(group, _)| {
            crate::ui::command_popup(
                &commands,
                group,
                self.prefix_hints(group),
                footer.width.saturating_sub(2) as usize,
            )
        });
        let allowed = popup
            .as_ref()
            .is_some_and(|popup| crate::ui::prefix_popup_can_render(area, footer, anchor, popup.rows.len()));
        if !allowed {
            popup = None;
            app.cancel_held_prefix();
        }
        if app.held_prefix_group().is_some()
            && app
                .set_held_prefix_layout(popup.as_ref().map_or_else(Vec::new, |popup| popup.items.clone()))
                .is_none()
        {
            popup = None;
        }
        let mut notice = app
            .held_prefix_selection()
            .and_then(|id| {
                commands.iter().find(|command| command.id == id).map(|command| Notice {
                    kind: NoticeKind::Attention,
                    text: format!(
                        "{} · release {} to run · Esc cancel",
                        command.help(app),
                        command.group.prefix()
                    ),
                })
            })
            .or_else(|| self.notice.clone());
        let notice_bottom = footer
            .y
            .saturating_sub(popup.as_ref().map_or(0, |popup| popup.rows.len() as u16));
        let mut notice_area = notice
            .as_ref()
            .and_then(|notice| notice_area(notice, body, body.y, notice_bottom));
        if notice.is_some() && notice_area.is_none() {
            app.cancel_held_prefix();
            popup = None;
            notice.clone_from(&self.notice);
            notice_area = notice
                .as_ref()
                .and_then(|notice| crate::ui::notice_area(notice, body, body.y, body.bottom()));
        }
        if let Some(notice_area) = notice_area {
            body.height = notice_area.y.saturating_sub(body.y);
        }
        let Some(overview) = self.overview.as_ref() else {
            frame.render_widget(Paragraph::new("ref-tree overview unavailable"), body);
            return;
        };
        let highlighted_edges = if let Some(hover) = self.hovered
            && self.topological_choice.is_none()
        {
            vec![hover.edge]
        } else if app.topological_preview && self.topological_choice.is_none() {
            overview
                .edges
                .iter()
                .enumerate()
                .filter_map(|(index, edge)| {
                    (self.selected == Some(edge.parent) || self.selected == Some(edge.child)).then_some(index)
                })
                .collect()
        } else {
            self.topological_choice
                .and_then(|choice| {
                    let parent = self.selected?;
                    let child = *overview.nodes[parent].children.get(choice)?;
                    overview
                        .edges
                        .iter()
                        .position(|edge| edge.parent == parent && edge.child == child)
                })
                .into_iter()
                .collect()
        };
        if self
            .placed
            .as_ref()
            .is_none_or(|placed| placed.highlighted_edges != highlighted_edges)
        {
            self.placed = Some(place_rail(
                overview,
                self.overlay.as_ref(),
                app.graphics_enabled,
                highlighted_edges,
            ));
        }
        let placed = self.placed.as_ref().expect("rail placement was just populated");
        let selected = self.selected;
        let ensure_visible = self.ensure_visible;
        let viewport_position = self.viewport_position.take();
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
            if let (Some(position), Some(point)) = (viewport_position, selected_point) {
                offset.y = position.offset(point.y, offset.page_height, offset.max_y);
            }
            *offset
        };
        self.ensure_visible = false;
        self.mouse_body = body;
        self.mouse_offset = offset;
        self.mouse_graphics = app.graphics_enabled;
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
            &mut self.node_hits,
        );
        if app.graphics_enabled || !placed.highlighted_edges.is_empty() {
            for (y, row) in placed
                .rail_rows
                .iter()
                .skip(offset.y)
                .take(usize::from(body.height))
                .enumerate()
            {
                let area = Rect::new(body.x, body.y + y as u16, body.width, 1);
                if app.graphics_enabled {
                    crate::ui::tag_graph(frame, area, &row.tiles, offset.x);
                } else {
                    crate::ui::highlight_graph(frame, area, &row.tiles, offset.x);
                }
            }
        }
        frame.render_widget(
            Paragraph::new(self.footer(app).0).style(Style::default().add_modifier(Modifier::DIM)),
            footer,
        );
        if let Some(popup) = popup {
            self.mouse_overlay =
                crate::ui::render_prefix_popup(frame, area, footer, anchor, popup, app.held_prefix_selection());
        }
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
        self.remote_deletions = deletions;
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
                temporary = place_rail(overview, self.overlay.as_ref(), false, Vec::new());
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
        self.viewport_position = None;
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
        self.viewport_position = None;
        if self
            .count_anchor
            .is_some_and(|id| !overview.nodes.iter().any(|node| node.id == id))
        {
            self.count_anchor = None;
        }
        self.topological_choice = None;
        self.overlay = None;
        self.placed = None;
        self.node_hits.clear();
        self.hovered = None;
        self.ensure_visible = true;
    }

    fn selection_changed(&mut self) {
        self.topological_choice = None;
        self.viewport_position = None;
        if self.count_anchor.is_none() {
            self.overlay = None;
            self.placed = None;
        }
    }

    fn pan(&mut self, direction: Direction, amount: usize) {
        self.viewport_position = None;
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
                temporary = place_rail(overview, self.overlay.as_ref(), false, Vec::new());
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
        DecorationKind::Remote | DecorationKind::Review | DecorationKind::WorktreeDetached => Some(kind),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn pin_references(
    repository: &gix::Repository,
    id: ObjectId,
    kinds: &[DecorationKind],
) -> Result<Vec<crate::history::Pin>> {
    pin_references_reporting(repository, id, kinds).map(|(pins, _changes)| pins)
}

pub(crate) fn pin_references_reporting(
    repository: &gix::Repository,
    id: ObjectId,
    kinds: &[DecorationKind],
) -> Result<(Vec<crate::history::Pin>, Vec<crate::edit::undo::RefChange>)> {
    let mut names = Vec::new();
    for reference in repository.references()?.all()? {
        let mut reference = match reference {
            Ok(reference) => reference,
            Err(err) if crate::history::is_missing_ref(&err) => continue,
            Err(err) => return Err(message!("could not read reference to pin: {err}").raise()),
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
    if kinds.contains(&DecorationKind::WorktreeDetached) {
        names.extend(
            crate::history::worktree_checkouts(repository)?
                .into_iter()
                .filter(|worktree| !worktree.is_current && worktree.is_detached && worktree.id == id)
                .map(|worktree| worktree.head_reference),
        );
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
        for (id, decorations) in decorations.iter() {
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
                current = graph.known_parents(index).first().copied();
            }
        }
        let mut children = vec![Vec::new(); graph.commit_count()];
        for child in included.iter().copied() {
            if let Some(parent) = graph
                .known_parents(child)
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
                        .known_parents(*index)
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
            let mut parent = graph.known_parents(nodes[child].commit).first().copied();
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
                parent = graph.known_parents(index).first().copied();
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
            pending.extend_from_slice(graph.known_parents(index));
        }
        let mut first_parent = vec![false; graph.commit_count()];
        let mut current = Some(selected_commit);
        while let Some(index) = current {
            first_parent[index.as_usize()] = true;
            current = graph.known_parents(index).first().copied();
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
                pending.extend_from_slice(graph.known_parents(index));
            }
            self.counts[node] = Some(count);
        }
    }
}

fn rail_routes(tiles: &[Tile], sources: &[Option<usize>]) -> Vec<RailRoutes> {
    tiles
        .iter()
        .enumerate()
        .map(|(column, tile)| {
            let mut routes = RailRoutes::default();
            for edge in &tile.edges {
                let route = column
                    .checked_add_signed(edge.from as isize)
                    .and_then(|source| sources.get(source).copied().flatten());
                routes.edges.push(route);
                let path = match (edge.from, edge.to) {
                    (0, 0) => {
                        if edge.from_node {
                            Tile::NODE_DOWN
                        } else {
                            Tile::UP_DOWN
                        }
                    }
                    (0, to) if to < 0 => {
                        if edge.from_node {
                            Tile::NODE_LEFT
                        } else {
                            Tile::UP_LEFT
                        }
                    }
                    (0, _) => {
                        if edge.from_node {
                            Tile::NODE_RIGHT
                        } else {
                            Tile::UP_RIGHT
                        }
                    }
                    (from, 0) if from < 0 => Tile::DOWN_LEFT,
                    (_, 0) => Tile::DOWN_RIGHT,
                    _ => Tile::LEFT_RIGHT,
                };
                routes.paths[path.ilog2() as usize] = routes.paths[path.ilog2() as usize].or(route);
            }
            routes
        })
        .collect()
}

fn place_rail(
    overview: &Overview,
    overlay: Option<&Overlay>,
    graphics_enabled: bool,
    highlighted_edges: Vec<usize>,
) -> Placed {
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
    let mut state = LaneState::new(true);
    let mut active_routes = Vec::new();
    let mut highlighted_incoming = Vec::new();
    let mut placed = Placed {
        nodes: vec![Point::default(); overview.nodes.len()],
        boundaries: vec![None; overview.edges.len()],
        highlighted_edges,
        ..Placed::default()
    };
    for item in items {
        let node_lane = state.node_line(item.id, item.marker);
        let mut transition = String::new();
        let edge = match item.kind {
            RailRowKind::Node(node) => edge_by_child[node],
            RailRowKind::Boundary(edge) => Some(edge),
            RailRowKind::NodeConnector | RailRowKind::BoundaryConnector => unreachable!("items are node rows"),
        };
        let highlighted_outgoing = edge
            .filter(|edge| placed.highlighted_edges.contains(edge))
            .and(item.parent);
        state.advance_ids(
            item.id,
            item.parent,
            Some(&mut transition),
            item.marker,
            (&highlighted_incoming, highlighted_outgoing.as_slice()),
        );
        highlighted_incoming.retain(|parent| *parent != item.id);
        if let Some(parent) = highlighted_outgoing
            && !highlighted_incoming.contains(&parent)
        {
            highlighted_incoming.push(parent);
        }
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
        active_routes.resize(state.tiles().len(), None);
        let incoming_route = active_routes[x];
        active_routes[x] = edge;
        let source_routes = std::mem::take(&mut active_routes);
        active_routes = state
            .tiles()
            .iter()
            .enumerate()
            .map(|(column, tile)| {
                tile.edges.iter().find_map(|edge| {
                    (edge.to == 0)
                        .then(|| column.checked_add_signed(edge.from as isize))
                        .flatten()
                        .and_then(|source| source_routes[source])
                })
            })
            .collect();
        let mut tiles = if rounded_transition {
            let mut tiles: Vec<_> = lane
                .chars()
                .map(|symbol| Tile::from_text(symbol).unwrap_or_default())
                .collect();
            tiles[x].paths =
                (state.tiles()[x].paths & Tile::NODE_UP) | if item.parent.is_some() { Tile::NODE_DOWN } else { 0 };
            for (index, tile) in tiles.iter_mut().enumerate() {
                let source = &state.tiles()[index];
                tile.edges.extend(
                    source
                        .edges
                        .iter()
                        .filter(|edge| edge.from == 0)
                        .cloned()
                        .map(|mut edge| {
                            edge.to = 0;
                            edge
                        }),
                );
                let mappings: &[(u16, u16)] = if index == x {
                    &[
                        (Tile::NODE_UP, Tile::NODE_UP),
                        (Tile::NODE_DOWN, Tile::NODE_DOWN),
                        (Tile::NODE_LEFT, Tile::NODE_DOWN),
                        (Tile::NODE_RIGHT, Tile::NODE_DOWN),
                    ]
                } else {
                    &[
                        (Tile::UP_DOWN, Tile::UP_DOWN),
                        (Tile::UP_LEFT, Tile::UP_DOWN),
                        (Tile::UP_RIGHT, Tile::UP_DOWN),
                    ]
                };
                for &(from, to) in mappings {
                    if source.paths & from != 0 {
                        tile.path_colors[to.ilog2() as usize] = source.path_colors[from.ilog2() as usize];
                    }
                    if source.highlighted_paths & from != 0 {
                        tile.highlighted_paths |= to;
                    }
                }
            }
            tiles
        } else {
            state.tiles().to_vec()
        };
        let mut routes = rail_routes(&tiles, &source_routes);
        routes[x].paths[Tile::NODE_UP.ilog2() as usize] = incoming_route;
        if !graphics_enabled {
            for tile in &mut tiles {
                tile.edges.clear();
            }
        }
        let y = placed.rail_rows.len();
        match item.kind {
            RailRowKind::Node(node) => placed.nodes[node] = Point { x, y },
            RailRowKind::Boundary(edge) => placed.boundaries[edge] = Some(Point { x, y }),
            RailRowKind::NodeConnector | RailRowKind::BoundaryConnector => {
                unreachable!("items are always node rows")
            }
        }
        placed.rail_width = placed.rail_width.max(lane.chars().count());
        placed.rail_rows.push(RailRow {
            lane,
            tiles,
            routes,
            kind: item.kind,
            edge,
        });
        if rounded_transition {
            let mut tiles = state.tiles().to_vec();
            for tile in &mut tiles {
                for edge in &mut tile.edges {
                    edge.from_node = false;
                }
            }
            {
                let tile = &mut tiles[x];
                tile.marker = ' ';
                let node_paths = tile.paths;
                let highlighted = tile.highlighted_paths;
                tile.paths &= !(Tile::NODE_UP | Tile::NODE_DOWN | Tile::NODE_LEFT | Tile::NODE_RIGHT);
                tile.highlighted_paths &= !(Tile::NODE_UP | Tile::NODE_DOWN | Tile::NODE_LEFT | Tile::NODE_RIGHT);
                for (arm, path) in [
                    (Tile::NODE_DOWN, Tile::UP_DOWN),
                    (Tile::NODE_LEFT, Tile::UP_LEFT),
                    (Tile::NODE_RIGHT, Tile::UP_RIGHT),
                ] {
                    if node_paths & arm != 0 {
                        tile.paths |= path;
                        tile.path_colors[path.ilog2() as usize] = tile.path_colors[arm.ilog2() as usize];
                    }
                    if highlighted & arm != 0 {
                        tile.highlighted_paths |= path;
                    }
                }
                tile.path_colors[Tile::NODE_UP.ilog2() as usize..].fill(0);
            }
            let routes = rail_routes(&tiles, &source_routes);
            if !graphics_enabled {
                for tile in &mut tiles {
                    tile.edges.clear();
                }
            }
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
            placed.rail_rows.push(RailRow {
                lane,
                tiles,
                routes,
                kind,
                edge,
            });
        }
    }
    if graphics_enabled {
        let mut rows: Vec<_> = placed
            .rail_rows
            .iter_mut()
            .map(|row| std::mem::take(&mut row.tiles))
            .collect();
        crate::graph_tiles::chamfer_rows(&mut rows);
        for (row, tiles) in placed.rail_rows.iter_mut().zip(rows) {
            row.tiles = tiles;
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
    node_hits: &mut Vec<(Rect, usize)>,
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
        if let Some(area) = put(frame, area, offset, point, marker, style) {
            node_hits.push((area, index));
        }
        if let Some(area) = draw_text(
            frame,
            area,
            offset,
            Point {
                x: placed.rail_width,
                y: point.y,
            },
            &text,
            style,
        ) {
            node_hits.push((area, index));
        }
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
) -> Result<String> {
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
    let authors = gix::parallel::OwnShared::new(gix::parallel::Mutable::new(crate::history::Authors::default()));
    let mut graph = None;
    crate::history::load(
        repository,
        &visible_revisions,
        hidden,
        true,
        &authors,
        &gix::interrupt::IS_INTERRUPTED,
        |event| {
            if let crate::history::Event::Complete(value) = event {
                graph = Some(value);
            }
            true
        },
    )?;
    crate::interrupt::check().or_error()?;
    let graph = graph.ok_or_raise(|| message("history traversal did not produce a graph"))?;
    let hidden_refs = hidden_refs.into_keys().collect();
    let decorations = crate::history::decorations_excluding(repository, &refs.pins, &refs.worktrees, &hidden_refs)?;
    refs.hidden_tips.clear();
    let overview = Overview::new(&graph, &refs, &decorations, show_tags);
    let labels = overview
        .nodes
        .iter()
        .filter(|node| node.raw_tip && node.decorations.is_empty())
        .map(|node| Ok((node.id, crate::change_id::display(repository, node.id, 7)?)))
        .collect::<Result<HashMap<_, _>>>()?;
    Ok(render_overview(&overview, unicode, &labels))
}

fn render_overview(overview: &Overview, unicode: bool, commit_labels: &HashMap<ObjectId, String>) -> String {
    if overview.nodes.is_empty() {
        return String::new();
    }
    let placed = place_rail(overview, None, false, Vec::new());
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

fn draw_text(
    frame: &mut Frame<'_>,
    area: Rect,
    offset: Offset,
    point: Point,
    text: &str,
    style: Style,
) -> Option<Rect> {
    let mut drawn: Option<Rect> = None;
    for (index, symbol) in text.chars().enumerate() {
        if let Some(cell) = put(
            frame,
            area,
            offset,
            Point {
                x: point.x + index,
                y: point.y,
            },
            symbol,
            style,
        ) {
            drawn = Some(drawn.map_or(cell, |area| area.union(cell)));
        }
    }
    drawn
}

fn put(frame: &mut Frame<'_>, area: Rect, offset: Offset, point: Point, symbol: char, style: Style) -> Option<Rect> {
    let x = point.x.checked_sub(offset.x)?;
    let y = point.y.checked_sub(offset.y)?;
    if x >= usize::from(area.width) || y >= usize::from(area.height) {
        return None;
    }
    frame.buffer_mut()[(area.x + x as u16, area.y + y as u16)]
        .set_char(symbol)
        .set_style(style);
    Some(Rect::new(area.x + x as u16, area.y + y as u16, 1, 1))
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
    use std::{collections::HashMap, sync::atomic::AtomicBool};

    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn id(n: u8) -> ObjectId {
        let mut bytes = [0; 20];
        bytes[19] = n;
        ObjectId::Sha1(bytes)
    }

    fn draw_tree(frame: &mut Frame<'_>, tree: &mut Tree, graph: &HistoryGraph) {
        let mut app = App::new(10);
        let area = frame.area();
        tree.draw(frame, area, Some(graph), &mut app);
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
            active_branch: None,
            #[cfg(feature = "blocking-network-client")]
            fetch_remote: None,
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
    fn drawing_stays_inside_the_supplied_area() -> gix_testtools::Result {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        app.graphics_enabled = true;
        tree.rebuild(&graph, &refs, &decorations);
        let bounds = Rect::new(4, 2, 32, 8);
        let mut terminal = Terminal::new(TestBackend::new(40, 12))?;

        terminal.draw(|frame| {
            for y in 0..frame.area().height {
                for x in 0..frame.area().width {
                    frame.buffer_mut()[(x, y)].set_symbol("x");
                }
            }
            tree.draw(frame, bounds, Some(&graph), &mut app);
        })?;

        for y in 0..12 {
            for x in 0..40 {
                if x < bounds.x || x >= bounds.right() || y < bounds.y || y >= bounds.bottom() {
                    assert_eq!(
                        terminal.backend().buffer()[(x, y)].symbol(),
                        "x",
                        "ref-tree drawing escaped its supplied area at ({x}, {y})"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn mouse_selection_uses_drawn_markers_and_clipped_labels() -> gix_error::TestResult {
        for graphics_enabled in [false, true] {
            let (graph, refs, mut decorations) = fixture();
            decorations.get_mut(&id(5)).expect("topic is decorated")[0].name = "topic-with-a-long-name".into();
            let mut tree = Tree::default();
            let mut app = App::new(10);
            app.graphics_enabled = graphics_enabled;
            tree.rebuild(&graph, &refs, &decorations);
            tree.toggle();
            tree.toggle_count_anchor();
            let bounds = Rect::new(4, 2, 32, 8);
            let mut terminal = Terminal::new(TestBackend::new(40, 12))?;
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            let overview = tree.overview.as_ref().expect("the ref-tree exists");
            let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
            let topic = overview.by_commit[&graph.index(id(5)).expect("topic exists")];
            let placed = tree.placed.as_ref().expect("drawing places the nodes");
            let main_point = placed.nodes[main];
            let topic_point = placed.nodes[topic];
            let label_x = bounds.x + placed.rail_width as u16;
            let topic_y = bounds.y + topic_point.y as u16;
            assert!(tree.select_at(label_x + 3, topic_y), "the topic label selects its node");
            assert_eq!(
                tree.selected_id(),
                Some(id(5)),
                "a label click moves the ref-tree cursor"
            );
            assert_eq!(tree.count_anchor, Some(id(6)), "clicking preserves the anchored counts");
            assert!(
                tree.select_at(bounds.x + main_point.x as u16, bounds.y + main_point.y as u16),
                "the commit marker also selects its node"
            );
            assert_eq!(
                tree.selected_id(),
                Some(id(6)),
                "marker clicks do not run the pin action"
            );
            assert!(
                !tree.select_at(bounds.right() - 1, topic_y),
                "empty space after a label is inert"
            );
            assert!(!tree.select_at(bounds.x - 1, topic_y), "outside the tree is inert");

            let narrow = Rect::new(4, 2, 8, 3);
            tree.offset.x = usize::from(label_x - bounds.x) + 5;
            tree.offset.y = topic_point.y;
            tree.ensure_visible = false;
            terminal.draw(|frame| tree.draw(frame, narrow, Some(&graph), &mut app))?;
            assert_eq!(
                terminal.backend().buffer()[(narrow.x, narrow.y)].symbol(),
                "p",
                "the viewport clips the count and the first two letters of topic"
            );
            let offset = (tree.offset.x, tree.offset.y);
            assert!(
                tree.select_at(narrow.x, narrow.y),
                "the visible suffix remains selectable"
            );
            assert_eq!(
                tree.selected_id(),
                Some(id(5)),
                "the clipped label selects the same node"
            );
            assert_eq!((tree.offset.x, tree.offset.y), offset, "clicking does not pan the view");
            assert!(
                !tree.select_at(narrow.right(), narrow.y),
                "clipped cells outside the body are inert"
            );
            assert!(!tree.select_at(narrow.x, narrow.bottom() - 1), "the footer is inert");
            tree.rebuild(&graph, &refs, &decorations);
            assert!(
                !tree.select_at(narrow.x, narrow.y),
                "rebuilding invalidates the old hit regions"
            );
        }
        Ok(())
    }

    #[test]
    fn mouse_selection_respects_drawn_popups_notices_and_numbered_choices() -> gix_error::TestResult {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        tree.toggle();
        tree.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE), &mut app);
        let mut terminal = Terminal::new(TestBackend::new(80, 10))?;
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        let popup = tree.mouse_overlay.expect("the help popup is visible");
        let covered = tree
            .node_hits
            .iter()
            .filter(|(_, node)| Some(*node) != tree.selected)
            .map(|(area, _)| area.intersection(popup))
            .find(|area| !area.is_empty())
            .expect("the popup covers an unselected node");
        let selected = tree.selected;
        app.information_expanded = false;
        assert!(
            !tree.select_at(covered.x, covered.y),
            "a popup click cannot select the node underneath even if input handling just closed it"
        );
        assert_eq!(tree.selected, selected, "popup clicks leave the cursor unchanged");

        tree.leave_error("remote deletion failed");
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        let notice_cells: Vec<_> = terminal
            .backend()
            .buffer()
            .area
            .positions()
            .filter(|position| terminal.backend().buffer()[*position].bg == Color::LightRed)
            .collect();
        assert!(!notice_cells.is_empty(), "the error notice is visible");
        assert!(
            notice_cells
                .iter()
                .all(|position| !tree.select_at(position.x, position.y)),
            "notice cells never select a node"
        );

        let overview = tree.overview.as_ref().expect("the ref-tree exists");
        let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
        let fork = overview.by_commit[&graph.index(id(2)).expect("fork exists")];
        tree.selected = Some(fork);
        tree.selection_changed();
        tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &mut app);
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        let choice = tree.topological_choice.expect("the fork has a numbered child choice");
        let main_area = tree
            .node_hits
            .iter()
            .find_map(|(area, node)| (*node == main).then_some(*area))
            .expect("the main node is visible");
        assert!(
            !tree.select_at(main_area.x, main_area.y),
            "clicks cannot bypass a numbered choice"
        );
        assert_eq!(tree.selected, Some(fork), "the chooser keeps its source node");
        assert_eq!(tree.topological_choice, Some(choice), "the pending choice is retained");
        Ok(())
    }

    #[test]
    fn alt_hover_previews_rails_and_selects_endpoints_independently_of_shift() -> gix_error::TestResult {
        let (graph, refs, decorations) = fixture();
        for graphics_enabled in [false, true] {
            let mut tree = Tree::default();
            let mut app = App::new(10);
            app.graphics_enabled = graphics_enabled;
            app.topological_preview = true;
            tree.rebuild(&graph, &refs, &decorations);
            tree.toggle();
            tree.count_anchor = Some(id(6));
            let overview = tree.overview.as_ref().expect("the overview exists");
            let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
            let topic = overview.by_commit[&graph.index(id(5)).expect("topic exists")];
            let fork = overview.by_commit[&graph.index(id(2)).expect("fork exists")];
            let main_edge = overview
                .edges
                .iter()
                .position(|edge| edge.child == main)
                .expect("main has a parent");
            let topic_edge = overview
                .edges
                .iter()
                .position(|edge| edge.child == topic)
                .expect("topic has a parent");
            tree.selected = Some(fork);
            let bounds = Rect::new(4, 2, 40, 12);
            let mut terminal = Terminal::new(TestBackend::new(48, 16))?;
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            let placed = tree.placed.as_ref().expect("drawing places the rail");
            assert_eq!(
                placed.highlighted_edges.len(),
                3,
                "Shift previews all edges incident to the fork"
            );
            let main_point = placed.nodes[main];
            let topic_point = placed.nodes[topic];
            let fork_point = placed.nodes[fork];
            let boundary = placed.boundaries[topic_edge].expect("anchored counts insert a topic boundary");
            let label_x = bounds.x + placed.rail_width as u16;
            let passing = Position::new(bounds.x + main_point.x as u16, bounds.y + topic_point.y as u16);
            assert_ne!(
                placed.rail_rows[topic_point.y].edge,
                Some(main_edge),
                "the hovered route passes another branch's row"
            );
            tree.update_hover(Some(passing), false);
            assert!(
                !tree.select_at(passing.x, passing.y),
                "Shift alone does not enable graph-line clicks"
            );

            assert!(
                tree.update_hover(Some(passing), true),
                "entering a route changes the preview"
            );
            assert_eq!(
                tree.hovered,
                Some(Hover {
                    edge: main_edge,
                    target: main
                }),
                "route identity follows its source column"
            );
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            assert_eq!(
                tree.placed.as_ref().expect("the rail is placed").highlighted_edges,
                [main_edge],
                "Alt-hover replaces Shift's selected-node incident edges"
            );
            app.topological_preview = false;
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            assert_eq!(
                tree.placed.as_ref().expect("the rail is placed").highlighted_edges,
                [main_edge],
                "releasing Shift preserves the Alt-hover preview"
            );
            assert!(
                tree.select_at(passing.x, passing.y),
                "Alt alone enables clicking a hovered rail"
            );
            assert_eq!(tree.selected, Some(main), "the upper segment is nearest to main");

            let arrival = Position::new(bounds.x + fork_point.x as u16, bounds.y + fork_point.y as u16 - 1);
            assert!(
                !tree.update_hover(Some(arrival), true),
                "moving along one edge changes its click target without redrawing"
            );
            assert_eq!(
                tree.hovered,
                Some(Hover {
                    edge: main_edge,
                    target: fork
                }),
                "the shared tail keeps the earlier branch's route identity"
            );
            assert!(
                tree.select_at(arrival.x, arrival.y),
                "the lower segment selects its endpoint"
            );
            assert_eq!(tree.selected, Some(fork), "the lower segment is nearest to the fork");
            assert_eq!(tree.count_anchor, Some(id(6)), "line clicks preserve the count anchor");

            let boundary = Position::new(bounds.x + boundary.x as u16, bounds.y + boundary.y as u16);
            tree.update_hover(Some(boundary), true);
            assert_eq!(
                tree.hovered,
                Some(Hover {
                    edge: topic_edge,
                    target: topic
                }),
                "count-boundary stems belong to the complete structural edge"
            );
            assert!(
                tree.select_at(boundary.x, boundary.y),
                "a boundary marker selects the nearer structural endpoint"
            );
            assert_eq!(
                tree.selected,
                Some(topic),
                "count boundaries never become selectable nodes"
            );

            tree.update_hover(Some(passing), true);
            assert!(
                tree.select_at(label_x, bounds.y + topic_point.y as u16),
                "label hits retain priority over the active hovered route"
            );
            assert_eq!(tree.selected, Some(topic), "the label selects its own node");
            app.topological_preview = true;
            assert!(
                tree.update_hover(Some(Position::new(label_x, bounds.y + topic_point.y as u16)), true),
                "leaving the graph clears its hover override"
            );
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            let overview = tree.overview.as_ref().expect("the overview exists");
            let expected: Vec<_> = overview
                .edges
                .iter()
                .enumerate()
                .filter_map(|(index, edge)| (edge.child == topic || edge.parent == topic).then_some(index))
                .collect();
            assert_eq!(
                tree.placed.as_ref().expect("the rail is placed").highlighted_edges,
                expected,
                "leaving restores the selected node's Shift preview"
            );
            tree.update_hover(Some(passing), true);
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            assert!(
                tree.update_hover(Some(passing), false),
                "releasing Alt clears the hover override"
            );
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            assert_eq!(
                tree.placed.as_ref().expect("the rail is placed").highlighted_edges,
                expected,
                "releasing Alt preserves Shift's selected-node preview"
            );
            assert!(
                !tree.select_at(passing.x, passing.y),
                "releasing Alt disables graph-line clicks"
            );
            app.topological_preview = false;
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            assert!(
                tree.placed
                    .as_ref()
                    .expect("the rail is placed")
                    .highlighted_edges
                    .is_empty(),
                "releasing both modifiers clears all previews"
            );
        }
        Ok(())
    }

    #[test]
    fn clicking_a_rail_reveals_its_offscreen_endpoint() -> gix_error::TestResult {
        let (graph, refs, decorations) = fixture();
        for graphics_enabled in [false, true] {
            let mut tree = Tree::default();
            let mut app = App::new(10);
            app.graphics_enabled = graphics_enabled;
            tree.rebuild(&graph, &refs, &decorations);
            tree.toggle();
            tree.count_anchor = Some(id(6));
            let overview = tree.overview.as_ref().expect("the overview exists");
            let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
            let fork = overview.by_commit[&graph.index(id(2)).expect("fork exists")];
            tree.selected = Some(fork);
            tree.offset.y = 2;
            tree.ensure_visible = false;
            let bounds = Rect::new(4, 2, 32, 3);
            let mut terminal = Terminal::new(TestBackend::new(40, 8))?;
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            assert!(
                !tree.node_hits.iter().any(|(_, node)| *node == main),
                "the upper endpoint starts outside the viewport"
            );
            let pointer = Position::new(bounds.x, bounds.y);
            tree.update_hover(Some(pointer), true);
            assert_eq!(
                tree.hovered.map(|hover| hover.target),
                Some(main),
                "a tie picks the child endpoint"
            );
            assert!(
                tree.select_at(pointer.x, pointer.y),
                "the visible line selects its hidden endpoint"
            );
            assert_eq!(tree.selected, Some(main), "the line selects the nearer commit");
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            assert!(
                tree.node_hits.iter().any(|(_, node)| *node == main),
                "drawing reveals the endpoint chosen by the line click"
            );
        }
        Ok(())
    }

    #[test]
    fn rail_hover_recomputes_after_panning_and_respects_overlays_and_choices() -> gix_error::TestResult {
        let (graph, refs, decorations) = fixture();
        for graphics_enabled in [false, true] {
            let mut tree = Tree::default();
            let mut app = App::new(10);
            app.graphics_enabled = graphics_enabled;
            tree.rebuild(&graph, &refs, &decorations);
            tree.toggle();
            tree.count_anchor = Some(id(6));
            let mut terminal = Terminal::new(TestBackend::new(80, 14))?;
            let bounds = Rect::new(4, 2, 32, 3);
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            let pointer = Position::new(bounds.x, bounds.y + 1);
            assert!(
                tree.update_hover(Some(pointer), true),
                "the initial visible route is hoverable"
            );
            let original = tree.hovered.expect("the pointer is on a route");
            tree.pan(Direction::Down, usize::MAX);
            terminal.draw(|frame| tree.draw(frame, bounds, Some(&graph), &mut app))?;
            assert_ne!(
                tree.hovered,
                Some(original),
                "drawing remaps a stationary pointer after scrolling"
            );
            let hovered = tree.hovered.expect("the final root has an incoming route");
            assert_eq!(
                tree.placed.as_ref().expect("the rail is placed").highlighted_edges,
                [hovered.edge],
                "the same draw updates the rendered preview"
            );
            assert!(
                tree.update_hover(Some(Position::new(bounds.x, bounds.bottom() - 1)), true),
                "footer cells clear the hover"
            );
            assert!(tree.hovered.is_none(), "the footer does not expose the rail beneath it");

            tree.ensure_visible = true;
            app.information_expanded = true;
            let large = Rect::new(0, 0, 80, 14);
            terminal.draw(|frame| tree.draw(frame, large, Some(&graph), &mut app))?;
            let popup = tree.mouse_overlay.expect("the information popup is visible");
            for position in popup.positions() {
                tree.update_hover(Some(position), true);
                assert!(tree.hovered.is_none(), "popup cells cannot hover hidden graph lines");
            }
            app.information_expanded = false;
            let fork = tree.overview.as_ref().expect("the overview exists").by_commit
                [&graph.index(id(2)).expect("fork exists")];
            tree.selected = Some(fork);
            tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &mut app);
            terminal.draw(|frame| tree.draw(frame, large, Some(&graph), &mut app))?;
            assert!(tree.topological_choice.is_some(), "the fork starts a numbered choice");
            tree.update_hover(Some(Position::new(0, 0)), true);
            assert!(tree.hovered.is_none(), "numbered choices suppress hover overrides");
            assert!(!tree.select_at(0, 0), "line clicks cannot bypass numbered choices");
            tree.leave();
            assert!(tree.hover_position.is_none(), "leaving clears the saved pointer");
        }
        Ok(())
    }

    #[test]
    fn graphical_rail_rounding_preserves_layout_and_fallback_paths() {
        let (graph, refs, decorations) = fixture();
        let overview = Overview::new(&graph, &refs, &decorations, true);
        let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
        let overlay = Overlay::new(&graph, &overview, main);
        for overlay in [None, Some(&overlay)] {
            let highlighted = (0..overview.edges.len()).collect::<Vec<_>>();
            let rounded = place_rail(&overview, overlay, true, highlighted.clone());
            let text = place_rail(&overview, overlay, false, highlighted);
            assert_eq!(
                rounded.nodes, text.nodes,
                "rounding keeps node and mouse-target positions"
            );
            assert_eq!(
                rounded.boundaries, text.boundaries,
                "count boundaries stay in their existing rows"
            );
            assert_eq!(
                (rounded.width, rounded.height),
                (text.width, text.height),
                "rounding preserves label placement and scroll bounds"
            );
            let mut curved_connectors = 0;
            let mut eased_joins = 0;
            for (row, text) in rounded.rail_rows.iter().zip(&text.rail_rows) {
                assert_eq!(row.lane, text.lane, "text fallback retains its exact connectors");
                let mut paths = row.tiles.clone();
                for tile in &mut paths {
                    tile.edges.clear();
                    tile.incoming = None;
                }
                assert_eq!(
                    paths, text.tiles,
                    "only graphical curve geometry changes, not markers, colors or highlights"
                );
                for edge in row.tiles.iter().flat_map(|tile| &tile.edges) {
                    if matches!(row.kind, RailRowKind::NodeConnector | RailRowKind::BoundaryConnector) {
                        assert!(
                            !edge.from_node,
                            "connector curves begin at the row boundary, below the node row"
                        );
                        curved_connectors += usize::from(edge.from != edge.to && edge.incoming.is_some());
                        eased_joins += usize::from(edge.from != edge.to && edge.end_vertical);
                    } else {
                        assert_eq!(
                            edge.from, edge.to,
                            "node rows retain straight stems above their connector curves"
                        );
                    }
                }
            }
            assert!(
                curved_connectors > 0,
                "connector turns share the preceding row's curve direction"
            );
            assert!(eased_joins > 0, "shared-parent arrivals ease into the vertical tail");
        }
    }

    #[test]
    fn graphical_rail_edges_join_at_cell_boundaries_and_stop_at_endpoints() {
        let (graph, refs, decorations) = fixture();
        let overview = Overview::new(&graph, &refs, &decorations, true);
        let placed = place_rail(&overview, None, true, Vec::new());
        let up = Tile::UP_DOWN | Tile::UP_LEFT | Tile::UP_RIGHT | Tile::NODE_UP;
        let down = Tile::UP_DOWN | Tile::DOWN_LEFT | Tile::DOWN_RIGHT | Tile::NODE_DOWN;
        let left = Tile::LEFT_RIGHT | Tile::UP_LEFT | Tile::DOWN_LEFT | Tile::NODE_LEFT;
        let right = Tile::LEFT_RIGHT | Tile::UP_RIGHT | Tile::DOWN_RIGHT | Tile::NODE_RIGHT;
        for (y, row) in placed.rail_rows.iter().enumerate() {
            for (x, tile) in row.tiles.iter().enumerate() {
                let above = y
                    .checked_sub(1)
                    .and_then(|y| placed.rail_rows[y].tiles.get(x))
                    .cloned()
                    .unwrap_or_default();
                let before = x
                    .checked_sub(1)
                    .and_then(|x| row.tiles.get(x))
                    .cloned()
                    .unwrap_or_default();
                assert_eq!(
                    tile.paths & up != 0,
                    above.paths & down != 0,
                    "vertical edge continuity at ({x}, {y})"
                );
                assert_eq!(
                    tile.paths & left != 0,
                    before.paths & right != 0,
                    "horizontal edge continuity at ({x}, {y})"
                );
            }
        }
        for (index, node) in overview.nodes.iter().enumerate() {
            let point = placed.nodes[index];
            let tile = &placed.rail_rows[point.y].tiles[point.x];
            assert_eq!(
                tile.paths & Tile::NODE_UP != 0,
                !node.children.is_empty(),
                "only nodes with children have an incoming arm"
            );
            assert_eq!(
                tile.paths & Tile::NODE_DOWN != 0,
                node.parent.is_some(),
                "only nodes with parents have an outgoing arm"
            );
        }
    }

    #[test]
    fn graphical_rail_colors_follow_branches_through_connectors_and_boundaries() {
        let (graph, refs, decorations) = fixture();
        let overview = Overview::new(&graph, &refs, &decorations, true);
        let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
        let topic = overview.by_commit[&graph.index(id(5)).expect("topic exists")];
        let fork = overview.by_commit[&graph.index(id(2)).expect("fork exists")];
        let topic_edge = overview
            .edges
            .iter()
            .position(|edge| edge.child == topic)
            .expect("topic has a contracted edge");
        let overlay = Overlay::new(&graph, &overview, main);
        for overlay in [None, Some(&overlay)] {
            let placed = place_rail(&overview, overlay, true, Vec::new());
            let text = place_rail(&overview, overlay, false, Vec::new());
            assert_eq!(
                placed.rail_rows.iter().map(|row| &row.lane).collect::<Vec<_>>(),
                text.rail_rows.iter().map(|row| &row.lane).collect::<Vec<_>>(),
                "route coloring leaves text geometry unchanged"
            );
            let main = placed.nodes[main];
            let topic = placed.nodes[topic];
            let fork = placed.nodes[fork];
            let main_color = placed.rail_rows[main.y].tiles[main.x].path_colors[Tile::NODE_DOWN.ilog2() as usize];
            let topic_color = placed.rail_rows[topic.y].tiles[topic.x].path_colors[Tile::NODE_DOWN.ilog2() as usize];
            assert_ne!(
                main_color, topic_color,
                "concurrent branches have distinct route colors"
            );
            for row in &placed.rail_rows[main.y + 1..fork.y] {
                assert_eq!(
                    row.tiles[main.x].path_colors[Tile::UP_DOWN.ilog2() as usize],
                    main_color,
                    "the main route keeps its color as topic nodes and connectors pass it"
                );
            }
            assert_eq!(
                placed.rail_rows[fork.y].tiles[fork.x].path_colors[Tile::NODE_UP.ilog2() as usize],
                main_color,
                "the shared tail retains the established route color"
            );
            let mut horizontal = 0;
            for row in &placed.rail_rows[topic.y..fork.y] {
                for tile in &row.tiles {
                    for path in [
                        Tile::LEFT_RIGHT,
                        Tile::UP_LEFT,
                        Tile::UP_RIGHT,
                        Tile::DOWN_LEFT,
                        Tile::DOWN_RIGHT,
                    ] {
                        if tile.paths & path != 0 {
                            horizontal += 1;
                            assert_eq!(
                                tile.path_colors[path.ilog2() as usize],
                                topic_color,
                                "the turning topic route keeps its identity across columns"
                            );
                        }
                    }
                }
            }
            assert!(horizontal > 0, "the fixture exercises the separate connector row");
            if let Some(boundary) = placed.boundaries[topic_edge] {
                let tile = &placed.rail_rows[boundary.y].tiles[boundary.x];
                for path in [Tile::NODE_UP, Tile::NODE_DOWN] {
                    assert_eq!(
                        tile.path_colors[path.ilog2() as usize],
                        topic_color,
                        "the route retains its color across the inserted boundary marker"
                    );
                }
            }
            for row in &placed.rail_rows {
                for tile in &row.tiles {
                    for (bit, color) in tile.path_colors.iter().enumerate() {
                        if tile.paths & (1 << bit) != 0 {
                            assert_ne!(*color, 0, "every displayed primitive retains its route color");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_arms_immediate_deletion_of_all_ordinary_local_branches() {
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
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.toggle(), "the ref-tree opens");

        let Input::ResolveRemoteReferences(remote) =
            tree.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), &mut app)
        else {
            panic!("a should request remote resolution")
        };
        assert_eq!(remote, ["refs/remotes/origin/main"]);
        assert!(app.actions_expanded);
        let Input::DeleteLocalBranches { names, .. } =
            tree.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), &mut app)
        else {
            panic!("d should submit branch deletion")
        };
        assert_eq!(
            names,
            ["refs/heads/also-main", "refs/heads/main"],
            "d immediately submits every ordinary local branch and no protected decoration"
        );
        assert!(
            app.actions_expanded,
            "a tapped Actions group remains open after its command"
        );
    }

    #[test]
    fn a_r_deletes_every_resolved_remote_reference_without_confirmation() {
        let (graph, refs, mut decorations) = fixture();
        decorations
            .get_mut(&id(6))
            .expect("main is decorated")
            .push(Decoration {
                name: "origin/main".into(),
                kind: DecorationKind::Remote,
            });
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.toggle());
        assert!(matches!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), &mut app),
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
            tree.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE), &mut app),
            Input::DeleteRemoteReferences { groups: submitted, .. } if submitted == groups
        ));
        assert!(
            app.actions_expanded,
            "a tapped Actions group remains open after its command"
        );
    }

    #[test]
    fn resolves_only_remote_tracking_refs_with_an_upstream_mapping() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        for args in [
            &["config", "remote.origin.url", "."][..],
            &["config", "remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"][..],
        ] {
            assert!(
                gix_testtools::git_command(fixture.path())
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
            let status = gix_testtools::git_command(fixture.path()).args(args).status()?;
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
    #[cfg(unix)]
    fn remote_deletion_ctrl_c_stops_before_the_next_remote() -> gix::error::TestResult {
        use std::os::unix::fs::PermissionsExt;

        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        for remote in ["a-first", "z-second"] {
            assert!(
                gix_testtools::git_command(fixture.path())
                    .args(["remote", "add", remote, "."])
                    .status()?
                    .success(),
                "both remotes point to the disposable fixture"
            );
        }
        let repository = crate::test_repository::open(fixture.path())?;
        let hook = repository.common_dir().join("hooks/pre-push");
        std::fs::create_dir_all(hook.parent().expect("the hook has a directory"))?;
        std::fs::write(
            &hook,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$1\" >> \"$0.log\"\nkill -INT {}\nexit 1\n",
                std::process::id()
            ),
        )?;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
        let _signals = crate::interrupt::install()?;
        let outcome = crate::push_remote_deletions(
            fixture.path(),
            &[
                RemoteDeletion {
                    remote: "a-first".into(),
                    references: vec!["refs/heads/topic".try_into()?],
                },
                RemoteDeletion {
                    remote: "z-second".into(),
                    references: vec!["refs/heads/topic".try_into()?],
                },
            ],
        );
        assert_eq!(
            std::fs::read_to_string(hook.with_file_name("pre-push.log"))?,
            "a-first\n",
            "Ctrl-C stops the remote group before the second push starts"
        );
        assert_eq!(outcome.deleted, 0, "the interrupted push deletes no reference");
        assert_eq!(outcome.failures.len(), 1, "only the attempted remote reports a failure");
        assert!(
            !crate::interrupt::requested(),
            "foreground Ctrl-C is consumed after the entire remote group finishes"
        );
        Ok(())
    }

    #[test]
    fn escape_cancels_the_actions_prefix_without_leaving_the_ref_tree() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.toggle(), "the ref-tree opens");

        tree.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), &mut app);
        tree.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut app);

        assert!(tree.is_active(), "Escape cancels the armed prefix first");
        assert!(!app.actions_expanded);
    }

    #[test]
    fn quit_returns_to_history_before_closing_its_panels() {
        use crossterm::event::KeyEventKind;

        for key_input in [false, true] {
            let (graph, refs, decorations) = fixture();
            let mut tree = Tree::default();
            let mut app = App::new(10);
            app.state = crate::app::State::Complete;
            tree.rebuild(&graph, &refs, &decorations);
            assert!(tree.toggle(), "the ref-tree opens");
            app.show_commit = true;
            app.commit_fullscreen = true;
            app.actions_expanded = true;
            app.start_background_task("pushing topic to origin…");
            let selected = tree.selected;
            let quit = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
            for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
                assert_eq!(tree.handle_key(KeyEvent { kind, ..quit }, &mut app), Input::Handled);
                assert!(tree.is_active(), "quit repeats and releases keep the ref-tree open");
            }
            let input = if key_input {
                tree.handle_key(quit, &mut app)
            } else {
                tree.handle_action(Action::Quit, &mut app)
            };
            assert_eq!(input, Input::Handled, "quit returns to history without exiting tix");
            assert!(!tree.is_active(), "quit closes the ref-tree");
            assert!(!app.actions_expanded, "returning to history closes shortcut groups");
            assert_eq!(tree.selected, selected, "returning preserves the ref-tree selection");
            assert!(
                app.show_commit && app.commit_fullscreen,
                "history's message remains open"
            );
            assert!(app.changes_mode.is_some(), "history's changes remain open");
            assert!(
                app.background_progress().is_some(),
                "returning keeps the background task running"
            );
            assert!(app.update(Action::Quit).is_empty(), "the next quit closes the message");
            assert!(!app.show_commit);
            assert!(app.update(Action::Quit).is_empty(), "the next quit closes changes");
            assert!(app.changes_mode.is_none());
            assert!(
                app.update(Action::Quit).is_empty(),
                "normal exit waits for the task to finish"
            );
            app.finish_background_task();
            assert_eq!(
                app.update(Action::Quit),
                vec![crate::app::Effect::Quit],
                "only a quit after returning and closing history's panels exits"
            );
        }
    }

    #[test]
    fn control_c_still_quits_from_the_ref_tree() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.toggle(), "the ref-tree opens");
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), &mut app),
            Input::Quit,
            "Ctrl-C exits directly"
        );
        assert!(tree.is_active(), "Ctrl-C does not first return to history");
    }

    #[test]
    fn actions_toggle_ignore_repeats_and_close_on_navigation() {
        use crossterm::event::KeyEventKind;
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        tree.toggle();
        let prefix = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        tree.handle_key(prefix, &mut app);
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            tree.handle_key(KeyEvent { kind, ..prefix }, &mut app);
            assert!(app.actions_expanded, "repeat and release preserve a tapped prefix");
        }
        tree.handle_key(prefix, &mut app);
        assert!(!app.actions_expanded, "a second tap closes Actions");
        tree.handle_key(prefix, &mut app);
        tree.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE), &mut app);
        assert!(!app.actions_expanded, "tree navigation closes Actions");
        tree.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE), &mut app);
        assert!(!app.actions_expanded, "the former edit prefix is no longer bound");
    }

    #[test]
    fn compact_footer_moves_view_controls_and_navigation_into_popouts() -> gix_testtools::Result {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        tree.toggle();
        let mut terminal = Terminal::new(TestBackend::new(80, 16))?;
        let line = |terminal: &Terminal<TestBackend>, y| {
            (0..80)
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect::<String>()
        };
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        assert_eq!(
            line(&terminal, 15).trim(),
            "ref-tree · p command · view · z position · actions · ? help · q history",
            "the footer stays compact at ordinary terminal widths"
        );
        tree.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE), &mut app);
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        assert!(line(&terminal, 14).contains("counts:auto"), "count state moves to View");
        tree.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE), &mut app);
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        let screen = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(
            screen.contains("Shift+mouse cursor"),
            "Information keeps mouse navigation discoverable"
        );
        assert!(
            screen.contains("Shift+pages pan"),
            "Information keeps paging discoverable"
        );
        assert!(
            screen.contains("zz center · zt top · zb bottom"),
            "Information lists all viewport positioning shortcuts"
        );
        assert!(
            screen.contains("Alt/Option+hover preview line"),
            "help explains line previews"
        );
        assert!(
            screen.contains("Alt/Option+click jump to commit"),
            "help explains line selection"
        );
        Ok(())
    }

    #[test]
    fn view_and_help_share_prefix_dispatch_hold_and_menu_scopes() -> gix_testtools::Result {
        use crate::{
            CommandMenuInput,
            menu::Menu,
            prefix_input::{Outcome, State},
        };
        use crossterm::event::{Event, KeyEventKind};
        use std::time::{Duration, Instant};

        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        tree.toggle();
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        tree.handle_key(key(KeyCode::Char('v')), &mut app);
        tree.handle_key(key(KeyCode::Char('t')), &mut app);
        assert!(tree.hide_tags, "v t toggles the tree's tags");
        assert!(
            app.history_display_expanded,
            "View stays open for consecutive display changes"
        );
        tree.handle_key(key(KeyCode::Char('c')), &mut app);
        assert_eq!(
            tree.count_anchor,
            tree.selected_id(),
            "v c anchors counts at the selection"
        );
        tree.handle_key(key(KeyCode::Char('?')), &mut app);
        assert!(!app.history_display_expanded, "Help closes View");
        tree.handle_key(key(KeyCode::Esc), &mut app);
        assert!(tree.is_active(), "Escape closes Help before leaving the tree");

        let mut terminal = Terminal::new(TestBackend::new(80, 16))?;
        for (group, command) in [
            (CommandGroup::View, CommandId::Tags),
            (CommandGroup::Viewport, CommandId::ViewportCenter),
            (CommandGroup::Information, CommandId::RefTreeTop),
        ] {
            let mut input = State::default();
            let now = Instant::now();
            let prefix = key(KeyCode::Char(group.prefix()));
            assert_eq!(
                input.handle_with(&Event::Key(prefix), &mut app, now, true, &[group], |key, app| tree
                    .action(key, app)),
                Outcome::Pass
            );
            tree.handle_key(prefix, &mut app);
            assert!(input.promote(&mut app, now + Duration::from_millis(300)));
            terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
            assert_eq!(
                app.held_prefix_selection(),
                Some(command),
                "holding selects the first rendered command in this group"
            );
            let release = Event::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..prefix
            });
            assert_eq!(
                input.handle_with(&release, &mut app, now, true, &[group], |key, app| tree
                    .action(key, app)),
                Outcome::Submit(command)
            );
        }
        let commands = tree.commands();
        let mut menu = Menu::default();
        menu.open(&crate::command_picker_items(&commands));
        crate::command_menu_input(&Event::Paste("v tags".into()), &mut menu, &commands);
        assert!(
            matches!(
                crate::command_menu_input(&Event::Key(key(KeyCode::Enter)), &mut menu, &commands),
                CommandMenuInput::Submit(Action::ToggleTags)
            ),
            "View commands use the shared scoped menu"
        );
        tree.handle_key(key(KeyCode::Char('?')), &mut app);
        tree.handle_key(key(KeyCode::Char('t')), &mut app);
        assert!(!tree.is_active(), "? t returns to history");
        Ok(())
    }

    #[test]
    fn tree_actions_share_held_navigation_help_and_single_submission() -> gix_testtools::Result {
        use crate::{
            command_menu::CommandGroup,
            prefix_input::{Outcome, State},
        };
        use crossterm::event::{Event, KeyEventKind};
        use std::time::{Duration, Instant};

        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        let mut input = State::default();
        let mut terminal = Terminal::new(TestBackend::new(90, 12))?;
        tree.rebuild(&graph, &refs, &decorations);
        tree.toggle();
        let now = Instant::now();
        let event = |code, kind| Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind));
        let prefix = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(
            input.handle_with(
                &Event::Key(prefix),
                &mut app,
                now,
                true,
                &[CommandGroup::Actions],
                |key, app| tree.action(key, app)
            ),
            Outcome::Pass
        );
        tree.handle_key(prefix, &mut app);
        assert!(input.promote(&mut app, now + Duration::from_millis(300)));
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        assert_eq!(app.held_prefix_selection(), Some(CommandId::PinReferences));
        assert_eq!(
            input.handle_with(
                &event(KeyCode::Right, KeyEventKind::Press),
                &mut app,
                now,
                true,
                &[CommandGroup::Actions],
                |key, app| tree.action(key, app)
            ),
            Outcome::Handled
        );
        assert_eq!(
            app.held_prefix_selection(),
            Some(CommandId::DeleteLocalBranches),
            "holding navigates the rendered commands rather than the tree"
        );
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        let rendered = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(
            rendered.contains("Delete every eligible local branch"),
            "help follows the tree command's exact identity"
        );
        let release = event(KeyCode::Char('a'), KeyEventKind::Release);
        assert_eq!(
            input.handle_with(&release, &mut app, now, true, &[CommandGroup::Actions], |key, app| tree
                .action(key, app)),
            Outcome::Submit(CommandId::DeleteLocalBranches)
        );
        assert!(matches!(
            tree.handle_action(Action::DeleteLocalBranches, &mut app),
            Input::DeleteLocalBranches { .. }
        ));
        assert_eq!(
            input.handle_with(&release, &mut app, now, true, &[CommandGroup::Actions], |key, app| tree
                .action(key, app)),
            Outcome::Pass,
            "a repeated release cannot delete twice"
        );
        assert!(!app.actions_expanded);
        Ok(())
    }

    #[test]
    fn tree_popout_styles_shortcuts_and_cancels_unavailable_held_selections() -> gix_testtools::Result {
        use crate::command_menu::CommandGroup;
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        let mut terminal = Terminal::new(TestBackend::new(90, 12))?;
        tree.rebuild(&graph, &refs, &decorations);
        tree.toggle();
        app.start_held_prefix(CommandGroup::Actions);
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        let footer = terminal.backend().buffer();
        let key = &footer[(
            Line::raw("ref-tree · p command · view · z position · ").width() as u16,
            11,
        )];
        assert!(
            key.modifier.contains(Modifier::REVERSED),
            "the open prefix has reversed styling"
        );
        assert_eq!(key.bg, Color::Cyan, "the reversed prefix keeps its shortcut cyan");
        assert_eq!(app.held_prefix_selection(), Some(CommandId::PinReferences));
        terminal.backend_mut().resize(24, 1);
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        assert_eq!(app.held_prefix_group(), None, "a popup without room cancels browsing");
        assert_eq!(app.held_prefix_selection(), None, "hidden commands cannot execute");
        Ok(())
    }

    #[test]
    fn tree_menu_scopes_recalls_and_redraws_from_its_background() -> gix_testtools::Result {
        use crate::{CommandMenuInput, menu::Menu};
        use crossterm::event::Event;

        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        tree.toggle();
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE), &mut app),
            Input::OpenCommandMenu(Vec::new()),
            "p opens commands instead of pinning"
        );
        let commands = crate::view_commands(&app, &decorations, &tree);
        assert_eq!(
            commands
                .iter()
                .filter(|command| command.group == CommandGroup::Actions)
                .map(|command| command.id)
                .collect::<Vec<_>>(),
            [CommandId::PinReferences, CommandId::DeleteLocalBranches]
        );
        let mut menu = Menu::default();
        menu.open(&crate::command_picker_items(&commands));
        let enter = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(
            matches!(
                crate::command_menu_input(&enter, &mut menu, &commands),
                CommandMenuInput::Handled
            ),
            "the first opening has no default submission"
        );
        let mut terminal = Terminal::new(TestBackend::new(90, 12))?;
        let mut background = None;
        terminal.draw(|frame| {
            let area = frame.area();
            tree.draw(frame, area, Some(&graph), &mut app);
            background = Some((area, frame.buffer_mut().clone()));
            crate::draw_active_menu(frame, area, &mut app, &mut menu, &decorations, &tree);
        })?;
        crate::command_menu_input(&Event::Paste("a delete".into()), &mut menu, &commands);
        assert!(
            crate::redraw_menu(
                &mut terminal,
                background.as_ref(),
                &mut app,
                &mut menu,
                &decorations,
                &tree
            )?,
            "tree menus reuse the rendered tree background"
        );
        let one = Event::Key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert!(matches!(
            crate::command_menu_input(&one, &mut menu, &commands),
            CommandMenuInput::Submit(Action::DeleteLocalBranches)
        ));
        menu.open(&crate::command_picker_items(&commands));
        assert!(
            matches!(
                crate::command_menu_input(&enter, &mut menu, &commands),
                CommandMenuInput::Submit(Action::DeleteLocalBranches)
            ),
            "the exact tree command is recalled"
        );
        Ok(())
    }

    #[test]
    fn protected_branches_leave_no_delete_action() {
        let (graph, refs, mut decorations) = fixture();
        decorations.get_mut(&id(5)).expect("topic is decorated")[0].kind = DecorationKind::HeadPinBranch;
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        tree.selected = tree.overview.as_ref().and_then(|overview| {
            graph
                .index(id(5))
                .and_then(|index| overview.by_commit.get(&index).copied())
        });

        tree.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), &mut app);
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE), &mut app),
            Input::Handled
        );
        assert_eq!(tree.notice.as_ref().map(|notice| notice.text.as_str()), None);
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
        let rail = place_rail(&overview, Some(&overlay), false, Vec::new());
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
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        let mut terminal = Terminal::new(TestBackend::new(100, 18))?;
        terminal.draw(|frame| draw_tree(frame, &mut tree, &graph))?;
        let main = tree
            .selected
            .and_then(|selected| tree.overview.as_ref()?.nodes.get(selected))
            .map(|node| node.id)
            .expect("the initial selection exists");

        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE), &mut app);
        assert_eq!(tree.count_anchor, Some(main), "Space anchors counts to the cursor");
        tree.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT), &mut app);
        assert!(tree.overlay.is_some(), "anchored navigation retains reachability");
        assert!(tree.placed.is_some(), "anchored navigation retains tree placement");
        assert_eq!(
            tree.overlay.as_ref().map(|overlay| overlay.selected),
            graph.index(main),
            "the retained overlay still uses the anchored commit"
        );
        tree.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE), &mut app);
        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
        let screen: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            screen.contains("Space counts:main"),
            "the View popout identifies the fixed count anchor"
        );

        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE), &mut app);
        assert!(
            tree.overlay.is_none(),
            "moving the anchor invalidates reachability once"
        );
        terminal.draw(|frame| draw_tree(frame, &mut tree, &graph))?;
        let moved = tree
            .selected
            .and_then(|selected| tree.overview.as_ref()?.nodes.get(selected))
            .map(|node| node.id)
            .expect("the moved selection exists");
        assert_eq!(tree.count_anchor, Some(moved));

        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE), &mut app);
        assert!(
            tree.count_anchor.is_none(),
            "Space on the anchor restores automatic counts"
        );
        assert!(tree.overlay.is_some(), "clearing at the cursor reuses the same overlay");
        tree.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT), &mut app);
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
            head_reference: "main-worktree/HEAD".try_into().expect("valid"),
            label_id: id(6),
            checkout_name: "main-wt".into(),
            reference: Some("refs/heads/main".try_into().expect("valid")),
            is_current: true,
            is_detached: true,
        });
        refs.worktrees.push(crate::history::WorktreeCheckout {
            id: id(4),
            head_reference: "worktrees/foreign-wt/HEAD".try_into().expect("valid"),
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
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE), &mut app);
        let main = tree.count_anchor.expect("main is anchored");
        tree.rebuild(&graph, &refs, &decorations);
        assert_eq!(tree.count_anchor, Some(main), "refresh preserves a visible anchor");

        tree.selected = tree.overview.as_ref().and_then(|overview| {
            graph
                .index(id(3))
                .and_then(|commit| overview.by_commit.get(&commit).copied())
        });
        tree.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE), &mut app);
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
        tree.placed = tree
            .overview
            .as_ref()
            .map(|overview| place_rail(overview, None, false, Vec::new()));
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
            active_branch: None,
            #[cfg(feature = "blocking-network-client")]
            fetch_remote: None,
            worktrees: Vec::new(),
        };
        let decorations = Decorations::from([
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
        tree.placed = tree
            .overview
            .as_ref()
            .map(|overview| place_rail(overview, None, false, Vec::new()));
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
        tree.placed = tree
            .overview
            .as_ref()
            .map(|overview| place_rail(overview, None, false, Vec::new()));
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
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        let overview = tree.overview.as_ref().expect("overview exists");
        let root = overview.by_commit[&graph.index(id(1)).expect("root exists")];
        let fork = overview.by_commit[&graph.index(id(2)).expect("fork exists")];
        let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
        tree.selected = Some(root);

        tree.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT), &mut app);
        assert_eq!(tree.selected, Some(fork), "Shift-Up moves toward a leaf");
        tree.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT), &mut app);
        assert_eq!(tree.selected, Some(root), "Shift-Down moves toward the root");

        tree.placed = tree
            .overview
            .as_ref()
            .map(|overview| place_rail(overview, None, false, Vec::new()));
        let nearest = nearest(
            &tree.placed.as_ref().expect("the ref-tree is placed").nodes,
            root,
            Direction::Up,
        );
        tree.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), &mut app);
        assert_eq!(tree.selected, nearest, "plain Up immediately returns to nearest motion");

        tree.selected = Some(fork);
        tree.ensure_visible = false;
        tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &mut app);
        assert_eq!(
            tree.selected,
            Some(fork),
            "an ambiguous child does not move immediately"
        );
        assert!(tree.topological_choice.is_some(), "the child choice remains pending");
        assert!(tree.ensure_visible, "starting a choice reveals its source marker");

        tree.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL), &mut app);
        assert_eq!(tree.topological_choice, Some(0), "modified choice keys are ignored");
        tree.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE), &mut app);
        assert_eq!(tree.topological_choice, Some(1));
        tree.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE), &mut app);
        assert_eq!(tree.topological_choice, Some(0));
        tree.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), &mut app);
        let chosen = tree.overview.as_ref().expect("overview exists").nodes[fork].children[1];
        tree.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut app);
        assert_eq!(
            tree.selected,
            Some(chosen),
            "h wraps to the last child and Enter follows it"
        );
        assert!(tree.topological_choice.is_none(), "submission leaves choice mode");

        tree.handle_key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE), &mut app);
        assert_eq!(
            tree.selected,
            Some(fork),
            "J follows the unique visible parent immediately"
        );
        tree.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT), &mut app);
        assert!(tree.topological_choice.is_some());
        tree.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), &mut app);
        assert!(!app.actions_expanded, "unrelated keys are consumed while choosing");
        assert_eq!(
            tree.topological_choice,
            Some(0),
            "unrelated keys keep the choice active"
        );
        tree.handle_mouse(MouseEventKind::ScrollUp, KeyModifiers::NONE, 1);
        assert_eq!(tree.selected, Some(fork), "mouse input is consumed while choosing");
        assert_eq!(tree.topological_choice, Some(0), "mouse input keeps the choice active");
        tree.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut app);
        assert!(tree.topological_choice.is_none(), "Escape cancels the choice");

        tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &mut app);
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE), &mut app),
            Input::Handled,
            "quit returns to history while choosing"
        );
        assert!(!tree.is_active(), "quit closes the ref-tree while choosing");
        assert!(tree.topological_choice.is_none(), "quit cancels the pending choice");
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.topological_choice.is_none(), "rebuilding cancels a stale choice");

        tree.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE), &mut app);
        assert_eq!(tree.selected, Some(main), "plain g reaches the top selectable node");
        tree.selected = Some(main);
        tree.handle_key(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT), &mut app);
        assert_eq!(
            tree.selected,
            Some(root),
            "uppercase G reaches the current component root"
        );
        tree.selected = Some(main);
        tree.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::SHIFT), &mut app);
        assert_eq!(tree.selected, Some(root), "shift-modified lowercase g reaches the root");
    }

    #[test]
    fn topological_choices_highlight_the_complete_rail_and_clear_cached_highlights() -> gix_testtools::Result {
        let (graph, refs, decorations) = fixture();
        for graphics_enabled in [false, true] {
            let mut tree = Tree::default();
            let mut app = App::new(10);
            app.graphics_enabled = graphics_enabled;
            tree.rebuild(&graph, &refs, &decorations);
            let overview = tree.overview.as_ref().expect("the tree has an overview");
            let fork = overview.by_commit[&graph.index(id(2)).expect("the fork exists")];
            let topic = overview.by_commit[&graph.index(id(5)).expect("the topic exists")];
            let topic_edge = overview
                .edges
                .iter()
                .position(|edge| edge.child == topic)
                .expect("the topic has an edge");
            tree.count_anchor = Some(id(6));
            tree.selected = Some(fork);
            let mut terminal = Terminal::new(TestBackend::new(80, 18))?;
            terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
            let ordinary = terminal.backend().buffer().clone();

            tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &mut app);
            let mut previous = Vec::new();
            for choice in 0..2 {
                terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
                let overview = tree.overview.as_ref().expect("the tree has an overview");
                let child = overview.nodes[fork].children[choice];
                let placed = tree.placed.as_ref().expect("drawing places the tree");
                let edge = *placed
                    .highlighted_edges
                    .first()
                    .expect("the chosen edge is cached with its placement");
                assert_eq!(overview.edges[edge].child, child, "cycling selects the matching rail");
                let start = placed.nodes[child];
                let end = placed.nodes[fork];
                assert_eq!(
                    placed.rail_rows[start.y].tiles[start.x].highlighted_paths,
                    Tile::NODE_DOWN,
                    "the preview starts at the chosen child"
                );
                assert_eq!(
                    placed.rail_rows[end.y].tiles[end.x].highlighted_paths,
                    Tile::NODE_UP,
                    "the preview stops at the source marker"
                );
                for row in &placed.rail_rows[start.y..=end.y] {
                    assert!(
                        row.tiles.iter().any(|tile| tile.highlighted_paths != 0),
                        "every intervening rail row carries the selected route"
                    );
                }
                let mut highlighted = Vec::new();
                for (y, row) in placed.rail_rows.iter().enumerate() {
                    for (x, tile) in row
                        .tiles
                        .iter()
                        .enumerate()
                        .filter(|(_, tile)| tile.highlighted_paths != 0)
                    {
                        highlighted.push((x, y, tile.highlighted_paths));
                        let cell = &terminal.backend().buffer()[(x as u16, y as u16)];
                        if graphics_enabled {
                            assert_eq!(
                                Tile::decode(cell.symbol())
                                    .expect("graphics carry tile metadata")
                                    .highlighted_paths,
                                tile.highlighted_paths,
                                "Kitty receives the exact selected arms"
                            );
                        } else if tile.marker == ' ' {
                            assert_eq!(cell.fg, Color::LightCyan, "text previews color selected graph glyphs");
                            assert!(
                                cell.modifier.contains(Modifier::BOLD) && !cell.modifier.contains(Modifier::DIM),
                                "reachability dimming does not obscure the preview"
                            );
                        }
                    }
                }
                if edge == topic_edge {
                    let boundary =
                        placed.boundaries[edge].expect("the anchored overlay inserts a reachability boundary");
                    assert_eq!(
                        placed.rail_rows[boundary.y].tiles[boundary.x].highlighted_paths,
                        Tile::NODE_UP | Tile::NODE_DOWN,
                        "the route continues through its reachability marker"
                    );
                    assert!(
                        placed
                            .rail_rows
                            .iter()
                            .any(|row| matches!(row.kind, RailRowKind::BoundaryConnector)
                                && row.tiles.iter().any(|tile| tile.highlighted_paths != 0)),
                        "the boundary connector also remains highlighted"
                    );
                }
                assert_ne!(
                    highlighted, previous,
                    "cycling rebuilds the selected path instead of retaining stale cached tiles"
                );
                previous = highlighted;
                tree.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE), &mut app);
            }
            tree.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut app);
            terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
            assert!(
                tree.placed
                    .as_ref()
                    .expect("drawing retains a placement")
                    .highlighted_edges
                    .is_empty(),
                "cancel clears the cached path even with fixed counts"
            );
            assert_eq!(
                terminal.backend().buffer(),
                &ordinary,
                "cancel restores the complete ordinary graph in both renderers"
            );
        }
        Ok(())
    }

    #[test]
    fn held_shift_previews_all_rails_until_a_numbered_choice_is_started() -> gix_error::TestResult {
        let (graph, refs, mut decorations) = fixture();
        decorations.insert(
            id(3),
            vec![Decoration {
                name: "middle".into(),
                kind: DecorationKind::Local,
            }],
        );
        for graphics_enabled in [false, true] {
            let mut tree = Tree::default();
            let mut app = App::new(10);
            app.graphics_enabled = graphics_enabled;
            tree.rebuild(&graph, &refs, &decorations);
            tree.count_anchor = Some(id(6));
            let mut terminal = Terminal::new(TestBackend::new(80, 20))?;
            for (selected_id, paths) in [
                (
                    id(2),
                    vec![
                        (1, Tile::NODE_UP),
                        (2, Tile::NODE_UP | Tile::NODE_DOWN),
                        (3, Tile::NODE_DOWN),
                        (5, Tile::NODE_DOWN),
                    ],
                ),
                (
                    id(3),
                    vec![
                        (2, Tile::NODE_UP),
                        (3, Tile::NODE_UP | Tile::NODE_DOWN),
                        (6, Tile::NODE_DOWN),
                    ],
                ),
            ] {
                tree.selected = tree
                    .overview
                    .as_ref()
                    .and_then(|overview| overview.nodes.iter().position(|node| node.id == selected_id));
                let selected = tree.selected;
                app.topological_preview = true;
                terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
                let overview = tree.overview.as_ref().expect("the tree has an overview");
                let placed = tree.placed.as_ref().expect("drawing places the tree");
                assert_eq!(
                    placed.highlighted_edges.len(),
                    paths.len() - 1,
                    "all available routes appear, including a unique parent and child"
                );
                for (index, node) in overview.nodes.iter().enumerate() {
                    let point = placed.nodes[index];
                    let expected = paths
                        .iter()
                        .find(|(n, _)| id(*n) == node.id)
                        .map_or(0, |(_, path)| *path);
                    assert_eq!(
                        placed.rail_rows[point.y].tiles[point.x].highlighted_paths, expected,
                        "only the source and its immediate destinations receive highlighted node arms"
                    );
                }
                assert_eq!(tree.selected, selected, "showing routes does not move the selection");
                assert!(
                    tree.topological_choice.is_none(),
                    "showing routes does not start a choice"
                );
                if selected_id == id(2) {
                    tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &mut app);
                    for choice in 0..2 {
                        if choice != 0 {
                            tree.handle_key(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE), &mut app);
                        }
                        terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
                        let placed = tree.placed.as_ref().expect("the rail is placed");
                        assert_eq!(
                            placed.highlighted_edges.len(),
                            1,
                            "a numbered choice highlights only its route while Shift is held"
                        );
                        let overview = tree.overview.as_ref().expect("the tree has an overview");
                        let child = overview.nodes[selected.expect("the source exists")].children[choice];
                        assert_eq!(
                            overview.edges[placed.highlighted_edges[0]].child, child,
                            "cycling changes the highlighted route while Shift remains held"
                        );
                    }
                }
                app.topological_preview = false;
                terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
                let placed = tree.placed.as_ref().expect("drawing retains a placement");
                if selected_id == id(2) {
                    assert_eq!(
                        tree.topological_choice,
                        Some(1),
                        "releasing Shift preserves the chosen child"
                    );
                    let overview = tree.overview.as_ref().expect("the tree has an overview");
                    let child = overview.nodes[selected.expect("the source exists")].children[1];
                    assert_eq!(
                        placed.highlighted_edges.len(),
                        1,
                        "release keeps the single chosen route"
                    );
                    assert_eq!(overview.edges[placed.highlighted_edges[0]].child, child);
                    app.topological_preview = true;
                    tree.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut app);
                    terminal.draw(|frame| tree.draw(frame, frame.area(), Some(&graph), &mut app))?;
                    assert_eq!(
                        tree.placed
                            .as_ref()
                            .expect("the rail is placed")
                            .highlighted_edges
                            .len(),
                        3,
                        "cancelling a choice while Shift is held restores both directions"
                    );
                } else {
                    assert!(
                        placed.highlighted_edges.is_empty(),
                        "release clears the cached route preview"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn viewport_commands_position_the_selected_row_with_final_dimensions_and_existing_clamps() -> gix_error::TestResult
    {
        let commits: Vec<_> = (1..=12)
            .map(|n| (id(n), (n > 1).then(|| id(n - 1)).into_iter().collect()))
            .collect();
        let graph = HistoryGraph::from_test_commits(&commits);
        let refs = RefSnapshot {
            view: HashMap::new(),
            hidden: HashMap::new(),
            view_tips: vec![id(12)],
            hidden_tips: Vec::new(),
            pins: Vec::new(),
            active_branch: None,
            #[cfg(feature = "blocking-network-client")]
            fetch_remote: None,
            worktrees: Vec::new(),
        };
        let mut decorations = Decorations::new();
        for n in 1..=12 {
            decorations.insert(
                id(n),
                vec![Decoration {
                    name: format!("branch-{n:02}").into(),
                    kind: DecorationKind::Local,
                }],
            );
        }
        for graphics_enabled in [false, true] {
            let mut tree = Tree::default();
            let mut app = App::new(10);
            app.graphics_enabled = graphics_enabled;
            tree.rebuild(&graph, &refs, &decorations);
            tree.toggle();
            tree.count_anchor = Some(id(12));
            let overview = tree.overview.as_ref().expect("the overview exists");
            let middle = overview.by_commit[&graph.index(id(6)).expect("the middle commit exists")];
            let first = overview.by_commit[&graph.index(id(12)).expect("the first commit exists")];
            let last = overview.by_commit[&graph.index(id(1)).expect("the last commit exists")];
            tree.selected = Some(middle);
            let mut terminal = Terminal::new(TestBackend::new(40, 12))?;
            let small = Rect::new(3, 2, 8, 3);
            let final_area = Rect::new(3, 2, 8, 6);
            for (position, viewport_row) in [
                (ViewportPosition::Top, 0),
                (ViewportPosition::Center, 2),
                (ViewportPosition::Bottom, 4),
            ] {
                tree.offset.x = 1;
                tree.offset.y = 0;
                tree.ensure_visible = false;
                terminal.draw(|frame| tree.draw(frame, small, Some(&graph), &mut app))?;
                app.viewport_expanded = true;
                tree.handle_action(Action::PositionSelection(position), &mut app);
                assert!(!app.viewport_expanded, "positioning closes the viewport prefix");
                assert_eq!(tree.offset.y, 0, "positioning waits for the final layout");
                terminal.draw(|frame| tree.draw(frame, final_area, Some(&graph), &mut app))?;
                let placed = tree.placed.as_ref().expect("the tree is placed");
                assert_eq!(
                    placed.nodes[middle].y - tree.offset.y,
                    viewport_row,
                    "the selected row reaches the requested position in the final five-row body"
                );
                assert_eq!(tree.selected, Some(middle), "positioning preserves the selected node");
                assert_eq!(
                    tree.count_anchor,
                    Some(id(12)),
                    "positioning preserves the count anchor"
                );
                assert_eq!(tree.offset.x, 1, "positioning preserves horizontal panning");
                assert!(tree.viewport_position.is_none(), "the request is consumed once");
                tree.pan(Direction::Down, 1);
                let offset = tree.offset.y;
                terminal.draw(|frame| tree.draw(frame, final_area, Some(&graph), &mut app))?;
                assert_eq!(tree.offset.y, offset, "later draws do not reapply the viewport command");
            }
            for (selected, position, offset) in [(first, ViewportPosition::Bottom, 0), (last, ViewportPosition::Top, 7)]
            {
                tree.selected = Some(selected);
                tree.handle_action(Action::PositionSelection(position), &mut app);
                terminal.draw(|frame| tree.draw(frame, final_area, Some(&graph), &mut app))?;
                assert_eq!(
                    tree.offset.y, offset,
                    "boundary rows use the closest position without overscrolling"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn viewport_prefix_dispatch_cancellation_repeats_and_discoverability() -> gix_error::TestResult {
        use crossterm::event::KeyEventKind;

        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        tree.toggle();
        let prefix = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE);
        let commands = tree.commands();
        for (suffix, position, command_id, shortcut) in [
            ('z', ViewportPosition::Center, CommandId::ViewportCenter, "zz"),
            ('t', ViewportPosition::Top, CommandId::ViewportTop, "zt"),
            ('b', ViewportPosition::Bottom, CommandId::ViewportBottom, "zb"),
        ] {
            let command = commands
                .iter()
                .find(|command| command.id == command_id)
                .expect("viewport commands are discoverable");
            assert_eq!(
                (command.group, command.shortcut, command.key()),
                (CommandGroup::Viewport, shortcut, Some(suffix)),
                "the picker and popup expose the same two-key command"
            );
            tree.handle_key(prefix, &mut app);
            for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
                tree.handle_key(KeyEvent { kind, ..prefix }, &mut app);
                assert!(app.viewport_expanded, "prefix repeat and release leave the popup open");
                assert!(
                    tree.viewport_position.is_none(),
                    "holding z does not prematurely center"
                );
            }
            let suffix = KeyEvent::new(KeyCode::Char(suffix), KeyModifiers::NONE);
            tree.handle_key(suffix, &mut app);
            assert_eq!(
                tree.viewport_position,
                Some(position),
                "the second key chooses its viewport position"
            );
            for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
                tree.handle_key(KeyEvent { kind, ..suffix }, &mut app);
                assert!(
                    !app.viewport_expanded,
                    "suffix repeat and release do not reopen the prefix"
                );
                assert!(tree.is_active(), "zt and its repeats never execute direct t");
            }
            tree.viewport_position = None;
        }
        tree.handle_key(prefix, &mut app);
        tree.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut app);
        assert!(
            tree.is_active() && !app.viewport_expanded,
            "Escape cancels z before leaving the tree"
        );
        assert!(
            tree.viewport_position.is_none(),
            "cancel does not reposition the selection"
        );
        let fork =
            tree.overview.as_ref().expect("the overview exists").by_commit[&graph.index(id(2)).expect("fork exists")];
        tree.selected = Some(fork);
        tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &mut app);
        assert!(tree.topological_choice.is_some(), "the fork has a numbered choice");
        tree.handle_key(prefix, &mut app);
        tree.handle_action(Action::PositionSelection(ViewportPosition::Center), &mut app);
        tree.handle_action(Action::ToggleViewport, &mut app);
        assert!(
            !app.viewport_expanded && tree.viewport_position.is_none(),
            "numbered choices suppress viewport commands"
        );
        Ok(())
    }

    #[test]
    fn plain_pages_move_the_cursor_while_shift_pages_and_plain_mouse_pan() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        tree.placed = tree
            .overview
            .as_ref()
            .map(|overview| place_rail(overview, None, false, Vec::new()));
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
        tree.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE), &mut app);
        let full_page = tree.selected.expect("PageDown retains a selection");
        let full_page_y = points[full_page].y;
        assert!(full_page_y > first_y, "plain PageDown advances the ref-tree cursor");
        assert!(
            tree.ensure_visible,
            "plain page navigation keeps the new cursor visible"
        );

        tree.selected = Some(first);
        tree.offset.y = 0;
        tree.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::SHIFT), &mut app);
        assert_eq!(tree.selected, Some(first), "Shift-PageDown leaves the cursor alone");
        assert!(tree.offset.y > 0, "Shift-PageDown pans the viewport");

        tree.selected = Some(first);
        tree.offset.y = 0;
        tree.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL), &mut app);
        let half_page_y = points[tree.selected.expect("Ctrl-d retains a selection")].y;
        assert!(
            half_page_y > first_y && half_page_y <= full_page_y,
            "plain half-page navigation advances no farther than a full page"
        );

        tree.selected = Some(first);
        tree.offset.y = 0;
        tree.handle_key(
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            &mut app,
        );
        assert_eq!(tree.selected, Some(first), "Shift-Ctrl-d leaves the cursor alone");
        assert!(tree.offset.y > 0, "Shift-Ctrl-d pans by half a viewport");

        tree.selected = Some(first);
        tree.placed = tree
            .overview
            .as_ref()
            .map(|overview| place_rail(overview, None, false, Vec::new()));
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
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);

        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut app),
            Input::PinReferences {
                id: id(6),
                kinds: vec![DecorationKind::Local, DecorationKind::Remote, DecorationKind::Tag],
            },
            "Enter retains every displayed reference namespace"
        );
        tree.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), &mut app);
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE), &mut app),
            Input::PinReferences {
                id: id(6),
                kinds: vec![DecorationKind::Local, DecorationKind::Remote, DecorationKind::Tag],
            },
            "a i retains every displayed reference namespace"
        );
        tree.selected = tree
            .overview
            .as_ref()
            .and_then(|overview| overview.by_commit.get(&graph.index(id(2))?).copied());
        assert_eq!(
            tree.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut app),
            Input::Handled,
            "fork-only nodes have no pin action"
        );
        tree.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE), &mut app);
        assert!(
            tree.commands()
                .iter()
                .all(|command| command.group != CommandGroup::Actions),
            "synthetic nodes offer no pin or deletion commands"
        );
    }

    #[test]
    fn enter_pins_foreign_detached_worktrees() {
        let (graph, refs, mut decorations) = fixture();
        for kind in [
            DecorationKind::WorktreeDetached,
            DecorationKind::CurrentWorktreeDetached,
        ] {
            decorations.get_mut(&id(6)).expect("main is decorated")[0] = Decoration {
                name: "experiment".into(),
                kind,
            };
            let mut tree = Tree::default();
            let mut app = App::new(10);
            tree.rebuild(&graph, &refs, &decorations);
            assert!(tree.toggle(), "the reference tree opens at HEAD");
            assert_eq!(
                tree.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &mut app),
                if kind == DecorationKind::WorktreeDetached {
                    Input::PinReferences {
                        id: id(6),
                        kinds: vec![DecorationKind::WorktreeDetached],
                    }
                } else {
                    Input::Handled
                },
                "Enter pins the foreign worktree, while the current detached checkout stays inert"
            );
        }
    }

    #[test]
    fn pin_detached_worktrees_tracks_physical_heads_and_reuses_each_pin() -> gix_testtools::Result {
        let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
        let main = crate::test_repository::open(fixture.path())?;
        let commit_id = main.rev_parse_single("main~2")?.detach();
        let remembered_commit_id = main.head_id()?.detach();
        let interrupt = AtomicBool::default();
        let checkouts = fixture.path().join("checkouts");
        std::fs::create_dir(&checkouts)?;
        let git = |path: &std::path::Path, args: &[&str]| -> gix_testtools::Result {
            let output = gix_testtools::git_command(path).args(args).output()?;
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(())
        };
        for name in ["source", "main", "second"] {
            main.add_worktree(
                checkouts.join(name),
                gix::worktree::add::Head::Detached(commit_id),
                gix::progress::Discard,
                &interrupt,
            )?;
        }
        git(fixture.path(), &["switch", "--detach", &commit_id.to_string()])?;
        let source = crate::test_repository::open(checkouts.join("source"))?;
        let linked = crate::test_repository::open(checkouts.join("main"))?;
        crate::edit::time_travel::create_named_pin(
            &linked,
            crate::history::HEAD_PIN_NAME.as_bstr().try_into()?,
            gix::refs::Target::Symbolic("refs/heads/main".try_into()?),
            remembered_commit_id,
            "remember the branch separately from the worktree HEAD",
        )?;
        let linked_pins = crate::history::all_pins(&linked)?;

        let (pins, changes) = pin_references_reporting(&source, commit_id, &[DecorationKind::WorktreeDetached])?;
        assert_eq!(
            pins.len(),
            3,
            "every foreign detached worktree at this commit gets a pin"
        );
        assert_eq!(changes.len(), 3, "each new pin is recorded for undo");
        assert_eq!(
            pins.iter()
                .filter_map(|pin| pin.target.try_name().map(gix::refs::FullNameRef::as_bstr))
                .collect::<Vec<_>>(),
            ["main-worktree/HEAD", "worktrees/main/HEAD", "worktrees/second/HEAD"],
            "main and linked worktrees remain distinct even when a linked worktree is named main"
        );
        assert!(
            pins.iter().all(|pin| pin.id == commit_id),
            "pins use physical HEAD rather than the remembered branch"
        );
        let (reused, changes) = pin_references_reporting(&source, commit_id, &[DecorationKind::WorktreeDetached])?;
        assert_eq!(reused, pins, "repeated pinning reuses each worktree target");
        assert!(changes.is_empty(), "reused pins create no additional undo changes");
        assert!(
            crate::history::all_pins(&main)?.is_empty(),
            "pins stay private to the source"
        );
        assert_eq!(
            crate::history::all_pins(&linked)?,
            linked_pins,
            "target pins stay unchanged"
        );

        for (name, pin) in ["", "main", "second"].into_iter().zip(&pins) {
            let path = if name.is_empty() {
                fixture.path().to_owned()
            } else {
                checkouts.join(name)
            };
            git(&path, &["commit", "--allow-empty", "-m", &format!("advance {name}")])?;
            if name == "main" {
                git(&path, &["switch", "main"])?;
                git(&path, &["commit", "--allow-empty", "-m", "advance attached worktree"])?;
            }
            let worktree = crate::test_repository::open(&path)?;
            let advanced_commit_id = worktree.head_id()?.detach();
            let mut reference = source.find_reference(pin.name.as_ref())?;
            assert_eq!(
                reference.target().into_owned(),
                pin.target,
                "the pin keeps its HEAD target"
            );
            assert_eq!(
                reference.peel_to_id()?,
                advanced_commit_id,
                "the pin follows external HEAD changes"
            );
            assert!(
                crate::history::snapshot(&source, &[], &[], false)?
                    .view_tips
                    .contains(&advanced_commit_id),
                "the updated worktree tip enters source history"
            );
        }
        assert!(
            pin_references(&source, commit_id, &[DecorationKind::WorktreeDetached])?.is_empty(),
            "a stale selection creates no pin after the foreign heads move away"
        );
        assert_eq!(
            source.head_id()?,
            commit_id,
            "pinning never changes the source checkout"
        );
        Ok(())
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
            gix_testtools::git_command(fixture.path())
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
        let mut app = App::new(10);
        tree.rebuild(&graph, &refs, &decorations);
        assert!(tree.toggle(), "the available ref-tree opens");
        let tagged = graph.index(id(3)).expect("tagged commit exists");
        assert!(
            tree.overview
                .as_ref()
                .is_some_and(|overview| overview.by_commit.contains_key(&tagged)),
            "tags retain otherwise linear commits"
        );

        tree.handle_key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT), &mut app);
        assert!(tree.hide_tags, "uppercase T hides tags");
        assert!(
            tree.overview
                .as_ref()
                .is_some_and(|overview| !overview.by_commit.contains_key(&tagged)),
            "a tag-only linear node disappears from the projection"
        );

        tree.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::SHIFT), &mut app);
        assert!(
            tree.overview
                .as_ref()
                .is_some_and(|overview| overview.by_commit.contains_key(&tagged)),
            "shift-modified lowercase t restores tag nodes"
        );
        tree.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE), &mut app);
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
        let mut app = App::new(10);
        tree.set_history_commits([id(6)]);
        tree.rebuild(&graph, &refs, &decorations);
        let mut terminal = Terminal::new(TestBackend::new(100, 18))?;

        terminal.draw(|frame| draw_tree(frame, &mut tree, &graph))?;
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
        terminal.draw(|frame| draw_tree(frame, &mut tree, &graph))?;
        assert_eq!(
            terminal.backend().buffer()[(rail_width as u16, point.y as u16)].fg,
            Color::Cyan,
            "history visibility takes precedence with the bright current-history color"
        );

        let fork = tree.overview.as_ref().expect("the ref-tree exists").by_commit
            [&graph.index(id(2)).expect("the fork exists")];
        tree.selected = Some(fork);
        tree.selection_changed();
        tree.handle_key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE), &mut app);
        terminal.draw(|frame| draw_tree(frame, &mut tree, &graph))?;
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
            footer.contains("child 1/2 · h/l cycle · <enter> move · Esc cancel"),
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
        terminal.draw(|frame| draw_tree(frame, &mut tree, &graph))?;
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
