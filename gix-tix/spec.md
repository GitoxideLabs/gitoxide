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
- `tix travel [--materialize-conflicts] REVSPEC` performs the same detached
  checkout, pending-rebase replay, stash handling, and pin reconciliation as
  TUI time travel. Travelling to the current `HEAD` is a no-op. A detached
  source may travel to a descendant without a pin, but travelling to an ancestor
  or unrelated commit requires an existing current-worktree pin at `HEAD` or a
  descendant. An attached source is preserved through the normal symbolic-pin
  rules. Replay conflicts change nothing unless explicitly materialized; an
  accepted conflict writes the checkout and unmerged index, then exits with an
  error so resolution cannot be mistaken for completion.
- `tix reword REVSPEC [-m MESSAGE ... | -f FILE]` applies the same signing,
  lazy-rebase, mutable-ref, and worktree-safety rules as the TUI. Without either
  message option it opens the standard Markdown reword document. Repeated
  `-m/--message` values form paragraphs; `-f/--file` reads the complete message
  from a file, or from standard input when given `-`. Explicit sources bypass
  the editor and do not add suggested trailers. An attached `HEAD` may reword
  itself without a pin. Every other target requires an existing current-worktree
  tix pin at that commit or a descendant;
  every such covering pin participates so retained forks are rewritten together.
  Eligibility is checked before the editor opens, and an unchanged document is
  an explicit no-op.
- `-h/--hide REVSPEC` excludes the revision and its reachable ancestry. The
  option may be repeated.
- `-w/--worktrees` adds every successfully resolved main and linked worktree
  `HEAD` to the visible traversal tips, in addition to implicit or explicit
  revisions. Hidden revisions still exclude matching ancestry.
- `--quit-on-finish` exits after traversal and lane computation, for measurement
  and non-interactive use.
- `tix rebase todo -h HIDDEN... [--onto REV] [TIP...]` writes a self-contained
  Markdown history-rebase plan to stdout. At least one hidden revision must
  resolve; visible tips default to `HEAD`, and an ambiguous derived fork point is
  an error. `--edit-and-apply` opens the same plan with Git's configured editor
  and applies it when the editor exits.
- `tix rebase apply [FILE]` applies such a plan from a file, or from standard
  input when `FILE` is omitted or `-`. Removing its state comment or emptying the
  document cancels successfully; malformed or unsupported state is an error.
- By default, a todo conflict changes nothing. Explicit
  `--materialize-conflicts [CONTINUE]` accepts the partial result, checks out the
  conflicting commit with an unmerged index, and writes a fresh editable
  continuation todo to `CONTINUE`, or stdout when `-` is used. A terminal stdout
  is refused. Materialization exits unsuccessfully so scripts cannot mistake the
  incomplete rebase for completion.
- Editor-launching commands honor Git's normal editor selection and
  `GIT_EDITOR` overrides it.
- Revisions must resolve and peel to commits. Invalid or non-commit visible
  revisions are errors. An unavailable hidden revision emits a warning and is
  ignored when another hidden revision resolves; if none resolve, startup fails.
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
  and can be selected, paged to, restored as a selection, copied, and inspected.
  They cannot be reworded, forgotten, or signature-verified. During review-base
  selection, only an eligible base boundary remains selectable among hidden rows.
  They may be used for time travel or as the parent of an independent fork commit.
  A boundary whose visible descendants contain no merge commit offers the
  history-rebase editor, including when those descendants fork into multiple
  linear stacks.
- If a boundary has exactly one leaf among its visible descendants, selecting it
  uses the boundary-to-leaf tree comparison for the changes block and selection
  diff-stat. Forks which merge back into one leaf qualify; multiple surviving
  leaves retain the boundary commit's ordinary parent diff. Enter opens the same
  complete branch diff, labelled `<base>..<leaf>`.
- Hidden revisions do not change the default reference display mode.
- `v`, then `h`, toggles the full hidden projection. Toggling preserves the
  selected commit when it still exists and otherwise selects the newest
  selectable row.
- When a hidden revspec names a local branch, its best common base with the
  visible tips permanently shows `⇣N` after the commit title when that branch has
  `N` commits not reachable from the view. The terminal edge pushes the marker
  left over a clipped title when necessary. A blank margin remains on each side.
  The cached history graph supplies the base and count; unrelated refs and
  zero-count relations add no marker. If multiple hidden branches share a base,
  the largest count is shown and its tip is retained as the update target; equal
  counts choose a deterministic object ID.

### Row content and visual states

- A row contains graph lanes, a seven-character object ID, optional references,
  committer date, author and attribution information, markers, and title.
