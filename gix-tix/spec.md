# gix-tix specification

This document describes the intended behavior of `tix` on this branch. It is the
behavioral contract for future changes; implementation details belong here only
when they preserve responsiveness, bounded memory, Git compatibility, or resource
lifetime.

## Purpose and invocation

`tix` is a minimal, `tig`-inspired commit-history browser optimized for large
repositories. It must remain useful on histories as large as the Linux kernel
without trading responsiveness for metadata that is not visible.

- `tix [REVISION]...` shows commits reachable from the supplied revisions, or
  from `HEAD` when none are supplied.
- `-h/--hide REVSPEC` excludes the revision and its reachable ancestry. The
  option may be repeated.
- `--quit-on-finish` exits after traversal and lane computation, for measurement
  and non-interactive use.
- Revisions must resolve and peel to commits. Invalid or non-commit revisions are
  errors.
- The UI always owns the alternate screen. Raw mode, focus reporting, mouse
  capture, and enhanced keyboard reporting are restored on every exit path.
- `Ctrl-C` exits immediately from any normal tix focus. `q` quits from history;
  `q` or `Escape` in a focused changes block returns focus to history.

## History model

### Traversal and projection

- Traversal streams commits before graph-lane computation finishes. The footer
  reports the number received while loading and switches to the selected row
  number after completion. Rows are numbered from the bottom, so the oldest row
  is `#1` and the newest row is `#N`.
- Commit topology, commit time, and generation are loaded through the same
  commit-graph-or-ODB lookup model as `gix-traverse`. A small object cache avoids
  repeated ODB decoding during a walk.
- Metadata already decoded from ODB is retained. Metadata omitted because a
  commit came from the commit-graph is populated lazily for visible rows.
- The persistent graph is append-only and index-addressed, with one compact copy
  of each commit and flat parent edges. View refreshes project rows from this
  cache and stop walking when complete cached ancestry is reached.
- Local branch targets are reverse-indexed. Configured upstream targets are added
  as internal traversal tips so ahead/behind calculations have complete ancestry
  without a second repository walk.
- Shallow boundaries are honored. Parent topology needed by future projections,
  hidden expansion, and ahead/behind calculations must not be pruned with the
  currently visible lane graph.

### Hidden history

- Hidden ancestry is removed from the selectable view by default. Direct parents
  that connect visible history to hidden history remain as boundary rows.
- Boundary rows retain graph styling but use terminal-default colors, are dimmed,
  and cannot be selected, paged to, copied, signature-verified, restored as a
  selection, or entered by Shift navigation.
- When hidden revisions are configured, references are hidden by default so
  metadata remains aligned.
- `v`, then `h`, toggles the full hidden projection. Toggling preserves the
  selected commit when it still exists and otherwise selects the newest
  selectable row.

### Row content and visual states

- A row contains graph lanes, a seven-character object ID, optional references,
  committer date, author and attribution information, markers, and title.
- The commit marker is blue when unsigned, orange when signed but unverified or
  being verified, green when verified, and bright red when verification fails.
- The current `HEAD` commit uses `@` instead of the normal commit disc and keeps
  the same signature and selection coloring. It remains visible when textual
  reference labels are hidden.
- The selected row uses `>` at the left. If the displayed worktree block is dirty,
  `D` is shown at the `HEAD` row instead; a separately selected row retains `>`.
- Selection inversion covers the left marker, graph, commit marker, and hash.
  Its graph background is derived from the commit-marker color. The selected
  row's right-hand tail and contextual information always have blank margins and
  never invert an adjacent character.
- A compared merge parent is cyan, including its commit marker, and its hash is
  inverted.
- Rows outside active Shift reachability are dimmed. When a changes block has
  focus, history is dimmed but its contextual selection information and main
  status line remain prominent.

### Metadata and attribution

- Mailmap resolution is enabled by default and is obtained from a non-isolated
  repository.
- Recognized attribution trailers are `Co-authored-by`, `Assisted-by`,
  `Reviewed-by`, `Acked-by`, `Tested-by`, and `Signed-off-by`.
