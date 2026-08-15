use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
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
    pub has_agent_marker: bool,
    pub is_review: bool,
    pub signature: SignatureState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Metadata<T> {
    pub committer_time: gix::date::Time,
    pub author: &'static Author,
    pub attributions: Range<usize>,
    pub title: T,
    pub has_agent_marker: bool,
    pub is_review: bool,
    pub signature: SignatureState,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SignatureState {
    #[default]
    Unsigned,
    Unverified,
    Verifying,
    Verified,
    Failed,
    PendingRebase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
}

impl ChangeKind {
    pub(crate) fn letter(self) -> char {
        match self {
            ChangeKind::Added => 'A',
            ChangeKind::Modified => 'M',
            ChangeKind::Deleted => 'D',
            ChangeKind::Renamed => 'R',
            ChangeKind::Copied => 'C',
            ChangeKind::TypeChanged => 'T',
            ChangeKind::Unmerged => 'U',
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ChangesMode {
    #[default]
    Tree,
    Both,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChangePane {
    Tree,
    Worktree,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ChangesLayout {
    #[default]
    SideBySide,
    Stacked,
}

#[derive(Debug)]
pub(crate) struct ChangesView {
    pub selected: usize,
    pub offset: usize,
    pub horizontal_offset: usize,
    pub error: Option<String>,
    page: usize,
    max: usize,
    horizontal_page: usize,
    horizontal_max: usize,
}

impl Default for ChangesView {
    fn default() -> Self {
        Self {
            selected: 0,
            offset: 0,
            horizontal_offset: 0,
            error: None,
            page: 1,
            max: 0,
            horizontal_page: 1,
            horizontal_max: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ChangeGroup {
    #[default]
    Tree,
    Staged,
    Unstaged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PathChange {
    pub kind: ChangeKind,
    pub group: ChangeGroup,
    pub source: Option<BString>,
    pub path: BString,
    pub lines: Option<(u32, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Changes {
    pub parent: Option<ComparedParent>,
    pub range: Option<ComparedRange>,
    pub paths: Vec<PathChange>,
    pub diffs: Vec<crate::FileChange>,
    pub lines_added: u64,
    pub lines_removed: u64,
    pub has_tracked_changes: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ComparedRange {
    pub base: ObjectId,
    pub tip: ObjectId,
}

impl Changes {
    pub(crate) fn is_visible(&self) -> bool {
        self.parent.is_some() || self.range.is_some() || !self.paths.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ComparedParent {
    pub index: usize,
    pub total: usize,
    pub id: ObjectId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionRelation {
    Tracking { ahead: usize, behind: usize },
    Visible(usize),
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

    pub fn is_github_noreply(&self) -> bool {
        let suffix = b"@users.noreply.github.com";
        self.email
            .get(self.email.len().saturating_sub(suffix.len())..)
            .is_some_and(|email| email.eq_ignore_ascii_case(suffix))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Attribution {
    pub kind: AttributionKind,
    pub author: &'static Author,
}

impl Attribution {
    pub fn is_agent(&self) -> bool {
        self.author.is_bot() || self.kind == AttributionKind::Assisted
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
pub(crate) type SharedCommitRow = Arc<CommitRow>;

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
    MoveUpBy(usize),
    MoveDownBy(usize),
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
    ToggleEmail,
    ToggleTrailers,
    ToggleMailmap,
    CycleRefs,
    ToggleRefs,
    Refresh,
    ToggleHidden,
    ToggleHistoryDisplay,
    ToggleEdit,
    ToggleInformation,
    ToggleAlign,
    ToggleCommit,
    ToggleChanges,
    ToggleChangesFocus,
    CycleChangesParent,
    OpenDiff,
    Reword,
    NewCommit,
    NewEmptyCommit,
    ForkCommit,
    Amend,
    Stash,
    Spill,
    Split,
    Forget,
    Rebase,
    RebaseUpdate,
    Review,
    TimeTravel,
    Unpin,
    VerifySignatures,
    Cancel,
    Copy,
    CopyPath(BString),
    CopyAuthor,
    ForceQuit,
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Effect {
    Cancel,
    CopyId(ObjectId),
    CopyPath(BString),
    CopyAuthor(&'static Author),
    Reload(bool),
    OpenDiff(ChangePane, usize),
    OpenCommitDiff(TreeDiffTarget),
    Reword(ObjectId),
    NewCommit {
        parent: Option<ObjectId>,
        empty: bool,
    },
    ForkCommit(ObjectId),
    Amend(ObjectId),
    Stash(ObjectId),
    Unstash(ObjectId),
    Spill(ObjectId),
    Split(ObjectId),
    Forget(ObjectId),
    Rebase {
        base: ObjectId,
        onto: ObjectId,
        commits: Vec<ObjectId>,
    },
    StartReview {
        tip: ObjectId,
        base: ObjectId,
    },
    FinishReview(ObjectId),
    TimeTravel(ObjectId),
    Unpin(ObjectId),
    VerifySignatures(Vec<ObjectId>),
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TreeDiffTarget {
    Commit { id: ObjectId, parent: usize },
    Branch { base: ObjectId, tip: ObjectId },
}

impl TreeDiffTarget {
    pub(crate) fn selected(self) -> ObjectId {
        match self {
            TreeDiffTarget::Commit { id, .. } => id,
            TreeDiffTarget::Branch { base, .. } => base,
        }
    }
}

#[derive(Debug)]
pub(crate) struct App {
    pub rows: Vec<SharedCommitRow>,
    all_rows: HashMap<ObjectId, SharedCommitRow>,
    all_order: Vec<ObjectId>,
    hidden_rows: HashSet<ObjectId>,
    hidden_branch_targets: HashMap<ObjectId, ObjectId>,
    hidden_rebase_bases: HashSet<ObjectId>,
    pending_hidden_rows: Option<HashSet<ObjectId>>,
    titles: Vec<u8>,
    notes: HashMap<ObjectId, Vec<BString>>,
    graph: Option<Graph>,
    attributions: Vec<Attribution>,
    #[cfg(test)]
    test_lanes: Vec<String>,
    pub selected: Option<usize>,
    pub offset: usize,
    pub state: State,
    pub(crate) deferred_history_state: Option<State>,
    pub viewport_rows: usize,
    pub lane_time: Option<Duration>,
    pub show_committer_date: bool,
    pub name_mode: NameMode,
    pub show_emails: bool,
    pub show_trailers: bool,
    pub use_mailmap: bool,
    pub ref_mode: RefMode,
    visible_ref_mode: RefMode,
    pub has_hidden_filter: bool,
    pub show_hidden: bool,
    pub align_metadata: bool,
    pub show_commit: bool,
    pub changes_mode: Option<ChangesMode>,
    worktree_changes_available: bool,
    pub(crate) changes_suppressed: bool,
    pub(crate) changes_focus: Option<ChangePane>,
    pub(crate) changes_layout: ChangesLayout,
    pub(crate) tree_changes_visible: bool,
    pub(crate) worktree_changes_visible: bool,
    pub(crate) tree_changes: ChangesView,
    pub(crate) worktree_changes: ChangesView,
    pub(crate) changes_parent: usize,
    pub(crate) commit_offset: usize,
    pub(crate) commit_pane_background: Option<(u8, u8, u8)>,
    commit_page: usize,
    commit_max: usize,
    pub(crate) show_selection_tail: bool,
    reachability_anchor: Option<ObjectId>,
    review_tip: Option<ObjectId>,
    reachable_rows: Option<Vec<bool>>,
    pub copy_feedback: Option<CopyKind>,
    pub(crate) focus_feedback: Option<&'static str>,
    message: Option<String>,
    pub(crate) unseen_filesystem_redraw: bool,
    pub(crate) history_display_expanded: bool,
    pub(crate) edit_expanded: bool,
    pub(crate) information_expanded: bool,
    forget_confirmation: Option<ObjectId>,
    pub estimated_lane_width: usize,
    pub horizontal_offset: usize,
    horizontal_page: usize,
    horizontal_max: usize,
    follow_tail: bool,
    reload_selection: Option<ObjectId>,
    pending_initial_selection: Option<ObjectId>,
    selection_after_refresh: Option<ObjectId>,
    worktree_head: Option<ObjectId>,
    worktree_head_has_descendants: bool,
    worktree_head_unborn: bool,
    pending_rebase_conflict: Option<ObjectId>,
    worktree_conflicted: bool,
    amend_available: bool,
    stash_available: bool,
    unstash_available: bool,
    worktree_path_amend_available: bool,
    finish_review_available: bool,
    spill_available: bool,
    split_available: bool,
    new_commit_available: bool,
    new_empty_commit_available: bool,
    known_descendants: HashSet<ObjectId>,
    known_merge_descendants: HashSet<ObjectId>,
    select_top_after_refresh: bool,
    pub(crate) signature_failures: usize,
    signature_verification_running: bool,
    pub(crate) selection_relation: Option<SelectionRelation>,
    hidden_branch_updates: HashMap<ObjectId, (usize, ObjectId)>,
}

impl App {
    pub fn new(viewport_rows: usize) -> Self {
        App {
            rows: Vec::new(),
            all_rows: HashMap::new(),
            all_order: Vec::new(),
            hidden_rows: HashSet::new(),
            hidden_branch_targets: HashMap::new(),
            hidden_rebase_bases: HashSet::new(),
            pending_hidden_rows: None,
            titles: Vec::new(),
            notes: HashMap::new(),
            graph: None,
            attributions: Vec::new(),
            #[cfg(test)]
            test_lanes: Vec::new(),
            selected: None,
            offset: 0,
            state: State::Loading,
            deferred_history_state: None,
            viewport_rows,
            lane_time: None,
            show_committer_date: true,
            name_mode: NameMode::All,
            show_emails: false,
            show_trailers: true,
            use_mailmap: true,
            ref_mode: RefMode::Default,
            visible_ref_mode: RefMode::Default,
            has_hidden_filter: false,
            show_hidden: false,
            align_metadata: true,
            show_commit: false,
            changes_mode: Some(ChangesMode::Both),
            worktree_changes_available: true,
            changes_suppressed: false,
            changes_focus: None,
            changes_layout: ChangesLayout::SideBySide,
            tree_changes_visible: false,
            worktree_changes_visible: false,
            tree_changes: ChangesView::default(),
            worktree_changes: ChangesView::default(),
            changes_parent: 0,
            commit_offset: 0,
            commit_pane_background: None,
            commit_page: 1,
            commit_max: 0,
            show_selection_tail: true,
            reachability_anchor: None,
            review_tip: None,
            reachable_rows: None,
            copy_feedback: None,
            focus_feedback: None,
            message: None,
            unseen_filesystem_redraw: false,
            history_display_expanded: false,
            edit_expanded: false,
            information_expanded: false,
            forget_confirmation: None,
            estimated_lane_width: 0,
            horizontal_offset: 0,
            horizontal_page: 1,
            horizontal_max: 0,
            follow_tail: false,
            reload_selection: None,
            pending_initial_selection: None,
            selection_after_refresh: None,
            worktree_head: None,
            worktree_head_has_descendants: false,
            worktree_head_unborn: false,
            pending_rebase_conflict: None,
            worktree_conflicted: false,
            amend_available: false,
            stash_available: false,
            unstash_available: false,
            worktree_path_amend_available: false,
            finish_review_available: false,
            spill_available: false,
            split_available: false,
            new_commit_available: true,
            new_empty_commit_available: true,
            known_descendants: HashSet::new(),
            known_merge_descendants: HashSet::new(),
            select_top_after_refresh: false,
            signature_failures: 0,
            signature_verification_running: false,
            selection_relation: None,
            hidden_branch_updates: HashMap::new(),
        }
    }

    pub(crate) fn leave_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    pub(crate) fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub(crate) fn configure_hidden_filter(&mut self, present: bool) {
        self.has_hidden_filter = present;
    }

    pub(crate) fn set_worktree_head(&mut self, head: Option<ObjectId>, select_on_load: bool) {
        if !select_on_load
            && self.worktree_head != head
            && self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) == self.worktree_head
        {
            self.selection_after_refresh = head;
        }
        self.worktree_head = head;
        self.pending_initial_selection = select_on_load.then_some(head).flatten();
        self.update_worktree_head_descendants();
    }

    pub(crate) fn set_worktree_head_unborn(&mut self, unborn: bool) {
        self.worktree_head_unborn = unborn;
    }

    pub(crate) fn arm_rebase_conflict(&mut self, id: ObjectId) {
        tracing::warn!(
            commit_id = %id,
            "rebase suspended on conflict; press <enter> to checkout for resolution"
        );
        self.pending_rebase_conflict = Some(id);
        self.leave_message("rebase conflict · <enter> checkout for resolution · any other key cancel");
    }

    pub(crate) fn has_rebase_conflict(&self) -> bool {
        self.pending_rebase_conflict.is_some()
    }

    pub(crate) fn clear_rebase_conflict(&mut self) {
        self.pending_rebase_conflict = None;
        self.message = None;
    }

    pub(crate) fn begin_conflict_resolution(&mut self) {
        self.pending_rebase_conflict = None;
        self.worktree_conflicted = true;
        self.changes_mode = Some(ChangesMode::Both);
        self.changes_focus = None;
    }

    pub(crate) fn set_worktree_conflicted(&mut self, conflicted: bool) {
        self.worktree_conflicted = conflicted;
    }

    pub(crate) fn conflict_marker(&self, id: ObjectId, head: bool) -> bool {
        self.pending_rebase_conflict == Some(id) || (head && self.worktree_conflicted)
    }

    pub(crate) fn set_known_descendants(&mut self, ids: HashSet<ObjectId>) {
        self.known_descendants = ids;
        self.update_worktree_head_descendants();
    }

    pub(crate) fn set_known_merge_descendants(&mut self, ids: HashSet<ObjectId>) {
        self.known_merge_descendants = ids;
    }

    pub(crate) fn worktree_head_has_descendants(&self, id: ObjectId) -> bool {
        self.worktree_head == Some(id) && self.worktree_head_has_descendants
    }

    pub(crate) fn extend_commits(&mut self, commits: impl Into<LoadedCommits>) {
        let commits = commits.into();
        if self.state != State::Loading || commits.rows.is_empty() {
            return;
        }
        let rows = self.store_commits(commits);
        let was_empty = self.rows.is_empty();
        self.rows.reserve(rows.len());
        for row in rows {
            self.rows.push(row);
        }
        if was_empty {
            self.estimated_lane_width = estimate_lane_width(&self.rows[..self.viewport_rows.min(self.rows.len())]);
            self.selected = self.first_selectable();
            self.ensure_visible();
        } else if self.follow_tail {
            self.selected = self.last_selectable();
            self.ensure_visible();
        }
        if let Some(index) = self
            .reload_selection
            .and_then(|id| self.rows.iter().position(|row| row.id == id))
        {
            self.selected = Some(index);
            self.reload_selection = None;
            self.ensure_visible();
        }
        if let Some(index) = self
            .pending_initial_selection
            .and_then(|id| self.rows.iter().position(|row| row.id == id))
        {
            self.selected = Some(index);
            self.ensure_visible();
        }
        if self.reachability_anchor.is_some() {
            self.compute_reachable_rows();
        }
    }

    fn store_commits(&mut self, commits: LoadedCommits) -> Vec<SharedCommitRow> {
        let LoadedCommits { rows, attributions } = commits;
        if !self.worktree_head_has_descendants
            && let Some(head) = self.worktree_head
        {
            self.worktree_head_has_descendants = rows.iter().any(|row| row.parent_ids.contains(&head));
        }
        self.titles.reserve(rows.iter().map(|row| row.title.len()).sum());
        let attribution_base = self.attributions.len();
        self.attributions.extend(attributions);
        rows.into_iter()
            .map(|row| {
                let start = self.titles.len();
                self.titles.extend_from_slice(&row.title);
                let row = Commit {
                    id: row.id,
                    parent_ids: row.parent_ids,
                    committer_time: row.committer_time,
                    author: row.author,
                    attributions: attribution_base + row.attributions.start..attribution_base + row.attributions.end,
                    title: start..self.titles.len(),
                    metadata_loaded: row.metadata_loaded,
                    has_agent_marker: row.has_agent_marker,
                    is_review: row.is_review,
                    signature: row.signature,
                };
                let row = Arc::new(row);
                if self.all_rows.insert(row.id, Arc::clone(&row)).is_none() {
                    self.all_order.push(row.id);
                }
                row
            })
            .collect()
    }

    pub(crate) fn extend_hidden_commits(&mut self, commits: impl Into<LoadedCommits>) {
        let commits = commits.into();
        self.hidden_rows.extend(commits.rows.iter().map(|row| row.id));
        self.extend_commits(commits);
    }

    pub(crate) fn is_row_hidden(&self, index: usize) -> bool {
        self.rows
            .get(index)
            .is_some_and(|row| self.hidden_rows.contains(&row.id))
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
        let row = Arc::make_mut(row);
        let Metadata {
            committer_time,
            author,
            attributions,
            title,
            has_agent_marker,
            is_review,
            signature,
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
        row.has_agent_marker = has_agent_marker;
        row.is_review = is_review;
        row.signature = signature;
        self.all_rows.insert(row.id, Arc::clone(&self.rows[index]));
    }

    pub(crate) fn title(&self, row: &CommitRow) -> &BStr {
        debug_assert!(row.metadata_loaded, "visible rows have metadata");
        self.titles[row.title.clone()].as_bstr()
    }

    pub(crate) fn notes_loaded(&self, id: ObjectId) -> bool {
        self.notes.contains_key(&id)
    }

    pub(crate) fn set_notes(&mut self, id: ObjectId, notes: Vec<BString>) {
        self.notes.insert(id, notes);
    }

    pub(crate) fn notes(&self, id: ObjectId) -> &[BString] {
        self.notes.get(&id).map(Vec::as_slice).unwrap_or_default()
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
        self.message = None;
        if !matches!(&action, Action::Forget) {
            self.forget_confirmation = None;
        }
        if !matches!(
            &action,
            Action::ToggleHistoryDisplay
                | Action::ToggleDate
                | Action::ToggleEmail
                | Action::ToggleName
                | Action::ToggleTrailers
                | Action::ToggleMailmap
                | Action::CycleRefs
                | Action::ToggleHidden
        ) {
            self.history_display_expanded = false;
        }
        if !matches!(
            &action,
            Action::ToggleEdit
                | Action::Reword
                | Action::NewCommit
                | Action::NewEmptyCommit
                | Action::ForkCommit
                | Action::Amend
                | Action::Stash
                | Action::Spill
                | Action::Split
                | Action::Forget
                | Action::Rebase
                | Action::RebaseUpdate
                | Action::Review
                | Action::TimeTravel
                | Action::Unpin
        ) {
            self.edit_expanded = false;
        }
        if !matches!(
            &action,
            Action::ToggleInformation
                | Action::VerifySignatures
                | Action::ToggleAlign
                | Action::ToggleCommit
                | Action::ToggleChanges
        ) {
            self.information_expanded = false;
        }
        match action {
            Action::Cancelled if self.state == State::Cancelling => self.state = State::Cancelled,
            Action::MoveUp if self.changes_focus.is_some() => self.move_changes(1, false),
            Action::MoveDown if self.changes_focus.is_some() => self.move_changes(1, true),
            Action::MoveUpBy(distance) if self.changes_focus.is_some() => self.move_changes(distance, false),
            Action::MoveDownBy(distance) if self.changes_focus.is_some() => self.move_changes(distance, true),
            Action::MoveUp => self.move_reachable(1, false),
            Action::MoveDown => self.move_reachable(1, true),
            Action::MoveUpBy(distance) => self.move_reachable(distance, false),
            Action::MoveDownBy(distance) => self.move_reachable(distance, true),
            Action::ScrollLeft => {
                if self.changes_focus.is_some() {
                    self.pan_changes(false);
                } else {
                    self.horizontal_offset = self.horizontal_offset.saturating_sub(self.horizontal_page);
                }
            }
            Action::ScrollRight => {
                if self.changes_focus.is_some() {
                    self.pan_changes(true);
                } else {
                    self.horizontal_offset = self
                        .horizontal_offset
                        .saturating_add(self.horizontal_page)
                        .min(self.horizontal_max);
                }
            }
            Action::HalfPageUp if self.changes_focus.is_some() => {
                self.move_changes((self.focused_changes().page / 2).max(1), false);
            }
            Action::HalfPageDown if self.changes_focus.is_some() => {
                self.move_changes((self.focused_changes().page / 2).max(1), true);
            }
            Action::PageUp if self.changes_focus.is_some() => self.move_changes(self.focused_changes().page, false),
            Action::PageDown if self.changes_focus.is_some() => self.move_changes(self.focused_changes().page, true),
            Action::PageUp if self.show_commit && self.commit_max > 0 => {
                self.commit_offset = self.commit_offset.saturating_sub(self.commit_page);
            }
            Action::PageDown if self.show_commit && self.commit_max > 0 => {
                self.commit_offset = self.commit_offset.saturating_add(self.commit_page).min(self.commit_max);
            }
            Action::HalfPageUp => self.move_selection((self.viewport_rows / 2).max(1), false),
            Action::HalfPageDown => self.move_selection((self.viewport_rows / 2).max(1), true),
            Action::PageUp => self.move_selection(self.viewport_rows.max(1), false),
            Action::PageDown => self.move_selection(self.viewport_rows.max(1), true),
            Action::First if self.changes_focus.is_some() => {
                let changes = self.focused_changes_mut();
                changes.selected = 0;
                changes.error = None;
                self.ensure_changes_visible();
            }
            Action::First => {
                if let Some(index) = self.first_selectable() {
                    self.select(index);
                }
            }
            Action::Last if self.changes_focus.is_some() => {
                let changes = self.focused_changes_mut();
                changes.selected = changes.max;
                changes.error = None;
                self.ensure_changes_visible();
            }
            Action::Last if self.last_selectable().is_some() => {
                self.pending_initial_selection = None;
                let previous = self.selected;
                self.selected = self.last_selectable();
                if self.selected != previous {
                    self.retry_failed_signatures();
                }
                self.follow_tail = self.state == State::Loading;
                self.ensure_visible();
            }
            Action::ToggleDate => self.show_committer_date = !self.show_committer_date,
            Action::ToggleEmail => self.show_emails = !self.show_emails,
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
            Action::ToggleHistoryDisplay => self.history_display_expanded = !self.history_display_expanded,
            Action::ToggleEdit => self.edit_expanded = !self.edit_expanded,
            Action::ToggleInformation => self.information_expanded = !self.information_expanded,
            Action::CycleRefs => {
                self.ref_mode = match self.ref_mode {
                    RefMode::All => RefMode::Default,
                    RefMode::Default => RefMode::None,
                    RefMode::None => RefMode::All,
                };
            }
            Action::ToggleRefs => match self.ref_mode {
                RefMode::None => self.ref_mode = self.visible_ref_mode,
                visible => {
                    self.visible_ref_mode = visible;
                    self.ref_mode = RefMode::None;
                }
            },
            Action::Refresh if matches!(self.state, State::Complete | State::Cancelled) => {
                return vec![Effect::Reload(self.show_hidden)];
            }
            Action::ToggleHidden
                if self.has_hidden_filter && matches!(self.state, State::Complete | State::Cancelled) =>
            {
                return vec![Effect::Reload(!self.show_hidden)];
            }
            Action::ToggleAlign => self.align_metadata = !self.align_metadata,
            Action::ToggleCommit => {
                self.show_commit = !self.show_commit;
                self.reset_commit_view();
            }
            Action::ToggleChanges => {
                self.focus_feedback = None;
                self.changes_mode = match self.changes_mode {
                    Some(ChangesMode::Both) => Some(ChangesMode::Tree),
                    Some(ChangesMode::Tree) => None,
                    None if self.worktree_changes_available => Some(ChangesMode::Both),
                    None => Some(ChangesMode::Tree),
                };
                self.reset_changes_view();
                self.changes_parent = 0;
                if self.changes_mode.is_none() {
                    self.changes_suppressed = false;
                    self.changes_focus = None;
                }
            }
            Action::ToggleChangesFocus if self.changes_mode.is_some() => {
                self.cycle_changes_focus();
                self.focus_feedback = Some(match self.changes_focus {
                    Some(ChangePane::Tree) => "tree changes",
                    Some(ChangePane::Worktree) => "worktree changes",
                    None => "history",
                });
            }
            Action::CycleChangesParent => {
                if self.changes_focus == Some(ChangePane::Tree) {
                    self.changes_parent = self.changes_parent.saturating_add(1);
                    self.tree_changes.error = None;
                }
            }
            Action::OpenDiff if self.changes_focus.is_some() => {
                let pane = self.changes_focus.expect("focus was checked");
                let changes = self.focused_changes_mut();
                changes.error = None;
                return vec![Effect::OpenDiff(pane, changes.selected)];
            }
            Action::OpenDiff if self.review_tip.is_some() => {
                let tip = self.review_tip.expect("review selection has a tip");
                let Some(base) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) else {
                    return Vec::new();
                };
                if base != tip && self.selected.is_some_and(|index| self.is_row_reachable(index)) {
                    self.clear_review_selection();
                    return vec![Effect::StartReview { tip, base }];
                }
            }
            Action::OpenDiff => {
                if let Some(target) = self.selected_tree_diff_target() {
                    return vec![Effect::OpenCommitDiff(target)];
                }
            }
            Action::Reword if self.can_reword() => {
                return vec![Effect::Reword(
                    self.rows[self.selected.expect("reword requires a selection")].id,
                )];
            }
            Action::NewCommit if self.can_create_commit() => {
                return vec![Effect::NewCommit {
                    parent: self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id),
                    empty: false,
                }];
            }
            Action::NewEmptyCommit if self.can_create_empty_commit() => {
                return vec![Effect::NewCommit {
                    parent: self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id),
                    empty: true,
                }];
            }
            Action::ForkCommit if self.can_fork_commit() => {
                return vec![Effect::ForkCommit(
                    self.rows[self.selected.expect("fork requires a selection")].id,
                )];
            }
            Action::Amend if self.can_amend() => {
                return vec![Effect::Amend(
                    self.rows[self.selected.expect("amend requires a selection")].id,
                )];
            }
            Action::Stash if self.can_unstash() => {
                return vec![Effect::Unstash(
                    self.rows[self.selected.expect("unstash requires a selection")].id,
                )];
            }
            Action::Stash if self.can_stash() => {
                return vec![Effect::Stash(
                    self.rows[self.selected.expect("stash requires a selection")].id,
                )];
            }
            Action::Spill if self.can_spill() => {
                return vec![Effect::Spill(
                    self.rows[self.selected.expect("spill requires a selection")].id,
                )];
            }
            Action::Split if self.can_split() => {
                return vec![Effect::Split(
                    self.rows[self.selected.expect("split requires a selection")].id,
                )];
            }
            Action::Forget if self.can_forget() => {
                let id = self.rows[self.selected.expect("forget requires a selection")].id;
                if self.forget_confirmation == Some(id) {
                    self.forget_confirmation = None;
                    return vec![Effect::Forget(id)];
                }
                self.forget_confirmation = Some(id);
            }
            Action::Rebase if self.can_rebase() => {
                let base = self.rows[self.selected.expect("rebase requires a selection")].id;
                return vec![Effect::Rebase {
                    base,
                    onto: base,
                    commits: self.hidden_descendants(base),
                }];
            }
            Action::RebaseUpdate if self.can_rebase_update() => {
                let base = self.rows[self.selected.expect("rebase-update requires a selection")].id;
                return vec![Effect::Rebase {
                    base,
                    onto: self.hidden_branch_updates[&base].1,
                    commits: self.hidden_descendants(base),
                }];
            }
            Action::Review if self.can_finish_review() => {
                return vec![Effect::FinishReview(
                    self.rows[self.selected.expect("finishing review requires a selection")].id,
                )];
            }
            Action::Review if self.can_review() => {
                let tip = self.rows[self.selected.expect("review requires a selection")].id;
                self.review_tip = Some(tip);
                self.reachability_anchor = Some(tip);
                self.compute_reachable_rows();
                self.leave_message("review base · j/k select ancestor · <enter> start · Esc cancel");
            }
            Action::TimeTravel if self.time_travel_shortcut_visible() => {
                return vec![Effect::TimeTravel(
                    self.rows[self.selected.expect("time-travel requires a selection")].id,
                )];
            }
            Action::Unpin => {
                if let Some(id) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) {
                    return vec![Effect::Unpin(id)];
                }
            }
            Action::VerifySignatures if !self.signature_verification_running => {
                let start = self.offset.min(self.rows.len());
                let end = start.saturating_add(self.viewport_rows).min(self.rows.len());
                let changed: Vec<_> = self.rows[start..end]
                    .iter_mut()
                    .filter(|row| !self.hidden_rows.contains(&row.id) && row.signature == SignatureState::Unverified)
                    .map(|row| {
                        Arc::make_mut(row).signature = SignatureState::Verifying;
                        (row.id, Arc::clone(row))
                    })
                    .collect();
                for (id, row) in &changed {
                    self.all_rows.insert(*id, Arc::clone(row));
                }
                let ids: Vec<_> = changed.into_iter().map(|(id, _)| id).collect();
                if !ids.is_empty() {
                    self.signature_verification_running = true;
                    return vec![Effect::VerifySignatures(ids)];
                }
            }
            Action::ForceQuit => return vec![Effect::Quit],
            Action::Cancel if self.review_tip.is_some() => self.clear_review_selection(),
            Action::Cancel | Action::Quit if self.changes_focus.is_some() => self.focus_history(),
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
            Action::CopyPath(path) => return vec![Effect::CopyPath(path)],
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

    pub(crate) fn start_lane_computation(&mut self) -> Option<Vec<SharedCommitRow>> {
        match self.state {
            State::Loading => {
                self.state = State::Computing;
                self.follow_tail = false;
                self.reload_selection = None;
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

    pub(crate) fn hidden_ids(&self) -> HashSet<ObjectId> {
        self.hidden_rows.clone()
    }

    pub(crate) fn set_hidden_branch_updates(&mut self, updates: HashMap<ObjectId, (usize, ObjectId)>) {
        self.hidden_branch_updates = updates;
    }

    pub(crate) fn hidden_branch_behind(&self, id: ObjectId) -> Option<usize> {
        self.hidden_branch_updates.get(&id).map(|(behind, _)| *behind)
    }

    pub(crate) fn selected_tree_diff_target(&self) -> Option<TreeDiffTarget> {
        let id = self.selected.and_then(|index| self.rows.get(index))?.id;
        Some(match self.hidden_branch_targets.get(&id) {
            Some(&tip) => TreeDiffTarget::Branch { base: id, tip },
            None => TreeDiffTarget::Commit {
                id,
                parent: self.changes_parent,
            },
        })
    }

    fn update_hidden_branch_targets(&mut self) {
        #[derive(Clone, Copy)]
        struct State {
            leaf: Option<ObjectId>,
            has_merge: bool,
        }

        let visible_parents: HashSet<_> = self
            .rows
            .iter()
            .filter(|row| !self.hidden_rows.contains(&row.id))
            .flat_map(|row| row.parent_ids.iter().copied())
            .collect();
        let mut states = HashMap::<ObjectId, State>::new();
        let mut targets = HashMap::new();
        let mut rebase_bases = HashSet::new();
        for row in &self.rows {
            let state = if self.hidden_rows.contains(&row.id) {
                states.get(&row.id).copied()
            } else if !visible_parents.contains(&row.id) {
                Some(State {
                    leaf: Some(row.id),
                    has_merge: row.parent_ids.len() > 1,
                })
            } else {
                states.get(&row.id).copied().map(|mut state| {
                    state.has_merge |= row.parent_ids.len() > 1;
                    state
                })
            };
            let Some(state) = state else { continue };
            if self.hidden_rows.contains(&row.id) {
                if let Some(tip) = state.leaf {
                    targets.insert(row.id, tip);
                }
                if !state.has_merge {
                    rebase_bases.insert(row.id);
                }
                continue;
            }
            for parent in &row.parent_ids {
                states
                    .entry(*parent)
                    .and_modify(|existing| {
                        if existing.leaf != state.leaf {
                            existing.leaf = None;
                        }
                        existing.has_merge |= state.has_merge;
                    })
                    .or_insert(state);
            }
        }
        self.hidden_branch_targets = targets;
        self.hidden_rebase_bases = rebase_bases;
    }

    fn hidden_descendants(&self, base: ObjectId) -> Vec<ObjectId> {
        let mut reachable = HashSet::from([base]);
        let mut out = Vec::new();
        for row in self.rows.iter().rev() {
            if self.hidden_rows.contains(&row.id) || !row.parent_ids.iter().any(|parent| reachable.contains(parent)) {
                continue;
            }
            reachable.insert(row.id);
            out.push(row.id);
        }
        out.reverse();
        out
    }

    pub(crate) fn hidden_rebase_candidates(&mut self) -> Vec<(ObjectId, Vec<ObjectId>)> {
        self.update_hidden_branch_targets();
        self.rows
            .iter()
            .filter(|row| self.hidden_rebase_bases.contains(&row.id))
            .map(|row| (row.id, self.hidden_descendants(row.id)))
            .collect()
    }

    pub(crate) fn start_refresh(
        &mut self,
        commits: LoadedCommits,
        view_tips: &[ObjectId],
        hidden_tips: &[ObjectId],
        select_top: bool,
    ) -> Option<Vec<SharedCommitRow>> {
        self.forget_confirmation = None;
        drop(self.store_commits(commits));

        let visible = self.reachable_from(view_tips);
        let hidden = self.reachable_from(hidden_tips);
        let visible: HashSet<_> = visible.difference(&hidden).copied().collect();
        let boundary: HashSet<_> = if hidden_tips.is_empty() {
            HashSet::new()
        } else {
            visible
                .iter()
                .filter_map(|id| self.all_rows.get(id))
                .flat_map(|row| row.parent_ids.iter().copied())
                .filter(|id| !visible.contains(id))
                .collect()
        };
        let rows: Vec<_> = self
            .all_order
            .iter()
            .filter(|id| visible.contains(*id) || boundary.contains(*id))
            .filter_map(|id| self.all_rows.get(id).map(Arc::clone))
            .collect();
        self.pending_hidden_rows = Some(boundary);
        self.select_top_after_refresh = select_top;
        self.state = State::Computing;
        self.follow_tail = false;
        Some(rows)
    }

    fn reachable_from(&self, tips: &[ObjectId]) -> HashSet<ObjectId> {
        let mut reachable = HashSet::new();
        let mut pending = tips.to_vec();
        while let Some(id) = pending.pop() {
            if !reachable.insert(id) {
                continue;
            }
            if let Some(row) = self.all_rows.get(&id) {
                pending.extend(row.parent_ids.iter().copied());
            }
        }
        reachable
    }

    pub(crate) fn finish_lane_computation(&mut self, rows: Vec<SharedCommitRow>, graph: Graph, lane_time: Duration) {
        if self.state != State::Computing {
            return;
        }
        let selected = if std::mem::take(&mut self.select_top_after_refresh) {
            self.selection_after_refresh = None;
            None
        } else {
            self.selection_after_refresh
                .take()
                .or_else(|| self.selected.map(|index| self.rows[index].id))
        };
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
                            has_agent_marker: row.has_agent_marker,
                            is_review: row.is_review,
                            signature: row.signature,
                        },
                    )
                })
                .collect()
        } else {
            HashMap::new()
        };
        self.rows = rows;
        if let Some(hidden) = self.pending_hidden_rows.take() {
            self.hidden_rows = hidden;
        }
        self.update_hidden_branch_targets();
        for row in &mut self.rows {
            if let Some(metadata) = metadata.get(&row.id) {
                let row = Arc::make_mut(row);
                row.committer_time = metadata.committer_time;
                row.author = metadata.author;
                row.attributions = metadata.attributions.clone();
                row.title = metadata.title.clone();
                row.metadata_loaded = true;
                row.has_agent_marker = metadata.has_agent_marker;
                row.signature = metadata.signature;
            }
        }
        self.graph = Some(graph);
        self.lane_time = Some(lane_time);
        self.update_worktree_head_descendants();
        self.selected = selected
            .and_then(|id| self.rows.iter().position(|row| row.id == id))
            .or_else(|| self.first_selectable());
        self.state = State::Complete;
        if self.reachability_anchor.is_some() {
            self.compute_reachable_rows();
        }
        self.ensure_visible();
    }

    #[cfg(test)]
    pub(crate) fn reload(&mut self, show_hidden: bool) {
        self.reload_selection = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id);
        self.select_top_after_refresh = false;
        self.rows = Vec::new();
        self.all_rows.clear();
        self.all_order.clear();
        self.hidden_rows.clear();
        self.hidden_branch_targets.clear();
        self.hidden_rebase_bases.clear();
        self.hidden_branch_updates.clear();
        self.pending_hidden_rows = None;
        self.titles = Vec::new();
        self.notes.clear();
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
        self.changes_suppressed = false;
        self.horizontal_offset = 0;
        self.focus_history();
        self.reset_commit_view();
        self.reset_changes_view();
        self.follow_tail = false;
        self.pending_initial_selection = None;
        self.selection_after_refresh = None;
        self.update_worktree_head_descendants();
        self.clear_review_selection();
        self.signature_failures = 0;
        self.signature_verification_running = false;
    }

    pub(crate) fn finish_signature_verification(&mut self, results: Vec<(ObjectId, bool)>) {
        let mut failed = 0;
        for (id, valid) in results {
            let Some(index) = self.rows.iter().position(|row| row.id == id) else {
                continue;
            };
            let row = Arc::make_mut(&mut self.rows[index]);
            row.signature = if valid {
                SignatureState::Verified
            } else {
                failed += 1;
                SignatureState::Failed
            };
            self.all_rows.insert(id, Arc::clone(&self.rows[index]));
        }
        self.signature_verification_running = false;
        self.signature_failures = failed;
    }

    fn move_changes(&mut self, distance: usize, down: bool) {
        let changes = self.focused_changes_mut();
        changes.error = None;
        changes.selected = if down {
            changes.selected.saturating_add(distance).min(changes.max)
        } else {
            changes.selected.saturating_sub(distance)
        };
        self.ensure_changes_visible();
    }

    fn clear_review_selection(&mut self) {
        self.review_tip = None;
        self.reachability_anchor = None;
        self.reachable_rows = None;
    }

    pub(crate) fn focus_history(&mut self) {
        self.changes_focus = None;
        self.focus_feedback = None;
    }

    pub(crate) fn set_worktree_changes_available(&mut self, available: bool) {
        self.worktree_changes_available = available;
        if !available {
            if self.changes_mode == Some(ChangesMode::Both) {
                self.changes_mode = Some(ChangesMode::Tree);
            }
            if self.changes_focus == Some(ChangePane::Worktree) {
                self.focus_history();
            }
        }
    }

    pub(crate) fn changes_visible(&self) -> bool {
        self.changes_mode.is_some() && !self.changes_suppressed && self.pending_rebase_conflict.is_none()
    }

    fn ensure_changes_visible(&mut self) {
        let changes = self.focused_changes_mut();
        if changes.selected < changes.offset {
            changes.offset = changes.selected;
        } else if changes.selected >= changes.offset.saturating_add(changes.page) {
            changes.offset = changes.selected + 1 - changes.page;
        }
        changes.offset = changes
            .offset
            .min(changes.max.saturating_add(1).saturating_sub(changes.page));
    }

    fn pan_changes(&mut self, right: bool) {
        let changes = self.focused_changes_mut();
        changes.horizontal_offset = if right {
            changes
                .horizontal_offset
                .saturating_add(changes.horizontal_page)
                .min(changes.horizontal_max)
        } else {
            changes.horizontal_offset.saturating_sub(changes.horizontal_page)
        };
    }

    fn move_selection(&mut self, distance: usize, down: bool) {
        self.pending_initial_selection = None;
        let Some(selected) = self.selected else { return };
        let target = if down {
            selected.saturating_add(distance).min(self.rows.len() - 1)
        } else {
            selected.saturating_sub(distance)
        };
        self.selected = self.nearest_selectable(target, down);
        if self.selected != Some(selected) {
            self.retry_failed_signatures();
        }
        self.follow_tail = false;
        self.ensure_visible();
    }

    fn move_reachable(&mut self, distance: usize, down: bool) {
        self.pending_initial_selection = None;
        let (Some(selected), Some(reachable)) = (self.selected, self.reachable_rows.as_ref()) else {
            self.move_selection(distance, down);
            return;
        };
        let distance = distance.max(1);
        let next = if down {
            (selected + 1..self.rows.len())
                .filter(|index| self.reachable_row_selectable(*index) && reachable.get(*index) == Some(&true))
                .nth(distance - 1)
        } else {
            (0..selected)
                .rev()
                .filter(|index| self.reachable_row_selectable(*index) && reachable.get(*index) == Some(&true))
                .nth(distance - 1)
        };
        if let Some(next) = next {
            self.select(next);
        }
    }

    fn compute_reachable_rows(&mut self) {
        if self.state != State::Complete {
            self.reachable_rows = None;
            return;
        }
        let Some(anchor) = self.reachability_anchor else {
            self.reachable_rows = None;
            return;
        };
        if !self.rows.iter().any(|row| row.id == anchor) {
            self.reachable_rows = Some(vec![false; self.rows.len()]);
            return;
        }
        let mut pending = HashSet::from([anchor]);
        let reachable: Vec<_> = self
            .rows
            .iter()
            .map(|row| {
                let reachable = pending.remove(&row.id);
                if reachable {
                    pending.extend(row.parent_ids.iter().copied());
                }
                reachable
            })
            .collect();
        self.reachable_rows = Some(reachable);
    }

    pub(crate) fn is_row_reachable(&self, index: usize) -> bool {
        self.reachable_rows
            .as_ref()
            .is_none_or(|reachable| reachable.get(index).copied().unwrap_or(false))
    }

    fn reachable_row_selectable(&self, index: usize) -> bool {
        !self.is_row_hidden(index)
            || (self.review_tip.is_some()
                && self
                    .rows
                    .get(index)
                    .is_some_and(|row| self.hidden_rebase_bases.contains(&row.id)))
    }

    fn select(&mut self, selected: usize) {
        self.pending_initial_selection = None;
        if !self.rows.is_empty() {
            let previous = self.selected;
            self.selected = Some(selected.min(self.rows.len() - 1));
            if self.selected != previous {
                self.retry_failed_signatures();
            }
            self.follow_tail = false;
            self.ensure_visible();
        }
    }

    fn first_selectable(&self) -> Option<usize> {
        (!self.rows.is_empty()).then_some(0)
    }

    fn update_worktree_head_descendants(&mut self) {
        self.worktree_head_has_descendants = self.worktree_head.is_some_and(|head| self.has_known_descendant(head));
    }

    fn has_known_descendant(&self, id: ObjectId) -> bool {
        self.known_descendants.contains(&id) || self.rows.iter().any(|row| row.parent_ids.contains(&id))
    }

    pub(crate) fn can_reword(&self) -> bool {
        self.state == State::Complete && self.reword_shortcut_visible()
    }

    pub(crate) fn reword_shortcut_visible(&self) -> bool {
        self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.selected.and_then(|index| self.rows.get(index)).is_some_and(|row| {
                !self.hidden_rows.contains(&row.id) && !self.known_merge_descendants.contains(&row.id)
            })
    }

    pub(crate) fn can_create_commit(&self) -> bool {
        self.new_commit_available && self.can_create_any_commit()
    }

    pub(crate) fn can_create_empty_commit(&self) -> bool {
        self.new_empty_commit_available && self.can_create_any_commit()
    }

    fn can_create_any_commit(&self) -> bool {
        self.state == State::Complete
            && self.worktree_changes_available
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && match self.selected.and_then(|index| self.rows.get(index)) {
                Some(row) => !self.hidden_rows.contains(&row.id) && !self.known_merge_descendants.contains(&row.id),
                None => self.worktree_head_unborn,
            }
    }

    pub(crate) fn set_new_commit_availability(&mut self, changes: Option<&Changes>) {
        let selected_is_head =
            self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) == self.worktree_head;
        let selected_is_head = selected_is_head || (self.selected.is_none() && self.worktree_head_unborn);
        match changes {
            Some(changes) if changes.paths.iter().any(|change| change.kind == ChangeKind::Unmerged) => {
                self.new_commit_available = false;
                self.new_empty_commit_available = false;
            }
            Some(changes) => {
                self.new_commit_available = selected_is_head && changes.has_tracked_changes;
                self.new_empty_commit_available = true;
            }
            None => {
                self.new_commit_available = true;
                self.new_empty_commit_available = true;
            }
        }
    }

    pub(crate) fn can_fork_commit(&self) -> bool {
        self.state == State::Complete
            && self.worktree_changes_available
            && !self.worktree_conflicted
            && self.pending_rebase_conflict.is_none()
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.selected.and_then(|index| self.rows.get(index)).is_some()
    }

    pub(crate) fn can_forget(&self) -> bool {
        self.state == State::Complete
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.selected.and_then(|index| self.rows.get(index)).is_some_and(|row| {
                !self.hidden_rows.contains(&row.id)
                    && row.parent_ids.len() <= 1
                    && !self.known_merge_descendants.contains(&row.id)
                    && (!row.is_review || !self.has_known_descendant(row.id))
            })
    }

    pub(crate) fn can_rebase(&self) -> bool {
        self.state == State::Complete
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self
                .selected
                .and_then(|index| self.rows.get(index))
                .is_some_and(|row| self.hidden_rebase_bases.contains(&row.id))
    }

    pub(crate) fn can_rebase_update(&self) -> bool {
        self.can_rebase()
            && self
                .selected
                .and_then(|index| self.rows.get(index))
                .is_some_and(|row| self.hidden_branch_updates.contains_key(&row.id))
    }

    pub(crate) fn can_review(&self) -> bool {
        self.state == State::Complete
            && self.worktree_changes_available
            && !self.worktree_conflicted
            && self.pending_rebase_conflict.is_none()
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.selected.and_then(|index| self.rows.get(index)).is_some_and(|row| {
                !self.hidden_rows.contains(&row.id)
                    && !row.parent_ids.is_empty()
                    && !self.known_merge_descendants.contains(&row.id)
                    && !row.is_review
            })
    }

    pub(crate) fn can_finish_review(&self) -> bool {
        self.state == State::Complete
            && self.finish_review_available
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self
                .selected
                .and_then(|index| self.rows.get(index))
                .is_some_and(|row| row.is_review && Some(row.id) == self.worktree_head)
    }

    pub(crate) fn review_selection_active(&self) -> bool {
        self.review_tip.is_some()
    }

    fn can_edit_head(&self) -> bool {
        self.state == State::Complete
            && self.worktree_changes_available
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.selected.and_then(|index| self.rows.get(index)).is_some_and(|row| {
                !self.hidden_rows.contains(&row.id)
                    && Some(row.id) == self.worktree_head
                    && !self.known_merge_descendants.contains(&row.id)
            })
    }

    pub(crate) fn can_amend(&self) -> bool {
        self.can_edit_head()
            && self.amend_available
            && match self.changes_focus {
                None => true,
                Some(ChangePane::Worktree) => self.worktree_path_amend_available,
                Some(ChangePane::Tree) => false,
            }
    }

    pub(crate) fn can_stash(&self) -> bool {
        self.state == State::Complete
            && self.worktree_changes_available
            && !self.worktree_conflicted
            && self.pending_rebase_conflict.is_none()
            && self.changes_focus.is_none()
            && self.stash_available
            && self
                .selected
                .and_then(|index| self.rows.get(index))
                .is_some_and(|row| !self.hidden_rows.contains(&row.id) && Some(row.id) == self.worktree_head)
    }

    pub(crate) fn can_unstash(&self) -> bool {
        self.state == State::Complete
            && self.pending_rebase_conflict.is_none()
            && self.changes_focus.is_none()
            && self.unstash_available
            && self
                .selected
                .and_then(|index| self.rows.get(index))
                .is_some_and(|row| !self.hidden_rows.contains(&row.id) && Some(row.id) == self.worktree_head)
    }

    pub(crate) fn can_spill(&self) -> bool {
        self.can_edit_head() && matches!(self.changes_focus, None | Some(ChangePane::Tree)) && self.spill_available
    }

    pub(crate) fn can_split(&self) -> bool {
        self.can_edit_head() && self.changes_focus.is_none() && self.split_available
    }

    #[expect(clippy::too_many_arguments)]
    pub(crate) fn set_head_edit_availability(
        &mut self,
        amend: bool,
        stash: bool,
        unstash: bool,
        worktree_path_amend: bool,
        finish_review: bool,
        spill: bool,
        split: bool,
    ) {
        self.amend_available = amend;
        self.stash_available = stash;
        self.unstash_available = unstash;
        self.worktree_path_amend_available = worktree_path_amend;
        self.finish_review_available = finish_review;
        self.spill_available = spill;
        self.split_available = split;
    }

    pub(crate) fn forget_confirmation_visible(&self) -> bool {
        self.selected
            .and_then(|index| self.rows.get(index))
            .is_some_and(|row| self.forget_confirmation == Some(row.id))
    }

    pub(crate) fn select_commit(&mut self, id: ObjectId) {
        if let Some(index) = self.rows.iter().position(|row| row.id == id) {
            self.select(index);
        }
    }

    pub(crate) fn select_commit_after_refresh(&mut self, id: ObjectId) {
        self.selection_after_refresh = Some(id);
    }

    pub(crate) fn time_travel_shortcut_visible(&self) -> bool {
        self.worktree_changes_available
            && self.pending_rebase_conflict.is_none()
            && !self.worktree_conflicted
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.selected.and_then(|index| self.rows.get(index)).is_some()
    }

    fn last_selectable(&self) -> Option<usize> {
        self.rows.len().checked_sub(1)
    }

    fn nearest_selectable(&self, target: usize, _down: bool) -> Option<usize> {
        (!self.rows.is_empty()).then_some(target.min(self.rows.len() - 1))
    }

    fn retry_failed_signatures(&mut self) {
        if self.signature_failures == 0 {
            return;
        }
        let mut changed = Vec::new();
        for row in &mut self.rows {
            if row.signature == SignatureState::Failed {
                Arc::make_mut(row).signature = SignatureState::Unverified;
                changed.push((row.id, Arc::clone(row)));
            }
        }
        for (id, row) in changed {
            self.all_rows.insert(id, row);
        }
        self.signature_failures = 0;
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

    pub(crate) fn center_initial_selection(&mut self) {
        let Some(id) = self.pending_initial_selection else {
            return;
        };
        let Some(selected) = self.rows.iter().position(|row| row.id == id) else {
            if matches!(self.state, State::Complete | State::Cancelled) {
                self.pending_initial_selection = None;
            }
            return;
        };
        let height = self.viewport_rows.max(1);
        let centered = selected.saturating_sub(height / 2);
        let max_offset = self.rows.len().saturating_sub(height);
        self.selected = Some(selected);
        self.offset = centered.min(max_offset);
        if matches!(self.state, State::Complete | State::Cancelled) {
            self.pending_initial_selection = None;
        }
    }

    pub(crate) fn set_horizontal_bounds(&mut self, page: usize, max: usize) {
        self.horizontal_page = page.max(1);
        self.horizontal_max = max;
        self.horizontal_offset = self.horizontal_offset.min(max);
    }

    pub(crate) fn set_commit_bounds(&mut self, page: usize, max: usize) {
        self.commit_page = page.max(1);
        self.commit_max = max;
        self.commit_offset = self.commit_offset.min(max);
    }

    pub(crate) fn reset_commit_view(&mut self) {
        self.commit_offset = 0;
        self.commit_max = 0;
    }

    pub(crate) fn set_changes_bounds(
        &mut self,
        pane: ChangePane,
        page: usize,
        len: usize,
        horizontal_page: usize,
        horizontal_max: usize,
    ) {
        let changes = self.changes_mut(pane);
        changes.page = page.max(1);
        changes.max = len.saturating_sub(1);
        if len == 0 {
            changes.selected = 0;
            changes.offset = 0;
        } else {
            changes.selected = changes.selected.min(changes.max);
            if changes.selected < changes.offset {
                changes.offset = changes.selected;
            } else if changes.selected >= changes.offset.saturating_add(changes.page) {
                changes.offset = changes.selected + 1 - changes.page;
            }
            changes.offset = changes
                .offset
                .min(changes.max.saturating_add(1).saturating_sub(changes.page));
        }
        changes.horizontal_page = horizontal_page.max(1);
        changes.horizontal_max = horizontal_max;
        changes.horizontal_offset = changes.horizontal_offset.min(horizontal_max);
    }

    pub(crate) fn reset_changes_view(&mut self) {
        self.tree_changes = ChangesView::default();
        self.worktree_changes = ChangesView::default();
    }

    pub(crate) fn changes(&self, pane: ChangePane) -> &ChangesView {
        match pane {
            ChangePane::Tree => &self.tree_changes,
            ChangePane::Worktree => &self.worktree_changes,
        }
    }

    pub(crate) fn changes_mut(&mut self, pane: ChangePane) -> &mut ChangesView {
        match pane {
            ChangePane::Tree => &mut self.tree_changes,
            ChangePane::Worktree => &mut self.worktree_changes,
        }
    }

    fn focused_changes(&self) -> &ChangesView {
        self.changes(self.changes_focus.expect("changes are focused"))
    }

    fn focused_changes_mut(&mut self) -> &mut ChangesView {
        self.changes_mut(self.changes_focus.expect("changes are focused"))
    }

    fn cycle_changes_focus(&mut self) {
        let (first, second) = match self.changes_layout {
            ChangesLayout::SideBySide => (ChangePane::Tree, ChangePane::Worktree),
            ChangesLayout::Stacked => (ChangePane::Worktree, ChangePane::Tree),
        };
        let visible = |pane| match pane {
            ChangePane::Tree => self.tree_changes_visible,
            ChangePane::Worktree => self.worktree_changes_visible,
        };
        self.changes_focus = match self.changes_focus {
            None if visible(first) => Some(first),
            None if visible(second) => Some(second),
            Some(current) if current == first && visible(second) => Some(second),
            Some(_) | None => None,
        };
    }

    pub(crate) fn set_changes_layout(&mut self, layout: ChangesLayout, tree_visible: bool, worktree_visible: bool) {
        self.changes_layout = layout;
        self.tree_changes_visible = tree_visible;
        self.worktree_changes_visible = worktree_visible;
        if self.changes_focus == Some(ChangePane::Tree) && !tree_visible {
            self.changes_focus = worktree_visible.then_some(ChangePane::Worktree);
        } else if self.changes_focus == Some(ChangePane::Worktree) && !worktree_visible {
            self.changes_focus = tree_visible.then_some(ChangePane::Tree);
        }
    }

    #[cfg(test)]
    pub(crate) fn set_lane(&mut self, index: usize, lane: &str) {
        self.test_lanes.resize(self.rows.len(), String::new());
        self.test_lanes[index] = lane.into();
    }
}

fn estimate_lane_width(rows: &[SharedCommitRow]) -> usize {
    let mut rows = rows.to_vec();
    let known: HashMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
    for row in &mut rows {
        if row.parent_ids.iter().any(|id| !known.contains_key(id)) {
            Arc::make_mut(row).parent_ids.retain(|id| known.contains_key(id));
        }
    }
    let graph = Graph::new(&rows);
    graph
        .render(&rows, 0..rows.len())
        .iter()
        .map(|lane| lane.trim_end().chars().count().saturating_add(1))
        .max()
        .unwrap_or_default()
}

pub(crate) fn compute_lanes(mut rows: Vec<SharedCommitRow>) -> (Vec<SharedCommitRow>, Graph, Duration) {
    let positions: HashMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
    for row in &mut rows {
        if row.parent_ids.iter().any(|id| !positions.contains_key(id)) {
            Arc::make_mut(row).parent_ids.retain(|id| positions.contains_key(id));
        }
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
    fn new(rows: &[SharedCommitRow]) -> Self {
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

    fn render(&self, rows: &[SharedCommitRow], range: Range<usize>) -> RenderedLanes {
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
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }
    }

    fn row_with_parents(n: u8, parents: &[u8]) -> LoadedCommit {
        let mut commit = row(n);
        commit.parent_ids = parents.iter().map(|n| row(*n).id).collect();
        commit
    }

    #[test]
    fn recognizes_all_assistants_as_agents() {
        let assistant = Box::leak(Box::new(Author {
            name: b"Anything".as_bstr(),
            email: b"".as_bstr(),
        }));

        assert!(
            Attribution {
                kind: AttributionKind::Assisted,
                author: assistant,
            }
            .is_agent()
        );
        assert!(
            !Attribution {
                kind: AttributionKind::Reviewed,
                author: assistant,
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

    fn show_tree_changes(app: &mut App) {
        app.set_changes_layout(ChangesLayout::SideBySide, true, false);
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
    fn refresh_projects_from_an_append_only_commit_cache() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(3, &[2]), row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);

        let rows = app
            .start_refresh(vec![row_with_parents(4, &[3])].into(), &[id(4)], &[], false)
            .expect("a refresh computes lanes");
        assert_eq!(
            app.rows.len(),
            3,
            "the current frame stays intact while lanes are computed"
        );
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);
        assert_eq!(
            app.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [id(4), id(3), id(2), id(1)]
        );
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(3)),
            "ordinary refreshes preserve the selected commit"
        );

        let rows = app
            .start_refresh(Vec::<LoadedCommit>::new().into(), &[id(2)], &[], false)
            .expect("a rewind reprojects cached topology");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);
        assert_eq!(app.rows.iter().map(|row| row.id).collect::<Vec<_>>(), [id(2), id(1)]);

        let rows = app
            .start_refresh(Vec::<LoadedCommit>::new().into(), &[id(4)], &[], false)
            .expect("a fast-forward to retained commits needs no new objects");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);
        assert_eq!(
            app.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [id(4), id(3), id(2), id(1)]
        );
        assert!(
            app.rows
                .iter()
                .all(|row| Arc::ptr_eq(row, app.all_rows.get(&row.id).expect("visible rows remain cached"))),
            "the active projection shares its immutable rows with the append-only cache"
        );
    }

    #[test]
    fn lane_computation_keeps_cached_parents_outside_the_current_view() {
        let mut app = App::new(3);
        app.extend_commits(vec![row_with_parents(2, &[1])]);
        app.extend_hidden_commits(vec![row_with_parents(1, &[0])]);
        let rows = app
            .start_lane_computation()
            .expect("loading rows starts lane computation");
        let (rows, graph, elapsed) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, elapsed);

        let rows = app
            .start_refresh(vec![row(0)].into(), &[id(2)], &[], false)
            .expect("refresh projects the extended ancestry");
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [id(2), id(1), id(0)],
            "lane pruning does not disconnect cached ancestry needed by a later expansion"
        );
    }

    #[test]
    fn filesystem_refresh_retains_selection_or_uses_the_first_selectable_row() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(3, &[2]), row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);
        app.update(Action::MoveDown);
        app.update(Action::MoveDown);
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(1)));

        let rows = app
            .start_refresh(vec![row_with_parents(4, &[3])].into(), &[id(4)], &[], false)
            .expect("a filesystem refresh computes lanes");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);

        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(1)),
            "a still-visible selection survives new commits"
        );

        app.update(Action::First);
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(4)));
        let rows = app
            .start_refresh(Vec::<LoadedCommit>::new().into(), &[id(3)], &[], false)
            .expect("a filesystem rewind computes lanes");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);
        assert_eq!(app.selected, app.first_selectable());
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(3)),
            "a removed selection falls back to the first selectable row"
        );
    }

    #[test]
    fn refresh_selects_the_rewritten_successor() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(3, &[2]), row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);
        app.update(Action::MoveDown);
        app.select_commit_after_refresh(id(4));

        let rows = app
            .start_refresh(
                vec![row_with_parents(5, &[4]), row_with_parents(4, &[1])].into(),
                &[id(5)],
                &[],
                false,
            )
            .expect("a rewritten stack computes lanes");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);

        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(4)),
            "selection follows the successor supplied by the edit"
        );
    }

    #[test]
    fn selected_head_follows_an_external_rewrite() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(3, &[2]), row_with_parents(2, &[1]), row(1)]);
        app.set_worktree_head(Some(id(3)), false);
        complete(&mut app);
        app.set_worktree_head(Some(id(5)), false);

        let rows = app
            .start_refresh(
                vec![row_with_parents(5, &[4]), row_with_parents(4, &[1])].into(),
                &[id(5)],
                &[],
                false,
            )
            .expect("an externally rewritten stack computes lanes");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);

        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(5)),
            "the selected HEAD follows its new target"
        );
    }

    #[test]
    fn completed_non_merge_stacks_can_be_reworded_from_any_row() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(2, &[1]), row(1)]);
        assert!(!app.can_reword(), "loading history cannot be reworded");
        complete(&mut app);
        assert_eq!(app.update(Action::Reword), vec![Effect::Reword(id(2))]);

        app.update(Action::MoveDown);
        assert!(app.can_reword(), "linear descendants can be rebased after rewording");
        assert_eq!(app.update(Action::Reword), vec![Effect::Reword(id(1))]);
    }

    #[test]
    fn head_edits_are_limited_to_the_current_worktree_head_and_available_changes() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(2, &[1]), row(1)]);
        app.set_worktree_head(Some(id(2)), false);
        app.set_head_edit_availability(true, true, false, true, false, true, false);
        complete(&mut app);
        assert_eq!(app.update(Action::Amend), vec![Effect::Amend(id(2))]);
        assert_eq!(app.update(Action::Stash), vec![Effect::Stash(id(2))]);
        assert_eq!(app.update(Action::Spill), vec![Effect::Spill(id(2))]);
        assert!(
            app.update(Action::Split).is_empty(),
            "split needs both kinds of changes"
        );
        app.set_head_edit_availability(true, true, false, true, false, true, true);
        assert_eq!(app.update(Action::Split), vec![Effect::Split(id(2))]);
        app.set_head_edit_availability(true, false, true, true, false, true, true);
        assert_eq!(
            app.update(Action::Stash),
            vec![Effect::Unstash(id(2))],
            "the stash key restores existing state instead of saving another stash"
        );
        app.changes_focus = Some(ChangePane::Tree);
        assert!(!app.can_amend(), "a tree path cannot be amended");
        assert!(!app.can_split(), "a tree path cannot be split");
        assert_eq!(
            app.update(Action::Spill),
            vec![Effect::Spill(id(2))],
            "tree focus scopes spill to its selected path"
        );
        app.changes_focus = Some(ChangePane::Worktree);
        assert!(!app.can_spill(), "worktree paths cannot be spilled from a commit");
        assert_eq!(
            app.update(Action::Amend),
            vec![Effect::Amend(id(2))],
            "worktree focus scopes amend to its selected path"
        );
        app.changes_focus = None;
        app.update(Action::MoveDown);
        assert!(!app.can_amend());
        assert!(!app.can_stash());
        assert!(app.update(Action::Amend).is_empty());
    }

    #[test]
    fn forgetting_a_non_merge_tip_requires_a_second_d_and_navigation_cancels_it() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(2, &[1]), row(1)]);
        assert!(!app.can_forget(), "loading history cannot forget commits");
        complete(&mut app);
        assert!(app.can_forget());
        assert!(
            app.update(Action::Forget).is_empty(),
            "the first d only arms confirmation"
        );
        assert!(app.forget_confirmation_visible());
        app.update(Action::MoveDown);
        assert!(!app.forget_confirmation_visible(), "navigation cancels confirmation");
        assert!(app.can_forget(), "a commit with linear descendants can be forgotten");
        app.update(Action::MoveUp);
        assert!(app.update(Action::Forget).is_empty());
        assert_eq!(app.update(Action::Forget), vec![Effect::Forget(id(2))]);

        let mut merge = App::new(10);
        merge.extend_commits(vec![row_with_parents(3, &[2, 1]), row(2), row(1)]);
        complete(&mut merge);
        assert!(!merge.can_forget(), "merge commits are not forgettable");
    }

    #[test]
    fn editing_rejects_merge_descendants_and_new_commits_support_unborn_head() {
        let mut app = App::new(10);
        app.extend_commits(vec![row(2)]);
        complete(&mut app);
        app.set_known_descendants(HashSet::from([id(2)]));
        app.set_known_merge_descendants(HashSet::from([id(2)]));
        assert!(
            !app.can_reword(),
            "a merge descendant outside the visible projection prevents rewording"
        );
        assert!(
            !app.can_create_commit(),
            "a merge descendant outside the visible projection prevents a child"
        );
        assert!(app.can_fork_commit(), "a fork does not rewrite merge descendants");
        assert_eq!(app.update(Action::ForkCommit), vec![Effect::ForkCommit(id(2))]);

        let mut unborn = App::new(10);
        unborn.set_worktree_head_unborn(true);
        complete(&mut unborn);
        assert!(
            unborn.can_create_commit(),
            "an unborn worktree can create its root commit"
        );
        assert_eq!(
            unborn.update(Action::NewCommit),
            vec![Effect::NewCommit {
                parent: None,
                empty: false,
            }]
        );
        assert!(!unborn.can_fork_commit(), "a fork requires a selected parent");
    }

    #[test]
    fn cached_worktree_changes_choose_new_or_new_empty_without_io() {
        let mut app = App::new(10);
        app.extend_commits(vec![row(1)]);
        app.set_worktree_head(Some(id(1)), false);
        complete(&mut app);

        app.set_new_commit_availability(Some(&Changes::default()));
        assert!(
            !app.can_create_commit(),
            "a known-clean worktree cannot create an implicit empty commit"
        );
        assert!(
            app.can_create_empty_commit(),
            "an explicit empty commit remains available"
        );

        let tracked = Changes {
            has_tracked_changes: true,
            ..Changes::default()
        };
        app.set_new_commit_availability(Some(&tracked));
        assert!(app.can_create_commit());
        assert!(app.can_create_empty_commit());

        let conflicted = Changes {
            paths: vec![PathChange {
                kind: ChangeKind::Unmerged,
                group: ChangeGroup::Unstaged,
                source: None,
                path: "conflict".into(),
                lines: None,
            }],
            has_tracked_changes: true,
            ..Changes::default()
        };
        app.set_new_commit_availability(Some(&conflicted));
        assert!(!app.can_create_commit());
        assert!(!app.can_create_empty_commit());

        app.set_new_commit_availability(None);
        assert!(app.can_create_commit(), "an absent cache leaves both choices available");
        assert!(app.can_create_empty_commit());
    }

    #[test]
    fn time_travel_requires_completed_history_and_a_worktree() {
        let mut app = App::new(10);
        app.extend_commits(vec![row(1)]);
        assert!(app.update(Action::TimeTravel).is_empty());
        complete(&mut app);
        assert_eq!(app.update(Action::TimeTravel), vec![Effect::TimeTravel(id(1))]);
        app.set_worktree_changes_available(false);
        assert!(app.update(Action::TimeTravel).is_empty());
        assert_eq!(app.update(Action::Unpin), vec![Effect::Unpin(id(1))]);
    }

    #[test]
    fn unresolved_conflicts_disable_time_travel() {
        let mut app = App::new(2);
        app.extend_commits(vec![row_with_parents(2, &[1]), row(1)]);
        app.set_worktree_head(Some(id(2)), false);
        complete(&mut app);
        app.update(Action::MoveDown);
        assert!(app.time_travel_shortcut_visible());

        app.set_worktree_conflicted(true);
        assert!(!app.time_travel_shortcut_visible());
        assert!(app.update(Action::TimeTravel).is_empty());
        app.set_worktree_conflicted(false);
        assert!(app.changes_visible(), "changes are normally shown while enabled");
        app.arm_rebase_conflict(id(1));
        assert!(!app.time_travel_shortcut_visible());
        assert!(
            !app.changes_visible(),
            "an in-memory conflict preview cannot be loaded by an on-disk changes view"
        );
        app.clear_rebase_conflict();
        assert!(app.time_travel_shortcut_visible());
        assert!(app.changes_visible(), "clearing the preview restores the changes view");
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
                has_agent_marker: false,
                is_review: false,
                signature: SignatureState::Unsigned,
            },
            Vec::new(),
        );
        let (rows, graph, lane_time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, lane_time);

        assert!(app.rows[0].metadata_loaded);
        assert_eq!(app.title(&app.rows[0]), "loaded");
    }

    #[test]
    fn verifies_only_visible_unchecked_signatures() {
        let mut app = App::new(2);
        app.extend_commits(vec![row(1), row(2), row(3)]);
        for row in &mut app.rows {
            Arc::make_mut(row).signature = SignatureState::Unverified;
        }
        app.offset = 1;

        assert_eq!(
            app.update(Action::VerifySignatures),
            vec![Effect::VerifySignatures(vec![id(2), id(3)])]
        );
        assert_eq!(app.rows[0].signature, SignatureState::Unverified);
        assert_eq!(app.rows[1].signature, SignatureState::Verifying);
        app.finish_signature_verification(vec![(id(2), true), (id(3), false)]);
        assert_eq!(app.rows[1].signature, SignatureState::Verified);
        assert_eq!(app.rows[2].signature, SignatureState::Failed);
        assert_eq!(app.signature_failures, 1);

        app.update(Action::MoveDown);
        assert_eq!(app.rows[2].signature, SignatureState::Unverified);
        assert_eq!(app.signature_failures, 0);
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
    fn review_selects_only_a_strict_ancestor_before_starting() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(5, &[3]),
            row_with_parents(4, &[2]),
            row_with_parents(3, &[1]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.selected = app.rows.iter().position(|row| row.id == id(5));
        app.hidden_rows.insert(id(1));
        app.hidden_rebase_bases.insert(id(1));

        assert!(app.can_review());
        assert!(app.update(Action::Review).is_empty());
        let tip_index = app.rows.iter().position(|row| row.id == id(5)).expect("tip is present");
        let unrelated = app
            .rows
            .iter()
            .position(|row| row.id == id(4))
            .expect("unrelated row is present");
        assert!(app.is_row_reachable(tip_index));
        assert!(!app.is_row_reachable(unrelated));
        app.update(Action::MoveDown);
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(3)));
        app.update(Action::MoveDown);
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(1)));
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![Effect::StartReview {
                tip: id(5),
                base: id(1),
            }]
        );
        assert!(
            app.is_row_reachable(unrelated),
            "confirming restores ordinary navigation"
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
    fn startup_selection_follows_the_worktree_head_until_the_user_moves() {
        let mut app = App::new(2);
        app.set_worktree_head(Some(id(2)), true);
        app.extend_commits(vec![row_with_parents(3, &[2])]);
        assert_eq!(app.selected, Some(0), "the newest row is selected provisionally");
        assert!(
            app.worktree_head_has_descendants(id(2)),
            "a streamed child marks HEAD as having visible descendants"
        );

        app.extend_commits(vec![row(2)]);
        assert_eq!(app.selected, Some(1), "selection moves to HEAD when its row arrives");
        complete(&mut app);
        assert_eq!(app.selected, Some(1), "lane computation retains the HEAD selection");

        let mut moved = App::new(2);
        moved.set_worktree_head(Some(id(2)), true);
        moved.extend_commits(vec![row_with_parents(3, &[2])]);
        moved.update(Action::MoveDown);
        moved.extend_commits(vec![row(2)]);
        assert_eq!(moved.selected, Some(0), "navigation cancels the pending jump to HEAD");
    }

    #[test]
    fn startup_selection_centers_the_worktree_head_once_the_viewport_is_known() {
        let mut app = App::new(1);
        app.set_worktree_head(Some(id(4)), true);
        app.extend_commits((1..=7).rev().map(row).collect::<Vec<_>>());
        assert_eq!(
            app.offset, 3,
            "the provisional one-row viewport only keeps HEAD visible"
        );

        app.viewport_rows = 5;
        app.center_initial_selection();
        assert_eq!(app.selected, Some(3), "HEAD remains selected");
        assert_eq!(app.offset, 1, "HEAD is centered once the real viewport height is known");
    }

    #[test]
    fn startup_head_selection_falls_back_when_head_is_unavailable() {
        let mut absent = App::new(2);
        absent.set_worktree_head(Some(id(9)), true);
        absent.extend_commits(vec![row(3), row(2)]);
        complete(&mut absent);
        assert_eq!(absent.selected, Some(0), "an absent HEAD retains the newest selection");

        let mut hidden = App::new(2);
        hidden.set_worktree_head(Some(id(2)), true);
        hidden.extend_commits(vec![row(3)]);
        hidden.extend_hidden_commits(vec![row(2)]);
        complete(&mut hidden);
        assert_eq!(
            hidden.rows[hidden.selected.expect("the boundary HEAD is selected")].id,
            id(2),
            "a selectable boundary retains the normal startup HEAD selection"
        );
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
        app.update(Action::MoveDownBy(3));
        assert_eq!(
            app.selected,
            Some(3),
            "batched mouse navigation moves once by its full distance"
        );
        app.update(Action::MoveUpBy(2));
        assert_eq!(app.selected, Some(1));
    }

    #[test]
    fn hidden_boundary_rows_are_selectable_for_inspection_and_independent_edits() {
        let mut app = App::new(4);
        app.extend_commits(vec![row(1), row(2), row(3)]);
        app.extend_hidden_commits(vec![row(4)]);
        complete(&mut app);
        Arc::make_mut(&mut app.rows[3]).signature = SignatureState::Unverified;

        app.update(Action::First);
        app.update(Action::PageDown);
        assert_eq!(app.selected, Some(3), "paging can select the hidden boundary");
        assert_eq!(app.update(Action::Copy), vec![Effect::CopyId(id(4))]);
        assert!(!app.can_reword());
        assert!(!app.can_forget());
        assert!(app.can_fork_commit());
        assert_eq!(app.update(Action::ForkCommit), vec![Effect::ForkCommit(id(4))]);
        assert_eq!(app.update(Action::TimeTravel), vec![Effect::TimeTravel(id(4))]);
        assert!(
            app.update(Action::VerifySignatures).is_empty(),
            "hidden signatures are not actionable"
        );
    }

    #[test]
    fn hidden_boundary_branch_diffs_require_one_descendant_leaf() {
        let target = |visible: Vec<LoadedCommit>| {
            let mut app = App::new(10);
            app.extend_commits(visible);
            app.extend_hidden_commits(vec![row(1)]);
            complete(&mut app);
            app.select_commit(id(1));
            app.selected_tree_diff_target()
        };

        assert_eq!(
            target(vec![row_with_parents(3, &[2]), row_with_parents(2, &[1])]),
            Some(TreeDiffTarget::Branch {
                base: id(1),
                tip: id(3),
            }),
            "a linear branch compares its boundary to its only leaf"
        );
        assert_eq!(
            target(vec![
                row_with_parents(4, &[2, 3]),
                row_with_parents(2, &[1]),
                row_with_parents(3, &[1]),
            ]),
            Some(TreeDiffTarget::Branch {
                base: id(1),
                tip: id(4),
            }),
            "forks which merge again still have one leaf"
        );
        assert_eq!(
            target(vec![row_with_parents(2, &[1]), row_with_parents(3, &[1])]),
            Some(TreeDiffTarget::Commit { id: id(1), parent: 0 }),
            "multiple leaves retain the boundary commit's ordinary diff"
        );
    }

    #[test]
    fn hidden_boundary_rebase_accepts_forks_but_rejects_descendant_merges() {
        let eligible = |visible: Vec<LoadedCommit>| {
            let mut app = App::new(10);
            app.extend_commits(visible);
            app.extend_hidden_commits(vec![row(1)]);
            complete(&mut app);
            app.select_commit(id(1));
            app
        };

        let mut fork = eligible(vec![row_with_parents(3, &[1]), row_with_parents(2, &[1])]);
        assert!(fork.can_rebase(), "forked linear stacks can be edited together");
        assert_eq!(
            fork.update(Action::Rebase),
            vec![Effect::Rebase {
                base: id(1),
                onto: id(1),
                commits: vec![id(3), id(2)],
            }],
            "the todo receives all visible descendants"
        );
        assert!(!fork.can_rebase_update(), "there is no newer hidden branch tip yet");
        fork.set_hidden_branch_updates(HashMap::from([(id(1), (2, id(4)))]));
        assert!(
            fork.can_rebase_update(),
            "a hidden branch ahead of the base can be used"
        );
        assert_eq!(
            fork.update(Action::RebaseUpdate),
            vec![Effect::Rebase {
                base: id(1),
                onto: id(4),
                commits: vec![id(3), id(2)],
            }],
            "rebase-update retains the editable scope but changes its root"
        );

        let merged = eligible(vec![
            row_with_parents(4, &[3, 2]),
            row_with_parents(3, &[1]),
            row_with_parents(2, &[1]),
        ]);
        assert!(!merged.can_rebase(), "a merge across editable descendants is rejected");
    }

    #[test]
    fn full_pages_target_changes_then_commit_messages_then_history() {
        let mut app = App::new(2);
        app.extend_commits((1..=5).map(row).collect::<Vec<_>>());
        app.show_commit = true;
        app.set_commit_bounds(3, 7);

        app.update(Action::PageDown);
        assert_eq!(app.commit_offset, 3);
        assert_eq!(app.selected, Some(0), "commit paging leaves history selection alone");
        app.update(Action::PageDown);
        assert_eq!(app.commit_offset, 6);

        app.changes_focus = Some(ChangePane::Tree);
        app.set_changes_bounds(ChangePane::Tree, 2, 5, 1, 0);
        app.update(Action::PageDown);
        assert_eq!(app.tree_changes.selected, 2, "focused changes retain paging priority");
        assert_eq!(app.commit_offset, 6);

        app.changes_focus = None;
        app.set_commit_bounds(3, 0);
        app.update(Action::PageDown);
        assert_eq!(app.selected, Some(2), "history paging resumes when the commit fits");
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
    fn focused_changes_redirect_navigation_to_the_path_viewport() {
        let mut app = App::new(2);
        app.extend_commits((1..=3).map(row).collect::<Vec<_>>());
        app.set_changes_bounds(ChangePane::Tree, 4, 10, 20, 45);
        show_tree_changes(&mut app);
        app.update(Action::ToggleChangesFocus);
        assert_eq!(app.changes_focus, Some(ChangePane::Tree));
        assert_eq!(app.focus_feedback.take(), Some("tree changes"));
        app.update(Action::ToggleChangesFocus);
        assert_eq!(app.changes_focus, None);
        assert_eq!(app.focus_feedback.take(), Some("history"));
        app.update(Action::ToggleChangesFocus);

        app.update(Action::MoveDown);
        assert_eq!((app.tree_changes.selected, app.tree_changes.offset), (1, 0));
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![Effect::OpenDiff(ChangePane::Tree, 1)]
        );
        assert_eq!(
            app.selected,
            Some(0),
            "path selection leaves commit selection untouched"
        );
        app.update(Action::PageDown);
        assert_eq!((app.tree_changes.selected, app.tree_changes.offset), (5, 2));
        app.update(Action::HalfPageDown);
        assert_eq!((app.tree_changes.selected, app.tree_changes.offset), (7, 4));
        app.update(Action::Last);
        assert_eq!((app.tree_changes.selected, app.tree_changes.offset), (9, 6));
        app.update(Action::First);
        assert_eq!((app.tree_changes.selected, app.tree_changes.offset), (0, 0));

        app.update(Action::ScrollRight);
        app.update(Action::ScrollRight);
        app.update(Action::ScrollRight);
        assert_eq!(app.tree_changes.horizontal_offset, 45);
        assert_eq!(app.horizontal_offset, 0, "path panning leaves the graph untouched");
        app.update(Action::ScrollLeft);
        assert_eq!(app.tree_changes.horizontal_offset, 25);

        app.update(Action::ToggleChanges);
        app.update(Action::ToggleChanges);
        assert_eq!(app.changes_focus, None, "closing the panel returns focus to history");
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![Effect::OpenCommitDiff(TreeDiffTarget::Commit { id: id(1), parent: 0 })]
        );
        assert_eq!(app.tree_changes.selected, 0);
        assert_eq!(app.tree_changes.offset, 0);
        assert_eq!(app.tree_changes.horizontal_offset, 0);
    }

    #[test]
    fn toggles_metadata_columns() {
        let mut app = App::new(1);
        assert!(app.show_trailers, "trailer attribution is visible by default");
        assert_eq!(
            app.changes_mode,
            Some(ChangesMode::Both),
            "tree and worktree changes are visible by default"
        );

        app.update(Action::ToggleDate);
        app.update(Action::ToggleEmail);
        app.update(Action::ToggleName);
        app.update(Action::ToggleTrailers);
        app.update(Action::ToggleMailmap);
        app.update(Action::CycleRefs);
        app.update(Action::ToggleAlign);
        app.update(Action::ToggleCommit);
        app.update(Action::CycleChangesParent);
        app.update(Action::ToggleChanges);

        assert!(!app.show_committer_date);
        assert!(app.show_emails);
        assert_eq!(app.name_mode, NameMode::None);
        assert!(!app.show_trailers);
        assert!(!app.use_mailmap);
        assert_eq!(app.ref_mode, RefMode::None);
        app.update(Action::CycleRefs);
        assert_eq!(app.ref_mode, RefMode::All);
        app.update(Action::CycleRefs);
        assert_eq!(app.ref_mode, RefMode::Default);
        assert!(!app.align_metadata);
        assert!(app.show_commit);
        assert_eq!(app.changes_mode, Some(ChangesMode::Tree));
        assert_eq!(app.changes_parent, 0);
        app.update(Action::ToggleAlign);
        assert!(app.align_metadata);
    }

    #[test]
    fn reference_visibility_toggle_restores_the_mode_it_hid() {
        let mut app = App::new(1);
        app.update(Action::CycleRefs);
        app.update(Action::CycleRefs);
        assert_eq!(app.ref_mode, RefMode::All);
        app.update(Action::ToggleRefs);
        assert_eq!(app.ref_mode, RefMode::None);
        app.update(Action::ToggleRefs);
        assert_eq!(app.ref_mode, RefMode::All);
        app.update(Action::CycleRefs);
        assert_eq!(app.ref_mode, RefMode::Default);
        app.update(Action::ToggleRefs);
        app.update(Action::ToggleRefs);
        assert_eq!(app.ref_mode, RefMode::Default);
    }

    #[test]
    fn history_display_group_stays_open_only_for_grouped_actions() {
        let mut app = App::new(1);

        app.update(Action::ToggleHistoryDisplay);
        assert!(app.history_display_expanded);
        app.update(Action::ToggleDate);
        app.update(Action::ToggleEmail);
        assert!(
            app.history_display_expanded,
            "grouped display changes keep the group open"
        );

        app.update(Action::MoveDown);
        assert!(!app.history_display_expanded, "navigation collapses the group");

        app.update(Action::ToggleHistoryDisplay);
        app.update(Action::ToggleAlign);
        assert!(
            !app.history_display_expanded,
            "direct display commands also collapse the group"
        );

        app.update(Action::ToggleHistoryDisplay);
        app.update(Action::ToggleHistoryDisplay);
        assert!(!app.history_display_expanded, "the prefix key toggles the group");
    }

    #[test]
    fn edit_group_stays_open_only_for_grouped_actions() {
        let mut app = App::new(1);

        app.update(Action::ToggleEdit);
        assert!(app.edit_expanded);
        app.update(Action::Reword);
        app.update(Action::NewCommit);
        app.update(Action::Forget);
        app.update(Action::Rebase);
        app.update(Action::RebaseUpdate);
        assert!(app.edit_expanded, "grouped edit commands keep the group open");

        app.update(Action::MoveDown);
        assert!(!app.edit_expanded, "navigation collapses the group");

        app.update(Action::ToggleEdit);
        app.update(Action::ToggleHistoryDisplay);
        assert!(!app.edit_expanded, "opening the view group closes the edit group");
        assert!(app.history_display_expanded);

        app.update(Action::ToggleEdit);
        assert!(app.edit_expanded);
        assert!(
            !app.history_display_expanded,
            "opening the edit group closes the view group"
        );
        app.update(Action::ToggleEdit);
        assert!(!app.edit_expanded, "the prefix key toggles the group");
    }

    #[test]
    fn information_group_keeps_direct_actions_and_excludes_other_prefixes() {
        let mut app = App::new(1);

        app.update(Action::ToggleInformation);
        assert!(app.information_expanded);
        for action in [
            Action::ToggleAlign,
            Action::ToggleCommit,
            Action::ToggleChanges,
            Action::VerifySignatures,
        ] {
            app.update(action);
            assert!(app.information_expanded, "information actions keep the group open");
        }

        app.update(Action::MoveDown);
        assert!(!app.information_expanded, "navigation collapses the group");

        app.update(Action::ToggleInformation);
        app.update(Action::ToggleHistoryDisplay);
        assert!(!app.information_expanded, "opening view closes information");
        app.update(Action::ToggleInformation);
        assert!(!app.history_display_expanded, "opening information closes view");
        app.update(Action::ToggleEdit);
        assert!(!app.information_expanded, "opening edit closes information");
    }

    #[test]
    fn cycles_both_tree_and_hidden_changes() {
        let mut app = App::new(1);
        assert_eq!(app.changes_mode, Some(ChangesMode::Both));

        app.update(Action::ToggleChanges);
        assert_eq!(app.changes_mode, Some(ChangesMode::Tree));

        app.changes_focus = Some(ChangePane::Tree);
        app.update(Action::ToggleChanges);
        assert_eq!(app.changes_mode, None);
        assert_eq!(app.changes_focus, None, "hiding changes returns focus to history");

        app.update(Action::ToggleChanges);
        assert_eq!(app.changes_mode, Some(ChangesMode::Both));
    }

    #[test]
    fn bare_repositories_cycle_only_tree_and_hidden_changes() {
        let mut app = App::new(1);
        app.changes_focus = Some(ChangePane::Worktree);

        app.set_worktree_changes_available(false);
        assert_eq!(app.changes_mode, Some(ChangesMode::Tree));
        assert_eq!(app.changes_focus, None, "a hidden worktree pane cannot retain focus");

        app.update(Action::ToggleChanges);
        assert_eq!(app.changes_mode, None);
        app.update(Action::ToggleChanges);
        assert_eq!(app.changes_mode, Some(ChangesMode::Tree));
    }

    #[test]
    fn cycles_changes_focus_in_visual_order_and_keeps_navigation_independent() {
        let mut app = App::new(1);
        app.changes_mode = Some(ChangesMode::Both);
        app.set_changes_bounds(ChangePane::Tree, 2, 4, 10, 20);
        app.set_changes_bounds(ChangePane::Worktree, 2, 4, 10, 20);
        app.set_changes_layout(ChangesLayout::SideBySide, true, true);

        app.update(Action::ToggleChangesFocus);
        assert_eq!(app.changes_focus, Some(ChangePane::Tree));
        app.update(Action::MoveDown);
        app.update(Action::ToggleChangesFocus);
        assert_eq!(app.changes_focus, Some(ChangePane::Worktree));
        app.update(Action::MoveDown);
        assert_eq!(app.tree_changes.selected, 1);
        assert_eq!(app.worktree_changes.selected, 1);
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![Effect::OpenDiff(ChangePane::Worktree, 1)]
        );
        app.update(Action::ToggleChangesFocus);
        assert_eq!(app.changes_focus, None);

        app.set_changes_layout(ChangesLayout::Stacked, true, true);
        app.update(Action::ToggleChangesFocus);
        assert_eq!(app.changes_focus, Some(ChangePane::Worktree));
        app.update(Action::ToggleChangesFocus);
        assert_eq!(app.changes_focus, Some(ChangePane::Tree));

        app.set_changes_layout(ChangesLayout::Stacked, false, true);
        assert_eq!(app.changes_focus, Some(ChangePane::Worktree));
        app.set_changes_layout(ChangesLayout::Stacked, false, false);
        assert_eq!(app.changes_focus, None);
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

        app.configure_hidden_filter(true);
        assert_eq!(
            app.ref_mode,
            RefMode::Default,
            "hidden ancestry keeps the normal reference display"
        );
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
    fn refresh_reloads_only_finished_history() {
        let mut app = App::new(1);
        assert!(
            app.update(Action::Refresh).is_empty(),
            "a running walk cannot be replaced"
        );

        app.extend_commits(vec![row(1)]);
        complete(&mut app);
        assert_eq!(app.update(Action::Refresh), vec![Effect::Reload(false)]);

        app.show_hidden = true;
        app.state = State::Cancelled;
        assert_eq!(
            app.update(Action::Refresh),
            vec![Effect::Reload(true)],
            "refresh preserves the hidden-history setting"
        );
    }

    #[test]
    fn reload_retains_selection_or_falls_back_to_the_top() {
        let mut app = App::new(3);
        app.extend_commits(vec![row(1), row(2), row(3)]);
        complete(&mut app);
        app.update(Action::MoveDown);
        let selected = app.rows[app.selected.expect("a row is selected")].id;
        app.set_changes_bounds(ChangePane::Tree, 1, 3, 1, 2);
        show_tree_changes(&mut app);
        app.update(Action::ToggleChangesFocus);
        app.update(Action::MoveDown);
        app.update(Action::ScrollRight);

        app.reload(true);
        assert_eq!(app.changes_focus, None, "reload returns focus to history");
        assert_eq!(app.tree_changes.selected, 0);
        assert_eq!((app.tree_changes.offset, app.tree_changes.horizontal_offset), (0, 0));
        app.extend_commits(vec![row(1), row(2), row(3)]);
        complete(&mut app);
        assert_eq!(
            app.rows[app.selected.expect("the old row remains selected")].id,
            selected
        );

        app.reload(false);
        app.extend_commits(vec![row(3)]);
        app.extend_hidden_commits(vec![row(2)]);
        complete(&mut app);
        assert_eq!(
            app.rows[app.selected.expect("the boundary selection is retained")].id,
            selected,
            "a selection which becomes a hidden boundary remains selected"
        );
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
    fn pane_exit_keys_return_to_history_but_control_c_quits() {
        let mut app = App::new(1);
        show_tree_changes(&mut app);
        app.update(Action::ToggleChangesFocus);

        assert!(app.update(Action::Quit).is_empty());
        assert_eq!(app.changes_focus, None, "q returns focus to history");

        app.update(Action::ToggleChangesFocus);
        assert_eq!(
            app.update(Action::ForceQuit),
            vec![Effect::Quit],
            "Ctrl-C quits even while changes have focus"
        );
        assert!(app.update(Action::Cancel).is_empty());
        assert_eq!(app.changes_focus, None, "Escape returns focus to history");
        assert_eq!(
            app.state,
            State::Loading,
            "Escape does not cancel while changes had focus"
        );

        assert_eq!(app.update(Action::Cancel), vec![Effect::Cancel]);
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
        assert_eq!(
            app.update(Action::CopyPath("dir/file".into())),
            vec![Effect::CopyPath("dir/file".into())]
        );
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