- The commit marker is blue when unsigned, orange when signed but unverified or
  being verified, green when verified, and bright red when verification fails.
- The current `HEAD` commit uses `@` instead of the normal commit disc and keeps
  the same signature and selection coloring. It remains visible when textual
  reference labels are hidden, and textual `HEAD` is never rendered alongside it.
- At startup, the current worktree's `@` row becomes selected as soon as it is
  loaded, unless the user navigates first. Once the viewport is known, the row
  is centered with normal history-boundary clamping so surrounding commits are
  visible. If it has visible descendants, its unselected non-whitespace content
  is underlined and `@` is bold; a selected row keeps only bold `@`.
- Local branches checked out in other worktrees are displayed as `short-name@`
  in light blue instead of their plain branch decoration. Other detached
  worktrees use their checkout directory basename. The current worktree's
  symbolic branch is displayed as `@short-name` in the local-reference color;
  if its HEAD is detached, a branch that happens to point at the same commit
  remains an ordinary branch label. Identical labels are deduplicated.
- When reference labels are hidden, worktree labels are visible only on the
  selected row. Stale, malformed, unborn, and otherwise unreadable worktree
  entries are skipped without failing history loading.
- The selected row uses `>` at the left. If the displayed worktree block is dirty,
  `D` is shown at the `HEAD` row instead; a separately selected row retains `>`.
- Selection inversion covers the left marker, graph, commit marker, and hash.
  Its graph background is derived from the commit-marker color. The selected
  row's right-hand tail and contextual information always have blank margins and
  never invert an adjacent character.
- A compared merge parent is cyan, including its commit marker, and its hash is
  inverted.
- Rows outside active review-base reachability are dimmed. When a changes block has
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
| `?` | Toggle the information key group. Its actions remain direct shortcuts. |
| `[` | Toggle graph/metadata alignment. |
| `v` | Toggle the history-display key group. Pressing `v` again closes it. |
| `v d` | Toggle committer dates. |
| `v e` | Toggle full actors/emails and titles. |
| `v n` | Cycle all attribution, author only, and no names, skipping inert states. |
| `v t` | Toggle attribution trailers. |
| `v m` | Toggle mailmap resolution. |
| `v r` | Cycle all, normal, and no reference labels. |
| `v h` | Show or hide configured hidden ancestry. |
| `r` | Hide reference labels or restore the mode visible when they were hidden. |
| `m`/`]` | Toggle the commit-message view. |
| `c` | Cycle the tree/worktree changes display. |
| `Shift-R` | Explicitly refresh the revision view and visible worktree status. |
| `y` | Copy the selected commit ID, or the selected raw path when a changes block is focused. |
| `Shift-y`/`Y` | Copy the selected author as `Name <email>`. |
| `s` | Verify signed, unverified commits currently visible on screen. |
| `@` | Time-travel to the selected commit, or return through its tix pin. Terminals reporting the base key as `Shift-2` are also accepted. |

The display group remains open for consecutive display changes and closes on
navigation or another recognized command. The `?` group similarly remains open
for signature verification, alignment, message, and changes actions, but these
actions retain their direct shortcuts while the group is closed. While expanded,
the group visually contains every following status action through `<enter> diff`,
including pane switching and navigation; none of those hints is visible while
the group is closed, and quit remains outside it. The footer underlines the `v`
in `view`; an open prefix reverses its complete expanded group for a strong,
terminal-theme-independent cue. While active, `view (` contains
every applicable display option and a closing `)` so direct shortcuts remain visibly
outside the prefix. The history status starts with the history position, then the
`v` prefix and the `e` prefix when it is addressable. Remaining history-level
actions end at the information prefix while it is closed. An available direct
time-travel action follows the edit group, and copy follows time travel; the reference toggle immediately precedes
the `?` group; quit is always last.
All status lines embed and underline a shortcut character in its action label when
possible; keys that cannot be expressed naturally in the label remain explicit.
The Enter key is written as `<enter>` throughout.

### Time-travel

- On a completed, focused history in a worktree repository, `@` on a non-`HEAD`
  row runs `git checkout --detach <commit>` without forcing local changes.
- Before every move, tix provisionally retains the previous `HEAD` with a
  `refs/worktree/tix/pins/<suffix>` ref. Git stores these refs privately for the
  current worktree. After a successful checkout it removes that pin
  when the old `HEAD` remains reachable from another view tip, and retains it
  otherwise. Pins use at least four alphanumeric characters;
  generated pins start with eight hexadecimal characters from the saved commit.
- A pin is symbolic when the previous `HEAD` named a local branch, so later branch
  advances move the pinned tip. An already detached `HEAD` receives a direct pin.