- Every displayed `Assisted-by` value is classified as an agent. Agent names are
  bracketed and agent emails are never displayed.
- Attribution keys with identical displayed actor lists are grouped, for example
  `Co, A: [GPT 5.6]`.
- Actors whose email ends in `@users.noreply.github.com` are italicized.
- Full-actor mode shows author emails and attribution actors but hides the commit
  title. Classified agent emails remain hidden.
- A commit message containing `--- agent` or `<!-- agent -->` receives a bright
  purple `[A]` before its title.
- A commit with notes in the configured notes ref receives a matching `[N]`.
  Notes are loaded lazily for visible commits.

### Selection context

- When tree changes are displayed, non-zero insertion and deletion counts for
  the selected commit appear immediately before the right selection tail.
- When a selected commit is pointed to by local refs, display at most one
  deterministic relationship. Prefer a configured-upstream relation as
  `⇡ahead⇣behind`; otherwise, when hidden ancestry exists, show the visible-only
  count as `⇡N`.
- Relationship walks use the in-memory graph, stop once no further distinction
  can be made, and cache completed results. They must never reopen a repository
  merely because selection moved.

## Interaction

### Navigation and display controls

| Key | Behavior |
| --- | --- |
| `j`/Down, `k`/Up | Move one selectable row or changed path. |
| Mouse/trackpad vertical scroll | Move history by the coalesced scroll distance; move paths when a changes block is focused. |
| `h`/`l` | Pan history or the focused changes block horizontally. |
| `Ctrl-u`/`Ctrl-d` | Move half a page. |
| `Ctrl-b`/`Ctrl-f`, `PageUp`/`PageDown` | Move a page; scroll an overflowing commit message when applicable. |
| `g`/Home, `G`/End | Select the newest/top or oldest/bottom selectable item. |
| `[` | Toggle graph/metadata alignment. |
| `v` | Toggle the history-display key group. Pressing `v` again closes it. |
| `v d` | Toggle committer dates. |
| `v e` | Toggle full actors/emails and titles. |
| `v n` | Cycle all attribution, author only, and no names, skipping inert states. |
| `v t` | Toggle attribution trailers. |
| `v m` | Toggle mailmap resolution. |
| `v r` | Cycle all, normal, and no reference labels. |
| `v h` | Show or hide configured hidden ancestry. |
| `Shift-R` | Explicitly refresh the revision view and visible worktree status. |
| `y` | Copy the selected commit ID, or the selected raw path when a changes block is focused. |
| `Shift-y`/`Y` | Copy the selected author as `Name <email>`. |
| `s` | Verify signed, unverified commits currently visible on screen. |

The display group remains open for consecutive display changes and closes on
navigation or another recognized command. `[` and overlay controls remain direct
shortcuts.

### Held Shift ancestry mode

- On terminals that report modifier press and release events, pressing Shift in
  focused history anchors navigation at the selected commit. Releasing Shift
  restores ordinary navigation.
- `j` and `k` then visit only commits reachable through the selected rail; other
  rows are dimmed.
- A non-merge anchor follows all of its ancestry. A merge anchor initially uses
  its second parent, excluding first-parent ancestry except shared fork points.
- `h` and `l` cycle the merge anchor's parents instead of panning. The chosen
  parent is numbered beside the junction marker. Later merges on the chosen rail
  traverse all their parents normally.
- Reachability is computed only after traversal and lane computation complete.
  Shift is ignored while another changes block has focus or while the terminal
  itself is unfocused.

## Overlay views

Overlay views paint over history without changing metadata alignment. Selection
is bounded above the top-most changes block: moving down at that boundary scrolls
history so the selected row stays visible. The commit view reserves right-side
space first; changes blocks adapt within the remaining history width.

### Commit message

- `o` or `]` toggles the commit view on the right. It uses at most half the
  terminal and reserves 80 content columns when space permits.
- The panel has a minimally shaded background derived from the detected terminal
  background, with the default background as fallback.
- The title begins on the first content row and is bold. Body text follows, then
  each note with a bold purple `Notes` prefix, then aligned trailers.
- Overflow is page-scrollable and gets a distinct pane status line only when
  scrolling is possible.

