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
    pub lane: Range<usize>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AttributionKind {
    CoAuthor,
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
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Effect {
    Cancel,
    Copy(ObjectId),
    Reload(bool),
    Quit,
}

#[derive(Debug)]
pub(crate) struct App {
    pub rows: Vec<CommitRow>,
    titles: Vec<u8>,
    lanes: String,
    attributions: Vec<Attribution>,
    pub selected: Option<usize>,
    pub offset: usize,
    pub state: State,
    pub viewport_rows: usize,
    pub lane_time: Option<Duration>,
    pub show_committer_date: bool,
    pub show_author_name: bool,
    pub show_trailers: bool,
    pub use_mailmap: bool,
    pub ref_mode: RefMode,
    pub has_hidden_filter: bool,
    pub show_hidden: bool,
    pub align_metadata: bool,
    pub show_commit: bool,
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
            lanes: String::new(),
            attributions: Vec::new(),
            selected: None,
            offset: 0,
            state: State::Loading,
            viewport_rows,
            lane_time: None,
            show_committer_date: true,
            show_author_name: true,
            show_trailers: true,
            use_mailmap: true,
            ref_mode: RefMode::Default,
            has_hidden_filter: false,
            show_hidden: false,
            align_metadata: true,
            show_commit: false,
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
            let Commit {
                id,
                parent_ids,
                lane,
                committer_time,
                author,
                attributions,
                title,
            } = row;
            let start = self.titles.len();
            self.titles.extend_from_slice(&title);
            self.rows.push(Commit {
                id,
                parent_ids,
                lane,
                committer_time,
                author,
                attributions: attribution_base + attributions.start..attribution_base + attributions.end,
                title: start..self.titles.len(),
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

    pub(crate) fn title(&self, row: &CommitRow) -> &BStr {
        self.titles[row.title.clone()].as_bstr()
    }

    pub(crate) fn lane(&self, row: &CommitRow) -> &str {
        &self.lanes[row.lane.clone()]
    }

    pub(crate) fn attributions(&self, row: &CommitRow) -> &[Attribution] {
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
            Action::ToggleName => self.show_author_name = !self.show_author_name,
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
            Action::Cancel if self.state == State::Loading => {
                self.state = State::Cancelling;
                return vec![Effect::Cancel];
            }
            Action::Copy => {
                if let Some(row) = self.selected.and_then(|index| self.rows.get(index)) {
                    return vec![Effect::Copy(row.id)];
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

    pub(crate) fn finish_lane_computation(&mut self, rows: Vec<CommitRow>, lanes: String, lane_time: Duration) {
        if self.state != State::Computing {
            return;
        }
        let selected = self.selected.map(|index| self.rows[index].id);
        self.rows = rows;
        self.lanes = lanes;
        self.lane_time = Some(lane_time);
        self.selected = selected.and_then(|id| self.rows.iter().position(|row| row.id == id));
        self.state = State::Complete;
        self.ensure_visible();
    }

    pub(crate) fn reload(&mut self, show_hidden: bool) {
        self.rows = Vec::new();
        self.titles = Vec::new();
        self.lanes = String::new();
        self.attributions = Vec::new();
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
        let start = self.lanes.len();
        self.lanes.push_str(lane);
        self.rows[index].lane = start..self.lanes.len();
    }
}

fn estimate_lane_width(rows: &[CommitRow]) -> usize {
    let mut rows = rows.to_vec();
    let known = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
    let lanes = render_lanes(&mut rows, &known);
    rows.iter()
        .map(|row| lanes[row.lane.clone()].chars().count())
        .max()
        .map_or(0, |width| width.saturating_add(2))
}

pub(crate) fn compute_lanes(mut rows: Vec<CommitRow>) -> (Vec<CommitRow>, String, Duration) {
    let positions: HashMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
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
    let mut order = Vec::with_capacity(rows.len());
    while let Some(index) = ready.pop() {
        order.push(index);
        for parent in rows[index].parent_ids.iter().rev() {
            if let Some(parent_index) = positions.get(parent) {
                children[*parent_index] -= 1;
                if children[*parent_index] == 0 {
                    ready.push(*parent_index);
                }
            }
        }
    }
    if order.len() == rows.len() {
        let mut old: Vec<_> = std::mem::take(&mut rows).into_iter().map(Some).collect();
        rows = order
            .into_iter()
            .map(|index| old[index].take().expect("each row is moved exactly once"))
            .collect();
    }
    let start = Instant::now();
    let lanes = render_lanes(&mut rows, &positions);
    (rows, lanes, start.elapsed())
}

fn render_lanes(rows: &mut [CommitRow], known: &HashMap<ObjectId, usize>) -> String {
    let mut columns = Vec::new();
    let mut next = Vec::new();
    let mut parents = Vec::new();
    let mut edges = Vec::new();
    let mut cells = Vec::new();
    let mut lanes = String::with_capacity(rows.len().saturating_mul(4));
    for row in rows {
        let current = columns.iter().position(|id| *id == row.id).unwrap_or_else(|| {
            columns.push(row.id);
            columns.len() - 1
        });

        parents.clear();
        for parent in row.parent_ids.iter().copied().filter(|id| known.contains_key(id)) {
            if !parents.iter().any(|(id, _, _)| *id == parent) {
                parents.push((parent, columns.iter().position(|id| *id == parent), 0));
            }
        }
        next.clear();
        edges.clear();
        for (index, id) in columns[..current].iter().copied().enumerate() {
            let destination = next.len();
            next.push(id);
            edges.push((index, destination));
        }
        for (parent, old_position, destination) in &mut parents {
            *destination = match old_position {
                Some(position) if *position < current => *position,
                _ => {
                    let destination = next.len();
                    next.push(*parent);
                    match old_position {
                        Some(position) if *position != current => edges.push((*position, destination)),
                        _ => {}
                    }
                    destination
                }
            };
        }
        for (index, id) in columns.iter().copied().enumerate().skip(current + 1) {
            if parents.iter().any(|(_, position, _)| *position == Some(index)) {
                continue;
            }
            let destination = next.len();
            next.push(id);
            edges.push((index, destination));
        }
        for (_, _, destination) in &parents {
            edges.push((current, *destination));
        }
        row.lane = transition(columns.len(), next.len(), current, &edges, &mut cells, &mut lanes);
        std::mem::swap(&mut columns, &mut next);
    }
    lanes
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

    fn row(n: u8) -> LoadedCommit {
        let mut bytes = [0; 20];
        bytes[19] = n;
        Commit {
            id: ObjectId::Sha1(bytes),
            parent_ids: ParentIds::new(),
            lane: 0..0,
            committer_time: gix::date::Time::default(),
            author: Box::leak(Box::new(Author {
                name: b"author".as_bstr(),
                email: b"author@example.com".as_bstr(),
            })),
            attributions: 0..0,
            title: format!("commit {n}").into(),
        }
    }

    fn row_with_parents(n: u8, parents: &[u8]) -> LoadedCommit {
        let mut commit = row(n);
        commit.parent_ids = parents.iter().map(|n| row(*n).id).collect();
        commit
    }

    fn complete(app: &mut App) {
        let rows = app
            .start_lane_computation()
            .expect("a loading app starts lane computation");
        let (rows, lanes, lane_time) = compute_lanes(rows);
        app.finish_lane_computation(rows, lanes, lane_time);
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
            app.estimated_lane_width, 6,
            "the provisional alignment leaves two extra cells after two estimated graph lanes"
        );

        complete(&mut app);

        assert_eq!(
            app.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [row(4).id, row(3).id, row(2).id, row(1).id]
        );
        assert_eq!(
            app.rows.iter().map(|row| app.lane(row)).collect::<Vec<_>>(),
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
        let (rows, lanes, lane_time) = compute_lanes(rows);
        app.finish_lane_computation(rows, lanes, lane_time);

        assert_eq!(app.state, State::Complete);
        assert_eq!(
            app.rows[app.selected.expect("selection survives final ordering")].id,
            selected
        );
    }

    #[test]
    fn lane_reuses_a_parent_that_is_already_to_the_right() {
        let mut app = App::new(10);
        for row in [row_with_parents(4, &[2, 3]), row_with_parents(2, &[3]), row(3)] {
            app.extend_commits(vec![row]);
        }

        complete(&mut app);

        assert_eq!(
            app.rows.iter().map(|row| app.lane(row)).collect::<Vec<_>>(),
            ["●─┐ ", "●─┘ ", "● "]
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
        assert!(!app.show_author_name);
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
        app.extend_commits(vec![row(7)]);

        assert_eq!(app.update(Action::Copy), vec![Effect::Copy(row(7).id)]);
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