- When a rebase rewrites a detached departure, checkout applies the rebase mapping
  before deciding whether to pin it. A departure rewritten into the selected `@`
  successor is not pinned; a distinct departure preserves its rewritten identity.
- While `HEAD` is detached, every valid pin from the current worktree augments
  implicit and explicit revision tips. While it is attached, only pins at strict
  descendants of `HEAD` do so, preserving rewritten leaves after a history rebase
  moves the checked-out branch down its stack. Pins from other worktrees,
  dangling, malformed, and non-commit pins do not
  enter the view or its decorations. Normal hidden-revision exclusions still apply.
- One or more worktree pins at a commit are shown as a single blue `📌`
  resource marker immediately after the hash and outside ordinary reference
  decorations. It remains visible when references are hidden, and internal pin
  names are omitted from history rows. `@` on a pinned
  tip checks out its underlying branch, or its direct commit in detached mode,
  then removes that one pin. Multiple matching pins prefer symbolic targets and
  then lexical ref-name order.
- The edit menu offers `unpin` on a pinned row. It atomically removes every pin
  for that commit in the current worktree and retains that row's selection.
- `tix pin <REVSPEC>...` resolves every argument before writing and deduplicates
  pin targets in argument order. A direct reference name creates or reuses a
  symbolic current-worktree pin so it follows later reference updates; derived
  revisions and object IDs remain fixed direct pins. Each unique target prints
  as `pin:<suffix> <short-id>`, and targets at the same commit remain distinct.
- Checkout failures retain the original `HEAD`, remove only a newly created
  source pin, and leave destination pins intact. Successful travel consumes a
  destination pin and applies the same source-pin reconciliation for ancestor,
  descendant, and sideways moves. Conflict acceptance, history-rebase checkout,
  and automatic fork travel use this same primitive. Successful travel preserves
  the selected row, refreshes history directly, and invalidates worktree status.
- Active review commits define review trees containing all of their descendants.
  Time travel within one review tree keeps ordinary checkout behavior and never
  creates or restores a stash. Crossing out of a dirty review tree saves tracked,
  staged, unstaged, and untracked state with Git under
  `refs/worktree/tix/review/stashes/N`; ignored files remain untouched. Crossing
  into any commit in that review tree restores the state with `git stash apply
  --index` and always removes the companion ref after Git returns. Apply conflicts
  remain in the ordinary index/worktree conflict workflow. Nested trees use the
  nearest review-root ancestor.
- When loaded worktree status shows staged, unstaged, or untracked changes without
  conflicts, the edit menu offers `stas[h]` at the selected `@` entry. Missing or
  stale worktree status hides the action instead of performing another status
  query. Saving uses Git with `--include-untracked`, leaves ignored files alone,
  preserves the ordinary stash stack, and records the stash commit at
  `refs/tix/stash/<full-commit-id>`. A commit can retain only one such stash.
- A commit stash is shown as a bright `🎁` beside any `📌`, directly after the
  hash and outside reference visibility. Time travel back to that exact commit
  restores it with `git stash apply --index` and consumes its companion ref after
  Git returns, including when application leaves conflicts to resolve. Manual
  commit stashes use the same plumbing during reviews, while automatic review
  stashes retain their review-tree identity and namespace.
- At a selected `@` with a commit stash, the edit menu offers `unstas[h]` even
  when other worktree changes are present. It applies and consumes the stash in
  place through the same path used when time travel returns to that commit.
- Rewriting a commit atomically renames its commit-stash association alongside
  other reference updates. Dropping a stashed commit, converging multiple stashes
  onto one result, or overwriting an existing destination stash is rejected before
  prepared objects or references are persisted.

## Overlay views

Overlay views paint over history without changing metadata alignment. Selection
is bounded above the top-most changes block: moving down at that boundary scrolls
history so the selected row stays visible. The commit view reserves right-side
space first; changes blocks adapt within the remaining history width.

### Commit message

- `m` or `]` toggles the commit view on the right. It uses at most half the
  terminal and reserves 80 content columns when space permits.
- Its history-status action says `message`, avoiding confusion with the edit
  group's commit-creation action.
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
- Repeated history keys, including printable `j`/`k` reported through enhanced
  keyboard input, and vertical mouse bursts temporarily hide changes
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
  diffstat in diff order. Each textual path retains Git's churn count and bar,
  followed by an aligned signed net `additions - deletions` count. Parent/root,
  kind totals, and aggregate line totals follow before the internal patch and any
  per-path external diff drivers.
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

## Signature verification and editing

### Signatures

- Presence of `gpgsig` or `gpgsig-sha256` marks a commit as signed but
  unverified; history loading does not validate signatures eagerly.