### Tree and worktree changes

- Changes start enabled as `Tree + Worktree`. `c` cycles `Tree + Worktree` →
  `Tree` → hidden. Bare repositories omit the worktree mode.
- Each block has a top border carrying its compact summary. Tree summaries show
  the selected short hash; worktree summaries distinguish staged and unstaged
  counts. Kind totals, total files when non-redundant, and non-zero line totals
  are color-coded. An empty enabled worktree says `Worktree clean` in green.
- Tree paths preserve tree-diff order. Worktree paths show staged entries first in
  green and unstaged/untracked/conflicted entries second in bright red, sorted by
  raw path within each group.
- Path kinds are `A`, `M`, `D`, `R`, `C`, `T`, and `U`. The selected path is
  subtly inverted and appends its already-computed non-zero line counts.
- Blocks are side by side when both condensed titles fit, otherwise Worktree is
  stacked above Tree. A shared vertical divider joins side-by-side blocks. Blocks
  size to content but together use no more than half the terminal.
- If paths overflow, the final row reports the remaining line count and updates
  while scrolling. A single path is never replaced by overflow text.
- `Tab` cycles focus in visual order through visible changes blocks and history.
  Inactive blocks, including paths and borders, are dimmed. Only the focused
  block shows its distinct status line.
- `p` cycles the comparison parent while Tree has focus. Merge commits are
  compared to one parent at a time; root commits compare against an empty tree.
- Repeated history keys and vertical mouse bursts temporarily hide changes
  overlays. They return after 75 ms of navigation idle, with the same path
  selection and viewport where possible.
- Tree diff results, detached diff resources, and line counts use a bounded MRU
  while changes remain enabled. Worktree results are cached separately and
  invalidated by relevant filesystem events.
- Per-file line information is computed once in an `available_parallelism`
  worker pool that exists only while changes are enabled.

### Diffs

- `Enter` in history opens the whole selected commit against the active parent.
  `Enter` in a focused changes block opens only its selected path.
- A whole-commit diff starts with commit identity and a Git-style per-path
  diffstat in diff order, followed by parent/root, kind totals, and aggregate line
  totals. It then shows the internal patch and invokes any per-path external diff
  drivers.
- Diff preparation honors Git attributes, text conversion, binary detection,
  external diff commands, and the configured `core.pager` pipeline.
- Binary, submodule, conflicted, and otherwise unavailable file diffs do not
  launch an inappropriate pager; the changes status line reports the reason.
- The built-in viewer takes over the alternate screen and supports the same
  vertical and horizontal navigation keys. `Enter` advances from a whole-commit
  internal diff to external drivers; `q` or `Escape` returns to tix.
- External programs run with the terminal suspended and restored afterward.
  Broken-pipe writes are accepted. If a pager exits within 250 ms, its already
  displayed output is retained until a keypress so short output remains readable.

## Signature verification and rewording

### Signatures

- Presence of `gpgsig` or `gpgsig-sha256` marks a commit as signed but
  unverified; history loading does not validate signatures eagerly.
- The `s` hint appears only while the viewport has work to verify and disappears
  after success. Verification uses Git-compatible repository configuration.
- Failures show their count with a bright-red marker. Moving the history
  selection resets failed visible states to unverified so verification can be
  retried.

### Reword

- `r` is available only after history completion and only on the newest
  selectable row, where tix assumes the commit has no displayed descendants.
- The configured Git editor receives a document containing `Author`,
  `AuthorDate`, `Committer`, `CommitterDate`, `CommentChar`, and the complete
  message. Author identity and time are retained; committer time defaults to now.
- `CommentChar` is a non-empty single-line byte prefix, defaults to `;`, and is
  recognized only at column zero. Parsing removes those lines and applies
  Git-style whitespace cleanup.
- Missing `Assisted-by: GPT 5.6` and
  `Co-authored-by: GPT 5.6 <codex@openai.com>` trailers are offered as commented
  opt-ins. A case-insensitive existing trailer key suppresses its suggestion,
  regardless of value.
