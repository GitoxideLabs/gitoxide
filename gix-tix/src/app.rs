use std::{
    collections::{HashMap, HashSet},
    ops::Range,
    sync::Arc,
    time::{Duration, Instant},
};

use gix::{
    ObjectId,
    bstr::{BStr, BString, ByteSlice},
    hash::ChangeId,
    traverse::commit::ParentIds,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Commit<T> {
    pub id: ObjectId,
    pub parent_ids: ParentIds,
    pub committer_time: gix::date::Time,
    pub author_time: gix::date::Time,
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
    pub author_time: gix::date::Time,
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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum NoticeKind {
    Success,
    Attention,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Notice {
    pub kind: NoticeKind,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UndoPosition {
    applied: usize,
    total: usize,
    title: String,
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
    len: usize,
    max: usize,
    separator: Option<usize>,
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
            len: 0,
            max: 0,
            separator: None,
            horizontal_page: 1,
            horizontal_max: 0,
        }
    }
}

impl ChangesView {
    fn display_index(&self, path_index: usize) -> usize {
        path_index + usize::from(self.separator.is_some_and(|separator| path_index >= separator))
    }

    fn ensure_visible(&mut self) {
        let selected = self.display_index(self.selected);
        if selected < self.offset {
            self.offset = selected;
        } else if selected >= self.offset.saturating_add(self.page) {
            self.offset = selected + 1 - self.page;
        }
        let display_len = self.len + usize::from(self.separator.is_some());
        self.offset = self.offset.min(display_len.saturating_sub(self.page));
    }

    fn visible_paths(&self) -> usize {
        let end = self.offset.saturating_add(self.page);
        self.page.saturating_sub(usize::from(
            self.separator
                .is_some_and(|separator| separator >= self.offset && separator < end),
        ))
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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum DateMode {
    #[default]
    Author,
    Committer,
    None,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum IdMode {
    Commit,
    Change,
    #[default]
    Off,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CopyKind {
    Id,
    Author,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TopologicalDirection {
    Parent,
    Child,
}

#[derive(Debug)]
struct TopologicalNavigation {
    direction: TopologicalDirection,
    candidates: Vec<usize>,
    choice: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Cancelled,
    Undo,
    Redo,
    MoveUp,
    MoveDown,
    MoveUpBy(usize),
    MoveDownBy(usize),
    TopologicalUp,
    TopologicalDown,
    PanUpBy(usize),
    PanDownBy(usize),
    PreviousChild,
    NextChild,
    SubmitTopological,
    CancelTopological,
    CycleDuplicate,
    ScrollLeft,
    ScrollRight,
    HalfPageUp,
    HalfPageDown,
    PageUp,
    PageDown,
    First,
    Last,
    ToggleDate,
    CycleIds,
    ToggleName,
    ToggleEmail,
    ToggleTrailers,
    ToggleMailmap,
    CycleRefs,
    ToggleRefs,
    SelectEntry,
    SelectEntryInput(String),
    SelectEntryBackspace,
    SubmitEntrySelection,
    Refresh,
    ToggleHidden,
    ToggleHistoryDisplay,
    ToggleRefTree,
    ToggleActions,
    ToggleEnrich,
    ToggleTodo,
    ToggleChecksPass,
    EditNote,
    EditGitNote,
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
    Amend,
    Stash,
    Spill,
    Split,
    Forget,
    Rebase,
    RebaseUpdate,
    Squash,
    CopyInsert,
    PasteInsert { source: ObjectId, target: ObjectId },
    MoveInsert,
    StackInsert,
    Review,
    ForkCommit,
    Attach,
    TimeTravel,
    TogglePin,
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
    Undo,
    Redo,
    CopyId(ObjectId),
    CopyChangeId(ChangeId),
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
    Squash {
        source: ObjectId,
        target: ObjectId,
    },
    Insert {
        source: ObjectId,
        base: ObjectId,
        target: ObjectId,
        copy: bool,
    },
    PasteInsert {
        source: ObjectId,
        target: ObjectId,
    },
    StartReview {
        tip: ObjectId,
        base: ObjectId,
    },
    FinishReview {
        review: ObjectId,
        return_to: Option<ObjectId>,
    },
    ForkCommit(ObjectId),
    Attach,
    TimeTravel(ObjectId),
    TogglePin(ObjectId),
    ToggleTodo(ObjectId),
    ToggleChecksPass(ObjectId),
    EditNote(ObjectId),
    EditGitNote(ObjectId),
    VerifySignatures(Vec<ObjectId>),
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Alignment {
    None,
    Title,
    Columns,
    Compressed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HistoryEntry {
    Commit(usize),
    Segment { representative: usize, count: usize },
}

#[derive(Debug)]
struct CompressedHistory {
    entries: Vec<HistoryEntry>,
    display_indices: HashMap<usize, usize>,
    member_indices: HashMap<ObjectId, usize>,
    members: Vec<Vec<usize>>,
    rows: Vec<SharedCommitRow>,
    graph: Graph,
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
    enrichments: HashMap<ObjectId, crate::enrich::Enrichment>,
    tree_enrichments: HashMap<ObjectId, crate::enrich::TreeEnrichment>,
    graph: Option<Graph>,
    view_tips: HashSet<ObjectId>,
    compressed_history: Option<CompressedHistory>,
    compressed_anchor: Option<ObjectId>,
    compressed_segment: Option<ObjectId>,
    compressed_expanded: HashSet<ObjectId>,
    time_travel_animation: Option<(ObjectId, usize)>,
    attributions: Vec<Attribution>,
    #[cfg(test)]
    test_lanes: Vec<String>,
    pub selected: Option<usize>,
    pub offset: usize,
    viewport_panned: bool,
    pub state: State,
    pub(crate) deferred_history_state: Option<State>,
    pub viewport_rows: usize,
    pub lane_time: Option<Duration>,
    pub date_mode: DateMode,
    pub id_mode: IdMode,
    pub name_mode: NameMode,
    pub show_emails: bool,
    pub show_trailers: bool,
    pub use_mailmap: bool,
    pub ref_mode: RefMode,
    visible_ref_mode: RefMode,
    pub has_hidden_filter: bool,
    pub show_hidden: bool,
    pub(crate) alignment: Alignment,
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
    reachability_anchor: Option<ObjectId>,
    review_tip: Option<ObjectId>,
    review_return: Option<(ObjectId, ObjectId)>,
    squash_source: Option<ObjectId>,
    stack_insert_base: Option<ObjectId>,
    reachable_rows: Option<Vec<bool>>,
    pub copy_feedback: Option<CopyKind>,
    pub(crate) focus_feedback: Option<&'static str>,
    notice: Option<Notice>,
    entry_selection: Option<String>,
    undo_position: Option<UndoPosition>,
    pub(crate) unseen_filesystem_redraw: bool,
    pub(crate) history_display_expanded: bool,
    pub(crate) actions_expanded: bool,
    pub(crate) enrich_expanded: bool,
    pub(crate) information_expanded: bool,
    topological_navigation: Option<TopologicalNavigation>,
    pub estimated_lane_width: usize,
    pub horizontal_offset: usize,
    horizontal_page: usize,
    horizontal_max: usize,
    follow_tail: bool,
    reload_selection: Option<ObjectId>,
    pending_initial_selection: Option<ObjectId>,
    selection_after_refresh: Option<ObjectId>,
    worktree_head: Option<ObjectId>,
    worktree_branch: Option<(ObjectId, bool)>,
    worktree_head_has_descendants: bool,
    worktree_head_unborn: bool,
    pending_rebase_conflict: Option<ObjectId>,
    rebase_continuation_pending: bool,
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
    change_ids: HashMap<ObjectId, ChangeId>,
    duplicate_change_ids: HashSet<ObjectId>,
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
            enrichments: HashMap::new(),
            tree_enrichments: HashMap::new(),
            graph: None,
            view_tips: HashSet::new(),
            compressed_history: None,
            compressed_anchor: None,
            compressed_segment: None,
            compressed_expanded: HashSet::new(),
            time_travel_animation: None,
            attributions: Vec::new(),
            #[cfg(test)]
            test_lanes: Vec::new(),
            selected: None,
            offset: 0,
            viewport_panned: false,
            state: State::Loading,
            deferred_history_state: None,
            viewport_rows,
            lane_time: None,
            date_mode: DateMode::default(),
            id_mode: IdMode::default(),
            name_mode: NameMode::All,
            show_emails: false,
            show_trailers: true,
            use_mailmap: true,
            ref_mode: RefMode::Default,
            visible_ref_mode: RefMode::Default,
            has_hidden_filter: false,
            show_hidden: false,
            alignment: Alignment::Title,
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
            reachability_anchor: None,
            review_tip: None,
            review_return: None,
            squash_source: None,
            stack_insert_base: None,
            reachable_rows: None,
            copy_feedback: None,
            focus_feedback: None,
            notice: None,
            entry_selection: None,
            undo_position: None,
            unseen_filesystem_redraw: false,
            history_display_expanded: false,
            actions_expanded: false,
            enrich_expanded: false,
            information_expanded: false,
            topological_navigation: None,
            estimated_lane_width: 0,
            horizontal_offset: 0,
            horizontal_page: 1,
            horizontal_max: 0,
            follow_tail: false,
            reload_selection: None,
            pending_initial_selection: None,
            selection_after_refresh: None,
            worktree_head: None,
            worktree_branch: None,
            worktree_head_has_descendants: false,
            worktree_head_unborn: false,
            pending_rebase_conflict: None,
            rebase_continuation_pending: false,
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
            change_ids: HashMap::new(),
            duplicate_change_ids: HashSet::new(),
        }
    }

    pub(crate) fn effective_id_mode(&self) -> IdMode {
        match self.id_mode {
            IdMode::Off if !self.duplicate_change_ids.is_empty() => IdMode::Change,
            mode => mode,
        }
    }

    pub(crate) fn change_id(&self, id: ObjectId) -> ChangeId {
        self.change_ids.get(&id).copied().unwrap_or_else(|| id.into())
    }

    pub(crate) fn has_duplicate_change_id(&self, id: ObjectId) -> bool {
        self.duplicate_change_ids.contains(&id)
    }

    pub(crate) fn has_duplicate_change_ids(&self) -> bool {
        !self.duplicate_change_ids.is_empty()
    }

    pub(crate) fn can_cycle_duplicate(&self) -> bool {
        self.changes_focus.is_none() && self.reachable_rows.is_none() && self.next_duplicate().is_some()
    }

    pub(crate) fn set_change_ids(&mut self, change_ids: HashMap<ObjectId, ChangeId>, duplicates: HashSet<ObjectId>) {
        self.change_ids = change_ids;
        self.duplicate_change_ids = duplicates;
    }

    #[cfg(test)]
    fn clear_change_ids(&mut self) {
        self.change_ids.clear();
        self.duplicate_change_ids.clear();
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

    pub(crate) fn close_shortcut_groups(&mut self) {
        self.history_display_expanded = false;
        self.actions_expanded = false;
        self.enrich_expanded = false;
        self.information_expanded = false;
    }

    fn leave_notice(&mut self, kind: NoticeKind, message: impl Into<String>) {
        self.notice = Some(Notice {
            kind,
            text: message.into(),
        });
    }

    pub(crate) fn notice(&self) -> Option<Notice> {
        let prompt = if let Some(navigation) = self.topological_navigation.as_ref() {
            Some(format!(
                "choose {} {}/{} · h/l cycle · <enter> move · Esc cancel",
                match navigation.direction {
                    TopologicalDirection::Parent => "ancestor",
                    TopologicalDirection::Child => "child",
                },
                navigation.choice + 1,
                navigation.candidates.len()
            ))
        } else if let Some(entry) = self.entry_selection.as_deref() {
            Some(format!(
                "select entry #{} · type number · <enter> jump · Esc cancel",
                if entry.is_empty() { "_" } else { entry }
            ))
        } else if self.rebase_continuation_pending() {
            Some(if self.rebase_continuation_conflicted() {
                "REBASE PAUSED · resolve conflicts, then <enter> continue · Esc stop".into()
            } else {
                "REBASE PAUSED · <enter> continue · Esc stop".into()
            })
        } else if self.pending_rebase_conflict.is_some() {
            Some("rebase conflict · <enter> checkout for resolution · Esc cancel".into())
        } else if self.selected_is_segment() && self.reachable_rows.is_some() {
            Some("compressed segment · <enter> expand · Esc cancel".into())
        } else if self.review_return_selection_active() {
            Some("review return is missing · j/k select commit · <enter> finish detached · Esc cancel".into())
        } else if self.review_selection_active() {
            Some("review base · j/k select ancestor · <enter> start · Esc cancel".into())
        } else if self.squash_selection_active() {
            Some("squash target · j/k select ancestor · <enter> squash · Esc cancel".into())
        } else if self.stack_insert_base.is_some() {
            Some("stack-insert target · j/k select insertion point · <enter> insert · Esc cancel".into())
        } else {
            None
        };
        match (prompt, self.notice.as_ref()) {
            (Some(prompt), Some(notice)) if notice.text != prompt => Some(Notice {
                kind: notice.kind.max(NoticeKind::Attention),
                text: format!("{prompt} · {}", notice.text),
            }),
            (Some(prompt), _) => Some(Notice {
                kind: NoticeKind::Attention,
                text: prompt,
            }),
            (None, notice) => notice.cloned(),
        }
    }

    pub(crate) fn show_undo_position(&mut self, applied: usize, total: usize, title: impl Into<String>) {
        let applied = applied.min(total);
        let title = if applied == 0 {
            "start of undo history".into()
        } else {
            title.into()
        };
        self.leave_success(format!("{title} · {applied} undo · {} redo", total - applied));
        self.undo_position = Some(UndoPosition { applied, total, title });
    }

    pub(crate) fn undo_position(&self) -> Option<(usize, usize, &str)> {
        self.undo_position
            .as_ref()
            .map(|position| (position.applied, position.total, position.title.as_str()))
    }

    pub(crate) fn dismiss_undo_position(&mut self) {
        self.undo_position = None;
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

    pub(crate) fn set_worktree_branch(&mut self, branch: Option<(ObjectId, bool)>) {
        self.worktree_branch = branch;
    }

    pub(crate) fn set_worktree_head_unborn(&mut self, unborn: bool) {
        self.worktree_head_unborn = unborn;
    }

    pub(crate) fn set_view_tips(&mut self, tips: &[ObjectId]) {
        self.view_tips.clear();
        self.view_tips.extend(tips.iter().copied());
        if self.alignment == Alignment::Compressed {
            if let Some(id) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) {
                self.compressed_anchor = Some(id);
            }
            self.rebuild_compressed_history();
        }
    }

    pub(crate) fn arm_rebase_conflict(&mut self, id: ObjectId) {
        self.materialize_compressed_selection();
        tracing::warn!(
            commit_id = %id,
            "rebase suspended on conflict; press <enter> to checkout for resolution"
        );
        self.pending_rebase_conflict = Some(id);
        self.rebase_continuation_pending = false;
        if let Some(index) = self.rows.iter().position(|row| row.id == id) {
            self.select(index);
            self.center_selection();
        } else {
            self.selection_after_refresh = Some(id);
        }
    }

    pub(crate) fn has_rebase_conflict(&self) -> bool {
        self.pending_rebase_conflict.is_some()
    }

    pub(crate) fn has_conflict_marker(&self) -> bool {
        self.pending_rebase_conflict.is_some() || self.worktree_conflicted
    }

    pub(crate) fn clear_rebase_conflict(&mut self) {
        self.pending_rebase_conflict = None;
        self.notice = None;
        self.restore_compressed_history_around_selection();
    }

    pub(crate) fn begin_conflict_resolution(&mut self) {
        self.pending_rebase_conflict = None;
        self.worktree_conflicted = true;
        self.changes_mode = Some(ChangesMode::Both);
        self.changes_focus = None;
        self.restore_compressed_history_around_selection();
    }

    pub(crate) fn arm_rebase_continuation(&mut self) {
        self.materialize_compressed_selection();
        self.rebase_continuation_pending = true;
        self.ensure_visible();
    }

    pub(crate) fn clear_rebase_continuation(&mut self) {
        self.rebase_continuation_pending = false;
        self.restore_compressed_history_around_selection();
    }

    pub(crate) fn rebase_continuation_pending(&self) -> bool {
        self.rebase_continuation_pending
    }

    pub(crate) fn rebase_continuation_conflicted(&self) -> bool {
        self.rebase_continuation_pending && self.worktree_conflicted
    }

    pub(crate) fn set_worktree_conflicted(&mut self, conflicted: bool) {
        if self.worktree_conflicted == conflicted {
            return;
        }
        if conflicted {
            self.materialize_compressed_selection();
        }
        self.worktree_conflicted = conflicted;
        if conflicted {
            self.ensure_visible();
        } else {
            self.restore_compressed_history_around_selection();
        }
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
                    author_time: row.author_time,
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
            author_time,
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
        row.author_time = author_time;
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

    pub(crate) fn clear_notes(&mut self, id: ObjectId) {
        self.notes.remove(&id);
    }

    pub(crate) fn notes(&self, id: ObjectId) -> &[BString] {
        self.notes.get(&id).map(Vec::as_slice).unwrap_or_default()
    }

    pub(crate) fn enrichment_loaded(&self, id: ObjectId) -> bool {
        self.enrichments.contains_key(&id)
    }

    pub(crate) fn set_enrichment(&mut self, id: ObjectId, enrichment: crate::enrich::Enrichment) {
        self.enrichments.insert(id, enrichment);
    }

    pub(crate) fn todo(&self, id: ObjectId) -> bool {
        self.enrichments.get(&id).is_some_and(|enrichment| enrichment.todo)
    }

    pub(crate) fn note(&self, id: ObjectId) -> Option<&BStr> {
        self.enrichments
            .get(&id)
            .and_then(|enrichment| enrichment.note.as_ref().map(|note| note.as_bstr()))
    }

    pub(crate) fn tree_enrichment_loaded(&self, id: ObjectId) -> bool {
        self.tree_enrichments.contains_key(&id)
    }

    pub(crate) fn set_tree_enrichment(&mut self, id: ObjectId, enrichment: crate::enrich::TreeEnrichment) {
        self.tree_enrichments.insert(id, enrichment);
    }

    pub(crate) fn checks_pass(&self, id: ObjectId) -> bool {
        self.tree_enrichments
            .get(&id)
            .is_some_and(|enrichment| enrichment.checks_pass)
    }

    pub(crate) fn clear_enrichments(&mut self) {
        self.enrichments.clear();
        self.tree_enrichments.clear();
    }

    pub(crate) fn history_len(&self) -> usize {
        self.active_compressed_history()
            .map_or(self.rows.len(), |history| history.entries.len())
    }

    pub(crate) fn history_entry(&self, index: usize) -> Option<HistoryEntry> {
        self.active_compressed_history().map_or_else(
            || (index < self.rows.len()).then_some(HistoryEntry::Commit(index)),
            |history| history.entries.get(index).copied(),
        )
    }

    pub(crate) fn history_index(&self, canonical_index: usize) -> Option<usize> {
        self.active_compressed_history().map_or_else(
            || (canonical_index < self.rows.len()).then_some(canonical_index),
            |history| history.display_indices.get(&canonical_index).copied(),
        )
    }

    pub(crate) fn selected_history_index(&self) -> Option<usize> {
        self.selected
            .and_then(|selected| self.history_index(selected))
            .or_else(|| {
                let history = self.active_compressed_history()?;
                let selected = self.compressed_segment?;
                let index = *history.member_indices.get(&selected)?;
                matches!(history.entries[index], HistoryEntry::Segment { .. }).then_some(index)
            })
    }

    pub(crate) fn selected_is_segment(&self) -> bool {
        self.compressed_segment.is_some() && self.selected_history_index().is_some()
    }

    pub(crate) fn visible_history_indices(&self, range: Range<usize>) -> Vec<usize> {
        let start = range.start.min(self.history_len());
        let end = range.end.min(self.history_len());
        (start..end)
            .filter_map(|index| match self.history_entry(index) {
                Some(HistoryEntry::Commit(index)) => Some(index),
                Some(HistoryEntry::Segment { .. }) | None => None,
            })
            .collect()
    }

    pub(crate) fn has_verifiable_signatures(&self) -> bool {
        self.visible_history_indices(self.offset..self.offset.saturating_add(self.viewport_rows))
            .into_iter()
            .any(|index| {
                !self.hidden_rows.contains(&self.rows[index].id)
                    && matches!(
                        self.rows[index].signature,
                        SignatureState::Unverified | SignatureState::Verifying
                    )
            })
    }

    fn active_compressed_history(&self) -> Option<&CompressedHistory> {
        (self.alignment == Alignment::Compressed && !self.compressed_history_suspended())
            .then_some(self.compressed_history.as_ref())
            .flatten()
    }

    fn compressed_history_suspended(&self) -> bool {
        self.time_travel_animation.is_some()
            || self.pending_rebase_conflict.is_some()
            || self.rebase_continuation_pending
            || self.worktree_conflicted
    }

    fn rebuild_compressed_history(&mut self) {
        let previous_display = self.selected_history_index();
        self.compressed_expanded.retain(|id| self.all_rows.contains_key(id));
        let history = (self.graph.is_some() && !self.rows.is_empty()).then(|| {
            CompressedHistory::new(
                &self.rows,
                &self.view_tips,
                &self.hidden_rows,
                &self.compressed_expanded,
                self.compressed_anchor,
            )
        });
        let selection = self.compressed_segment.and_then(|id| {
            let history = history.as_ref()?;
            let display = *history.member_indices.get(&id)?;
            history.entries.get(display).copied()
        });
        let fallback = previous_display.and_then(|display| {
            let history = history.as_ref()?;
            history
                .entries
                .get(display.min(history.entries.len().saturating_sub(1)))
                .copied()
        });
        self.compressed_history = history;
        if self.compressed_segment.is_some() {
            match selection.or(fallback) {
                Some(HistoryEntry::Commit(index)) => {
                    self.selected = Some(index);
                    self.compressed_segment = None;
                }
                Some(HistoryEntry::Segment { representative, .. }) => {
                    self.selected = None;
                    self.compressed_segment = self.rows.get(representative).map(|row| row.id);
                }
                None => {
                    self.selected = (!self.rows.is_empty()).then_some(0);
                    self.compressed_segment = None;
                }
            }
        }
    }

    fn materialize_compressed_selection(&mut self) {
        let Some(id) = self.compressed_segment.take() else {
            return;
        };
        self.selected = self.rows.iter().position(|row| row.id == id);
    }

    pub(crate) fn history_entry_selectable(&self, display: usize) -> bool {
        match self.history_entry(display) {
            Some(HistoryEntry::Commit(index)) => self.history_row_selectable(index),
            Some(HistoryEntry::Segment { .. }) => self
                .active_compressed_history()
                .and_then(|history| history.members.get(display))
                .is_some_and(|members| members.iter().copied().any(|index| self.history_row_selectable(index))),
            None => false,
        }
    }

    fn history_row_selectable(&self, index: usize) -> bool {
        self.reachable_rows.as_ref().is_none() || (self.is_row_reachable(index) && self.reachable_row_selectable(index))
    }

    fn topological_graph(&self) -> Option<&Graph> {
        self.active_compressed_history()
            .map(|history| &history.graph)
            .or(self.graph.as_ref())
    }

    fn topological_candidates(&self, selected: usize, direction: TopologicalDirection) -> Vec<usize> {
        let fallback;
        let graph = match self.topological_graph() {
            Some(graph) => graph,
            None => {
                fallback = Graph::new(&self.rows);
                &fallback
            }
        };
        let mut pending = graph
            .neighbors(selected, direction)
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>();
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        while let Some(index) = pending.pop() {
            if !seen.insert(index) {
                continue;
            }
            if self.history_entry_selectable(index) {
                candidates.push(index);
            } else {
                pending.extend(graph.neighbors(index, direction).iter().rev().copied());
            }
        }
        if direction == TopologicalDirection::Child {
            candidates.sort_unstable();
        }
        candidates
    }

    fn move_topologically(&mut self, direction: TopologicalDirection) {
        let Some(selected) = self.selected_history_index() else {
            return;
        };
        if matches!(self.history_entry(selected), Some(HistoryEntry::Segment { .. })) {
            let _ = self.peel_compressed_segment(selected, direction == TopologicalDirection::Child, false);
            return;
        }
        let candidates = self.topological_candidates(selected, direction);
        let [next] = candidates.as_slice() else {
            if candidates.len() > 1 {
                self.topological_navigation = Some(TopologicalNavigation {
                    direction,
                    candidates,
                    choice: 0,
                });
                self.follow_tail = false;
                self.ensure_visible();
            }
            return;
        };
        self.move_topologically_to(*next, direction);
    }

    fn move_topologically_to(&mut self, next: usize, direction: TopologicalDirection) {
        if matches!(self.history_entry(next), Some(HistoryEntry::Segment { .. })) {
            let _ = self.peel_compressed_segment(next, direction == TopologicalDirection::Parent, true);
        } else {
            self.select_history_index(next);
        }
    }

    fn peel_compressed_segment(&mut self, display: usize, newest: bool, select_peeled: bool) -> Option<usize> {
        let members = self
            .active_compressed_history()
            .filter(|history| matches!(history.entries.get(display), Some(HistoryEntry::Segment { .. })))?
            .members
            .get(display)?
            .clone();
        let peeled = if newest { *members.first()? } else { *members.last()? };
        let remaining = if newest {
            *members.get(1)?
        } else {
            *members.get(members.len().checked_sub(2)?)?
        };
        let peeled_selectable = self.history_row_selectable(peeled);
        let remaining_selectable = members
            .iter()
            .copied()
            .filter(|index| *index != peeled)
            .any(|index| self.history_row_selectable(index));
        if !peeled_selectable && !remaining_selectable {
            return None;
        }

        self.compressed_expanded.insert(self.rows[peeled].id);
        self.compressed_segment = None;
        if select_peeled {
            self.rebuild_compressed_history();
            if peeled_selectable {
                self.select(peeled);
            } else {
                self.ensure_visible();
            }
        } else if members.len() == 2 || !remaining_selectable {
            self.rebuild_compressed_history();
            self.select(if remaining_selectable { remaining } else { peeled });
        } else {
            self.selected = None;
            self.compressed_segment = Some(self.rows[remaining].id);
            self.rebuild_compressed_history();
            self.ensure_visible();
        }
        Some(peeled)
    }

    fn adjust_topological_choice(&mut self, right: bool) {
        let Some(navigation) = self.topological_navigation.as_mut() else {
            return;
        };
        navigation.choice = if right {
            (navigation.choice + 1) % navigation.candidates.len()
        } else {
            navigation
                .choice
                .checked_sub(1)
                .unwrap_or(navigation.candidates.len() - 1)
        };
    }

    fn submit_topological_choice(&mut self) {
        let Some(navigation) = self.topological_navigation.take() else {
            return;
        };
        let Some(next) = navigation.candidates.get(navigation.choice).copied() else {
            return;
        };
        self.move_topologically_to(next, navigation.direction);
    }

    pub(crate) fn topological_choice(&self) -> Option<(usize, usize)> {
        self.topological_navigation
            .as_ref()
            .map(|navigation| (navigation.choice + 1, navigation.candidates.len()))
    }

    pub(crate) fn topological_navigation_active(&self) -> bool {
        self.topological_navigation.is_some()
    }

    fn select_history_index(&mut self, display: usize) {
        if !self.history_entry_selectable(display) {
            return;
        }
        self.topological_navigation = None;
        match self.history_entry(display) {
            Some(HistoryEntry::Commit(index)) => self.select(index),
            Some(HistoryEntry::Segment { representative, .. }) => {
                let id = self.rows[representative].id;
                let changed = self.selected.is_some() || self.compressed_segment != Some(id);
                self.selected = None;
                self.compressed_segment = Some(id);
                self.pending_initial_selection = None;
                self.actions_expanded = false;
                self.enrich_expanded = false;
                if changed {
                    self.retry_failed_signatures();
                }
                self.follow_tail = false;
                self.ensure_visible();
            }
            None => {}
        }
    }

    fn expand_selected_segment(&mut self) -> bool {
        let Some(display) = self.selected_history_index() else {
            return false;
        };
        let Some(HistoryEntry::Segment { .. }) = self.history_entry(display) else {
            return false;
        };
        let Some(members) = self
            .active_compressed_history()
            .and_then(|history| history.members.get(display))
            .cloned()
        else {
            return false;
        };
        let Some(selected) = members
            .iter()
            .copied()
            .find(|index| self.history_row_selectable(*index))
        else {
            return false;
        };
        self.compressed_expanded
            .extend(members.iter().map(|index| self.rows[*index].id));
        self.compressed_segment = None;
        self.rebuild_compressed_history();
        self.select(selected);
        true
    }

    fn restore_compressed_history_around_selection(&mut self) {
        if self.alignment != Alignment::Compressed {
            return;
        }
        if let Some(id) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) {
            self.compressed_anchor = Some(id);
        }
        self.rebuild_compressed_history();
        self.ensure_visible();
    }

    pub(crate) fn render_lanes(&self, range: Range<usize>) -> RenderedLanes {
        #[cfg(test)]
        if self.active_compressed_history().is_none() && !self.test_lanes.is_empty() {
            return RenderedLanes::from_lanes(
                self.test_lanes[range.start.min(self.test_lanes.len())..range.end.min(self.test_lanes.len())].iter(),
            );
        }
        let choice_marker = self.topological_choice().and_then(|(choice, _)| {
            self.selected_history_index().map(|index| {
                (
                    index,
                    if choice < 10 {
                        char::from(b'0' + choice as u8)
                    } else {
                        '+'
                    },
                )
            })
        });
        if let Some(history) = self.active_compressed_history() {
            return history.graph.render_with_markers(&history.rows, range, |index| {
                if choice_marker.is_some_and(|(selected, _)| selected == index) {
                    choice_marker.expect("the marker was checked above").1
                } else if matches!(history.entries[index], HistoryEntry::Segment { .. }) {
                    '○'
                } else {
                    '●'
                }
            });
        }
        match &self.graph {
            Some(graph) => graph.render_with_markers(&self.rows, range, |index| {
                choice_marker
                    .filter(|(selected, _)| *selected == index)
                    .map_or('●', |(_, marker)| marker)
            }),
            None => RenderedLanes::empty(range.len()),
        }
    }

    pub(crate) fn visual_count(&self, index: usize) -> Option<usize> {
        self.graph
            .as_ref()
            .and_then(|graph| graph.visual_counts.get(index))
            .copied()
    }

    pub(crate) fn can_select_entry(&self) -> bool {
        self.state == State::Complete
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.changes_focus.is_none()
            && self.reachable_rows.is_none()
            && self.selected.and_then(|index| self.visual_count(index)).is_some()
    }

    pub(crate) fn entry_selection_active(&self) -> bool {
        self.entry_selection.is_some()
    }

    fn entry_number_target(&self, number: usize) -> Option<usize> {
        let selected = self.selected?;
        let base = selected.checked_add(self.visual_count(selected)?)?;
        self.rows.get(base)?;
        let target = base.checked_sub(number)?;
        (self.visual_count(target) == Some(number)).then_some(target)
    }

    pub(crate) fn attributions(&self, row: &CommitRow) -> &[Attribution] {
        debug_assert!(row.metadata_loaded, "visible rows have metadata");
        &self.attributions[row.attributions.clone()]
    }

    pub fn update(&mut self, action: Action) -> Vec<Effect> {
        self.notice = None;
        if !matches!(&action, Action::Undo | Action::Redo) {
            self.undo_position = None;
        }
        if !matches!(
            &action,
            Action::ToggleHistoryDisplay
                | Action::ToggleDate
                | Action::CycleIds
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
            Action::ToggleActions
                | Action::Reword
                | Action::NewCommit
                | Action::NewEmptyCommit
                | Action::Amend
                | Action::Spill
                | Action::Split
                | Action::Forget
                | Action::TimeTravel
                | Action::TogglePin
                | Action::Rebase
                | Action::RebaseUpdate
                | Action::Squash
                | Action::CopyInsert
                | Action::MoveInsert
                | Action::StackInsert
                | Action::Stash
                | Action::Review
                | Action::ForkCommit
                | Action::Attach
        ) {
            self.actions_expanded = false;
        }
        if !matches!(
            &action,
            Action::ToggleEnrich
                | Action::ToggleTodo
                | Action::ToggleChecksPass
                | Action::EditNote
                | Action::EditGitNote
        ) {
            self.enrich_expanded = false;
        }
        if !matches!(
            &action,
            Action::ToggleInformation
                | Action::VerifySignatures
                | Action::ToggleAlign
                | Action::ToggleCommit
                | Action::ToggleChanges
                | Action::ToggleRefTree
        ) {
            self.information_expanded = false;
        }
        match action {
            Action::Cancelled if self.state == State::Cancelling => self.state = State::Cancelled,
            Action::Undo if self.undo_redo_allowed() => return vec![Effect::Undo],
            Action::Redo if self.undo_redo_allowed() => return vec![Effect::Redo],
            Action::MoveUp if self.changes_focus.is_some() => self.move_changes(1, false),
            Action::MoveDown if self.changes_focus.is_some() => self.move_changes(1, true),
            Action::MoveUpBy(distance) if self.changes_focus.is_some() => self.move_changes(distance, false),
            Action::MoveDownBy(distance) if self.changes_focus.is_some() => self.move_changes(distance, true),
            Action::MoveUp => self.move_selection(1, false),
            Action::MoveDown => self.move_selection(1, true),
            Action::MoveUpBy(distance) => self.move_selection(distance, false),
            Action::MoveDownBy(distance) => self.move_selection(distance, true),
            Action::TopologicalUp => {
                self.pending_initial_selection = None;
                self.move_topologically(TopologicalDirection::Child);
            }
            Action::TopologicalDown => {
                self.pending_initial_selection = None;
                self.move_topologically(TopologicalDirection::Parent);
            }
            Action::PanUpBy(distance) => self.pan_history(distance, false),
            Action::PanDownBy(distance) => self.pan_history(distance, true),
            Action::PreviousChild => self.adjust_topological_choice(false),
            Action::NextChild => self.adjust_topological_choice(true),
            Action::SubmitTopological => self.submit_topological_choice(),
            Action::CancelTopological => self.topological_navigation = None,
            Action::CycleDuplicate => {
                if self.changes_focus.is_none()
                    && self.reachable_rows.is_none()
                    && let Some(selected) = self.next_duplicate()
                {
                    self.select(selected);
                }
            }
            Action::ScrollLeft => self.pan_horizontal(false),
            Action::ScrollRight => self.pan_horizontal(true),
            Action::HalfPageUp if self.changes_focus.is_some() => {
                self.move_changes((self.focused_changes().visible_paths() / 2).max(1), false);
            }
            Action::HalfPageDown if self.changes_focus.is_some() => {
                self.move_changes((self.focused_changes().visible_paths() / 2).max(1), true);
            }
            Action::PageUp if self.changes_focus.is_some() => {
                self.move_changes(self.focused_changes().visible_paths().max(1), false);
            }
            Action::PageDown if self.changes_focus.is_some() => {
                self.move_changes(self.focused_changes().visible_paths().max(1), true);
            }
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
                if let Some(index) = (0..self.history_len()).find(|index| self.history_entry_selectable(*index)) {
                    self.select_history_index(index);
                }
            }
            Action::Last if self.changes_focus.is_some() => {
                let changes = self.focused_changes_mut();
                changes.selected = changes.max;
                changes.error = None;
                self.ensure_changes_visible();
            }
            Action::Last => {
                if let Some(index) = (0..self.history_len())
                    .rev()
                    .find(|index| self.history_entry_selectable(*index))
                {
                    self.pending_initial_selection = None;
                    self.select_history_index(index);
                    self.follow_tail = self.state == State::Loading;
                    self.ensure_visible();
                }
            }
            Action::ToggleDate => {
                self.date_mode = match self.date_mode {
                    DateMode::Author => DateMode::Committer,
                    DateMode::Committer => DateMode::None,
                    DateMode::None => DateMode::Author,
                };
            }
            Action::CycleIds => {
                self.id_mode = match self.id_mode {
                    IdMode::Commit => IdMode::Change,
                    IdMode::Change => IdMode::Off,
                    IdMode::Off => IdMode::Commit,
                };
            }
            Action::ToggleEmail => self.show_emails = !self.show_emails,
            Action::ToggleName => {
                let visible = self.visible_history_indices(self.offset..self.offset.saturating_add(self.viewport_rows));
                let has_visible_attributions = visible
                    .into_iter()
                    .filter_map(|index| self.rows.get(index))
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
            Action::ToggleActions if !self.selected_is_segment() => self.actions_expanded = !self.actions_expanded,
            Action::ToggleEnrich if !self.selected_is_segment() => self.enrich_expanded = !self.enrich_expanded,
            Action::ToggleInformation => self.information_expanded = !self.information_expanded,
            Action::CycleRefs => {
                self.ref_mode = match self.ref_mode {
                    RefMode::All => RefMode::Default,
                    RefMode::Default => RefMode::None,
                    RefMode::None => RefMode::All,
                };
            }
            Action::SelectEntry if self.can_select_entry() => self.entry_selection = Some(String::new()),
            Action::SelectEntryInput(input) if self.entry_selection.is_some() => {
                let input = input.trim();
                let digits = input.strip_prefix('#').unwrap_or(input);
                if !digits.is_empty() {
                    if digits.bytes().all(|byte| byte.is_ascii_digit()) {
                        self.entry_selection
                            .as_mut()
                            .expect("entry selection was checked above")
                            .push_str(digits);
                    } else {
                        self.leave_attention("entry number must contain only digits");
                    }
                }
            }
            Action::SelectEntryBackspace if self.entry_selection.is_some() => {
                self.entry_selection
                    .as_mut()
                    .expect("entry selection was checked above")
                    .pop();
            }
            Action::SubmitEntrySelection if self.entry_selection.is_some() => {
                let input = self
                    .entry_selection
                    .as_deref()
                    .expect("entry selection was checked above");
                if input.is_empty() {
                    self.leave_attention("enter an entry number");
                } else if let Ok(number) = input.parse::<usize>() {
                    if let Some(target) = self.entry_number_target(number) {
                        self.entry_selection = None;
                        self.select(target);
                    } else {
                        self.leave_attention(format!("entry #{number} is not in the current tree"));
                    }
                } else {
                    self.leave_attention("entry number is too large");
                }
            }
            Action::ToggleRefs => match self.ref_mode {
                RefMode::None => self.ref_mode = self.visible_ref_mode,
                visible => {
                    self.visible_ref_mode = visible;
                    self.ref_mode = RefMode::None;
                }
            },
            Action::Refresh if matches!(self.state, State::Complete | State::Cancelled) => {
                self.materialize_compressed_selection();
                self.compressed_expanded.clear();
                self.compressed_anchor = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id);
                if self.alignment == Alignment::Compressed {
                    self.rebuild_compressed_history();
                }
                return vec![Effect::Reload(self.show_hidden)];
            }
            Action::ToggleHidden
                if self.has_hidden_filter && matches!(self.state, State::Complete | State::Cancelled) =>
            {
                self.materialize_compressed_selection();
                self.compressed_expanded.clear();
                self.compressed_anchor = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id);
                if self.alignment == Alignment::Compressed {
                    self.rebuild_compressed_history();
                }
                return vec![Effect::Reload(!self.show_hidden)];
            }
            Action::ToggleAlign => {
                let viewport_row = self
                    .selected_history_index()
                    .map(|selected| selected.saturating_sub(self.offset));
                let leaving_compressed = self.alignment == Alignment::Compressed;
                if leaving_compressed {
                    self.materialize_compressed_selection();
                }
                self.alignment = match self.alignment {
                    Alignment::Title => Alignment::Columns,
                    Alignment::Columns => Alignment::None,
                    Alignment::None => Alignment::Compressed,
                    Alignment::Compressed => Alignment::Title,
                };
                if self.alignment == Alignment::Compressed {
                    self.compressed_expanded.clear();
                    self.compressed_segment = None;
                    self.compressed_anchor = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id);
                    self.rebuild_compressed_history();
                } else if leaving_compressed {
                    self.compressed_history = None;
                    self.compressed_anchor = None;
                    self.compressed_expanded.clear();
                }
                if let (Some(selected), Some(viewport_row)) = (self.selected_history_index(), viewport_row) {
                    self.offset = selected.saturating_sub(viewport_row);
                }
                self.ensure_visible();
            }
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
            Action::OpenDiff if self.expand_selected_segment() => {}
            Action::OpenDiff if self.review_return.is_some() => {
                let (review, _) = self.review_return.expect("review return selection is active");
                let Some(return_to) = self
                    .selected
                    .filter(|index| self.is_row_reachable(*index) && self.reachable_row_selectable(*index))
                    .and_then(|index| self.rows.get(index))
                    .map(|row| row.id)
                else {
                    return Vec::new();
                };
                self.clear_reachability_selection();
                return vec![Effect::FinishReview {
                    review,
                    return_to: Some(return_to),
                }];
            }
            Action::OpenDiff if self.review_tip.is_some() => {
                let tip = self.review_tip.expect("review selection has a tip");
                let Some(base) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) else {
                    return Vec::new();
                };
                if base != tip && self.selected.is_some_and(|index| self.is_row_reachable(index)) {
                    self.clear_reachability_selection();
                    return vec![Effect::StartReview { tip, base }];
                }
            }
            Action::OpenDiff if self.squash_source.is_some() => {
                let source = self.squash_source.expect("squash selection has a source");
                let Some(target) = self
                    .selected
                    .filter(|index| self.is_row_reachable(*index) && self.reachable_row_selectable(*index))
                    .and_then(|index| self.rows.get(index))
                    .map(|row| row.id)
                else {
                    return Vec::new();
                };
                self.clear_reachability_selection();
                return vec![Effect::Squash { source, target }];
            }
            Action::OpenDiff if self.stack_insert_base.is_some() => {
                let source = self.worktree_head.expect("stack insertion requires HEAD");
                let base = self.stack_insert_base.expect("stack insertion has a base");
                let Some(target) = self
                    .selected
                    .filter(|index| self.is_row_reachable(*index) && self.reachable_row_selectable(*index))
                    .and_then(|index| self.rows.get(index))
                    .map(|row| row.id)
                else {
                    return Vec::new();
                };
                self.clear_reachability_selection();
                return vec![Effect::Insert {
                    source,
                    base,
                    target,
                    copy: false,
                }];
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
                return vec![Effect::Forget(id)];
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
            Action::Squash if self.can_squash() => {
                let source = self.rows[self.selected.expect("squash requires a selection")].id;
                self.squash_source = Some(source);
                self.reachability_anchor = Some(source);
                self.compute_reachable_rows();
                let mut targets = self
                    .rows
                    .iter()
                    .enumerate()
                    .filter(|(index, row)| {
                        row.id != source && self.is_row_reachable(*index) && self.reachable_row_selectable(*index)
                    })
                    .map(|(_, row)| row.id);
                if let Some(target) = targets.next().filter(|_| targets.next().is_none()) {
                    self.clear_reachability_selection();
                    return vec![Effect::Squash { source, target }];
                }
            }
            Action::CopyInsert if self.can_copy_insert() => {
                let source = self.worktree_head.expect("copy-insert availability requires HEAD");
                return vec![Effect::Insert {
                    source,
                    base: source,
                    target: self.rows[self.selected.expect("copy-insert requires a selection")].id,
                    copy: true,
                }];
            }
            Action::PasteInsert { source, target } => {
                return vec![Effect::PasteInsert { source, target }];
            }
            Action::MoveInsert if self.can_move_insert() => {
                let source = self.worktree_head.expect("move-insert availability requires HEAD");
                return vec![Effect::Insert {
                    source,
                    base: source,
                    target: self.rows[self.selected.expect("move-insert requires a selection")].id,
                    copy: false,
                }];
            }
            Action::StackInsert => {
                if let Some((source, base, base_parent, stack)) = self.stack_insert() {
                    let reachable: Vec<_> = self
                        .rows
                        .iter()
                        .enumerate()
                        .map(|(index, row)| self.is_stack_insert_target(index, row.id, base, base_parent, &stack))
                        .collect();
                    if reachable.iter().any(|reachable| *reachable) {
                        debug_assert_eq!(source, self.worktree_head.expect("a stack insertion has HEAD"));
                        self.stack_insert_base = Some(base);
                        self.reachable_rows = Some(reachable);
                        self.ensure_visible();
                    }
                }
            }
            Action::Review if self.can_finish_review() => {
                return vec![Effect::FinishReview {
                    review: self.rows[self.selected.expect("finishing review requires a selection")].id,
                    return_to: None,
                }];
            }
            Action::Review if self.can_review() => {
                let tip = self.rows[self.selected.expect("review requires a selection")].id;
                self.review_tip = Some(tip);
                self.reachability_anchor = Some(tip);
                self.compute_reachable_rows();
                let mut bases = self
                    .rows
                    .iter()
                    .enumerate()
                    .filter(|(index, row)| {
                        row.id != tip && self.is_row_reachable(*index) && self.reachable_row_selectable(*index)
                    })
                    .map(|(_, row)| row.id);
                if let Some(base) = bases.next().filter(|_| bases.next().is_none()) {
                    self.clear_reachability_selection();
                    return vec![Effect::StartReview { tip, base }];
                }
            }
            Action::ForkCommit if self.can_fork_commit() => {
                return vec![Effect::ForkCommit(
                    self.rows[self.selected.expect("fork requires a selection")].id,
                )];
            }
            Action::Attach if self.can_attach() => return vec![Effect::Attach],
            Action::TimeTravel if self.can_time_travel() => {
                return vec![Effect::TimeTravel(
                    self.rows[self.selected.expect("time-travel requires a selection")].id,
                )];
            }
            Action::TogglePin => {
                if let Some(id) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) {
                    return vec![Effect::TogglePin(id)];
                }
            }
            Action::ToggleTodo if self.can_reword() => {
                return vec![Effect::ToggleTodo(
                    self.rows[self.selected.expect("todo requires a selection")].id,
                )];
            }
            Action::ToggleChecksPass => {
                if let Some(id) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) {
                    return vec![Effect::ToggleChecksPass(id)];
                }
            }
            Action::EditNote if self.can_reword() => {
                return vec![Effect::EditNote(
                    self.rows[self.selected.expect("note requires a selection")].id,
                )];
            }
            Action::EditGitNote => {
                if let Some(id) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) {
                    return vec![Effect::EditGitNote(id)];
                }
            }
            Action::VerifySignatures if !self.signature_verification_running => {
                let visible = self.visible_history_indices(self.offset..self.offset.saturating_add(self.viewport_rows));
                let mut changed = Vec::new();
                for index in visible {
                    let row = &mut self.rows[index];
                    if !self.hidden_rows.contains(&row.id) && row.signature == SignatureState::Unverified {
                        Arc::make_mut(row).signature = SignatureState::Verifying;
                        changed.push((row.id, Arc::clone(row)));
                    }
                }
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
            Action::Cancel if self.entry_selection.is_some() => self.entry_selection = None,
            Action::Cancel
                if self.review_tip.is_some()
                    || self.review_return.is_some()
                    || self.squash_source.is_some()
                    || self.stack_insert_base.is_some() =>
            {
                self.clear_reachability_selection();
            }
            Action::Cancel | Action::Quit if self.changes_focus.is_some() => self.focus_history(),
            Action::Cancel if self.state == State::Loading => {
                self.state = State::Cancelling;
                return vec![Effect::Cancel];
            }
            Action::Copy => {
                if let Some(id) = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) {
                    let effect = match self.effective_id_mode() {
                        IdMode::Change => Effect::CopyChangeId(self.change_id(id)),
                        IdMode::Commit | IdMode::Off => Effect::CopyId(id),
                    };
                    self.copy_feedback = Some(CopyKind::Id);
                    return vec![effect];
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

    fn undo_redo_allowed(&self) -> bool {
        self.pending_rebase_conflict.is_none()
            && !self.rebase_continuation_pending
            && !self.worktree_conflicted
            && self.reachable_rows.is_none()
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

    pub(crate) fn hidden_branch_update(&self, id: ObjectId) -> Option<ObjectId> {
        self.hidden_branch_updates.get(&id).map(|(_, target)| *target)
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
                Some(states.get(&row.id).copied().unwrap_or(State {
                    leaf: None,
                    has_merge: false,
                }))
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
        self.topological_navigation = None;
        self.set_view_tips(view_tips);
        if self.stack_insert_base.is_some() {
            self.clear_reachability_selection();
        }
        let previous_order: HashMap<_, _> = self
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| (row.id, index))
            .collect();
        drop(self.store_commits(commits));

        let (visible, boundary) = crate::history::view_scope(view_tips, hidden_tips, |id, out| {
            if let Some(row) = self.all_rows.get(&id) {
                out.extend(row.parent_ids.iter().copied());
            }
        });
        let rows: Vec<_> = self
            .all_order
            .iter()
            .filter(|id| visible.contains(*id) || boundary.contains(*id))
            .filter_map(|id| self.all_rows.get(id).map(Arc::clone))
            .collect();
        let positions: HashMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
        let mut children = vec![Vec::new(); rows.len()];
        let mut ranks = vec![None; rows.len()];
        let mut pending = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            if let Some(rank) = previous_order.get(&row.id) {
                ranks[index] = Some(*rank);
                pending.push(index);
                continue;
            }
            if let Some(parent) = row.parent_ids.iter().next().and_then(|parent| positions.get(parent)) {
                children[*parent].push(index);
            } else {
                pending.push(index);
            }
        }
        while let Some(parent) = pending.pop() {
            for child in &children[parent] {
                // First-parent ancestry identifies the old lane.
                ranks[*child] = ranks[parent];
                pending.push(*child);
            }
        }
        let mut rows: Vec<_> = rows.into_iter().zip(ranks).collect();
        rows.sort_by_key(|(_, rank)| (rank.is_none(), *rank));
        let rows = rows.into_iter().map(|(row, _)| row).collect();
        self.pending_hidden_rows = Some(boundary);
        self.select_top_after_refresh = select_top;
        self.state = State::Computing;
        self.follow_tail = false;
        Some(rows)
    }

    fn is_known_ancestor(&self, ancestor: ObjectId, descendant: ObjectId) -> bool {
        let mut seen = HashSet::new();
        let mut pending = vec![descendant];
        while let Some(id) = pending.pop() {
            if id == ancestor {
                return true;
            }
            if seen.insert(id)
                && let Some(row) = self.all_rows.get(&id)
            {
                pending.extend(row.parent_ids.iter().copied());
            }
        }
        false
    }

    pub(crate) fn finish_lane_computation(&mut self, rows: Vec<SharedCommitRow>, graph: Graph, lane_time: Duration) {
        if self.state != State::Computing {
            return;
        }
        let select_top = std::mem::take(&mut self.select_top_after_refresh);
        let previous_selection = if select_top { None } else { self.selected };
        let previous_visual_position = previous_selection.and_then(|index| {
            let count = self.visual_count(index)?;
            let base = self.rows.get(index.checked_add(count)?)?.id;
            Some((base, count, index))
        });
        let segment = (!select_top && self.selection_after_refresh.is_none())
            .then_some(self.compressed_segment)
            .flatten();
        let selected = if select_top {
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
                            author_time: row.author_time,
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
        self.topological_navigation = None;
        self.rows = rows;
        if let Some(hidden) = self.pending_hidden_rows.take() {
            self.hidden_rows = hidden;
        }
        self.update_hidden_branch_targets();
        for row in &mut self.rows {
            if let Some(metadata) = metadata.get(&row.id) {
                let row = Arc::make_mut(row);
                row.committer_time = metadata.committer_time;
                row.author_time = metadata.author_time;
                row.author = metadata.author;
                row.attributions = metadata.attributions.clone();
                row.title = metadata.title.clone();
                row.metadata_loaded = true;
                row.has_agent_marker = metadata.has_agent_marker;
                row.is_review = metadata.is_review;
                row.signature = metadata.signature;
            }
        }
        self.graph = Some(graph);
        self.lane_time = Some(lane_time);
        self.update_worktree_head_descendants();
        let visual_counts = &self
            .graph
            .as_ref()
            .expect("the completed graph was just stored")
            .visual_counts;
        let visual_selection = previous_visual_position.and_then(|(base, count, previous)| {
            self.rows
                .iter()
                .enumerate()
                .filter(|(index, _)| {
                    visual_counts.get(*index) == Some(&count)
                        && index
                            .checked_add(count)
                            .and_then(|base_index| self.rows.get(base_index))
                            .is_some_and(|row| row.id == base)
                })
                .min_by_key(|(index, _)| index.abs_diff(previous))
                .map(|(index, _)| index)
        });
        self.selected = selected
            .and_then(|id| self.rows.iter().position(|row| row.id == id))
            .or(visual_selection)
            .or_else(|| {
                previous_selection
                    .filter(|_| !self.rows.is_empty())
                    .map(|index| index.min(self.rows.len() - 1))
            })
            .or_else(|| (!self.rows.is_empty()).then_some(0));
        self.compressed_segment = segment;
        if self.compressed_segment.is_some() {
            self.selected = None;
        }
        if self.alignment == Alignment::Compressed {
            if self.compressed_segment.is_none() {
                self.compressed_anchor = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id);
            }
            self.rebuild_compressed_history();
        }
        self.state = State::Complete;
        if self.reachability_anchor.is_some() {
            self.compute_reachable_rows();
        }
        if self
            .pending_rebase_conflict
            .is_some_and(|id| self.selected.is_some_and(|index| self.rows[index].id == id))
        {
            self.center_selection();
        } else {
            self.prepare_history_viewport();
        }
    }

    #[cfg(test)]
    pub(crate) fn reload(&mut self, show_hidden: bool) {
        self.materialize_compressed_selection();
        self.reload_selection = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id);
        self.select_top_after_refresh = false;
        self.rows = Vec::new();
        self.all_rows.clear();
        self.all_order.clear();
        self.hidden_rows.clear();
        self.hidden_branch_targets.clear();
        self.hidden_rebase_bases.clear();
        self.hidden_branch_updates.clear();
        self.clear_change_ids();
        self.pending_hidden_rows = None;
        self.titles = Vec::new();
        self.notes.clear();
        self.clear_enrichments();
        self.graph = None;
        self.entry_selection = None;
        self.topological_navigation = None;
        self.view_tips.clear();
        self.compressed_history = None;
        self.compressed_anchor = None;
        self.compressed_segment = None;
        self.compressed_expanded.clear();
        self.attributions = Vec::new();
        #[cfg(test)]
        self.test_lanes.clear();
        self.selected = None;
        self.offset = 0;
        self.viewport_panned = false;
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
        self.clear_reachability_selection();
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

    fn clear_reachability_selection(&mut self) {
        self.review_tip = None;
        self.review_return = None;
        self.squash_source = None;
        self.stack_insert_base = None;
        self.reachability_anchor = None;
        self.reachable_rows = None;
        self.restore_compressed_history_around_selection();
    }

    pub(crate) fn select_review_return(&mut self, review: ObjectId, tip: ObjectId) -> bool {
        let reachable: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| !row.is_review && !self.is_row_hidden(index) && self.is_known_ancestor(tip, row.id))
            .collect();
        let current = self.selected.unwrap_or_default();
        let selected = self
            .rows
            .iter()
            .position(|row| row.id == tip)
            .filter(|index| reachable[*index])
            .or_else(|| {
                reachable
                    .iter()
                    .enumerate()
                    .filter_map(|(index, reachable)| reachable.then_some(index))
                    .min_by_key(|index| index.abs_diff(current))
            });
        let Some(selected) = selected else { return false };
        self.review_tip = None;
        self.review_return = Some((review, tip));
        self.reachability_anchor = None;
        self.reachable_rows = Some(reachable);
        self.select(selected);
        true
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
        self.focused_changes_mut().ensure_visible();
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

    fn pan_horizontal(&mut self, right: bool) {
        if self.changes_focus.is_some() {
            self.pan_changes(right);
        } else if right {
            self.horizontal_offset = self
                .horizontal_offset
                .saturating_add(self.horizontal_page)
                .min(self.horizontal_max);
        } else {
            self.horizontal_offset = self.horizontal_offset.saturating_sub(self.horizontal_page);
        }
    }

    fn pan_history(&mut self, distance: usize, down: bool) {
        self.pending_initial_selection = None;
        self.follow_tail = false;
        self.viewport_panned = true;
        let max = self.history_len().saturating_sub(self.viewport_rows.max(1));
        self.offset = if down {
            self.offset.saturating_add(distance.max(1)).min(max)
        } else {
            self.offset.saturating_sub(distance.max(1))
        };
    }

    fn move_selection(&mut self, distance: usize, down: bool) {
        self.pending_initial_selection = None;
        let Some(display_selected) = self.selected_history_index() else {
            return;
        };
        let distance = distance.max(1);
        let target = if down {
            display_selected
                .saturating_add(distance)
                .min(self.history_len().saturating_sub(1))
        } else {
            display_selected.saturating_sub(distance)
        };
        if self.reachable_rows.is_some() {
            let next = if down {
                (display_selected + 1..self.history_len())
                    .filter(|index| self.history_entry_selectable(*index))
                    .min_by_key(|index| index.abs_diff(target))
            } else {
                (0..display_selected)
                    .filter(|index| self.history_entry_selectable(*index))
                    .min_by_key(|index| index.abs_diff(target))
            };
            if let Some(next) = next {
                self.select_history_index(next);
            }
            return;
        }
        self.select_history_index(target);
    }

    fn next_duplicate(&self) -> Option<usize> {
        let selected = self.selected?;
        let id = self.rows.get(selected)?.id;
        if !self.has_duplicate_change_id(id) {
            return None;
        }
        let change_id = self.change_id(id);
        ((selected + 1)..self.rows.len())
            .chain(0..selected)
            .find(|index| self.change_id(self.rows[*index].id) == change_id)
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
        if self.alignment == Alignment::Compressed {
            self.ensure_visible();
        }
    }

    pub(crate) fn is_row_reachable(&self, index: usize) -> bool {
        self.reachable_rows
            .as_ref()
            .is_none_or(|reachable| reachable.get(index).copied().unwrap_or(false))
    }

    fn reachable_row_selectable(&self, index: usize) -> bool {
        if self.squash_source.is_some() {
            return self
                .rows
                .get(index)
                .is_some_and(|row| Some(row.id) != self.squash_source)
                && self.is_squash_target(index);
        }
        !self.is_row_hidden(index)
            || (self.review_tip.is_some()
                && self
                    .rows
                    .get(index)
                    .is_some_and(|row| self.hidden_rebase_bases.contains(&row.id)))
    }

    fn is_squash_target(&self, index: usize) -> bool {
        self.rows.get(index).is_some_and(|row| {
            !self.is_row_hidden(index) && row.parent_ids.len() == 1 && !self.known_merge_descendants.contains(&row.id)
        })
    }

    fn select(&mut self, selected: usize) {
        self.topological_navigation = None;
        self.pending_initial_selection = None;
        self.compressed_segment = None;
        if !self.rows.is_empty() {
            let selected = selected.min(self.rows.len() - 1);
            if self.alignment == Alignment::Compressed
                && !self.compressed_history_suspended()
                && self.history_index(selected).is_none()
            {
                self.compressed_anchor = Some(self.rows[selected].id);
                self.rebuild_compressed_history();
            }
            let previous = self.selected;
            self.selected = Some(selected);
            if self.selected != previous {
                self.retry_failed_signatures();
            }
            self.follow_tail = false;
            self.ensure_visible();
        }
    }

    fn first_selectable(&self) -> Option<usize> {
        (0..self.history_len()).find_map(|index| match self.history_entry(index) {
            Some(HistoryEntry::Commit(index)) => Some(index),
            Some(HistoryEntry::Segment { .. }) | None => None,
        })
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
            && !self.selected_is_segment()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && match self.selected.and_then(|index| self.rows.get(index)) {
                Some(row) => {
                    (self.worktree_head_unborn
                        || !self.hidden_rows.contains(&row.id)
                        || self.worktree_head == Some(row.id))
                        && !self.known_merge_descendants.contains(&row.id)
                }
                None => self.worktree_head_unborn,
            }
    }

    pub(crate) fn set_new_commit_availability(&mut self, changes: Option<&Changes>) {
        let selected_is_head =
            self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id) == self.worktree_head;
        let selected_is_head = selected_is_head
            || self.worktree_head_unborn && self.selected.is_none_or(|index| self.is_row_hidden(index));
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
            && !self.worktree_head_unborn
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

    pub(crate) fn can_squash(&self) -> bool {
        self.state == State::Complete
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self
                .selected
                .and_then(|index| self.rows.get(index))
                .is_some_and(|source| {
                    !self.hidden_rows.contains(&source.id)
                        && source.parent_ids.len() == 1
                        && !self.known_merge_descendants.contains(&source.id)
                        && self.rows.iter().enumerate().any(|(index, target)| {
                            target.id != source.id
                                && self.is_squash_target(index)
                                && self.is_known_ancestor(target.id, source.id)
                        })
                })
    }

    pub(crate) fn can_copy_insert(&self) -> bool {
        self.can_insert(true)
    }

    pub(crate) fn paste_insert_target(&self) -> Option<ObjectId> {
        if self.state != State::Complete
            || self.deferred_history_state.unwrap_or(self.state) != State::Complete
            || self.changes_focus.is_some()
            || self.has_conflict_marker()
        {
            return None;
        }
        self.selected
            .filter(|index| !self.is_row_hidden(*index))
            .and_then(|index| self.rows.get(index))
            .filter(|row| !self.known_merge_descendants.contains(&row.id))
            .map(|row| row.id)
    }

    pub(crate) fn can_move_insert(&self) -> bool {
        self.can_insert(false)
    }

    fn can_insert(&self, copy: bool) -> bool {
        if self.state != State::Complete
            || self.changes_focus.is_some()
            || self.deferred_history_state.unwrap_or(self.state) != State::Complete
        {
            return false;
        }
        let Some(source) = self
            .worktree_head
            .and_then(|head| self.rows.iter().find(|row| row.id == head))
        else {
            return false;
        };
        let Some((target_index, target)) = self
            .selected
            .and_then(|index| self.rows.get(index).map(|row| (index, row)))
        else {
            return false;
        };
        source.parent_ids.len() == 1
            && source.id != target.id
            && (copy || source.parent_ids.first().copied() != Some(target.id))
            && !self.is_row_hidden(target_index)
            && (copy || !self.known_merge_descendants.contains(&source.id))
            && !self.known_merge_descendants.contains(&target.id)
    }

    pub(crate) fn can_stack_insert(&self) -> bool {
        self.stack_insert().is_some_and(|(_, base, base_parent, stack)| {
            self.rows
                .iter()
                .enumerate()
                .any(|(index, row)| self.is_stack_insert_target(index, row.id, base, base_parent, &stack))
        })
    }

    fn stack_insert(&self) -> Option<(ObjectId, ObjectId, ObjectId, HashSet<ObjectId>)> {
        if self.state != State::Complete
            || self.changes_focus.is_some()
            || self.deferred_history_state.unwrap_or(self.state) != State::Complete
        {
            return None;
        }
        let source = self.worktree_head?;
        let base = self.selected.and_then(|index| self.rows.get(index))?.id;
        if self.hidden_rows.contains(&base) || self.known_merge_descendants.contains(&base) {
            return None;
        }

        let mut current = source;
        let mut stack = HashSet::new();
        loop {
            if !stack.insert(current) {
                return None;
            }
            let row = self.all_rows.get(&current)?;
            let [parent] = row.parent_ids.as_slice() else {
                return None;
            };
            if current == base {
                return Some((source, base, *parent, stack));
            }
            current = *parent;
        }
    }

    fn is_stack_insert_target(
        &self,
        index: usize,
        id: ObjectId,
        base: ObjectId,
        base_parent: ObjectId,
        stack: &HashSet<ObjectId>,
    ) -> bool {
        !self.is_row_hidden(index)
            && !stack.contains(&id)
            && id != base_parent
            && !self.known_merge_descendants.contains(&id)
            && !stack
                .iter()
                .any(|ancestor| *ancestor != base && self.is_known_ancestor(*ancestor, id))
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
            && self.selected.and_then(|index| self.rows.get(index)).is_some_and(|row| {
                row.is_review
                    && self
                        .worktree_head
                        .is_some_and(|head| self.is_known_ancestor(row.id, head))
            })
    }

    pub(crate) fn can_fork_commit(&self) -> bool {
        self.state == State::Complete
            && self.worktree_changes_available
            && !self.worktree_conflicted
            && self.pending_rebase_conflict.is_none()
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.worktree_head.is_some()
            && self.selected.and_then(|index| self.rows.get(index)).is_some()
    }

    pub(crate) fn can_attach(&self) -> bool {
        self.state == State::Complete
            && self.worktree_changes_available
            && !self.worktree_conflicted
            && self.pending_rebase_conflict.is_none()
            && self.changes_focus.is_none()
            && self.deferred_history_state.unwrap_or(self.state) == State::Complete
            && self.worktree_head.is_some()
            && self.worktree_branch.is_some_and(|(_, detached)| detached)
    }

    pub(crate) fn review_selection_active(&self) -> bool {
        self.review_tip.is_some()
    }

    pub(crate) fn squash_selection_active(&self) -> bool {
        self.squash_source.is_some()
    }

    pub(crate) fn review_return_selection_active(&self) -> bool {
        self.review_return.is_some()
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

    pub(crate) fn select_commit(&mut self, id: ObjectId) {
        if let Some(index) = self.rows.iter().position(|row| row.id == id) {
            self.select(index);
        }
    }

    pub(crate) fn select_commit_for_time_travel(&mut self, id: ObjectId) {
        let Some(index) = self.rows.iter().position(|row| row.id == id) else {
            return;
        };
        let previous_offset = self.offset;
        self.select(index);
        if let Some(selected) = self
            .selected_history_index()
            .filter(|selected| *selected < previous_offset)
        {
            self.offset = selected.saturating_sub(self.viewport_rows.max(1) - 1);
        }
    }

    pub(crate) fn begin_time_travel_animation(&mut self) {
        let viewport_row = self
            .selected_history_index()
            .map(|selected| selected.saturating_sub(self.offset));
        self.time_travel_animation = self
            .selected
            .and_then(|index| self.rows.get(index))
            .map(|row| (row.id, viewport_row.unwrap_or_default()));
        if let (Some(selected), Some(viewport_row)) = (self.selected_history_index(), viewport_row) {
            self.offset = selected.saturating_sub(viewport_row);
        }
        self.ensure_visible();
    }

    pub(crate) fn finish_time_travel_animation(&mut self) {
        let Some((origin, viewport_row)) = self.time_travel_animation else {
            return;
        };
        self.select_commit(origin);
        self.time_travel_animation = None;
        if self.alignment == Alignment::Compressed {
            self.compressed_anchor = self.selected.and_then(|index| self.rows.get(index)).map(|row| row.id);
            self.rebuild_compressed_history();
        }
        if let Some(selected) = self.selected_history_index() {
            self.offset = selected.saturating_sub(viewport_row);
        }
        self.ensure_visible();
    }

    pub(crate) fn time_travel_animation_origin(&self) -> Option<ObjectId> {
        self.time_travel_animation.map(|(origin, _)| origin)
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

    fn can_time_travel(&self) -> bool {
        self.state == State::Complete && self.time_travel_shortcut_visible()
    }

    fn last_selectable(&self) -> Option<usize> {
        (0..self.history_len())
            .rev()
            .find_map(|index| match self.history_entry(index) {
                Some(HistoryEntry::Commit(index)) => Some(index),
                Some(HistoryEntry::Segment { .. }) | None => None,
            })
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
        self.viewport_panned = false;
        let Some(selected) = self.selected_history_index() else {
            return;
        };
        let height = self.viewport_rows.max(1);
        if selected < self.offset {
            self.offset = selected;
        } else if selected >= self.offset.saturating_add(height) {
            self.offset = selected + 1 - height;
        }
    }

    pub(crate) fn prepare_history_viewport(&mut self) {
        if self.viewport_panned {
            self.offset = self
                .offset
                .min(self.history_len().saturating_sub(self.viewport_rows.max(1)));
        } else {
            self.ensure_visible();
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
        self.compressed_segment = None;
        self.selected = Some(selected);
        if self.alignment == Alignment::Compressed
            && !self.compressed_history_suspended()
            && self.history_index(selected).is_none()
        {
            self.compressed_anchor = Some(self.rows[selected].id);
            self.rebuild_compressed_history();
        }
        self.center_selection();
        if matches!(self.state, State::Complete | State::Cancelled) {
            self.pending_initial_selection = None;
        }
    }

    fn center_selection(&mut self) {
        let Some(selected) = self.selected_history_index() else {
            return;
        };
        let height = self.viewport_rows.max(1);
        self.offset = selected
            .saturating_sub(height / 2)
            .min(self.history_len().saturating_sub(height));
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

    pub(crate) fn commit_paging_active(&self) -> bool {
        self.show_commit && self.commit_max > 0
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
        separator: Option<usize>,
        horizontal_page: usize,
        horizontal_max: usize,
    ) {
        let changes = self.changes_mut(pane);
        changes.page = page.max(1);
        changes.len = len;
        changes.max = len.saturating_sub(1);
        changes.separator = separator.filter(|separator| *separator > 0 && *separator < len);
        if len == 0 {
            changes.selected = 0;
            changes.offset = 0;
        } else {
            changes.selected = changes.selected.min(changes.max);
            changes.ensure_visible();
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

impl CompressedHistory {
    fn new(
        rows: &[SharedCommitRow],
        view_tips: &HashSet<ObjectId>,
        hidden_rows: &HashSet<ObjectId>,
        expanded: &HashSet<ObjectId>,
        selected: Option<ObjectId>,
    ) -> Self {
        let positions: HashMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
        let mut children = vec![(0usize, 0usize); rows.len()];
        for (child, row) in rows.iter().enumerate() {
            for parent in &row.parent_ids {
                if let Some(parent) = positions.get(parent) {
                    children[*parent] = (children[*parent].0 + 1, child);
                }
            }
        }
        let is_anchor = |index: usize| {
            let row = &rows[index];
            view_tips.contains(&row.id)
                || hidden_rows.contains(&row.id)
                || expanded.contains(&row.id)
                || selected == Some(row.id)
                || row.parent_ids.len() != 1
                || children[index].0 != 1
        };
        let mut representatives = Vec::new();
        let mut members = Vec::<Vec<usize>>::new();
        let mut group_of = vec![usize::MAX; rows.len()];
        for (index, row) in rows.iter().enumerate() {
            let joined = (!is_anchor(index))
                .then_some(children[index])
                .and_then(|(count, child)| match count {
                    1 if !is_anchor(child)
                        && rows[child].parent_ids.len() == 1
                        && rows[child].parent_ids[0] == row.id
                        && group_of[child] != usize::MAX =>
                    {
                        Some(group_of[child])
                    }
                    _ => None,
                });
            let group = joined.unwrap_or_else(|| {
                representatives.push(index);
                members.push(Vec::new());
                representatives.len() - 1
            });
            group_of[index] = group;
            members[group].push(index);
        }

        let representative_ids: Vec<_> = representatives.iter().map(|index| rows[*index].id).collect();
        let mut parents: Vec<ParentIds> = representatives.iter().map(|_| ParentIds::new()).collect();
        for (member, row) in rows.iter().enumerate() {
            let group = group_of[member];
            for parent in &row.parent_ids {
                let Some(parent_index) = positions.get(parent) else {
                    continue;
                };
                let parent_group = group_of[*parent_index];
                if parent_group != group {
                    parents[group].push(representative_ids[parent_group]);
                }
            }
        }

        let mut compact_rows = Vec::with_capacity(representatives.len());
        let mut entries = Vec::with_capacity(representatives.len());
        for (group, representative) in representatives.into_iter().enumerate() {
            let mut row = rows[representative].as_ref().clone();
            row.parent_ids = std::mem::take(&mut parents[group]);
            compact_rows.push(Arc::new(row));
            entries.push(if is_anchor(representative) || members[group].len() == 1 {
                HistoryEntry::Commit(representative)
            } else {
                HistoryEntry::Segment {
                    representative,
                    count: members[group].len(),
                }
            });
        }
        let graph = Graph::new(&compact_rows);
        let display_indices = entries
            .iter()
            .enumerate()
            .filter_map(|(display, entry)| match entry {
                HistoryEntry::Commit(canonical) => Some((*canonical, display)),
                HistoryEntry::Segment { .. } => None,
            })
            .collect();
        let member_indices = members
            .iter()
            .enumerate()
            .flat_map(|(display, members)| members.iter().map(move |index| (rows[*index].id, display)))
            .collect();
        CompressedHistory {
            entries,
            display_indices,
            member_indices,
            members,
            rows: compact_rows,
            graph,
        }
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
    visual_counts: Vec<usize>,
    parent_offsets: Vec<usize>,
    parents: Vec<usize>,
    children: Vec<Vec<usize>>,
}

impl Graph {
    fn new(rows: &[SharedCommitRow]) -> Self {
        let positions: HashMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
        let mut parent_offsets = Vec::with_capacity(rows.len() + 1);
        let mut parents = Vec::new();
        let mut children = vec![Vec::new(); rows.len()];
        for (child, row) in rows.iter().enumerate() {
            parent_offsets.push(parents.len());
            for parent in row
                .parent_ids
                .iter()
                .filter_map(|parent| positions.get(parent).copied())
            {
                parents.push(parent);
                children[parent].push(child);
            }
        }
        parent_offsets.push(parents.len());
        let mut state = LaneState::default();
        let mut graph = Graph {
            offsets: Vec::with_capacity(rows.len().div_ceil(CHECKPOINT_INTERVAL) + 1),
            columns: Vec::new(),
            visual_counts: visual_counts(rows),
            parent_offsets,
            parents,
            children,
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

    fn neighbors(&self, index: usize, direction: TopologicalDirection) -> &[usize] {
        match direction {
            TopologicalDirection::Parent => self
                .parent_offsets
                .get(index..=index.saturating_add(1))
                .map_or(&[], |offsets| &self.parents[offsets[0]..offsets[1]]),
            TopologicalDirection::Child => self.children.get(index).map_or(&[], Vec::as_slice),
        }
    }

    fn render(&self, rows: &[SharedCommitRow], range: Range<usize>) -> RenderedLanes {
        self.render_with_markers(rows, range, |_| '●')
    }

    fn render_with_markers(
        &self,
        rows: &[SharedCommitRow],
        range: Range<usize>,
        marker: impl Fn(usize) -> char,
    ) -> RenderedLanes {
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
            if let Some(range) = state.advance_ids(
                row.id,
                row.parent_ids.iter().copied(),
                (index >= start).then_some(&mut rendered.data),
                marker(index),
            ) {
                rendered.ranges.push(range);
            }
        }
        rendered
    }
}

fn visual_counts(rows: &[SharedCommitRow]) -> Vec<usize> {
    let positions: HashMap<_, _> = rows.iter().enumerate().map(|(index, row)| (row.id, index)).collect();
    let mut counts = vec![0; rows.len()];
    for (index, row) in rows.iter().enumerate().rev() {
        let base = row
            .parent_ids
            .iter()
            .filter_map(|parent| positions.get(parent).copied())
            .map(|parent| parent.saturating_add(counts[parent]))
            .min()
            .unwrap_or(index);
        counts[index] = base.saturating_sub(index);
    }
    counts
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
pub(crate) struct LaneState {
    columns: Vec<ObjectId>,
    next: Vec<ObjectId>,
    parents: Vec<(ObjectId, Option<usize>, usize)>,
    edges: Vec<(usize, usize)>,
    cells: Vec<u8>,
}

impl LaneState {
    fn advance(&mut self, row: &CommitRow, out: Option<&mut String>) -> Option<Range<usize>> {
        self.advance_ids(row.id, row.parent_ids.iter().copied(), out, '●')
    }

    pub(crate) fn advance_ids(
        &mut self,
        id: ObjectId,
        parent_ids: impl IntoIterator<Item = ObjectId>,
        out: Option<&mut String>,
        marker: char,
    ) -> Option<Range<usize>> {
        let render = out.is_some();
        let current = self
            .columns
            .iter()
            .position(|candidate| *candidate == id)
            .unwrap_or_else(|| {
                self.columns.push(id);
                self.columns.len() - 1
            });

        self.parents.clear();
        for parent in parent_ids {
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
                marker,
            )
        });
        std::mem::swap(&mut self.columns, &mut self.next);
        range
    }

    pub(crate) fn node_line(&self, id: ObjectId, marker: char) -> String {
        let current = self
            .columns
            .iter()
            .position(|candidate| *candidate == id)
            .unwrap_or(self.columns.len());
        let mut cells = vec![' '; self.columns.len().max(current + 1) * 2 - 1];
        for column in 0..self.columns.len() {
            cells[column * 2] = '│';
        }
        cells[current * 2] = marker;
        cells.push(' ');
        cells.into_iter().collect()
    }
}

fn transition(
    before: usize,
    after: usize,
    current: usize,
    edges: &[(usize, usize)],
    cells: &mut Vec<u8>,
    out: &mut String,
    marker: char,
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
            marker
        } else {
            match cell {
                0 => ' ',
                CROSS => '┼',
                VERTICAL_RIGHT => '├',
                VERTICAL_LEFT => '┤',
                DOWN_HORIZONTAL => '┬',
                UP_HORIZONTAL => '┴',
                DOWN_RIGHT => '╭',
                DOWN_LEFT => '╮',
                UP_RIGHT => '╰',
                UP_LEFT => '╯',
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
            author_time: gix::date::Time::default(),
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

    fn compressed_linear_app() -> App {
        let mut app = App::new(3);
        app.extend_commits(vec![
            row_with_parents(6, &[5]),
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.set_view_tips(&[id(6)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);
        app
    }

    #[test]
    fn undo_and_redo_retain_their_position_until_another_action() {
        let mut app = App::new(2);
        app.show_undo_position(9, 4, "reword commit");
        assert_eq!(app.undo_position(), Some((4, 4, "reword commit")));
        assert_eq!(
            app.notice().map(|notice| notice.text),
            Some("reword commit · 4 undo · 0 redo".into())
        );

        assert_eq!(app.update(Action::Undo), vec![Effect::Undo]);
        assert_eq!(app.undo_position(), Some((4, 4, "reword commit")));
        app.leave_error("undo failed");
        assert_eq!(app.update(Action::Redo), vec![Effect::Redo]);
        assert_eq!(app.undo_position(), Some((4, 4, "reword commit")));

        app.show_undo_position(0, 4, "ignored at the sentinel");
        assert_eq!(app.undo_position(), Some((0, 4, "start of undo history")));
        assert_eq!(
            app.notice().map(|notice| notice.text),
            Some("start of undo history · 0 undo · 4 redo".into())
        );
        assert!(app.update(Action::MoveDown).is_empty());
        assert_eq!(app.undo_position(), None, "ordinary movement dismisses undo progress");
    }

    #[test]
    fn mandatory_prompts_block_undo_without_dismissing_its_position() {
        let mut app = App::new(2);
        app.show_undo_position(1, 2, "reword commit");
        app.arm_rebase_continuation();

        assert!(app.update(Action::Undo).is_empty());
        assert_eq!(app.undo_position(), Some((1, 2, "reword commit")));
    }

    #[test]
    fn cycles_duplicate_change_ids_and_wraps() {
        let mut app = App::new(2);
        app.extend_commits(vec![row(1), row(2), row(3), row(4), row(5)]);
        complete(&mut app);
        let duplicate = ChangeId::from(id(9));
        app.set_change_ids(
            HashMap::from([(id(1), duplicate), (id(3), duplicate), (id(5), duplicate)]),
            HashSet::from([id(1), id(3), id(5)]),
        );

        assert!(app.can_cycle_duplicate());
        for expected in [id(3), id(5), id(1)] {
            app.update(Action::CycleDuplicate);
            assert_eq!(
                app.selected.map(|index| app.rows[index].id),
                Some(expected),
                "duplicate cycling skips unrelated rows and wraps"
            );
            assert!(
                app.selected
                    .is_some_and(|selected| selected >= app.offset && selected < app.offset + 2),
                "duplicate cycling keeps the new selection visible"
            );
        }

        app.changes_focus = Some(ChangePane::Tree);
        assert!(!app.can_cycle_duplicate());
        app.update(Action::CycleDuplicate);
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(1)),
            "duplicate cycling does not move history while a changes pane is focused"
        );
        app.changes_focus = None;
        app.select_commit(id(2));
        assert!(!app.can_cycle_duplicate());
        app.update(Action::CycleDuplicate);
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(2)));
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
            ["●─╮ ", "● │ ", "├─● ", "● "]
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
    fn refresh_keeps_the_current_tip_as_the_base_of_an_empty_view() {
        let mut app = App::new(10);
        app.extend_hidden_commits(vec![row(1)]);
        app.set_worktree_head(Some(id(1)), false);
        complete(&mut app);

        let rows = app
            .start_refresh(vec![row_with_parents(2, &[1])].into(), &[id(1)], &[id(2)], false)
            .expect("a hidden-only refresh computes lanes");
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [id(1)],
            "the current view tip remains as the base instead of jumping forward"
        );
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);
        assert!(app.is_row_hidden(0), "the fallback remains a hidden boundary");
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(1)),
            "the empty view selects its hidden base"
        );
        app.set_new_commit_availability(Some(&Changes {
            has_tracked_changes: true,
            ..Changes::default()
        }));
        assert_eq!(
            app.update(Action::NewCommit),
            vec![Effect::NewCommit {
                parent: Some(id(1)),
                empty: false,
            }],
            "the checked-out base accepts a first stack commit"
        );
        assert_eq!(
            app.update(Action::NewEmptyCommit),
            vec![Effect::NewCommit {
                parent: Some(id(1)),
                empty: true,
            }],
            "the checked-out base accepts a first empty commit"
        );
        assert_eq!(
            app.update(Action::Rebase),
            vec![Effect::Rebase {
                base: id(1),
                onto: id(1),
                commits: Vec::new(),
            }],
            "a selected base supports rebasing an empty stack"
        );
        app.set_hidden_branch_updates(HashMap::from([(id(1), (1, id(2)))]));
        assert_eq!(
            app.update(Action::RebaseUpdate),
            vec![Effect::Rebase {
                base: id(1),
                onto: id(2),
                commits: Vec::new(),
            }],
            "an empty stack can update to a newer hidden tip"
        );
    }

    #[test]
    fn refresh_keeps_an_advanced_tip_on_its_existing_side() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(3, &[1]), row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);

        let rows = app
            .start_refresh(vec![row_with_parents(4, &[3])].into(), &[id(4), id(2)], &[], false)
            .expect("a refresh computes lanes");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);

        assert_eq!(
            app.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [id(4), id(3), id(2), id(1)],
            "advancing a tip keeps its history ahead of the independent tip"
        );
        assert_eq!(
            app.render_lanes(0..app.rows.len()).iter().collect::<Vec<_>>(),
            ["● ", "● ", "├─● ", "● "],
            "the advanced tip stays in its existing left lane"
        );

        let rows = app
            .start_refresh(vec![row_with_parents(5, &[4])].into(), &[id(5), id(2)], &[], false)
            .expect("a second refresh computes lanes");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);
        assert_eq!(
            app.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [id(5), id(4), id(3), id(2), id(1)],
            "successive commits keep inheriting the current lane order"
        );

        app.select_commit(id(5));
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);
        assert_eq!(
            app.render_lanes(0..app.history_len()).iter().collect::<Vec<_>>(),
            ["● ", "○ ", "├─● ", "● "],
            "compressed history inherits the stable canonical lanes"
        );
    }

    #[test]
    fn refresh_keeps_change_ids_until_the_replacement_projection_is_ready() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);
        let change_id = ChangeId::from(id(9));
        app.set_change_ids(HashMap::from([(id(2), change_id)]), HashSet::from([id(1), id(2)]));

        let _rows = app
            .start_refresh(vec![row_with_parents(3, &[2])].into(), &[id(3)], &[], false)
            .expect("the refresh starts lane computation");

        assert_eq!(
            app.change_id(id(2)),
            change_id,
            "the current frame retains its change ID during refresh"
        );
        assert!(
            app.has_duplicate_change_id(id(1)) && app.has_duplicate_change_id(id(2)),
            "the duplicate gutter remains stable until rows and IDs are replaced together"
        );
    }

    #[test]
    fn ref_tree_pin_refresh_adds_cached_branch_ancestry_and_selects_its_tip() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(3, &[2]), row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);

        let rows = app
            .start_refresh(
                vec![row_with_parents(5, &[4]), row_with_parents(4, &[2])].into(),
                &[id(3)],
                &[],
                false,
            )
            .expect("ref-tree expansion caches its branch while retaining history tips");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);
        assert_eq!(
            app.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [id(3), id(2), id(1)],
            "worktree expansion does not broaden history"
        );

        app.select_commit_after_refresh(id(5));
        let rows = app
            .start_refresh(Vec::<LoadedCommit>::new().into(), &[id(3), id(5)], &[], false)
            .expect("the new pin reprojects already cached commits");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);
        assert_eq!(
            app.rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            [id(3), id(5), id(4), id(2), id(1)],
            "the pinned branch and its shared base appear without restarting"
        );
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(5)),
            "the first frame of the refreshed history selects the pinned entry"
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
    fn refresh_restores_a_rewritten_selection_by_its_visual_position() {
        let mut app = App::new(3);
        app.extend_commits(vec![
            row_with_parents(6, &[5]),
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.select_commit(id(4));
        assert_eq!(app.visual_count(app.selected.expect("commit 4 is selected")), Some(3));
        assert_eq!(app.offset, 0, "the original selection is at the bottom of the viewport");

        let rows = app.store_commits(
            vec![
                numbered_row(17, Some(16)),
                numbered_row(16, Some(15)),
                numbered_row(15, Some(14)),
                numbered_row(14, Some(13)),
                numbered_row(13, Some(2)),
                numbered_row(2, Some(1)),
                numbered_row(1, None),
            ]
            .into(),
        );
        app.state = State::Computing;
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);

        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(14)),
            "the same base and displayed entry number identify the rewritten commit"
        );
        assert_eq!(
            app.offset, 1,
            "the restored selection stays at the bottom of the viewport"
        );

        let rows = app.store_commits(vec![numbered_row(9, Some(8)), numbered_row(8, None)].into());
        app.state = State::Computing;
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);

        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(8)),
            "a missing visual position clamps the previous row before falling back to the top"
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
    fn a_clean_review_can_finish_while_head_is_on_its_successor() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row_with_parents(4, &[1]),
            row(1),
        ]);
        std::sync::Arc::make_mut(&mut app.rows[1]).is_review = true;
        app.selected = Some(1);
        app.set_worktree_head(Some(id(3)), false);
        app.set_head_edit_availability(false, false, false, false, true, false, false);
        complete(&mut app);

        assert_eq!(
            app.update(Action::Review),
            vec![Effect::FinishReview {
                review: id(2),
                return_to: None,
            }]
        );

        app.set_worktree_head(Some(id(4)), false);
        assert!(
            app.update(Action::Review).is_empty(),
            "an unrelated checkout cannot finish the selected review"
        );
    }

    #[test]
    fn a_missing_review_return_can_select_a_detached_checkout() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(6, &[4]),
            row_with_parents(5, &[4]),
            row_with_parents(4, &[1]),
            row_with_parents(2, &[1]),
            row_with_parents(3, &[1]),
            row(1),
        ]);
        std::sync::Arc::make_mut(&mut app.rows[3]).is_review = true;
        app.hidden_rows.insert(id(6));
        app.selected = Some(3);
        complete(&mut app);

        assert!(app.select_review_return(id(2), id(4)));
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(4)));
        assert!(!app.is_row_reachable(0), "hidden descendants are not return candidates");
        assert!(!app.is_row_reachable(3), "review commits are not return candidates");
        assert!(!app.is_row_reachable(4), "unrelated commits are not return candidates");
        app.update(Action::MoveUp);
        assert_eq!(
            app.notice().map(|notice| notice.text),
            Some("review return is missing · j/k select commit · <enter> finish detached · Esc cancel".into()),
            "navigation retains the return-selection prompt"
        );
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(5)),
            "navigation skips ineligible rows"
        );
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![Effect::FinishReview {
                review: id(2),
                return_to: Some(id(5)),
            }]
        );
        assert!(!app.review_return_selection_active());

        assert!(app.select_review_return(id(2), id(4)));
        assert!(app.update(Action::Cancel).is_empty());
        assert!(!app.review_return_selection_active());
    }

    #[test]
    fn forgetting_a_non_merge_tip_is_immediate() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(2, &[1]), row(1)]);
        assert!(!app.can_forget(), "loading history cannot forget commits");
        complete(&mut app);
        assert!(app.can_forget());
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
        app.set_worktree_head(Some(id(2)), false);
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

        let mut unborn_with_history = App::new(10);
        unborn_with_history.set_worktree_head_unborn(true);
        unborn_with_history.extend_hidden_commits(vec![row(1)]);
        complete(&mut unborn_with_history);
        unborn_with_history.set_new_commit_availability(Some(&Changes {
            has_tracked_changes: true,
            ..Changes::default()
        }));
        assert_eq!(
            unborn_with_history.update(Action::NewCommit),
            vec![Effect::NewCommit {
                parent: Some(id(1)),
                empty: false,
            }],
            "the hidden tip can parent the unborn branch's first commit"
        );
        assert!(
            !unborn_with_history.can_fork_commit(),
            "a fork requires an existing worktree HEAD even when another ref is selected"
        );
        assert!(!unborn_with_history.can_rebase(), "rebase todos require a born HEAD");
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
        app.deferred_history_state = Some(State::Complete);
        app.state = State::Computing;
        assert!(
            app.time_travel_shortcut_visible(),
            "the shortcut remains stable during deferred lane computation"
        );
        assert!(
            app.update(Action::TimeTravel).is_empty(),
            "time travel waits for the current lane generation"
        );
        app.deferred_history_state = None;
        app.state = State::Complete;
        app.set_worktree_branch(Some((id(1), true)));
        app.set_worktree_head(Some(id(1)), false);
        assert!(app.can_attach());
        assert_eq!(app.update(Action::Attach), vec![Effect::Attach]);
        app.set_worktree_changes_available(false);
        assert!(app.update(Action::TimeTravel).is_empty());
        assert!(app.update(Action::Attach).is_empty());
        assert_eq!(app.update(Action::TogglePin), vec![Effect::TogglePin(id(1))]);
    }

    #[test]
    fn unresolved_conflicts_disable_time_travel() {
        let mut app = App::new(2);
        app.extend_commits(vec![row_with_parents(2, &[1]), row(1)]);
        app.set_worktree_head(Some(id(2)), false);
        complete(&mut app);
        app.update(Action::MoveDown);
        assert!(app.time_travel_shortcut_visible());
        assert!(app.can_fork_commit());

        app.set_worktree_conflicted(true);
        assert!(!app.time_travel_shortcut_visible());
        assert!(!app.can_fork_commit());
        assert!(app.update(Action::TimeTravel).is_empty());
        app.set_worktree_conflicted(false);
        assert!(app.changes_visible(), "changes are normally shown while enabled");
        app.arm_rebase_conflict(id(1));
        assert!(!app.time_travel_shortcut_visible());
        assert!(!app.can_fork_commit());
        assert!(
            !app.changes_visible(),
            "an in-memory conflict preview cannot be loaded by an on-disk changes view"
        );
        app.clear_rebase_conflict();
        assert!(app.time_travel_shortcut_visible());
        assert!(app.changes_visible(), "clearing the preview restores the changes view");
    }

    #[test]
    fn rebase_conflicts_are_selected_and_centered() {
        let mut app = App::new(3);
        app.extend_commits((1..=7).rev().map(row).collect::<Vec<_>>());
        complete(&mut app);

        app.arm_rebase_conflict(id(4));

        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(4)));
        assert_eq!(app.offset, 2, "the conflict is centered in the three-row viewport");
        assert_eq!(
            app.notice().map(|notice| notice.text),
            Some("rebase conflict · <enter> checkout for resolution · Esc cancel".into())
        );
    }

    #[test]
    fn an_unloaded_rebase_conflict_is_centered_after_refresh() {
        let mut app = App::new(3);
        app.extend_commits(vec![row_with_parents(3, &[2]), row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);
        app.arm_rebase_conflict(id(4));

        let rows = app
            .start_refresh(
                vec![row_with_parents(5, &[4]), row_with_parents(4, &[2])].into(),
                &[id(3), id(5)],
                &[],
                false,
            )
            .expect("the conflict refresh computes lanes");
        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);

        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(4)));
        assert_eq!(app.offset, 1, "the newly loaded conflict is centered");
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
                author_time: gix::date::Time::new(123, 60),
                committer_time: gix::date::Time::new(456, 120),
                author: row(1).author,
                attributions: 0..0,
                title: "loaded".into(),
                has_agent_marker: true,
                is_review: true,
                signature: SignatureState::Verified,
            },
            Vec::new(),
        );
        let (rows, graph, lane_time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, lane_time);

        assert!(app.rows[0].metadata_loaded);
        assert_eq!(app.title(&app.rows[0]), "loaded");
        assert_eq!(app.rows[0].author_time, gix::date::Time::new(123, 60));
        assert_eq!(app.rows[0].committer_time, gix::date::Time::new(456, 120));
        assert!(app.rows[0].has_agent_marker);
        assert!(app.rows[0].is_review);
        assert_eq!(app.rows[0].signature, SignatureState::Verified);
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
            ["●─╮ ", "●─╯ ", "● "]
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
        assert_eq!(
            app.notice().map(|notice| notice.text),
            Some("review base · j/k select ancestor · <enter> start · Esc cancel".into()),
            "navigation retains the review-base prompt"
        );
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
    fn review_starts_immediately_with_only_one_possible_base() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);
        app.selected = Some(0);

        assert_eq!(
            app.update(Action::Review),
            vec![Effect::StartReview {
                tip: id(2),
                base: id(1),
            }],
            "the sole strict ancestor needs no separate selection step"
        );
        assert!(!app.review_selection_active());
    }

    #[test]
    fn squash_selects_a_visible_strict_ancestor_and_can_be_cancelled() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(6, &[1]),
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.selected = app.rows.iter().position(|row| row.id == id(5));

        assert!(app.can_squash());
        assert!(app.update(Action::Squash).is_empty());
        assert!(app.squash_selection_active());
        app.update(Action::MoveDown);
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(4)));
        app.update(Action::MoveDown);
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(3)));
        assert_eq!(
            app.notice().map(|notice| notice.text),
            Some("squash target · j/k select ancestor · <enter> squash · Esc cancel".into())
        );
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![Effect::Squash {
                source: id(5),
                target: id(3),
            }]
        );
        assert!(!app.squash_selection_active());

        app.selected = app.rows.iter().position(|row| row.id == id(5));
        assert!(app.update(Action::Squash).is_empty());
        assert!(app.update(Action::Cancel).is_empty());
        assert!(!app.squash_selection_active());
    }

    #[test]
    fn squash_starts_immediately_with_one_eligible_target() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(3, &[2]), row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);
        app.selected = app.rows.iter().position(|row| row.id == id(3));

        assert_eq!(
            app.update(Action::Squash),
            vec![Effect::Squash {
                source: id(3),
                target: id(2),
            }]
        );
        assert!(!app.squash_selection_active());

        app.hidden_rows.insert(id(2));
        assert!(!app.can_squash(), "hidden commits cannot be squash targets");
    }

    #[test]
    fn single_commit_inserts_use_current_head_and_the_selected_target() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        app.set_worktree_head(Some(id(5)), false);
        complete(&mut app);
        app.selected = app.rows.iter().position(|row| row.id == id(3));

        assert!(app.can_copy_insert());
        assert!(app.can_move_insert());
        assert_eq!(
            app.update(Action::CopyInsert),
            vec![Effect::Insert {
                source: id(5),
                base: id(5),
                target: id(3),
                copy: true,
            }]
        );
        assert_eq!(
            app.update(Action::MoveInsert),
            vec![Effect::Insert {
                source: id(5),
                base: id(5),
                target: id(3),
                copy: false,
            }]
        );

        app.selected = app.rows.iter().position(|row| row.id == id(4));
        assert!(
            app.can_copy_insert(),
            "copying directly above the current parent adds another occurrence"
        );
        assert!(
            !app.can_move_insert(),
            "the current parent is already directly below HEAD"
        );
        app.set_known_merge_descendants(HashSet::from([id(3)]));
        app.selected = app.rows.iter().position(|row| row.id == id(3));
        assert!(!app.can_move_insert(), "a target with an affected merge is rejected");

        let mut review_head = row_with_parents(5, &[4]);
        review_head.is_review = true;
        let mut review_app = App::new(5);
        review_app.extend_commits(vec![
            review_head,
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        review_app.set_worktree_head(Some(id(5)), false);
        complete(&mut review_app);
        review_app.selected = review_app.rows.iter().position(|row| row.id == id(3));
        assert!(review_app.can_copy_insert(), "anything movable can also be copied");
        assert!(review_app.can_move_insert(), "review commits can still be moved");

        let mut merge_target = App::new(10);
        merge_target.extend_commits(vec![
            row_with_parents(5, &[4]),
            row(4),
            row_with_parents(3, &[2, 1]),
            row(2),
            row(1),
        ]);
        merge_target.set_worktree_head(Some(id(5)), false);
        complete(&mut merge_target);
        merge_target.selected = merge_target.rows.iter().position(|row| row.id == id(3));
        assert!(merge_target.can_move_insert(), "an unchanged merge target is allowed");
    }

    #[test]
    fn paste_insert_uses_only_an_editable_history_selection() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);

        assert_eq!(app.paste_insert_target(), Some(id(2)));
        app.hidden_rows.insert(id(2));
        assert_eq!(app.paste_insert_target(), None, "a hidden boundary is not editable");
    }

    #[test]
    fn stack_insert_selects_only_valid_insertion_targets() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(7, &[6]),
            row_with_parents(6, &[1]),
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        app.set_worktree_head(Some(id(5)), false);
        app.set_known_merge_descendants(HashSet::from([id(6)]));
        complete(&mut app);
        app.select_commit(id(3));

        assert!(app.can_stack_insert());
        assert!(app.update(Action::StackInsert).is_empty());
        assert_eq!(
            app.notice().map(|notice| notice.text),
            Some("stack-insert target · j/k select insertion point · <enter> insert · Esc cancel".into())
        );
        for (n, reachable) in [
            (7, true),
            (6, false),
            (5, false),
            (4, false),
            (3, false),
            (2, false),
            (1, true),
        ] {
            let index = app
                .rows
                .iter()
                .position(|row| row.id == id(n))
                .expect("the fixture commit is visible");
            assert_eq!(app.is_row_reachable(index), reachable, "commit {n} target eligibility");
        }

        app.update(Action::MoveDown);
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(1)));
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![Effect::Insert {
                source: id(5),
                base: id(3),
                target: id(1),
                copy: false,
            }]
        );

        app.select_commit(id(3));
        app.update(Action::StackInsert);
        app.update(Action::Cancel);
        assert!(app.notice().is_none(), "Escape cancels insertion-target selection");
        assert!(
            app.rows
                .iter()
                .enumerate()
                .all(|(index, _)| app.is_row_reachable(index))
        );
    }

    #[test]
    fn stack_insert_requires_a_linear_head_ancestry() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3, 2]),
            row_with_parents(3, &[1]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        app.set_worktree_head(Some(id(5)), false);
        complete(&mut app);
        app.select_commit(id(3));

        assert!(!app.can_stack_insert());
        assert!(app.update(Action::StackInsert).is_empty());
        assert!(app.notice().is_none());
    }

    #[test]
    fn stack_insert_hides_targets_that_would_cycle_through_an_internal_commit() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(8, &[4]),
            row_with_parents(7, &[3]),
            row_with_parents(6, &[1]),
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        app.set_worktree_head(Some(id(5)), false);
        complete(&mut app);
        app.select_commit(id(3));

        assert!(app.update(Action::StackInsert).is_empty());
        for (commit, expected) in [(8, false), (7, true), (6, true)] {
            let index = app
                .rows
                .iter()
                .position(|row| row.id == id(commit))
                .expect("the candidate is visible");
            assert_eq!(
                app.is_row_reachable(index),
                expected,
                "candidate {commit} has the expected cycle safety"
            );
        }
    }

    #[test]
    fn refresh_cancels_stack_insert_selection_before_rows_change() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        app.set_worktree_head(Some(id(5)), false);
        complete(&mut app);
        app.select_commit(id(3));
        app.update(Action::StackInsert);
        assert!(app.stack_insert_base.is_some(), "target selection is active");

        let rows = app
            .start_refresh(vec![row_with_parents(6, &[5])].into(), &[id(6)], &[], false)
            .expect("the changed history starts lane computation");
        assert!(
            app.stack_insert_base.is_none(),
            "refresh cancels the index-based selection"
        );
        assert!(
            app.reachable_rows.is_none(),
            "the stale row mask is discarded immediately"
        );
        assert!(app.notice().is_none(), "the cancelled selection no longer prompts");

        let (rows, graph, time) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, time);
        assert_eq!(app.rows.first().map(|row| row.id), Some(id(6)));
        assert!(
            app.rows
                .iter()
                .enumerate()
                .all(|(index, _)| app.is_row_reachable(index)),
            "the replacement projection has no stale eligibility mask"
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
    fn entry_numbers_select_within_the_current_tree() {
        let mut app = App::new(2);
        app.extend_commits(vec![
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row(3),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.select_commit(id(1));
        assert_eq!(app.visual_count(1), Some(1), "the other tree also contains #1");
        assert_eq!(app.visual_count(3), Some(1), "the current tree contains #1");

        app.update(Action::SelectEntry);
        app.update(Action::SelectEntryInput("1".into()));
        assert_eq!(
            app.notice().map(|notice| notice.text),
            Some("select entry #1 · type number · <enter> jump · Esc cancel".into())
        );
        app.update(Action::SubmitEntrySelection);
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(2)),
            "#1 resolves against the selected root instead of another tree"
        );
        assert!(!app.entry_selection_active());

        app.update(Action::SelectEntry);
        app.update(Action::SelectEntryInput("2".into()));
        app.update(Action::SubmitEntrySelection);
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(2)),
            "a number absent from the current tree keeps the cursor in place"
        );
        assert!(app.entry_selection_active(), "an invalid number remains editable");
        assert_eq!(
            app.notice().map(|notice| notice.text),
            Some(
                "select entry #2 · type number · <enter> jump · Esc cancel · entry #2 is not in the current tree"
                    .into()
            )
        );
        app.update(Action::Cancel);
        assert!(!app.entry_selection_active());
    }

    #[test]
    fn topological_navigation_chooses_all_parents_and_children() {
        let mut app = App::new(1);
        app.extend_commits(vec![
            row_with_parents(6, &[4, 5]),
            row_with_parents(5, &[3]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);

        app.update(Action::PanDownBy(3));
        app.update(Action::TopologicalDown);
        assert_eq!(app.rows[app.selected.expect("the merge stays selected")].id, id(6));
        assert_eq!(app.topological_choice(), Some((1, 2)));
        assert_eq!(app.offset, 0, "starting a choice reveals its source marker");
        app.update(Action::NextChild);
        assert_eq!(app.topological_choice(), Some((2, 2)));
        app.update(Action::SubmitTopological);
        assert_eq!(app.rows[app.selected.expect("the second parent is selected")].id, id(5));

        app.update(Action::TopologicalUp);
        assert_eq!(
            app.rows[app.selected.expect("the merge is selected again")].id,
            id(6),
            "a commit is a child even through its secondary-parent edge"
        );

        app.update(Action::TopologicalDown);
        app.update(Action::PreviousChild);
        assert_eq!(app.topological_choice(), Some((2, 2)), "choices wrap to the end");
        app.update(Action::CancelTopological);
        assert_eq!(
            app.rows[app.selected.expect("cancel keeps the source selected")].id,
            id(6)
        );
        assert_eq!(app.topological_choice(), None);

        app.select_commit(id(3));
        app.update(Action::TopologicalUp);
        assert_eq!(app.topological_choice(), Some((1, 2)));
        app.update(Action::NextChild);
        app.update(Action::SubmitTopological);
        assert_eq!(app.rows[app.selected.expect("the second child is selected")].id, id(5));
    }

    #[test]
    fn lane_computation_cancels_indexed_topological_choices() {
        let mut app = App::new(2);
        app.extend_commits(vec![
            row_with_parents(4, &[2, 3]),
            row(3),
            row(1),
            row_with_parents(2, &[1]),
        ]);

        app.follow_tail = true;
        app.update(Action::TopologicalDown);
        assert_eq!(app.topological_choice(), Some((1, 2)));
        assert!(
            !app.follow_tail,
            "choosing keeps streamed commits from moving the source"
        );
        app.update(Action::CancelTopological);

        let rows = app
            .start_lane_computation()
            .expect("loading completion starts lane work");
        app.update(Action::TopologicalDown);
        assert_eq!(app.topological_choice(), Some((1, 2)));
        let (rows, graph, elapsed) = compute_lanes(rows);
        app.finish_lane_computation(rows, graph, elapsed);
        assert_eq!(
            app.topological_choice(),
            None,
            "lane reordering invalidates indexed choices"
        );
    }

    #[test]
    fn topological_navigation_contracts_unselectable_rows() {
        let mut app = App::new(4);
        app.extend_commits(vec![
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.reachable_rows = Some(vec![true, false, true, true]);

        app.update(Action::TopologicalDown);
        assert_eq!(app.rows[app.selected.expect("the ancestor is selected")].id, id(2));
        app.update(Action::TopologicalUp);
        assert_eq!(
            app.rows[app.selected.expect("the descendant is selected")].id,
            id(4),
            "the contracted edge works in both directions"
        );

        app.selected = Some(1);
        app.update(Action::TopologicalUp);
        assert_eq!(
            app.rows[app.selected.expect("the selectable child is selected")].id,
            id(4),
            "an ineligible current row still anchors its child walk"
        );
    }

    #[test]
    fn topological_navigation_deduplicates_contracted_paths() {
        let mut app = App::new(3);
        app.extend_commits(vec![row_with_parents(3, &[2, 1]), row_with_parents(2, &[1]), row(1)]);
        complete(&mut app);
        app.reachable_rows = Some(vec![true, false, true]);

        app.update(Action::TopologicalDown);

        assert_eq!(app.rows[app.selected.expect("the sole ancestor is selected")].id, id(1));
        assert_eq!(app.topological_choice(), None, "the shared ancestor is offered once");
    }

    #[test]
    fn viewport_navigation_pans_without_moving_the_cursor() {
        let mut app = App::new(2);
        app.extend_commits(
            (1..=6)
                .rev()
                .map(|commit| numbered_row(commit, (commit > 1).then_some(commit - 1)))
                .collect::<Vec<_>>(),
        );

        app.update(Action::PanDownBy(3));
        app.prepare_history_viewport();
        assert_eq!((app.selected, app.offset), (Some(0), 3));
        app.update(Action::PanDownBy(app.viewport_rows));
        assert_eq!(
            (app.selected, app.offset),
            (Some(0), 4),
            "page movement clamps the viewport"
        );
        app.update(Action::TopologicalDown);
        assert_eq!(
            (app.selected, app.offset),
            (Some(1), 1),
            "a topo step brings its destination back into view"
        );
        app.update(Action::PanDownBy(app.viewport_rows));
        app.update(Action::PanUpBy((app.viewport_rows / 2).max(1)));
        assert_eq!((app.selected, app.offset), (Some(1), 2));
        complete(&mut app);
        assert_eq!(
            (app.selected, app.offset),
            (Some(1), 2),
            "lane completion preserves a detached viewport"
        );
    }

    #[test]
    fn time_travel_selection_crosses_each_viewport_before_paging() {
        let mut app = App::new(4);
        app.extend_commits((1..=10).map(row).collect::<Vec<_>>());
        complete(&mut app);

        let mut positions = Vec::new();
        for commit in (1u16..=10).rev() {
            app.select_commit_for_time_travel(id(commit));
            positions.push((app.selected, app.offset));
        }

        assert_eq!(
            positions,
            [
                (Some(9), 6),
                (Some(8), 6),
                (Some(7), 6),
                (Some(6), 6),
                (Some(5), 2),
                (Some(4), 2),
                (Some(3), 2),
                (Some(2), 2),
                (Some(1), 0),
                (Some(0), 0),
            ],
            "selection crosses a full viewport before the next page is bottom-aligned"
        );
    }

    #[test]
    fn time_travel_animation_returns_to_its_origin() {
        let mut app = App::new(4);
        app.extend_commits(
            (1u16..=10)
                .rev()
                .map(|commit| numbered_row(commit, (commit > 1).then_some(commit - 1)))
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.select_commit(id(6));
        assert_eq!((app.selected, app.offset), (Some(4), 1));

        app.begin_time_travel_animation();
        app.select_commit_for_time_travel(id(10));
        app.finish_time_travel_animation();

        assert_eq!(
            (app.selected.map(|index| app.rows[index].id), app.offset),
            (Some(id(6)), 1),
            "the animated cursor returns to the requested entry and viewport row"
        );

        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);
        let viewport_row = app
            .selected_history_index()
            .expect("the origin is displayed in compressed history")
            .saturating_sub(app.offset);
        app.begin_time_travel_animation();
        app.select_commit_for_time_travel(id(10));
        app.finish_time_travel_animation();

        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(6)));
        assert_eq!(
            app.selected_history_index()
                .expect("the origin remains displayed")
                .saturating_sub(app.offset),
            viewport_row,
            "compressed history restores the origin to its former viewport row"
        );
    }

    #[test]
    fn time_travel_animation_temporarily_expands_compressed_history() {
        let mut app = App::new(4);
        app.extend_commits(
            (1u16..=10)
                .rev()
                .map(|commit| numbered_row(commit, (commit > 1).then_some(commit - 1)))
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.set_view_tips(&[id(10)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);
        assert!(app.history_len() < app.rows.len(), "the linear history is compressed");

        app.begin_time_travel_animation();
        assert_eq!(app.history_len(), app.rows.len(), "animation uses canonical rows");
        let mut offsets = Vec::new();
        for commit in 1..=10 {
            app.select_commit_for_time_travel(id(commit));
            offsets.push(app.offset);
        }
        assert_eq!(offsets, [6, 6, 6, 6, 2, 2, 2, 2, 0, 0]);
        app.finish_time_travel_animation();

        assert_eq!(app.alignment, Alignment::Compressed);
        assert!(app.history_len() < app.rows.len(), "compression is restored");
    }

    #[test]
    fn compressed_history_retains_tips_and_selection_and_selects_segments() {
        let mut app = App::new(2);
        app.extend_commits(vec![
            row_with_parents(6, &[5]),
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.set_view_tips(&[id(6)]);
        app.select_commit(id(3));
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);

        assert_eq!(
            (0..app.history_len())
                .filter_map(|index| app.history_entry(index))
                .collect::<Vec<_>>(),
            [
                HistoryEntry::Commit(0),
                HistoryEntry::Segment {
                    representative: 1,
                    count: 2,
                },
                HistoryEntry::Commit(3),
                HistoryEntry::Commit(4),
                HistoryEntry::Commit(5),
            ],
            "linear runs collapse only when their summary saves vertical space"
        );
        assert_eq!(
            app.render_lanes(0..app.history_len()).iter().collect::<Vec<_>>(),
            ["● ", "○ ", "● ", "● ", "● "],
            "only segments use the quieter ring marker"
        );

        app.update(Action::MoveUp);
        assert!(app.selected_is_segment(), "navigation stops on a segment summary");
        app.update(Action::MoveDown);
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(3)),
            "navigation returns from a summary to its adjacent retained commit"
        );

        app.select_commit(id(4));
        assert_eq!(app.selected.map(|index| app.rows[index].id), Some(id(4)));
        assert!(
            app.selected_history_index().is_some(),
            "a programmatic jump rebuilds compression around the destination"
        );
    }

    #[test]
    fn topological_navigation_peels_and_selects_commits_towards_segments() {
        let mut app = compressed_linear_app();
        let compressed_len = app.history_len();
        assert!(compressed_len < app.rows.len());

        app.update(Action::TopologicalDown);
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(5)),
            "moving rootward exposes and selects the segment's connected commit"
        );
        assert_eq!(
            app.history_len(),
            compressed_len + 1,
            "one topological step exposes exactly one commit"
        );
        app.update(Action::TopologicalDown);
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(4)),
            "repeating the movement peels the next connected commit"
        );
        let selected = app.selected_history_index().expect("the peeled commit is displayed");
        assert!(
            (app.offset..app.offset + app.viewport_rows).contains(&selected),
            "the peeled selection remains inside the viewport"
        );

        let mut app = compressed_linear_app();
        app.update(Action::Last);
        app.update(Action::TopologicalUp);
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(2)),
            "moving leafward exposes and selects the opposite segment boundary"
        );
    }

    #[test]
    fn topological_navigation_peels_while_retaining_a_segment_selection() {
        let mut app = compressed_linear_app();
        app.update(Action::MoveDown);
        assert!(app.selected_is_segment(), "ordinary movement selects the summary");

        for peeled in [2, 3] {
            app.update(Action::TopologicalDown);
            assert!(app.selected_is_segment(), "the remaining summary stays selected");
            let canonical = app
                .rows
                .iter()
                .position(|row| row.id == id(peeled))
                .expect("the peeled commit exists");
            assert!(
                app.history_index(canonical).is_some(),
                "the rootward boundary commit is exposed"
            );
        }
        app.update(Action::TopologicalDown);
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(5)),
            "the remaining singleton replaces the vanished summary selection"
        );

        let mut app = compressed_linear_app();
        app.update(Action::MoveDown);
        app.update(Action::TopologicalUp);
        assert!(
            app.selected_is_segment(),
            "the summary remains selected after a leafward peel"
        );
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            None,
            "the exposed leafward commit is not selected"
        );
        let peeled = app
            .rows
            .iter()
            .position(|row| row.id == id(5))
            .expect("the peeled commit exists");
        assert!(
            app.history_index(peeled).is_some(),
            "the leafward boundary commit is exposed"
        );
    }

    #[test]
    fn topological_peeling_does_not_select_ineligible_segment_members() {
        let mut app = compressed_linear_app();
        app.reachable_rows = Some(vec![true, false, false, true, false, true]);

        for peeled in [5, 4] {
            app.update(Action::TopologicalDown);
            assert_eq!(
                app.selected.map(|index| app.rows[index].id),
                Some(id(6)),
                "an ineligible boundary leaves the current eligible commit selected"
            );
            let canonical = app
                .rows
                .iter()
                .position(|row| row.id == id(peeled))
                .expect("the peeled commit exists");
            assert!(
                app.history_index(canonical).is_some(),
                "the ineligible boundary is still exposed one commit at a time"
            );
        }
        app.update(Action::TopologicalDown);
        assert_eq!(
            app.selected.map(|index| app.rows[index].id),
            Some(id(3)),
            "the first eligible peeled commit becomes selected"
        );
    }

    #[test]
    fn compressed_mode_transitions_keep_the_selected_viewport_row() {
        let mut app = App::new(4);
        app.extend_commits(
            (0..=11)
                .rev()
                .map(|n| numbered_row(n, n.checked_sub(1)))
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.set_view_tips(&[id(11)]);
        app.select_commit(id(5));
        app.offset = 4;
        let viewport_row = app.selected_history_index().expect("the commit is selected") - app.offset;

        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);
        assert_eq!(
            app.selected_history_index().map(|selected| selected - app.offset),
            Some(viewport_row),
            "entering compressed mode keeps the selected commit on screen"
        );

        app.update(Action::MoveDown);
        assert!(app.selected_is_segment(), "the lower compressed segment is selected");
        app.offset = 1;
        let viewport_row = app.selected_history_index().expect("the segment is selected") - app.offset;
        app.update(Action::ToggleAlign);
        assert_eq!(
            app.selected_history_index().map(|selected| selected - app.offset),
            Some(viewport_row),
            "leaving compressed mode keeps its representative on the segment's screen row"
        );
    }

    #[test]
    fn compressed_history_keeps_ancestor_tips_and_merge_topology() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(7, &[6, 5]),
            row_with_parents(6, &[4]),
            row_with_parents(5, &[3]),
            row_with_parents(4, &[2]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.set_view_tips(&[id(7), id(2)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);

        let entries: Vec<_> = (0..app.history_len())
            .filter_map(|index| app.history_entry(index))
            .collect();
        assert_eq!(entries.first(), Some(&HistoryEntry::Commit(0)));
        assert!(
            entries.contains(&HistoryEntry::Commit(5)),
            "an explicit ancestor tip remains visible even though it is not a graph leaf"
        );
        assert_eq!(
            entries
                .iter()
                .filter_map(|entry| match entry {
                    HistoryEntry::Segment { count, .. } => Some(*count),
                    HistoryEntry::Commit(_) => None,
                })
                .sum::<usize>(),
            4,
            "every non-anchor commit is represented exactly once"
        );
        let lanes = app.render_lanes(0..app.history_len());
        assert!(
            lanes
                .iter()
                .any(|lane| lane.contains('╮') || lane.contains('╯') || lane.contains('─')),
            "the quotient graph preserves merge branches"
        );
    }

    #[test]
    fn compressed_history_stays_active_for_target_selection() {
        let mut app = App::new(2);
        app.extend_commits(vec![
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.set_view_tips(&[id(4)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);
        assert_eq!(app.history_len(), 3);

        app.reachability_anchor = Some(id(4));
        app.compute_reachable_rows();
        assert_eq!(
            app.history_len(),
            3,
            "target selection retains the compressed projection"
        );
        app.update(Action::MoveDown);
        assert!(app.selected_is_segment(), "an eligible target segment is selectable");
        app.update(Action::OpenDiff);
        assert_eq!(app.history_len(), app.rows.len(), "Enter expands the eligible segment");
        app.clear_reachability_selection();
        assert_eq!(
            app.history_len(),
            app.rows.len(),
            "expanded segments remain open for the compressed cycle"
        );
    }

    #[test]
    fn compressed_history_keeps_graph_junctions_and_endpoints_as_commits() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(6, &[4]),
            row_with_parents(5, &[3]),
            row_with_parents(4, &[2]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        app.set_worktree_head(Some(id(6)), false);
        complete(&mut app);
        app.set_view_tips(&[id(6), id(5)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);

        for (n, description) in [
            (6, "the current graph tip"),
            (5, "the other graph tip"),
            (2, "the shared merge base"),
            (1, "the graph root"),
        ] {
            let canonical = app
                .rows
                .iter()
                .position(|row| row.id == id(n))
                .expect("the structural commit is present");
            let display = app
                .history_index(canonical)
                .expect("structural commits have their own display row");
            assert_eq!(
                app.history_entry(display),
                Some(HistoryEntry::Commit(canonical)),
                "{description} remains a full commit"
            );
        }

        app.select_commit(id(2));
        assert!(
            app.can_copy_insert(),
            "a retained merge base is an ordinary insertion target"
        );
        assert_eq!(
            app.update(Action::CopyInsert),
            vec![Effect::Insert {
                source: id(6),
                base: id(6),
                target: id(2),
                copy: true,
            }]
        );
    }

    #[test]
    fn compressed_history_keeps_the_first_hidden_boundary_as_a_commit() {
        let mut app = App::new(10);
        app.extend_commits(vec![row_with_parents(3, &[2]), row_with_parents(2, &[1])]);
        app.extend_hidden_commits(vec![row_with_parents(1, &[0])]);
        complete(&mut app);
        app.set_view_tips(&[id(3)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);

        let boundary = app
            .rows
            .iter()
            .position(|row| row.id == id(1))
            .expect("the hidden boundary is present");
        assert!(app.is_row_hidden(boundary));
        let display = app
            .history_index(boundary)
            .expect("the hidden boundary has its own display row");
        assert_eq!(app.history_entry(display), Some(HistoryEntry::Commit(boundary)));

        app.select_commit(id(1));
        assert!(
            app.can_rebase(),
            "the retained boundary keeps its existing base actions"
        );
        assert_eq!(
            app.update(Action::Rebase),
            vec![Effect::Rebase {
                base: id(1),
                onto: id(1),
                commits: vec![id(3), id(2)],
            }]
        );
    }

    #[test]
    fn compressed_segments_are_selectable_and_expand_in_place() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(7, &[6]),
            row_with_parents(6, &[5]),
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        let ordinary_lanes = app
            .render_lanes(0..app.rows.len())
            .iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        app.set_view_tips(&[id(7)]);
        app.select_commit(id(4));
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);

        app.update(Action::MoveUp);
        assert!(app.selected_is_segment(), "navigation stops on the upper summary");
        assert_eq!(app.selected_history_index(), Some(1));
        app.set_worktree_head_unborn(true);
        for action in [Action::ToggleActions, Action::ToggleEnrich] {
            app.update(action);
        }
        assert!(!app.actions_expanded && !app.enrich_expanded);
        assert!(
            app.update(Action::NewEmptyCommit).is_empty(),
            "a synthetic selection is not mistaken for an unborn HEAD"
        );
        assert!(
            app.update(Action::OpenDiff).is_empty(),
            "Enter expands instead of diffing a summary"
        );
        assert_eq!(
            app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id),
            Some(id(6)),
            "expansion selects the newest commit represented by the summary"
        );
        assert!(
            (0..app.history_len()).any(|index| matches!(app.history_entry(index), Some(HistoryEntry::Segment { .. }))),
            "expanding one summary leaves the other summary compressed"
        );

        app.update(Action::Last);
        app.update(Action::MoveUp);
        assert!(app.selected_is_segment(), "navigation stops on the remaining summary");
        assert!(app.update(Action::OpenDiff).is_empty());
        assert_eq!(
            (0..app.history_len())
                .filter_map(|index| app.history_entry(index))
                .collect::<Vec<_>>(),
            (0..app.rows.len()).map(HistoryEntry::Commit).collect::<Vec<_>>(),
            "expanding every summary produces the ordinary row sequence"
        );
        assert_eq!(
            app.render_lanes(0..app.history_len())
                .iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            ordinary_lanes,
            "fully expanded compression uses the ordinary graph"
        );
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![Effect::OpenCommitDiff(TreeDiffTarget::Commit { id: id(3), parent: 0 })],
            "Enter resumes its normal commit behavior after expansion"
        );
    }

    #[test]
    fn compressed_modal_navigation_only_selects_segments_with_eligible_members() {
        let mut app = App::new(10);
        app.extend_commits(vec![
            row_with_parents(6, &[5]),
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.set_view_tips(&[id(6)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);

        app.viewport_rows = 10;
        app.reachable_rows = Some(vec![false, false, false, true, false, false]);
        app.update(Action::PageDown);
        assert!(
            app.selected_is_segment(),
            "paged navigation reaches a nearby eligible segment"
        );
        app.reachable_rows = Some(vec![false; 6]);
        app.update(Action::OpenDiff);
        assert!(
            app.selected_is_segment(),
            "an ineligible segment keeps its selection instead of expanding without a target"
        );

        app.select_commit(id(6));
        app.reachable_rows = Some(vec![false, false, false, false, false, true]);
        app.update(Action::MoveDown);
        assert_eq!(
            app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id),
            Some(id(1)),
            "navigation skips a segment without an eligible target"
        );

        app.select_commit(id(6));
        app.reachable_rows = Some(vec![false, false, false, true, false, false]);
        app.update(Action::MoveDown);
        assert!(
            app.selected_is_segment(),
            "a segment containing an eligible target is selectable"
        );
        app.update(Action::OpenDiff);
        assert_eq!(
            app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id),
            Some(id(3)),
            "expansion selects the newest eligible member"
        );
    }

    #[test]
    fn compressed_selection_and_expansions_survive_in_place_refreshes() {
        let commits = || {
            vec![
                row_with_parents(5, &[4]),
                row_with_parents(4, &[3]),
                row_with_parents(3, &[2]),
                row_with_parents(2, &[1]),
                row(1),
            ]
        };
        let refresh = |app: &mut App| {
            let rows = app
                .start_refresh(commits().into(), &[id(5)], &[], false)
                .expect("the in-place refresh computes lanes");
            let (rows, graph, time) = compute_lanes(rows);
            app.finish_lane_computation(rows, graph, time);
        };
        let mut app = App::new(10);
        app.extend_commits(commits());
        complete(&mut app);
        app.set_view_tips(&[id(5)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);
        app.update(Action::MoveDown);

        refresh(&mut app);
        assert!(app.selected_is_segment(), "a synthetic selection survives by object ID");
        app.update(Action::OpenDiff);
        refresh(&mut app);
        assert_eq!(
            (0..app.history_len())
                .filter_map(|index| app.history_entry(index))
                .collect::<Vec<_>>(),
            (0..app.rows.len()).map(HistoryEntry::Commit).collect::<Vec<_>>(),
            "expanded members survive an in-place refresh"
        );
    }

    #[test]
    fn suspending_compression_materializes_a_segment_selection() {
        let mut app = App::new(2);
        app.extend_commits(vec![
            row_with_parents(5, &[4]),
            row_with_parents(4, &[3]),
            row_with_parents(3, &[2]),
            row_with_parents(2, &[1]),
            row(1),
        ]);
        complete(&mut app);
        app.set_view_tips(&[id(5)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);
        app.update(Action::MoveDown);
        assert!(app.selected_is_segment());

        app.set_worktree_conflicted(true);
        assert_eq!(
            app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id),
            Some(id(4)),
            "the segment representative becomes the full-history selection"
        );
        app.update(Action::MoveDown);
        assert_eq!(
            app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id),
            Some(id(3))
        );
    }

    #[test]
    fn compressed_expansions_reset_on_mode_exit_and_full_reload() {
        let commits = || {
            vec![
                row_with_parents(6, &[5]),
                row_with_parents(5, &[4]),
                row_with_parents(4, &[3]),
                row_with_parents(3, &[2]),
                row_with_parents(2, &[1]),
                row(1),
            ]
        };
        let has_segment = |app: &App| {
            (0..app.history_len()).any(|index| matches!(app.history_entry(index), Some(HistoryEntry::Segment { .. })))
        };

        let mut app = App::new(10);
        app.extend_commits(commits());
        complete(&mut app);
        app.set_view_tips(&[id(6)]);
        app.alignment = Alignment::None;
        app.update(Action::ToggleAlign);
        app.update(Action::MoveDown);
        assert!(app.selected_is_segment());
        app.update(Action::OpenDiff);
        assert!(!has_segment(&app), "the only linear summary is expanded");

        app.update(Action::ToggleAlign);
        app.update(Action::ToggleAlign);
        app.update(Action::ToggleAlign);
        app.update(Action::ToggleAlign);
        assert_eq!(app.alignment, Alignment::Compressed);
        assert!(has_segment(&app), "leaving compressed mode discards expansions");

        app.update(Action::MoveDown);
        if !app.selected_is_segment() {
            app.update(Action::MoveDown);
        }
        assert!(app.selected_is_segment(), "a summary is available to expand again");
        app.update(Action::OpenDiff);
        app.reload(false);
        app.set_view_tips(&[id(6)]);
        app.extend_commits(commits());
        complete(&mut app);
        assert_eq!(app.alignment, Alignment::Compressed);
        assert!(
            has_segment(&app),
            "a full reload discards expansions even for unchanged commit IDs"
        );
    }

    #[test]
    fn hidden_boundary_rows_are_selectable_for_inspection_and_independent_edits() {
        let mut app = App::new(4);
        app.extend_commits(vec![row(1), row(2), row(3)]);
        app.extend_hidden_commits(vec![row(4)]);
        app.set_worktree_head(Some(id(1)), false);
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
        app.set_changes_bounds(ChangePane::Tree, 2, 5, None, 1, 0);
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
        app.set_changes_bounds(ChangePane::Tree, 4, 10, None, 20, 45);
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

        assert_eq!(app.date_mode, DateMode::Committer);
        app.update(Action::ToggleDate);
        assert_eq!(app.date_mode, DateMode::None);
        app.update(Action::ToggleDate);
        assert_eq!(app.date_mode, DateMode::Author);
        assert_eq!(app.id_mode, IdMode::Off);
        app.update(Action::CycleIds);
        assert_eq!(app.id_mode, IdMode::Commit);
        app.update(Action::CycleIds);
        assert_eq!(app.id_mode, IdMode::Change);
        app.update(Action::CycleIds);
        assert_eq!(app.id_mode, IdMode::Off);
        app.set_change_ids(HashMap::new(), HashSet::from([id(1), id(2)]));
        assert_eq!(
            app.effective_id_mode(),
            IdMode::Change,
            "off automatically reveals duplicate change IDs"
        );
        app.update(Action::CycleIds);
        assert_eq!(
            app.effective_id_mode(),
            IdMode::Commit,
            "an explicit commit-ID mode overrides automatic change IDs"
        );
        assert!(app.show_emails);
        assert_eq!(app.name_mode, NameMode::None);
        assert!(!app.show_trailers);
        assert!(!app.use_mailmap);
        assert_eq!(app.ref_mode, RefMode::None);
        app.update(Action::CycleRefs);
        assert_eq!(app.ref_mode, RefMode::All);
        app.update(Action::CycleRefs);
        assert_eq!(app.ref_mode, RefMode::Default);
        assert_eq!(app.alignment, Alignment::Columns);
        assert!(app.show_commit);
        assert_eq!(app.changes_mode, Some(ChangesMode::Tree));
        assert_eq!(app.changes_parent, 0);
        app.update(Action::ToggleAlign);
        assert_eq!(app.alignment, Alignment::None);
        app.update(Action::ToggleAlign);
        assert_eq!(app.alignment, Alignment::Compressed);
        app.update(Action::ToggleAlign);
        assert_eq!(app.alignment, Alignment::Title);
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
    fn actions_group_stays_open_only_for_its_actions() {
        let mut app = App::new(1);

        app.update(Action::ToggleActions);
        assert!(app.actions_expanded);
        app.update(Action::Reword);
        app.update(Action::NewCommit);
        app.update(Action::Forget);
        app.update(Action::TimeTravel);
        app.update(Action::Rebase);
        app.update(Action::RebaseUpdate);
        app.update(Action::Review);
        app.update(Action::Squash);
        app.update(Action::CopyInsert);
        app.update(Action::MoveInsert);
        app.update(Action::StackInsert);
        app.update(Action::Stash);
        app.update(Action::ForkCommit);
        app.update(Action::Attach);
        assert!(app.actions_expanded, "all grouped actions keep the group open");

        app.update(Action::MoveDown);
        assert!(!app.actions_expanded, "navigation collapses the group");

        app.update(Action::ToggleActions);
        app.update(Action::ToggleHistoryDisplay);
        assert!(!app.actions_expanded, "opening the view group closes actions");
        assert!(app.history_display_expanded);

        app.update(Action::ToggleActions);
        assert!(app.actions_expanded);
        assert!(!app.history_display_expanded, "opening actions closes the view group");
        app.update(Action::ToggleActions);
        assert!(!app.actions_expanded, "the prefix key toggles the group");
    }

    #[test]
    fn enrich_group_keeps_git_notes_available_on_immutable_commits() {
        let mut app = App::new(1);
        app.extend_commits(vec![row(1)]);
        complete(&mut app);

        app.update(Action::ToggleEnrich);
        assert!(app.enrich_expanded);
        assert_eq!(
            app.update(Action::ToggleTodo),
            vec![Effect::ToggleTodo(id(1))],
            "todo reuses reword eligibility"
        );
        assert_eq!(
            app.update(Action::EditNote),
            vec![Effect::EditNote(id(1))],
            "note uses the same eligibility"
        );
        assert_eq!(
            app.update(Action::EditGitNote),
            vec![Effect::EditGitNote(id(1))],
            "Git notes are available on every selection"
        );
        assert_eq!(
            app.update(Action::ToggleChecksPass),
            vec![Effect::ToggleChecksPass(id(1))],
            "tree enrichments are available on every selection"
        );
        assert!(app.enrich_expanded, "the grouped actions keep enrich open");

        app.update(Action::MoveDown);
        assert!(!app.enrich_expanded, "navigation closes enrich");
        app.update(Action::ToggleEnrich);
        app.update(Action::ToggleActions);
        assert!(!app.enrich_expanded, "opening actions closes enrich");

        app.hidden_rows.insert(id(1));
        app.update(Action::ToggleEnrich);
        assert!(
            app.update(Action::ToggleTodo).is_empty(),
            "hidden boundaries are immutable"
        );
        assert!(app.update(Action::EditNote).is_empty());
        assert_eq!(
            app.update(Action::EditGitNote),
            vec![Effect::EditGitNote(id(1))],
            "immutable boundaries still accept Git notes"
        );
        assert_eq!(
            app.update(Action::ToggleChecksPass),
            vec![Effect::ToggleChecksPass(id(1))],
            "immutable boundaries still accept tree enrichments"
        );
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
            Action::ToggleRefTree,
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
        app.update(Action::ToggleActions);
        assert!(!app.information_expanded, "opening actions closes information");
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
        app.set_changes_bounds(ChangePane::Tree, 2, 4, None, 10, 20);
        app.set_changes_bounds(ChangePane::Worktree, 2, 4, None, 10, 20);
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
    fn changes_navigation_skips_display_separators() {
        let mut app = App::new(1);
        app.changes_focus = Some(ChangePane::Worktree);
        app.set_changes_bounds(ChangePane::Worktree, 3, 5, Some(2), 10, 0);

        app.update(Action::PageDown);
        assert_eq!(app.worktree_changes.selected, 2, "page movement counts only paths");
        assert_eq!(
            app.worktree_changes.offset, 1,
            "the divider remains in the display viewport"
        );
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![Effect::OpenDiff(ChangePane::Worktree, 2)],
            "actions retain path indices"
        );

        app.update(Action::Last);
        assert_eq!(app.worktree_changes.selected, 4);
        assert_eq!(
            app.worktree_changes.offset, 3,
            "the display offset includes the divider row"
        );
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
        app.set_changes_bounds(ChangePane::Tree, 1, 3, None, 1, 2);
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

        assert_eq!(
            app.update(Action::Copy),
            vec![Effect::CopyId(row(7).id)],
            "hidden identifiers copy the commit ID"
        );
        app.id_mode = IdMode::Commit;
        assert_eq!(
            app.update(Action::Copy),
            vec![Effect::CopyId(row(7).id)],
            "shown commit IDs copy the commit ID"
        );
        let change_id = ChangeId::from(id(8));
        app.set_change_ids(HashMap::from([(row(7).id, change_id)]), HashSet::new());
        app.id_mode = IdMode::Change;
        assert_eq!(
            app.update(Action::Copy),
            vec![Effect::CopyChangeId(change_id)],
            "shown change IDs copy the change ID"
        );
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