- The `s` hint appears only while the viewport has work to verify and disappears
  after success. Verification uses Git-compatible repository configuration.
- Failures show their count with a bright-red marker. Moving the history
  selection resets failed visible states to unverified so verification can be
  retried.

### Reword

- `e`, then `r`, is available after history completion when no known descendant
  of the selected commit is a merge commit.
- The configured Git editor receives a document containing `Author`,
  `AuthorDate`, `Committer`, `CommitterDate`, `CommentChar`, and the complete
  message in a temporary `.md` file for syntax highlighting. Author identity and
  time are retained; the committer fields show the repository's configured
  current committer.
- `CommentChar` is a non-empty single-line byte prefix, defaults to `;`, and is
  recognized only at column zero. Parsing removes those lines and applies
  Git-style whitespace cleanup.
- Missing `Assisted-by: GPT 5.6` and
  `Co-authored-by: GPT 5.6 <codex@openai.com>` trailers are offered as commented
  opt-ins. A case-insensitive existing trailer key suppresses its suggestion,
  regardless of value.
- An unchanged editor document is a no-op. Otherwise tix recreates the commit,
  signs it when commit-signing configuration is enabled, and rewrites every
  linear descendant with unchanged trees and corrected parentage. Descendants
  whose parent changed retain that original parent for cherry-pick replay during
  time travel; the edited commit itself needs no replay marker.
  Mutable refs follow every rewritten commit; tags and remote-tracking refs remain
  unchanged.
- Every commit object actually rewritten by an edit receives the repository's
  current committer identity and date immediately before signing and writing.
  Edited committer fields cannot override it; untouched commit objects retain
  their existing identity and object ID.
- Command-line message inputs replace only the message, retain
  editor-comment-looking lines as content, and are a no-op when their cleaned
  message already matches the commit.
- Editor, signing, parsing, writing, or reference-update failures are shown in
  the main status line and do not leave a repository retained by the UI.

### New commits

- `e n` creates a child of the selected commit from tracked changes, or a root
  commit for an unborn `HEAD`. A changed index wins; otherwise, tracked worktree
  changes are used. Untracked files never enter an implicit new commit and remain
  untracked. It is available only with a live worktree, after history completion,
  and when the selected parent has no known merge descendant.
- `e m` creates an explicit empty commit which reuses the selected parent's tree,
  or the empty tree for an unborn history. Existing index and worktree state is
  preserved exactly. Both forms reject unresolved index conflicts.
- A current worktree-changes cache controls which actions are advertised without
  opening a repository: tracked changes offer both `new` and `new-empty`, while a
  clean or untracked-only worktree offers only `new-empty`. If no current cache is
  available, both are shown and `new` validates its candidate before opening the
  editor, directing an empty candidate to `new-empty`.
- Before launching the editor, tix resolves identities, signing configuration,
  index conflicts, filters,
  candidate tree, per-path diffstat, and a provisional commit entirely through an
  in-memory object database. Cancellation and preflight failure write no object,
  reference, index, or worktree state.
- A changed index supplies the complete commit tree and wins over unstaged
  changes. Otherwise, when the worktree `HEAD` is the selected parent, tracked
  worktree changes are filtered into a tree. A normal `new` rejects a tree equal
  to its parent; `new-empty` deliberately reuses it.
- The Markdown editor buffer contains editable identities and dates, a `what`
  title, a `why` body, optional attribution trailers, and a commented Git-style
  per-path diffstat with signed net line counts. Commit hooks are not run.
- After editing, tix revalidates the destination, applies configured signing,
  marks linear descendants for lazy replay, persists the prepared objects, and atomically
  advances mutable refs throughout the rewritten stack. This includes local
  branches, custom refs, direct tix pins, and a detached `HEAD`, while excluding
  tags and remote-tracking refs. Checked-out affected worktrees are preflighted;
  inaccessible or conflicting affected worktrees abort safely.

### Fork commits

- `e f` creates an independent child of any selected commit, including a hidden
  boundary or merge commit. It requires completed history and a live,
  conflict-free worktree, but unlike `e n` it is not restricted by descendants
  because it rewrites none of them. It is unavailable for unborn history.
- Fork preparation reuses the new-commit editor, candidate-tree, identity, and
  signing rules. Saving writes only the new commit and a temporary direct
  `refs/worktree/tix/pins/*` ref; existing refs, descendants, indexes, and
  worktrees do not move during creation.
- Tix immediately time-travels to the new fork. A successful checkout consumes
  its temporary pin and reconciles the departed `HEAD` through the standard pin
  primitive. If checkout fails, the fork remains pinned and visible.