- An unchanged editor document is a no-op. Otherwise tix recreates the commit,
  signs it when commit-signing configuration is enabled, and atomically retargets
  mutable local refs that pointed directly at the old commit. Tags and
  remote-tracking refs remain unchanged; a detached `HEAD` is retargeted.
- Editor, signing, parsing, writing, or reference-update failures are shown in
  the main status line and do not leave a repository retained by the UI.

## Refresh, focus, and diagnostics

- Native reference watchers observe `HEAD`, loose and packed refs, and the direct
  or symbolic refs used by view and hide revspecs. Missing refs during an atomic
  update are transient; malformed or inaccessible refs remain errors.
- Ref changes that affect view or hidden tips trigger an incremental history
  refresh. Decoration-only changes avoid traversal. Filesystem-driven traversal
  changes select the newest selectable row; manual refresh and display toggles
  preserve selection when possible.
- The worktree watcher exists only while the combined worktree block is enabled.
  It observes the index and ignore-aware directories that Git status would walk,
  using non-recursive registrations so ignored build trees do not generate work.
- Access-only and incomplete `.lock` activity are ignored. Completed atomic
  renames, index/HEAD updates, relevant worktree paths, and backend rescan requests
  invalidate the appropriate cache.
- Worktree updates retain the history selection and restore changed-path
  selection by raw path and relative viewport position. They never select the
  newest commit merely because status changed.
- Event batches are bounded and coalesced. Worktree status waits 75 ms of quiet;
  reference transactions wait for their final update. Watchers retry after
  failure while still needed.
- Refresh status remains hidden for 500 ms so quick background work does not
  flicker the footer.
- A filesystem history refresh is presented immediately. New or replaced visible
  rows are bold for 180 ms, matched first by commit ID and then by tree ID so
  rewords retain visual identity. Removals, unrelated replacements, manual
  actions, hidden toggles, and worktree-only updates are immediate without
  emphasis. Input, focus changes, and resizing end emphasis immediately.
- While the terminal is unfocused, filesystem-attributed redraws replace footer
  separators with persistent orange discs. Focus restores normal separators.
- Filesystem responses receive correlated IDs in daily tracing logs, including
  semantic trigger, coalesced paths, phases, presentation count, elapsed time,
  and outcome. Logs use the platform application-log directory, retain seven
  days, and are best-effort.
- If a linked worktree disappears, tix lexically normalizes and enters the common
  repository, reopens it as bare, drops worktree state, keeps tree/history views
  live, and reports recovery in the status line. If recovery fails, terminal state
  is restored and the contextual error is returned.

## Resource and responsiveness invariants

- No `gix::Repository`, commit-graph, object platform, notes platform, or other
  repository-owning value may remain in idle application/event-loop state.
- View population opens a fresh non-isolated repository so mailmap, notes, diff
  drivers, pagers, signing, and other Git configuration are current. It starts
  without an object cache; bounded diff operations may enable one temporarily and
  disable it again before any navigation reuse. Detached display data is retained.
- One fill repository may be shared by commit, tree, worktree, and metadata loads
  during continuous key-repeat or mouse navigation. It is dropped after the
  75 ms idle boundary.
- Traversal and incremental refresh workers may use a bounded object cache and
  must drop their repository when finished. Lane, verification, and line-diff
  workers exist only for active work and do not form persistent pools.
- Redraw is reactive and capped at approximately 60 frames per second while
  streaming. Mouse events are drained and coalesced in bounded batches so input
  storms cannot starve the main loop.
- Main status remains readable regardless of pane focus. Errors are surfaced in
  the nearest relevant status line; diagnostics never replace user-visible
  errors.

## Regression coverage

- Unit tests cover navigation, projections, pane layout, status summaries,
  selection restoration, watcher classification, cached graph walks, diff
  preparation, signatures, rewording, and terminal rendering.
- Filesystem row emphasis uses `insta` snapshots containing every distinct frame;
  unchanged hold frames are omitted. Run `cargo insta test -p gix-tix -F sha1`
  and review with `cargo insta review`; never edit snapshots manually.
- Behavior changes to this specification require corresponding tests and an
  update to this document in the same semantic patch.
