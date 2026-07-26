use std::{
    collections::HashMap,
    ops::Range,
    time::{Duration, Instant},
};

use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice},
    traverse::commit::ParentIds,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Commit<T> {
    pub id: ObjectId,
    pub parent_ids: ParentIds,
    pub committer_time: gix::date::Time,
    pub author: &'static Author,
    pub attributions: Range<usize>,
    pub title: T,
    pub metadata_loaded: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Metadata<T> {
    pub committer_time: gix::date::Time,
    pub author: &'static Author,
    pub attributions: Range<usize>,
    pub title: T,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub(crate) struct Author {
    pub name: &'static BStr,
    pub email: &'static BStr,
}

impl Author {
    pub fn is_bot(&self) -> bool {
        [b"codex@openai.com".as_slice(), b"noreply@anthropic.com".as_slice()]
            .iter()
            .any(|candidate| self.email.eq_ignore_ascii_case(candidate))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Attribution {
    pub kind: AttributionKind,
    pub author: &'static Author,
}

impl Attribution {
    pub fn is_agent(&self) -> bool {
        self.author.is_bot()
            || self.kind == AttributionKind::Assisted
                && [b"opus".as_slice(), b"gpt".as_slice()].iter().any(|name| {
                    self.author
                        .name
                        .get(..name.len())
                        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(name))
                        && self.author.name.get(name.len()).is_none_or(u8::is_ascii_whitespace)
                })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AttributionKind {
    CoAuthor,
    Assisted,
    Reviewed,
    Acked,
    Tested,
    SignedOff,
}

pub(crate) type LoadedCommit = Commit<BString>;
pub(crate) type CommitRow = Commit<Range<usize>>;

#[derive(Debug)]
pub(crate) struct LoadedCommits {
    pub rows: Vec<LoadedCommit>,
    pub attributions: Vec<Attribution>,
}

impl From<Vec<LoadedCommit>> for LoadedCommits {
    fn from(rows: Vec<LoadedCommit>) -> Self {
        LoadedCommits {
            rows,
            attributions: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum State {
    Loading,
    Cancelling,
    Computing,
    Complete,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RefMode {
    All,
    Default,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NameMode {
    All,
    Author,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CopyKind {
    Id,
    Author,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Cancelled,
    MoveUp,
    MoveDown,
    ScrollLeft,
    ScrollRight,
    HalfPageUp,
    HalfPageDown,
    PageUp,
    PageDown,
    First,
    Last,
    ToggleDate,
    ToggleName,
    ToggleTrailers,
    ToggleMailmap,
    ToggleRefs,
    ToggleHidden,
    ToggleAlign,
    ToggleCommit,
    Cancel,
    Copy,
    CopyAuthor,
    PreviewAuthorCopy(bool),
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Effect {
    Cancel,
    CopyId(ObjectId),
    CopyAuthor(&'static Author),
    Reload(bool),
    Quit,
}

#[derive(Debug)]
pub(crate) struct App {
    pub rows: Vec<CommitRow>,
    titles: Vec<u8>,
    graph: Option<Graph>,
    attributions: Vec<Attribution>,
    #[cfg(test)]
    test_lanes: Vec<String>,
    pub selected: Option<usize>,
    pub offset: usize,
    pub state: State,
    pub viewport_rows: usize,
    pub lane_time: Option<Duration>,
    pub show_committer_date: bool,
    pub name_mode: NameMode,
    pub show_trailers: bool,
    pub use_mailmap: bool,
    pub ref_mode: RefMode,
    pub has_hidden_filter: bool,
    pub show_hidden: bool,
    pub align_metadata: bool,
    pub show_commit: bool,
    pub(crate) show_selection_tail: bool,
    pub inline: bool,
    pub preview_author_copy: bool,
    pub copy_feedback: Option<CopyKind>,
    pub estimated_lane_width: usize,
    pub horizontal_offset: usize,
    horizontal_page: usize,
    horizontal_max: usize,
    follow_tail: bool,
}

impl App {
    pub fn new(viewport_rows: usize) -> Self {
        App {
            rows: Vec::new(),
            titles: Vec::new(),
            graph: None,
            attributions: Vec::new(),
            #[cfg(test)]
            test_lanes: Vec::new(),
            selected: None,
            offset: 0,
            state: State::Loading,
            viewport_rows,
            lane_time: None,
            show_committer_date: true,
            name_mode: NameMode::All,
            show_trailers: true,
            use_mailmap: true,
            ref_mode: RefMode::Default,
            has_hidden_filter: false,
            show_hidden: false,
            align_metadata: true,
            show_commit: false,
            show_selection_tail: true,
            inline: false,
            preview_author_copy: false,
            copy_feedback: None,
            estimated_lane_width: 0,
            horizontal_offset: 0,
            horizontal_page: 1,
            horizontal_max: 0,
            follow_tail: false,
        }
    }

    pub(crate) fn extend_commits(&mut self, commits: impl Into<LoadedCommits>) {
        let LoadedCommits { rows, attributions } = commits.into();
        if self.state != State::Loading || rows.is_empty() {
            return;
        }
        let was_empty = self.rows.is_empty();
        self.titles.reserve(rows.iter().map(|row| row.title.len()).sum());
        let attribution_base = self.attributions.len();
        self.attributions.extend(attributions);
        self.rows.reserve(rows.len());
        for row in rows {
            let start = self.titles.len();
            self.titles.extend_from_slice(&row.title);
            self.rows.push(Commit {
                id: row.id,
                parent_ids: row.parent_ids,
                committer_time: row.committer_time,
                author: row.author,
                attributions: attribution_base + row.attributions.start..attribution_base + row.attributions.end,
                title: start..self.titles.len(),
                metadata_loaded: row.metadata_loaded,
            });
        }
        if was_empty {
            self.estimated_lane_width = estimate_lane_width(&self.rows[..self.viewport_rows.min(self.rows.len())]);
            self.selected = Some(0);
            self.ensure_visible();
        } else if self.follow_tail {
            self.selected = Some(self.rows.len() - 1);
            self.ensure_visible();
        }
    }

    pub(crate) fn set_metadata(
        &mut self,
        index: usize,
        metadata: Metadata<BString>,
        new_attributions: Vec<Attribution>,
    ) {
        let Some(row) = self.rows.get_mut(index) else { return };
        if row.metadata_loaded {
            return;
        }
        let Metadata {
            committer_time,
            author,
            attributions,
            title,
        } = metadata;
        let title_start = self.titles.len();
        self.titles.extend_from_slice(&title);
        let attribution_start = self.attributions.len();
        self.attributions.extend(new_attributions);
        row.committer_time = committer_time;
        row.author = author;
        row.attributions = attribution_start + attributions.start..attribution_start + attributions.end;
        row.title = title_start..self.titles.len();
        row.metadata_loaded = true;
    }

    pub(crate) fn title(&self, row: &CommitRow) -> &BStr {
        debug_assert!(row.metadata_loaded, "visible rows have metadata");
        self.titles[row.title.clone()].as_bstr()
    }

    pub(crate) fn render_lanes(&self, range: Range<usize>) -> RenderedLanes {
        #[cfg(test)]
        if !self.test_lanes.is_empty() {
            return RenderedLanes::from_lanes(
                self.test_lanes[range.start.min(self.test_lanes.len())..range.end.min(self.test_lanes.len())].iter(),
            );
        }
        match &self.graph {
            Some(graph) => graph.render(&self.rows, range),
            None => RenderedLanes::empty(range.len()),
        }
    }

    pub(crate) fn attributions(&self, row: &CommitRow) -> &[Attribution] {
        debug_assert!(row.metadata_loaded, "visible rows have metadata");
        &self.attributions[row.attributions.clone()]
    }

    pub fn update(&mut self, action: Action) -> Vec<Effect> {
        match action {
            Action::Cancelled if self.state == State::Cancelling => self.state = State::Cancelled,
            Action::MoveUp => self.move_selection(1, false),
            Action::MoveDown => self.move_selection(1, true),
            Action::ScrollLeft => {
                self.horizontal_offset = self.horizontal_offset.saturating_sub(self.horizontal_page);
            }
            Action::ScrollRight => {
                self.horizontal_offset = self
                    .horizontal_offset
                    .saturating_add(self.horizontal_page)
                    .min(self.horizontal_max);
            }
            Action::HalfPageUp => self.move_selection((self.viewport_rows / 2).max(1), false),
            Action::HalfPageDown => self.move_selection((self.viewport_rows / 2).max(1), true),
            Action::PageUp => self.move_selection(self.viewport_rows.max(1), false),
            Action::PageDown => self.move_selection(self.viewport_rows.max(1), true),
            Action::First => self.select(0),
            Action::Last if !self.rows.is_empty() => {
                self.selected = Some(self.rows.len() - 1);
                self.follow_tail = self.state == State::Loading;
                self.ensure_visible();
            }
            Action::ToggleDate => self.show_committer_date = !self.show_committer_date,
            Action::ToggleName => {
                let start = self.offset.min(self.rows.len());
                let end = start.saturating_add(self.viewport_rows).min(self.rows.len());
                let has_visible_attributions = self.rows[start..end]
                    .iter()
                    .any(|row| row.metadata_loaded && !row.attributions.is_empty());
                self.name_mode = match self.name_mode {
                    NameMode::All if has_visible_attributions => NameMode::Author,
                    NameMode::All => NameMode::None,
                    NameMode::Author => NameMode::None,
                    NameMode::None => NameMode::All,
                };
            }
            Action::ToggleTrailers => self.show_trailers = !self.show_trailers,
            Action::ToggleMailmap => self.use_mailmap = !self.use_mailmap,
            Action::ToggleRefs => {
                self.ref_mode = match self.ref_mode {
                    RefMode::All => RefMode::Default,
                    RefMode::Default => RefMode::None,
                    RefMode::None => RefMode::All,
                };
            }
            Action::ToggleHidden
                if self.has_hidden_filter && matches!(self.state, State::Complete | State::Cancelled) =>
            {
                return vec![Effect::Reload(!self.show_hidden)];
            }
            Action::ToggleAlign => self.align_metadata = !self.align_metadata,
            Action::ToggleCommit => self.show_commit = !self.show_commit,
            Action::PreviewAuthorCopy(value) => self.preview_author_copy = value,
            Action::Cancel if self.state == State::Loading => {
                self.state = State::Cancelling;
                return vec![Effect::Cancel];
            }
            Action::Copy => {
                if let Some(id) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) {
                    self.copy_feedback = Some(CopyKind::Id);
                    return vec![Effect::CopyId(id)];
                }
            }
            Action::CopyAuthor => {
                if let Some(author) = self
                    .selected
                    .and_then(|index| self.rows.get(index))
                    .filter(|row| row.metadata_loaded)
                    .map(|row| row.author)
                {
                    self.copy_feedback = Some(CopyKind::Author);
                    return vec![Effect::CopyAuthor(author)];
                }
            }
            Action::Quit => {
                return if matches!(self.state, State::Loading | State::Cancelling) {
                    vec![Effect::Cancel, Effect::Quit]
                } else {
                    vec![Effect::Quit]
                };
            }
            _ => {}
        }
        Vec::new()
    }

    pub(crate) fn start_lane_computation(&mut self) -> Option<Vec<CommitRow>> {
        match self.state {
            State::Loading => {
                self.state = State::Computing;
                self.follow_tail = false;
                Some(self.rows.clone())
            }
            State::Cancelling => {
                self.state = State::Cancelled;
                self.follow_tail = false;
                None
            }
            _ => None,
        }
    }

    pub(crate) fn finish_lane_computation(&mut self, rows: Vec<CommitRow>, graph: Graph, lane_time: Duration) {
        if self.state != State::Computing {
            return;
        }
        let selected = self.selected.map(|index| self.rows[index].id);
        let metadata: HashMap<_, _> = if rows.iter().any(|row| !row.metadata_loaded) {
            self.rows
                .iter()
                .filter(|row| row.metadata_loaded)
                .map(|row| {
                    (
                        row.id,
                        Metadata {
                            committer_time: row.committer_time,
                            author: row.author,
                            attributions: row.attributions.clone(),
                            title: row.title.clone(),
                        },
                    )
                })
                .collect()
        } else {
            HashMap::new()
        };
        self.rows = rows;
        for row in &mut self.rows {
            if let Some(metadata) = metadata.get(&row.id) {
                row.committer_time = metadata.committer_time;
                row.author = metadata.author;
                row.attributions = metadata.attributions.clone();
                row.title = metadata.title.clone();
                row.metadata_loaded = true;
            }
        }
        self.graph = Some(graph);
        self.lane_time = Some(lane_time);
        self.selected = selected.and_then(|id| self.rows.iter().position(|row| row.id == id));
        self.state = State::Complete;
        self.ensure_visible();
    }

    pub(crate) fn reload(&mut self, show_hidden: bool) {
        self.rows = Vec::new();
        self.titles = Vec::new();
        self.graph = None;
        self.attributions = Vec::new();
        #[cfg(test)]
        self.test_lanes.clear();
        self.selected = None;
        self.offset = 0;
        self.state = State::Loading;
        self.lane_time = None;
        self.estimated_lane_width = 0;
        self.show_hidden = show_hidden;
        self.horizontal_offset = 0;
        self.follow_tail = false;
    }

    fn move_selection(&mut self, distance: usize, down: bool) {
        let Some(selected) = self.selected else { return };
        self.selected = Some(if down {
            selected.saturating_add(distance).min(self.rows.len() - 1)
        } else {
            selected.saturating_sub(distance)
        });
        self.follow_tail = false;
        self.ensure_visible();
    }

    fn select(&mut self, selected: usize) {
        if !self.rows.is_empty() {
            self.selected = Some(selected.min(self.rows.len() - 1));
            self.follow_tail = false;
            self.ensure_visible();
        }
    }

    pub(crate) fn ensure_visible(&mut self) {
        let Some(selected) = self.selected else { return };
        let height = self.viewport_rows.max(1);
        if selected < self.offset {
            self.offset = selected;
        } else if selected >= self.offset.saturating_add(height) {
            self.offset = selected + 1 - height;
        }
    }

    pub(crate) fn set_horizontal_bounds(&mut self, page: usize, max: usize) {
        self.horizontal_page = page.max(1);
        self.horizontal_max = max;
        self.horizontal_offset = self.horizontal_offset.min(max);
    }

    #[cfg(test)]
    pub(crate) fn set_lane(&mut self, index: usize, lane: &str) {
        self.test_lanes.resize(self.rows.len(), String::new());
        self.test_lanes[index] = lane.into();
    }
}

fn estimate_lane_width(rows: &[CommitRow]) -> usize {
    let mut rows = rows.to_vec();
    let known: HashMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
    for row in &mut rows {
        row.parent_ids.retain(|id| known.contains_key(id));
    }
    let graph = Graph::new(&rows);
    graph
        .render(&rows, 0..rows.len())
        .iter()
        .map(|lane| lane.trim_end().chars().count().saturating_add(1))
        .max()
        .unwrap_or_default()
}

pub(crate) fn compute_lanes(mut rows: Vec<CommitRow>) -> (Vec<CommitRow>, Graph, Duration) {
    let positions: HashMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
    for row in &mut rows {
        row.parent_ids.retain(|id| positions.contains_key(id));
    }
    let mut children = vec![0usize; rows.len()];
    for row in rows.iter() {
        for parent in &row.parent_ids {
            if let Some(index) = positions.get(parent) {
                children[*index] += 1;
            }
        }
    }

    let mut ready: Vec<_> = children
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(index, count)| (*count == 0).then_some(index))
        .collect();
    let mut ordered = 0;
    while let Some(index) = ready.pop() {
        for parent in rows[index].parent_ids.iter().rev() {
            if let Some(parent_index) = positions.get(parent) {
                children[*parent_index] -= 1;
                if children[*parent_index] == 0 {
                    ready.push(*parent_index);
                }
            }
        }
        // A ready row's child count is dead, so reuse it as the row's destination.
        children[index] = ordered;
        ordered += 1;
    }
    if ordered == rows.len() {
        for index in 0..rows.len() {
            while children[index] != index {
                let destination = children[index];
                rows.swap(index, destination);
                children.swap(index, destination);
            }
        }
    }
    let start = Instant::now();
    let graph = Graph::new(&rows);
    (rows, graph, start.elapsed())
}

const CHECKPOINT_INTERVAL: usize = 256;

#[derive(Debug)]
pub(crate) struct Graph {
    offsets: Vec<usize>,
    columns: Vec<ObjectId>,
}

impl Graph {
    fn new(rows: &[CommitRow]) -> Self {
        let mut state = LaneState::default();
        let mut graph = Graph {
            offsets: Vec::with_capacity(rows.len().div_ceil(CHECKPOINT_INTERVAL) + 1),
            columns: Vec::new(),
        };
        for (index, row) in rows.iter().enumerate() {
            if index % CHECKPOINT_INTERVAL == 0 {
                graph.offsets.push(graph.columns.len());
                graph.columns.extend_from_slice(&state.columns);
            }
            state.advance(row, None);
        }
        graph.offsets.push(graph.columns.len());
        graph
    }

    fn render(&self, rows: &[CommitRow], range: Range<usize>) -> RenderedLanes {
        let start = range.start.min(rows.len());
        let end = range.end.min(rows.len());
        if start >= end {
            return RenderedLanes::default();
        }
        let checkpoint = start / CHECKPOINT_INTERVAL;
        let mut state = LaneState {
            columns: self.columns[self.offsets[checkpoint]..self.offsets[checkpoint + 1]].to_vec(),
            ..LaneState::default()
        };
        let mut rendered = RenderedLanes {
            data: String::with_capacity((end - start).saturating_mul(4)),
            ranges: Vec::with_capacity(end - start),
        };
        for (index, row) in rows[checkpoint * CHECKPOINT_INTERVAL..end].iter().enumerate() {
            let index = checkpoint * CHECKPOINT_INTERVAL + index;
            if let Some(range) = state.advance(row, (index >= start).then_some(&mut rendered.data)) {
                rendered.ranges.push(range);
            }
        }
        rendered
    }
}

#[derive(Debug, Default)]
pub(crate) struct RenderedLanes {
    data: String,
    ranges: Vec<Range<usize>>,
}

impl RenderedLanes {
    pub(crate) fn lane(&self, index: usize) -> &str {
        &self.data[self.ranges[index].clone()]
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &str> {
        self.ranges.iter().map(|range| &self.data[range.clone()])
    }

    fn empty(len: usize) -> Self {
        RenderedLanes {
            data: String::new(),
            ranges: vec![0..0; len],
        }
    }

    #[cfg(test)]
    fn from_lanes<'a>(lanes: impl IntoIterator<Item = &'a String>) -> Self {
        let mut rendered = RenderedLanes::default();
        for lane in lanes {
            let start = rendered.data.len();
            rendered.data.push_str(lane);
            rendered.ranges.push(start..rendered.data.len());
        }
        rendered
    }
}

#[derive(Default)]
struct LaneState {
    columns: Vec<ObjectId>,
    next: Vec<ObjectId>,
    parents: Vec<(ObjectId, Option<usize>, usize)>,
    edges: Vec<(usize, usize)>,
    cells: Vec<u8>,
}

impl LaneState {
    fn advance(&mut self, row: &CommitRow, out: Option<&mut String>) -> Option<Range<usize>> {
        let render = out.is_some();
        let current = self.columns.iter().position(|id| *id == row.id).unwrap_or_else(|| {
            self.columns.push(row.id);
            self.columns.len() - 1
        });

        self.parents.clear();
        for parent in row.parent_ids.iter().copied() {
            if !self.parents.iter().any(|(id, _, _)| *id == parent) {
                self.parents
                    .push((parent, self.columns.iter().position(|id| *id == parent), 0));
            }
        }
        self.next.clear();
        self.edges.clear();
        for (index, id) in self.columns[..current].iter().copied().enumerate() {
            let destination = self.next.len();
            self.next.push(id);
            if render {
                self.edges.push((index, destination));
            }
        }
        for (parent, old_position, destination) in &mut self.parents {
            *destination = match old_position {
                Some(position) if *position < current => *position,
                _ => {
                    let destination = self.next.len();
                    self.next.push(*parent);
                    if render && old_position.is_some_and(|position| position != current) {
                        self.edges
                            .push((old_position.expect("checked as present"), destination));
                    }
                    destination
                }
            };
        }
        for (index, id) in self.columns.iter().copied().enumerate().skip(current + 1) {
            if self.parents.iter().any(|(_, position, _)| *position == Some(index)) {
                continue;
            }
            let destination = self.next.len();
            self.next.push(id);
            if render {
                self.edges.push((index, destination));
            }
        }
        if render {
            for (_, _, destination) in &self.parents {
                self.edges.push((current, *destination));
            }
        }
        let range = out.map(|out| {
            transition(
                self.columns.len(),
                self.next.len(),
                current,
                &self.edges,
                &mut self.cells,
                out,
            )
        });
        std::mem::swap(&mut self.columns, &mut self.next);
        range
    }
}

fn transition(
    before: usize,
    after: usize,
    current: usize,
    edges: &[(usize, usize)],
    cells: &mut Vec<u8>,
    out: &mut String,
) -> Range<usize> {
    const UP: u8 = 1;
    const DOWN: u8 = 2;
    const LEFT: u8 = 4;
    const RIGHT: u8 = 8;
    const VERTICAL: u8 = UP | DOWN;
    const HORIZONTAL: u8 = LEFT | RIGHT;
    const CROSS: u8 = VERTICAL | HORIZONTAL;
    const VERTICAL_RIGHT: u8 = VERTICAL | RIGHT;
    const VERTICAL_LEFT: u8 = VERTICAL | LEFT;
    const DOWN_HORIZONTAL: u8 = DOWN | HORIZONTAL;
    const UP_HORIZONTAL: u8 = UP | HORIZONTAL;
    const DOWN_RIGHT: u8 = DOWN | RIGHT;
    const DOWN_LEFT: u8 = DOWN | LEFT;
    const UP_RIGHT: u8 = UP | RIGHT;
    const UP_LEFT: u8 = UP | LEFT;

    let width = before.max(after).max(current + 1) * 2 - 1;
    cells.clear();
    cells.resize(width, 0);
    for &(from, to) in edges {
        let from = from * 2;
        let to = to * 2;
        cells[from] |= UP;
        cells[to] |= DOWN;
        if from < to {
            cells[from] |= RIGHT;
            cells[to] |= LEFT;
            for cell in &mut cells[from + 1..to] {
                *cell |= LEFT | RIGHT;
            }
        } else if to < from {
            cells[from] |= LEFT;
            cells[to] |= RIGHT;
            for cell in &mut cells[to + 1..from] {
                *cell |= LEFT | RIGHT;
            }
        }
    }

    let start = out.len();
    for (index, cell) in cells.iter().copied().enumerate() {
        out.push(if index == current * 2 {
            '●'
        } else {
            match cell {
                0 => ' ',
                CROSS => '┼',
                VERTICAL_RIGHT => '├',
                VERTICAL_LEFT => '┤',
                DOWN_HORIZONTAL => '┬',
                UP_HORIZONTAL => '┴',
                DOWN_RIGHT => '┌',
                DOWN_LEFT => '┐',
                UP_RIGHT => '└',
                UP_LEFT => '┘',
                HORIZONTAL => '─',
                _ => '│',
            }
        });
    }
    out.push(' ');
    start..out.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u16) -> ObjectId {
        let mut bytes = [0; 20];
        bytes[18..].copy_from_slice(&n.to_be_bytes());
        ObjectId::Sha1(bytes)
    }

    fn row(n: u8) -> LoadedCommit {
        Commit {
            id: id(n.into()),
            parent_ids: ParentIds::new(),
            committer_time: gix::date::Time::default(),
            author: Box::leak(Box::new(Author {
                name: b"author".as_bstr(),
                email: b"author@example.com".as_bstr(),
            })),
            attributions: 0..0,
            title: format!("commit {n}").into(),
            metadata_loaded: true,
        }
    }

    fn row_with_parents(n: u8, parents: &[u8]) -> LoadedCommit {
        let mut commit = row(n);
        commit.parent_ids = parents.iter().map(|n| row(*n).id).collect();
        commit
    }

    #[test]
    fn recognizes_named_agents_only_when_assisting() {
        let opus = Box::leak(Box::new(Author {
            name: b"Opus 4.7".as_bstr(),
            email: b"".as_bstr(),
        }));
        let gpt = Box::leak(Box::new(Author {
            name: b"GPT 5.6".as_bstr(),
            email: b"".as_bstr(),
        }));

        assert!(
            Attribution {
                kind: AttributionKind::Assisted,
                author: opus,
            }
            .is_agent()
        );
        assert!(
            Attribution {
                kind: AttributionKind::Assisted,
                author: gpt,
            }
            .is_agent()
        );
        assert!(
            !Attribution {
                kind: AttributionKind::Reviewed,
                author: opus,
            }
            .is_agent()
        );
    }

    fn numbered_row(n: u16, parent: Option<u16>) -> LoadedCommit {
        let mut commit = row(0);
        commit.id = id(n);
        commit.parent_ids = parent.map(id).into_iter().collect();
        commit.title = format!("commit {n}").into();
        commit
    }

    fn complete(app: &mut App) {
        let rows = app
            .start_lane_computation()
            .expect("a loading app starts lane computation");
        let (rows, graph, lane_time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, lane_time);
    }

    #[test]
    fn completion_orders_and_draws_merge_lanes() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(4, &[3, 2]),
            row_with_parents(3, &[1]),
            row(1),
            row_with_parents(2, &[1]),
        ]);

        assert_eq!(
            app.estimated_lane_width, 4,
            "the provisional and rendered graph widths use the same trailing separator"
        );

        complete(&mut app);

        assert_eq!(
            app.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [row(4).id, row(3).id, row(2).id, row(1).id]
        );
        assert_eq!(
            app.render_lanes(0..app.rows.len()).iter().collect::<Vec<_>>(),
            ["●─┐ ", "● │ ", "├─● ", "● "]
        );
    }

    #[test]
    fn lane_computation_keeps_provisional_rows_interactive() {
        let mut app = App::new(2);
        app.extend_commits(vec![row(1), row(2)]);

        let rows = app
            .start_lane_computation()
            .expect("history completion starts lane computation");
        assert_eq!(app.state, State::Computing);
        assert_eq!(app.rows.len(), 2, "provisional rows remain available to render");

        app.update(Action::MoveDown);
        let selected = app.rows[app.selected.expect("selection remains active")].id;
        let (rows, graph, lane_time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, lane_time);

        assert_eq!(app.state, State::Complete);
        assert_eq!(
            app.rows[app.selected.expect("selection survives final ordering")].id,
            selected
        );
    }

    #[test]
    fn lane_computation_preserves_metadata_loaded_while_it_runs() {
        let mut deferred = row(1);
        deferred.metadata_loaded = false;
        deferred.title.clear();
        let mut app = App::new(1);
        app.extend_commits(vec![deferred]);
        let rows = app
            .start_lane_computation()
            .expect("history completion starts lane computation");

        app.set_metadata(
            0,
            Metadata {
                committer_time: gix::date::Time::default(),
                author: row(1).author,
                attributions: 0..0,
                title: "loaded".into(),
            },
            Vec::new(),
        );
        let (rows, graph, lane_time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, lane_time);

        assert!(app.rows[0].metadata_loaded);
        assert_eq!(app.title(&app.rows[0]), "loaded");
    }

    #[test]
    fn lane_reuses_a_parent_that_is_already_to_the_right() {
        let mut app = App::new(10);
        for row in [row_with_parents(4, &[2, 3]), row_with_parents(2, &[3]), row(3)] {
            app.extend_commits(vec![row]);
        }

        complete(&mut app);

        assert_eq!(
            app.render_lanes(0..app.rows.len()).iter().collect::<Vec<_>>(),
            ["●─┐ ", "●─┘ ", "● "]
        );
    }

    #[test]
    fn lanes_render_identically_after_a_checkpoint() {
        let mut app = App::new(10);
        app.extend_commits(
            (0..=300)
                .rev()
                .map(|n| numbered_row(n, n.checked_sub(1)))
                .collect::<Vec<_>>(),
        );
        complete(&mut app);

        let all = app.render_lanes(0..app.rows.len());
        let window = app.render_lanes(257..300);
        assert_eq!(
            window.iter().collect::<Vec<_>>(),
            all.iter().skip(257).take(43).collect::<Vec<_>>(),
            "restoring a checkpoint produces the same graph as replaying from the beginning"
        );
    }

    #[test]
    fn completion_keeps_independent_lines_of_history_together() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(5, &[3]),
            row_with_parents(4, &[2]),
            row_with_parents(3, &[1]),
            row_with_parents(2, &[1]),
            row(1),
        ]);

        complete(&mut app);

        assert_eq!(
            app.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [row(5).id, row(3).id, row(4).id, row(2).id, row(1).id],
            "topological order finishes one line before showing another"
        );
    }

    #[test]
    fn selection_follows_the_oldest_commit_until_the_user_moves() {
        let mut app = App::new(2);
        app.extend_commits(vec![row(1), row(2), row(3)]);

        app.update(Action::Last);
        assert_eq!(app.selected, Some(2), "Last selects the oldest loaded commit");
        assert_eq!(app.offset, 1, "the selection remains visible");

        app.extend_commits(vec![row(4)]);
        assert_eq!(app.selected, Some(3), "new commits extend the followed tail");
        assert_eq!(app.offset, 2, "the viewport follows the tail");

        app.update(Action::MoveUp);
        app.extend_commits(vec![row(5)]);
        assert_eq!(app.selected, Some(2), "manual navigation stops following the tail");
    }

    #[test]
    fn navigation_is_clamped_and_uses_the_viewport_for_pages() {
        let mut app = App::new(2);
        app.extend_commits((1..=5).map(row).collect::<Vec<_>>());

        app.update(Action::PageDown);
        assert_eq!(app.selected, Some(2), "page-down advances by the viewport height");
        app.update(Action::PageDown);
        assert_eq!(app.selected, Some(4), "page-down clamps at the last row");
        app.update(Action::MoveDown);
        assert_eq!(app.selected, Some(4), "moving past the last row is a no-op");
        app.update(Action::First);
        assert_eq!(app.selected, Some(0), "First selects the newest commit");
        assert_eq!(app.offset, 0, "the newest commit is visible");
    }

    #[test]
    fn half_pages_use_half_the_viewport() {
        let mut app = App::new(4);
        app.extend_commits((1..=5).map(row).collect::<Vec<_>>());

        app.update(Action::HalfPageDown);
        assert_eq!(app.selected, Some(2));
        app.update(Action::HalfPageUp);
        assert_eq!(app.selected, Some(0));
    }

    #[test]
    fn horizontal_pages_are_clamped_to_available_content() {
        let mut app = App::new(1);
        app.set_horizontal_bounds(10, 25);

        app.update(Action::ScrollRight);
        app.update(Action::ScrollRight);
        app.update(Action::ScrollRight);
        assert_eq!(app.horizontal_offset, 25);
        app.update(Action::ScrollLeft);
        assert_eq!(app.horizontal_offset, 15);

        app.set_horizontal_bounds(10, 0);
        app.update(Action::ScrollRight);
        assert_eq!(app.horizontal_offset, 0, "scrolling is disabled when content fits");
    }

    #[test]
    fn toggles_metadata_columns() {
        let mut app = App::new(1);
        assert!(app.show_trailers, "trailer attribution is visible by default");

        app.update(Action::ToggleDate);
        app.update(Action::ToggleName);
        app.update(Action::ToggleTrailers);
        app.update(Action::ToggleMailmap);
        app.update(Action::ToggleRefs);
        app.update(Action::ToggleAlign);
        app.update(Action::ToggleCommit);

        assert!(!app.show_committer_date);
        assert_eq!(app.name_mode, NameMode::None);
        assert!(!app.show_trailers);
        assert!(!app.use_mailmap);
        assert_eq!(app.ref_mode, RefMode::None);
        app.update(Action::ToggleRefs);
        assert_eq!(app.ref_mode, RefMode::All);
        app.update(Action::ToggleRefs);
        assert_eq!(app.ref_mode, RefMode::Default);
        assert!(!app.align_metadata);
        assert!(app.show_commit);
        app.update(Action::ToggleAlign);
        assert!(app.align_metadata);
    }

    #[test]
    fn cycles_author_names_without_inert_states() {
        let mut app = App::new(1);
        let attribution = Attribution {
            kind: AttributionKind::CoAuthor,
            author: row(2).author,
        };
        let mut attributed = row(2);
        attributed.attributions = 0..1;
        app.extend_commits(LoadedCommits {
            rows: vec![row(1), attributed],
            attributions: vec![attribution],
        });

        app.update(Action::ToggleName);
        assert_eq!(
            app.name_mode,
            NameMode::None,
            "the visible author is hidden immediately when no attributions are visible"
        );
        app.name_mode = NameMode::All;
        app.offset = 1;
        app.update(Action::ToggleName);
        assert_eq!(app.name_mode, NameMode::Author);
        app.update(Action::ToggleName);
        assert_eq!(app.name_mode, NameMode::None);
        app.update(Action::ToggleName);
        assert_eq!(app.name_mode, NameMode::All);
    }

    #[test]
    fn hidden_history_is_reloaded_only_when_configured() {
        let mut app = App::new(1);
        assert!(
            app.update(Action::ToggleHidden).is_empty(),
            "the key is inert without hidden revisions"
        );

        app.has_hidden_filter = true;
        app.extend_commits(vec![row(1)]);
        assert!(
            app.update(Action::ToggleHidden).is_empty(),
            "a running walk cannot be replaced by another detached worker"
        );
        complete(&mut app);
        assert_eq!(app.update(Action::ToggleHidden), vec![Effect::Reload(true)]);
        app.reload(true);
        assert!(app.rows.is_empty(), "reloading drops rows from the previous view");
        assert!(app.show_hidden);
        assert_eq!(app.state, State::Loading);
        assert!(
            app.update(Action::ToggleHidden).is_empty(),
            "the replacement walk must finish before it can be toggled again"
        );
        complete(&mut app);
        assert_eq!(app.update(Action::ToggleHidden), vec![Effect::Reload(false)]);
    }

    #[test]
    fn cancellation_preserves_rows_and_ignores_late_worker_events() {
        let mut app = App::new(10);
        app.extend_commits(vec![row(1)]);

        assert_eq!(app.update(Action::Cancel), vec![Effect::Cancel]);
        assert_eq!(app.state, State::Cancelling);
        app.extend_commits(vec![row(2)]);
        assert_eq!(app.rows.len(), 1, "commits arriving after cancellation are ignored");

        assert!(app.start_lane_computation().is_none());
        assert_eq!(app.state, State::Cancelled);
        assert_eq!(
            app.rows.len(),
            1,
            "completion racing cancellation keeps already displayed commits"
        );
    }

    #[test]
    fn completion_and_copy_effects_use_the_current_selection() {
        let mut app = App::new(10);
        assert!(
            app.update(Action::Copy).is_empty(),
            "there is nothing to copy without a selection"
        );
        assert!(
            app.update(Action::CopyAuthor).is_empty(),
            "there is no author to copy without a selection"
        );
        app.extend_commits(vec![row(7)]);

        assert_eq!(app.update(Action::Copy), vec![Effect::CopyId(row(7).id)]);
        assert_eq!(app.update(Action::CopyAuthor), vec![Effect::CopyAuthor(row(7).author)]);
        complete(&mut app);
        assert_eq!(app.state, State::Complete);
        assert_eq!(app.rows.len(), 1, "the loaded row count is the completed total");
        assert_eq!(app.update(Action::Quit), vec![Effect::Quit]);
    }

    #[test]
    fn packs_titles_as_raw_bytes() {
        let mut first = row(1);
        first.title = vec![b'a', 0xff].into();
        let mut second = row(2);
        second.title = "second".into();
        let mut app = App::new(2);

        app.extend_commits(vec![first]);
        app.extend_commits(vec![second]);

        assert_eq!(app.titles, b"a\xffsecond", "title bytes share one allocation");
        assert_eq!(
            app.title(&app.rows[0]),
            b"a\xff".as_bstr(),
            "the first span preserves arbitrary bytes"
        );
        assert_eq!(
            app.title(&app.rows[1]),
            b"second".as_bstr(),
            "the second span starts at the right offset"
        );
    }

    #[test]
    fn packs_attributions_across_history_batches() {
        let first_attribution = Attribution {
            kind: AttributionKind::CoAuthor,
            author: row(1).author,
        };
        let second_attribution = Attribution {
            kind: AttributionKind::Reviewed,
            author: row(2).author,
        };
        let mut first = row(1);
        first.attributions = 0..1;
        let mut second = row(2);
        second.attributions = 0..1;
        let mut app = App::new(2);

        app.extend_commits(LoadedCommits {
            rows: vec![first],
            attributions: vec![first_attribution],
        });
        app.extend_commits(LoadedCommits {
            rows: vec![second],
            attributions: vec![second_attribution],
        });

        assert_eq!(
            app.attributions,
            [first_attribution, second_attribution],
            "all attribution entries share one application-owned buffer"
        );
        assert_eq!(
            app.attributions(&app.rows[0]),
            [first_attribution],
            "the first batch retains its attribution range"
        );
        assert_eq!(
            app.attributions(&app.rows[1]),
            [second_attribution],
            "later batch ranges are offset into the shared buffer"
        );
    }
}