### Amend, spill, and split

- `e a` amends the current worktree's `@` commit with the changed index, or
  worktree changes when the index already matches `HEAD`. `e s` spills that
  commit's tree delta into the worktree by replacing its tree with its first
  parent's tree, or the empty tree for a root commit. Clean operations are
  unavailable and report a no-op through `tix amend|spill`.
- Command-line edits use the same default HEAD, applicable pin, and review tips
  as the history view. Unrelated refs do not broaden their descendant rewrite
  scope, while mutable refs pointing into that scope are still retargeted.
- With a path selected in the focused tree-changes block, the main `e` prefix
  offers `spill` and `e s` spills only that path against the displayed parent.
  The CLI intentionally supports only whole-commit spilling.
- With a path selected in the focused worktree-changes block, the main `e`
  prefix offers `amend` and `e a` amends only that path. A staged row uses its
  index version; an unstaged row uses its filtered worktree version. If both
  rows exist for one path, the selected row determines the version. Review
  commits accept only staged rows, and unresolved indexes cannot be amended.
  Unrelated staged entries retain their index state. The CLI intentionally
  supports only whole-commit amending.
- `e p` is offered at `@` only when both staged and unstaged changes exist. It
  amends the unstaged changes into the source commit, then creates a new upper
  commit from the staged delta using the standard Markdown editor buffer. Both
  deltas are three-way applied in memory before the editor opens, so overlapping
  changes abort without writing objects or changing refs, the index, or files.
- `tix split` performs the same split at `HEAD`: worktree changes are amended
  into the source commit and staged index changes become the new commit on top.
- A successful split leaves the worktree bytes untouched and resets the index to
  the new upper commit. The rewritten source retains its message and ancestry;
  the upper commit receives the edited message. Their final trees and ancestry
  need no replay marker; rewritten descendants use the same lazy rebase as amend
  and spill.
- All three operations leave worktree files untouched and cheaply rewrite linear
  descendants with their trees unchanged. Whole-commit edits reset the affected
  worktree's index to the rewritten commit; selected-path amend synchronizes only
  its destination and renamed source. A directly amended or spilled commit already
  has its final tree and unchanged parent, so an empty signature field alone keeps
  a formerly signed commit pending. Reparented descendants additionally carry
  `tix-rebase-parent`, retaining the original parent needed for later replay. Both
  pending forms use a bright-cyan commit marker.
- Edit graph discovery follows refs that point to commits and ignores refs whose
  targets are trees, blobs, or other non-commit objects.
- Time travel toward a pending destination cherry-picks and signs only the pending
  ancestry through that destination. Every later descendant is reparented and
  becomes or remains lazy and unsigned, including ordinary commits created above
  pending history; traveling toward a non-pending ancestor leaves the entire
  pending region untouched. A conflict retains the ours tree, exact merge-result
  tree, conflict stages, prepared commits, and in-memory objects without changing
  the repository. The conflicting row shows a blinking red `C`; `<enter>` persists
  the prepared rebase, leaves later descendants lazy, checks out the conflicting
  commit at the ours tree, then checks out the merge result and derives the
  unmerged index from it. Any other key discards
  the suspended operation; key-release events are not actions and leave it armed.
  Diagnostics warn when a conflict suspends the rebase and record whether it is
  accepted, discarded, or fails during checkout.
- A checked-out unresolved index keeps `C` at `@`, overrides dirty `D`, and
  disables time travel until all conflict stages are resolved. The worktree
  changes block is shown for resolution.

### Reviews

- `e v` starts a review from any non-boundary commit without merge descendants.
  It limits navigation to the selected commit's ancestry; the connected hidden
  base remains selectable, `<enter>` confirms it, and Escape cancels before any
  repository change.
- Starting requires a completely clean index and worktree, including no untracked
  files, and non-pending reviewed-tip and base commits. Only after confirmation,
  tix creates the first unused `refs/worktree/tix/review/N` ref at the reviewed
  tip and an unsigned ordinary `review` commit at the base with
  `tix-rebase: onto refs/worktree/tix/review/N`. HEAD is detached at that commit,
  its base tree fills the index, and the reviewed tip tree remains in the worktree
  as unstaged changes. If attached HEAD pointed directly to the reviewed tip when
  starting, the review ref symbolically targets that ref instead. Finishing then
  reattaches HEAD after confirming the ref moved to the finished review commit;
  otherwise the review ref remains a direct anchor and HEAD stays detached.
- Review refs are always traversal tips and remain visible in every ref mode. One
  active ref is shown as `review`; multiple refs are shown as `review:N`. Review
  commits replace the signature disc with a filled diamond while retaining independent
  signature state. Ordinary edits preserve the review header and otherwise keep
  their normal signing and lazy-rebase behavior.
