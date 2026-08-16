use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    fmt::Write,
    sync::atomic::AtomicBool,
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use gix::{ObjectId, bstr::ByteSlice};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Clear, Paragraph},
};

use crate::{
    app::LaneState,
    history::{CommitIndex, Decoration, DecorationKind, Decorations, HistoryGraph, RefSnapshot},
    ui::decoration_style,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Motion {
    #[default]
    Nearest,
    Topological,
}

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

pub(crate) enum Input {
    Handled,
    Quit,
}

#[derive(Default)]
pub(crate) struct Tree {
    active: bool,
    motion: Motion,
    overview: Option<Overview>,
    alternate_overview: Option<Overview>,
    overlay: Option<Overlay>,
    selected: Option<usize>,
    choices: Vec<usize>,
    offset: Offset,
    placed: Option<Placed>,
    ensure_visible: bool,
    hide_tags: bool,
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
            .or(head);
        self.choices = vec![0; overview.nodes.len()];
        self.overview = Some(overview);
        self.alternate_overview = Some(alternate_overview);
        self.overlay = None;
        self.placed = None;
        self.ensure_visible = true;
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Input {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
            || key.code == KeyCode::Char('q')
        {
            return Input::Quit;
        }
        if key.code == KeyCode::Esc {
            self.leave();
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
        if key.code == KeyCode::Tab {
            self.motion = match self.motion {
                Motion::Nearest => Motion::Topological,
                Motion::Topological => Motion::Nearest,
            };
            return Input::Handled;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            let amount = self.offset().page_height.max(1);
            match key.code {
                KeyCode::Char('u') => self.pan(Direction::Up, (amount / 2).max(1)),
                KeyCode::Char('d') => self.pan(Direction::Down, (amount / 2).max(1)),
                KeyCode::Char('b') => self.pan(Direction::Up, amount),
                KeyCode::Char('f') => self.pan(Direction::Down, amount),
                _ => return Input::Handled,
            }
            return Input::Handled;
        }
        match key.code {
            KeyCode::PageUp => {
                self.pan(Direction::Up, self.offset().page_height.max(1));
                return Input::Handled;
            }
            KeyCode::PageDown => {
                self.pan(Direction::Down, self.offset().page_height.max(1));
                return Input::Handled;
            }
            _ => {}
        }
        let Some(direction) = direction(key.code) else {
            return Input::Handled;
        };
        if key.modifiers.contains(KeyModifiers::SHIFT) || matches!(key.code, KeyCode::Char('H' | 'J' | 'K' | 'L')) {
            self.pan(direction, 1);
        } else {
            self.navigate(direction);
        }
        Input::Handled
    }

    pub(crate) fn handle_mouse(&mut self, kind: MouseEventKind, distance: usize) -> bool {
        let direction = match kind {
            MouseEventKind::ScrollUp => Direction::Up,
            MouseEventKind::ScrollDown => Direction::Down,
            MouseEventKind::ScrollLeft => Direction::Left,
            MouseEventKind::ScrollRight => Direction::Right,
            _ => return false,
        };
        self.pan(direction, distance.max(1));
        true
    }

    pub(crate) fn draw(&mut self, frame: &mut Frame<'_>, graph: Option<&HistoryGraph>) {
        if self.overlay.as_ref().map(|overlay| overlay.selected)
            != self
                .selected
                .and_then(|selected| self.overview.as_ref()?.nodes.get(selected))
                .map(|node| node.commit)
            && let (Some(graph), Some(selected)) = (graph, self.selected)
        {
            self.overlay = self
                .overview
                .as_ref()
                .map(|overview| Overlay::new(graph, overview, selected));
        }
        let [body, footer] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
        frame.render_widget(Clear, frame.area());
        let Some(overview) = self.overview.as_ref() else {
            frame.render_widget(Paragraph::new("tree overview unavailable"), body);
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
        draw_rail_edges(
            frame,
            body,
            overview,
            self.overlay.as_ref(),
            placed,
            offset,
            selected,
            &self.choices,
        );
        draw_nodes(
            frame,
            body,
            overview,
            self.overlay.as_ref(),
            placed,
            offset,
            selected,
            self.motion,
            &self.choices,
        );
        let motion = match self.motion {
            Motion::Nearest => "nearest",
            Motion::Topological => "topo ↑ leaves · ↓ roots · ←/→ branch",
        };
        let tags = if self.hide_tags { "off" } else { "on" };
        frame.render_widget(
            Paragraph::new(format!(
                "tree · {motion} · g top · G root · T tags:{tags} · Shift+directions pan · t/Esc history"
            ))
            .style(Style::default().add_modifier(Modifier::DIM)),
            footer,
        );
    }

    fn navigate(&mut self, direction: Direction) {
        let Some(overview) = self.overview.as_ref() else { return };
        let Some(selected) = self.selected else { return };
        let next = if self.motion == Motion::Topological {
            match direction {
                Direction::Up => overview.nodes[selected]
                    .children
                    .get(self.choices[selected].min(overview.nodes[selected].children.len().saturating_sub(1)))
                    .copied(),
                Direction::Down => overview.nodes[selected].parent,
                Direction::Left => {
                    self.choices[selected] = self.choices[selected].saturating_sub(1);
                    None
                }
                Direction::Right => {
                    self.choices[selected] = self.choices[selected]
                        .saturating_add(1)
                        .min(overview.nodes[selected].children.len().saturating_sub(1));
                    None
                }
            }
        } else {
            self.placed
                .as_ref()
                .and_then(|placed| nearest(&placed.nodes, selected, direction))
        };
        if let Some(next) = next {
            self.selected = Some(next);
            self.overlay = None;
            self.placed = None;
            self.ensure_visible = true;
        }
    }

    fn jump_to_root(&mut self) {
        let (Some(overview), Some(mut selected)) = (self.overview.as_ref(), self.selected) else {
            return;
        };
        while let Some(parent) = overview.nodes[selected].parent {
            selected = parent;
        }
        self.selected = Some(selected);
        self.overlay = None;
        self.placed = None;
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
        self.overlay = None;
        self.placed = None;
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
        self.choices = vec![0; overview.nodes.len()];
        self.overlay = None;
        self.placed = None;
        self.ensure_visible = true;
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

    fn offset(&self) -> &Offset {
        &self.offset
    }
}

impl Overview {
    fn new(graph: &HistoryGraph, refs: &RefSnapshot, decorations: &Decorations, show_tags: bool) -> Self {
        let mut labels = HashMap::<CommitIndex, Vec<Decoration>>::new();
        let mut heads = HashSet::new();
        let mut anchors = HashSet::new();
        for (id, decorations) in decorations {
            let Some(index) = graph.index(*id) else { continue };
            for decoration in decorations {
                if !show_tags && matches!(decoration.kind, DecorationKind::Tag | DecorationKind::AnnotatedTag) {
                    continue;
                }
                if decoration.kind == DecorationKind::Head {
                    heads.insert(index);
                    anchors.insert(index);
                } else if decoration.kind != DecorationKind::Special {
                    labels.entry(index).or_default().push(decoration.clone());
                    anchors.insert(index);
                }
            }
        }
        let raw: HashSet<_> = refs
            .view_tips
            .iter()
            .chain(&refs.hidden_tips)
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
        transition = transition
            .chars()
            .map(|symbol| match symbol {
                '┌' => '╭',
                '┐' => '╮',
                '└' => '╰',
                '┘' => '╯',
                _ => symbol,
            })
            .collect();
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

#[expect(
    clippy::too_many_arguments,
    reason = "drawing receives detached layout and style state"
)]
fn draw_rail_edges(
    frame: &mut Frame<'_>,
    area: Rect,
    overview: &Overview,
    overlay: Option<&Overlay>,
    placed: &Placed,
    offset: Offset,
    selected: Option<usize>,
    choices: &[usize],
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
                .map_or_else(Style::default, |edge| {
                    let chosen = selected == Some(edge.parent)
                        && overview.nodes[edge.parent].children.get(
                            choices[edge.parent].min(overview.nodes[edge.parent].children.len().saturating_sub(1)),
                        ) == Some(&edge.child);
                    if chosen {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        edge_style(overview, overlay, edge)
                    }
                }),
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
    motion: Motion,
    choices: &[usize],
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
        let mut text = rail_label(node, count);
        if selected == Some(index) && motion == Motion::Topological && node.children.len() > 1 {
            write!(
                text,
                " {}/{}",
                choices[index].min(node.children.len() - 1) + 1,
                node.children.len()
            )
            .expect("writing to a string cannot fail");
        }
        let mut style = node.decorations.first().map_or_else(
            || Style::default().fg(Color::LightBlue),
            |decoration| decoration_style(decoration.kind),
        );
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
        put(frame, area, offset, point, if node.is_head { '@' } else { '●' }, style);
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
    }
}

fn rail_label(node: &Node, count: Option<usize>) -> String {
    let mut out = count.map_or_else(String::new, |count| count.to_string());
    let labels: Vec<_> = node
        .decorations
        .iter()
        .map(|decoration| decoration.name.to_str_lossy())
        .collect();
    let suffix = if !labels.is_empty() {
        labels.join(", ")
    } else if node.raw_tip {
        node.id.to_hex_with_len(7).to_string()
    } else if node.parent.is_none() {
        "<root>".into()
    } else {
        String::new()
    };
    if !out.is_empty() && !suffix.is_empty() {
        out.push(' ');
    }
    out.push_str(&suffix);
    out
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
    worktrees: bool,
    show_tags: bool,
    unicode: bool,
) -> anyhow::Result<String> {
    let refs = crate::history::snapshot(repository, revisions, hidden, worktrees)?;
    let authors = gix::features::threading::OwnShared::new(gix::features::threading::Mutable::new(
        crate::history::Authors::default(),
    ));
    let mut graph = None;
    let mut decorations = None;
    crate::history::load(
        repository,
        revisions,
        hidden,
        worktrees,
        &authors,
        &AtomicBool::new(false),
        |event| {
            match event {
                crate::history::Event::Decorations(value) => decorations = Some(value),
                crate::history::Event::Complete(value) => graph = Some(value),
                _ => {}
            }
            true
        },
    )?;
    let graph = graph.ok_or_else(|| anyhow::anyhow!("history traversal did not produce a graph"))?;
    let decorations = decorations.unwrap_or_default();
    Ok(render_overview(
        &Overview::new(&graph, &refs, &decorations, show_tags),
        unicode,
    ))
}

fn render_overview(overview: &Overview, unicode: bool) -> String {
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
            out.push_str(&rail_label(&overview.nodes[node], None));
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
    fn topological_navigation_and_g_shortcuts_reach_the_top_and_root() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);
        tree.motion = Motion::Topological;
        let overview = tree.overview.as_ref().expect("overview exists");
        let root = overview.by_commit[&graph.index(id(1)).expect("root exists")];
        let fork = overview.by_commit[&graph.index(id(2)).expect("fork exists")];
        let main = overview.by_commit[&graph.index(id(6)).expect("main exists")];
        tree.selected = Some(root);

        tree.navigate(Direction::Up);
        assert_eq!(tree.selected, Some(fork), "up moves toward a leaf");
        tree.navigate(Direction::Down);
        assert_eq!(tree.selected, Some(root), "down moves toward the root");

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
        assert!(tree.toggle(), "the available tree opens");
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
        let rendered = render_overview(&overview, false);

        assert!(rendered.contains("main"), "HEAD is rendered without selection state");
        assert!(rendered.contains("topic"), "ordinary nodes retain their label");
        assert!(rendered.contains("<root>"), "the full projection reaches its root");
        assert!(rendered.contains('o'), "ordinary nodes use an ASCII marker");
        assert!(!rendered.contains('●'), "ASCII output has no Unicode node glyphs");
        assert!(!rendered.contains('\u{1b}'), "plain output has no terminal styles");
    }

    #[test]
    fn rail_layout_uses_rounded_unicode_connections_and_ascii_fallbacks() {
        let (graph, refs, decorations) = fixture();
        let overview = Overview::new(&graph, &refs, &decorations, true);
        let unicode = render_overview(&overview, true);
        let ascii = render_overview(&overview, false);

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
    fn interactive_tree_renders_exact_visible_counts() -> gix_testtools::Result {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
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
        assert!(rendered.contains("main"), "the rounded tree shows main");
        assert!(rendered.contains("topic"), "the rounded tree shows topic");
        assert!(rendered.contains("tree ·"), "the footer identifies the tree");
        assert!(
            rendered.contains("5 main"),
            "the selected reference shows its exact count"
        );
        assert!(
            rendered.contains("1 topic"),
            "visible references show relative exact counts"
        );
        assert!(
            !rendered.contains("5●"),
            "the rail marker is not repeated after a count"
        );
        Ok(())
    }

    #[test]
    fn tree_toggle_opens_and_closes_the_single_layout() {
        let (graph, refs, decorations) = fixture();
        let mut tree = Tree::default();
        tree.rebuild(&graph, &refs, &decorations);

        assert!(tree.toggle());
        assert!(tree.is_active());
        assert!(tree.toggle());
        assert!(!tree.is_active());
    }
}