- At a checked-out review commit, amend is offered only for staged changes and
  consumes only the index tree. It leaves worktree bytes and the review header
  intact, removes signatures, and marks only affected descendants for lazy replay.
- `e v` finishes a checked-out review only when status is completely clean. The
  review commit is inserted after its reviewed tip with its exact tree, review
  header removed, updated committer, and configured signature. Review-side
  descendants retain exact trees and are signed without pending markers. With one
  review-side leaf, the reviewed tip's prior descendants are lazily reparented
  after it; with multiple leaves they branch directly after the finished review.
  The review ref is deleted in the same atomic ref/worktree transaction.
- Forget is unavailable for a review commit with descendants. Forgetting a review
  leaf, finishing a review, or dropping a review commit through a rebase todo also
  deletes its review ref and optional saved-worktree ref atomically; reordering or
  rewriting it preserves the header and resources. Review stash refs are internal:
  they are neither traversal tips nor decorations.

### Forget commits

- `e`, then `d`, is available after history completion for a selected non-merge
  commit with no known merge descendant. The first `d` arms a
  `d again forget` confirmation; the second performs it. Navigation, refresh,
  cancellation, selection changes, and other commands disarm confirmation.
- Forgetting does not require a worktree. Linear descendants are reparented with
  unchanged trees and marked for lazy replay; mutable refs throughout the
  rewritten stack move atomically. Tags and remote-tracking refs remain unchanged.
- When the selected commit is the current worktree `HEAD`, Git preflights and
  applies a two-tree index/worktree transition which discards only that commit's
  tracked delta. Conflicting staged, tracked, or untracked state refuses the
  operation; unrelated untracked content survives. When `HEAD` is unrelated, only
  refs move and the worktree is untouched.
- Forgetting an attached root deletes the branch and leaves symbolic `HEAD`
  unborn. A selected detached root is rejected because it cannot produce a valid
  unborn `HEAD`. Success refreshes history and selects the parent when present.

### Transactional rebases

- All edits share one in-memory rebase primitive.
  Forks are preserved, descendant merges are rejected, and all commit/tree
  preparation—including cherry-pick conflict detection—finishes before objects
  become reachable through refs.
- `Tree::LeaveAsIs` rewrites parentage without changing trees;
  `LeaveAsIsAndMark` writes the original first parent to `tix-rebase-parent` only
  when later replay needs it; and `CherryPick` transplants each tree delta. User
  edits use `LeaveAsIsAndMark`; time travel is the only eager `CherryPick` caller.
  A successful repeated rebase clears the marker through its checkout destination.
  On conflict, `tix-rebase-parent` identifies the original base and later descendants
  remain marked instead of being cherry-picked.
- `Signature::RedoIfNeeded` signs every rewritten commit when signing is
  configured and otherwise removes stale signature headers.
  `InvalidateExisting` empties existing signature values when signing is
  configured, making the empty field a pending-signature signal, or removes them
  when it is not. A pending-rebase commit can only use the invalidation policy,
  so it never carries a usable signature. Automatically rebased descendants
  retain their author and receive one configured current committer identity and
  timestamp for the operation.
- Ordinary edits retarget mutable local refs pointing into the rewritten set.
  History todos instead use their explicit reference lines. Ref changes use
  compare-and-swap transactions; a checkout failure rolls back already-applied
  worktree transitions and the ref transaction, except that deleting the branch
  being departed necessarily follows the successful checkout. Newly written
  unreachable objects may remain for normal Git garbage collection.
- A suspended conflict temporarily owns a cloned repository with object memory
  while awaiting one confirmation key. Dropping it writes nothing; accepting it
  consumes the repository immediately after persisting the commit at the ours
  tree and materializing the retained merge result in the worktree and index.

### History rebase editor

- Selecting an eligible hidden boundary and pressing `e b` opens a Markdown
  `.md` todo. It grows upward like the history view: each `## fork <id>` section
  lists its commits oldest-to-newest as `` `pick <short-id>` <displayed metadata> ``.
  IDs are shortened through repository configuration; metadata repeats the
  information visible in history and always includes the subject. Base-level
  stacks begin with `## fork <id> (base) <title>`, using the Markdown-escaped
  title exactly as displayed in history. Fork points within the editable tree
  remain plain `## fork <id>` headings.
- When that boundary shows `⇣N`, `e u` opens the same editor with each base-level
  stack rooted at the corresponding hidden branch tip. Its otherwise unfamiliar
  heading is `## fork <id> (updated-base) <title>`, with the Markdown-escaped
  title exactly as shown in history, including `[A]` and `[N]`. The hidden branch
  itself is not moved.
- Pick lines may be reordered or removed. `squash <id>` folds an existing
  non-merge commit into the preceding `pick` or `empty` in the same fork; it may
  carry `@`, and fork headings naming any folded ID resolve to the combined
  result. A fork cannot begin with `squash`. Fork headings may otherwise target
  an earlier pick or any existing commit, so adding and removing headings creates
  and joins branches. `empty <title>` inserts an empty commit. Markdown code
  spans and equivalent plain commands are accepted; display text after an ID is
  informational.
- Squash groups are materialized eagerly on every fork by applying their source
  deltas in todo order. The result retains the first member's author, author
  time, encoding, extra headers, and message, receives the operation's committer,
  and is signed once. Before every later full message, a permanent
  `# <short-id> <subject>` line identifies its source. Distinct raw authors of
  later commits are appended in first-seen order as `Co-authored-by` trailers,
  excluding the first author and identities already named by a valid such
  trailer in any source message. Name and email pairs are compared without
  mailmap. All folded IDs and mutable refs map to the one resulting commit;
  resources owned by a later folded review commit are removed.
- The first line points to complete self-documenting help after the editable
  todo. All instructions are enclosed in Markdown comments so only headings and
  command lines participate in the editable plan.
- A versioned Markdown state comment makes the document independently
  applicable in a later process. It records full base, target, scope and tip IDs,
  checkout requirements, and compare-and-swap state for mutable refs. Ref names
  use Git-compatible C-style quoting so arbitrary ref bytes round-trip. Missing
  state cancels; present invalid state never reaches repository mutation. The
  state comment follows the complete help at the end of the document.
- Standalone `(ref, ref)` lines place direct mutable refs at the preceding fork
  heading or command result. Multiple consecutive lines share that destination.
  Commit command metadata omits ref decorations because these lines are their
  sole editable representation.
  Existing displayed names may be moved or removed, and new unqualified names
  create local branches; explicit editable `refs/...` names are also accepted.
  Short names follow the history display, ambiguous names expand to full names,
  and Git quoting preserves arbitrary bytes. Tags, remote-tracking refs, general
  symbolic refs, tix pins, and review resources remain hidden and unchanged.
- Pick lines use display-only state symbols documented in the footer: `↻` for a
  lazy rebase, `◌` for an invalidated signature awaiting signing, `◐` for an
  unverified signature, and `○` for an unsigned commit. Applicable states may be
  combined without changing plan semantics.
- `@pick`, `@squash`, or `@empty` chooses the post-rebase commit. A generated
  todo keeps this marker even when `HEAD` is attached, but shows its branch as an
  ordinary ref. Versioned state remembers that attachment while the branch stays
  at the marked result. Moving it elsewhere detaches `HEAD`; adding `@` to one
  local branch explicitly attaches it and is valid only at the marked result.
  Removing the name deletes the branch. Checkout markers are invalid without a
  worktree. Todo generation and application reject an unborn `HEAD`.
- Within the ancestry ending at `@`, unchanged picks whose original parent is
  still their planned parent retain their IDs. Eager cherry-picking and re-signing
  starts at the first pending or structurally changed commit. Any descendants
  above `@` and other resulting stacks retain their trees, receive pending-rebase
  markers, and invalidate old signatures for later time travel. With no `@`,
  ordinary steps remain lazy while squash groups are still materialized.
  Any conflict while applying a history todo first remains entirely in memory.
  The TUI projects the partial result, selects the actual conflicting result, and
  marks it with a blinking red `C`; predicted ref decorations remain at their
  repository positions. Repository-backed overlay content is hidden while these
  candidate objects exist only in memory. `<enter>` accepts the partial result,
  moves already-final refs, records the ours tree in the conflicting commit, and
  checks out the retained merge result with an unmerged index,
  and retains an in-memory continuation plan. Any other key discards the preview
  without writes.
  Once the index has no unresolved stages, `<enter>` continues; another conflict
  repeats the same choice.
- Command-line apply reports a conflict without changes unless
  `--materialize-conflicts` was explicitly supplied. Its continuation document
  uses the full null object ID for the command whose tree must come from the
  resolved index. Already produced commits use their new IDs, completed drops and
  squash sources disappear, unapplied squash sources remain, and the remaining
  todo stays editable. Applying it
  requires only that `HEAD` names a commit and the index has no unresolved stages;
  the index tree, including additional staged changes, becomes the resolved tree.
  There is no hidden sequencer state or separate continue/abort command.
- Interactive todo application runs on a scoped worker so the terminal can
  remain renderable without cloning the cached history graph. If application is
  still running after 300 ms, a modal gauge shows processed versus total todo
  source commits and live cherry-pick and signature counts with their cumulative
  durations. `pick`, `squash`, and `empty` commands each contribute to the total;
  squash sources advance separately while their combined commit is signed once.
  Fast operations and command-line `tix rebase apply` do not show progress.
- Displayed mutable refs follow their explicit locations in the edited todo;
  omission deletes them and newly named refs require nonexistence. Refs checked
  out by linked worktrees are displayed normally and may move, with their index
  and worktree updated through the same preflighted transition as other rebases,
  but may not be deleted. The current worktree's branch may be deleted only when
  the todo also moves or detaches `HEAD`; deletion is deferred until checkout
  succeeds. All remaining moves use one compare-and-swap transaction. Every
  other resulting leaf gets a direct
  `refs/worktree/tix/pins/*` ref, except the checked-out leaf. When `@` moves below
  a referenced leaf, the existing time-travel checkout detaches `HEAD` there while
  the ref stays at the leaf. Concurrent ref edits win by making the transaction
  fail; the editor result is not rebuilt against a later graph snapshot. Leaving
  the document unchanged is a no-op unless the ancestry ending at `@` contains
  pending commits or rebase-update selected a newer base; pending commits on
  other forks remain lazy and do not replay a clean checkout ancestry. Explicit
  `tix rebase apply` always
  applies a valid plan, even when its editable commands are unchanged. The first
  Markdown comment states which of these modes applies and explains that emptying
  the file or removing the `tix-rebase-state-v1` comment cancels. Continuation
  todos likewise state that saving unchanged continues the materialized rebase.

### Editing shortcuts

- `e` toggles the edit shortcut group. `e b` rebases an eligible hidden base and
  `e u` rebases it onto the newer hidden branch tip when available,
  `e r` rewords, `e n` creates a rebased child, `e f` forks an independent child,
  `e a` amends `@`, `e h` stashes changes at `@`, `e s` spills `@`, `e p` splits staged from unstaged changes,
  and `e d d` confirms forgetting a top commit when each action is available.
- `@` invokes time travel directly, outside the edit group. Invoking it leaves an
  already expanded edit group open.
- Edit shortcuts keep the group open. Navigation or another recognized command
  closes it, matching the `v` display shortcut group. Plain `r` does not mutate
  the repository, and plain `t` has no action.
- The footer underlines the `e` in `edit`; while active, `edit (` contains only the
  actions available for the current selection, followed by `)`. An empty group
  says `no actions`.
- While the `v` group is open, `d`, `e`, `r`, and `t` retain their display
  meanings for dates, emails, references, and trailers.

## Refresh, focus, and diagnostics

- Native reference watchers observe `HEAD`, loose and packed refs, linked-worktree
  HEAD and membership changes, and the direct or symbolic refs used by view and
  hide revspecs. Linked indexes, logs, locks, and unrelated metadata do not
  trigger history refreshes. Missing refs during an atomic update are transient;
  malformed or inaccessible ordinary refs remain errors.
- Ref changes that affect view or hidden tips trigger an incremental history
  refresh. Decoration-only changes avoid traversal. Filesystem-driven traversal
  changes, manual refresh, and display toggles preserve selection by commit ID.
  Edits retain the selection on the successor ID returned by the rewrite. A
  selected worktree HEAD or other moving reference follows its changed target,
  covering external branch and StGit patch rewrites. If none remains visible,
  selection falls back to the first selectable row.
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
- After every event-loop wait, tix assumes that the original worktree and process
  working directory may have disappeared. Before processing filesystem events or
  redrawing, it lexically normalizes and enters the common repository, reopens it
  as bare, drops worktree state, keeps tree/history views live, and reports recovery
  in the status line. If recovery fails, terminal state is restored and the
  contextual error is returned.

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
- Global command and recovery feedback uses one transient message channel. A
  message replaces the main status line until the next recognized user action;
  pane-specific errors remain in their pane status line.
- Closing the new-commit editor without changing its prepared buffer leaves the
  repository untouched and reports `no commit created: no input was provided`.

## Regression coverage

- Unit tests cover navigation, projections, pane layout, status summaries,
  selection restoration, watcher classification, cached graph walks, diff
  preparation, signatures, rewording, and terminal rendering.
- Filesystem row emphasis uses `insta` snapshots containing every distinct frame;
  unchanged hold frames are omitted. Run `cargo insta test -p gix-tix -F sha1`
  and review with `cargo insta review`; never edit snapshots manually.
- Behavior changes to this specification require corresponding tests and an
  update to this document in the same semantic patch.
