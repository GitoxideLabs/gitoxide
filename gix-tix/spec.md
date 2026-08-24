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
- `tix show [-x HIDDEN...] [--no-auto-hide] [TIP...]`, also available through
  the visible `tix status` alias, prints the complete
  history view without opening a terminal UI. Tips default to `HEAD`, and
  applicable pins participate exactly as they do in the history view. Output
  uses the history view's graph lanes and default metadata, without colors,
  selection, clipping, or a footer. Each visible root replaces its ordinary row
  with a centered `──── base <metadata> ────` separator; distinct roots therefore
  delineate their trees while retaining the commit's markers and metadata. Each
  seven-character commit hash is followed by its seven-character reverse-hex
  change ID. Colliding or duplicated prefixes remain visible and receive a `💥`
  gutter marker.
- `tix travel [--materialize-conflicts] (REVSPEC | --to first|parent|child|tip)`
  performs the same detached checkout, pending-rebase replay, stash handling,
  and pin reconciliation as TUI time travel. Its target may also be an
  unambiguous reverse-hex change-ID prefix from the default Tix view. `parent`
  and `child` move one edge from `HEAD`; `first` selects its oldest reachable
  root and `tip` its reachable leaf, considering only commits visible in the
  default view. Multiple direct or terminal candidates are reported with their
  commit and change IDs and must be selected with a direct `tix travel REVSPEC`.
  Travelling to the current `HEAD` is a no-op.
  A detached source may travel to a descendant without a pin, but travelling to
  an ancestor or unrelated commit requires an existing current-worktree pin at
  `HEAD` or a descendant. An attached source is preserved through the singleton
  HEAD-pin rules. Replay conflicts change nothing unless explicitly
  materialized; an accepted conflict writes the checkout and unmerged index,
  then exits with an error so resolution cannot be mistaken for completion.
- `tix stash` saves the index and worktree state in a gix stash associated with
  the `HEAD` commit through the same commit-stash operation as the TUI.
- `tix copy-insert [--materialize-conflicts [CONTINUE]] C I` exposes the TUI
  copy-insert action without opening it.
  Both operands accept Git revisions or unambiguous reverse-hex change-ID
  prefixes from the default Tix view; `C` is the commit to copy and `I` is the
  commit above which the copy is inserted. A conflict aborts without changing
  repository state unless explicitly materialized into the ordinary rebase
  continuation workflow.
- `tix admin clear-undo` atomically and idempotently deletes the current
  worktree's undo and redo queue. It does not apply or reverse queued operations,
  change their recorded references, or affect another worktree's queue.
- `tix enrich commit todo [--clear] [REVSPEC]`, `tix enrich commit note
  [REVSPEC]`, `tix enrich commit git-note [REVSPEC]`, and `tix enrich tree
  checks-pass [--clear] [REVSPEC]` expose the TUI's enrichment actions without
  opening it. Targets default to `HEAD` and accept Git revisions or unambiguous
  reverse-hex change-ID prefixes from the default Tix view. Boolean commands
  idempotently set their marker, or clear it with `--clear`; note commands use
  Git's editor, remove empty notes, and leave unchanged notes alone. Output
  starts with the target's abbreviated commit and change IDs before its status.
- `tix new [--index | --worktree | --worktree-untracked] [--allow-empty] [--todo]
  [--author "Name <email>"] [-m MESSAGE ... | -f FILE]` creates a child of `HEAD`, or a root commit for
  unborn `HEAD`, with the same signing, editor, enrichment, lazy-rebase, and
  worktree-safety rules as `a w`. By default a changed index wins and tracked
  worktree changes are used only when the index is unchanged. `--index` uses
  only the index delta; `--worktree` applies only unstaged tracked-worktree
  changes to the `HEAD` tree and omits staged-only changes. `--worktree-untracked`
  additionally includes non-ignored untracked files. An unchanged selected tree
  is rejected unless `--allow-empty` is given. Message files,
  repeated messages, standard input, editor bypass, and `--author` follow
  `tix reword`; the author uses the prepared new-commit date. `--todo` enables
  the new commit's editable Todo header.
- `tix reword REVSPEC [--author "Name <email>"] [-m MESSAGE ... | -f FILE]`
  applies the same signing,
  lazy-rebase, mutable-ref, and worktree-safety rules as the TUI. Without either
  message option it opens the standard Markdown reword document. Repeated
  `-m/--message` values form paragraphs; `-f/--file` reads the complete message
  from a file, or from standard input when given `-`. Explicit sources bypass
  the editor and do not add suggested trailers. `--author` replaces the author
  actor while preserving its date. Without an explicit message it prefills the
  normal editor document; with one it is applied non-interactively. An attached
  `HEAD` may reword itself without a pin. Every other target requires an existing
  current-worktree tix pin at that commit or a descendant. As with `tix travel`,
  an unambiguous default-view change-ID prefix may replace the Git revspec;
  every such covering pin participates so retained forks are rewritten together.
  Eligibility is checked before the editor opens, and an unchanged document is
  an explicit no-op. Editor documents also contain commented `Todo` and
  `Message:` enrichment headers. An uncommented bare `Todo` enables the flag;
  commenting or deleting it disables the flag. `Message:` accepts one title
  line. Editing that title preserves an existing message body byte-for-byte,
  while commenting, deleting, or emptying the header removes the whole message.
  Explicit `-m` and `-f` messages preserve enrichments.
- Primary command output follows every displayed abbreviated commit hash with
  its reverse-hex change ID. The two abbreviations have equal widths. This
  applies to mutation results, rewritten-ref mappings, pins, stash and travel
  notices, raw commit labels in `ref-tree`, and visible commit identifiers in
  rebase todos. Diagnostics on stderr and the full object IDs in the hidden
  `tix-rebase-state-v2` block remain unchanged.
- `-x/--hide REVSPEC` excludes the revision and its reachable ancestry. The
  option may be repeated.
- `-h/--help` prints Clap's standard help for `tix` and every subcommand.
- `--quit-on-finish[=INPUTS]` exits after traversal, lane computation, and one
  completed frame, for measurement and non-interactive inspection. Optional
  characters are replayed as read-only keyboard input before the retained final
  frame, allowing navigation such as `--quit-on-finish=jjjl`. Inputs that would
  mutate the repository, launch another program, or copy data are ignored. The
  frame is drawn on the normal screen and remains visible after exit.
- `--no-alt-screen` runs the interactive UI in a full-height inline viewport on
  the normal screen, retaining its frame and panic output for diagnostics. Input
  handling otherwise matches the default interactive mode.
- `tix rebase todo [-x HIDDEN...] [--no-auto-hide]
  [--onto REV | --update-base] [TIP...]`
  writes a self-contained Markdown history-rebase plan to stdout. Visible tips
  default to `HEAD`, and an ambiguous derived fork point is an error. With
  `--update-base`, the uniquely derived fork point is rebased onto the same newer
  hidden local branch tip offered by TUI `rebase-update`; absence of such a tip
  is an error. The resulting `(updated-base)` plan remains actionable when saved
  unchanged. `--update-base` and an explicit `--onto` are mutually exclusive.
  `--edit-and-apply` opens the same plan with Git's configured editor and applies
  it when the editor exits. It also accepts `--materialize-conflicts [CONTINUE]`
  to opt into the same conflict checkout and continuation-document workflow as
  `tix rebase apply`; the option requires `--edit-and-apply`.
- `tix show`, `tix ref-tree`, and `tix rebase todo` automatically inspect symbolic
  `refs/remotes/<remote>/HEAD` references. Their targets are reverse-mapped
  through each remote's fetch refspec, and existing local commit branches are
  added to the explicit hidden revisions. Multiple remote defaults are
  deduplicated; stale, direct, ambiguous, unmappable, missing, and non-commit
  results are ignored. At least one explicit or inferred hidden revision is
  required by commands that need a hidden boundary. `--no-auto-hide` disables
  inference. The interactive history retains its explicit-only behavior.
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
- The interactive UI owns the alternate screen by default. `--no-alt-screen`
  instead draws interactively on the normal screen. Raw mode, focus reporting,
  mouse capture, and enhanced keyboard reporting are restored on every exit path.
  Shutdown leaves the alternate screen without clearing it or writing afterward.
  `--quit-on-finish` draws without input reporting on the normal screen.
- `Ctrl-C` exits immediately from any normal tix focus without recovery
  bookkeeping. `q` always quits from history, including while a conflict or
  rebase continuation is suspended. Before that normal exit, tix journals
  already-materialized reference progress and drops only in-memory candidates;
  it never rolls repository state back. `q` or `Escape` in a focused changes
  block still returns focus to history.

## History model

### Traversal and projection

- Traversal streams commits before graph-lane computation finishes. The footer
  reports the number received while loading and switches to the selected row
  number after completion. Every visible root starts at `#0`; descendants use
  their on-screen row distance from that root. A merge reachable from multiple
  visible roots uses the visually closest root, so only one count is shown.
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
  author date by default, author and attribution information, markers, and title.
  Simple lane turns use rounded corners in both the TUI and `tix show`; merge
  tees and crossings remain orthogonal so every commit still occupies one row.
- The commit marker is blue when unsigned, orange when signed but unverified or
  being verified, green when verified, and bright red when verification fails.
- The current `HEAD` commit, including a review commit, uses `@` instead of the
  normal commit disc. The marker is italic when HEAD is attached to a branch and
  keeps the same signature and selection coloring. It remains visible when textual
  reference labels are hidden, and textual `HEAD` is never rendered alongside it.
- At startup, the current worktree's `@` row becomes selected as soon as it is
  loaded, unless the user navigates first. Once the viewport is known, the row
  is centered with normal history-boundary clamping so surrounding commits are
  visible. While the row is unselected, its title is shown in reverse video and
  `@` is bold; the selected row's normal inversion replaces that title emphasis
  while keeping `@` bold.
- Local branches checked out in other worktrees are displayed as `short-name@`
  in light blue instead of their plain branch decoration. The current worktree's
  symbolic branch is displayed as `@short-name` in the local-reference color.
  A detached foreign worktree is shown as `directory@` at its actual `HEAD`,
  without a pin marker. Its symbolic HEAD pin is shown separately as `★branch`
  at that branch's actual tip. The worktree administration name is used when no
  directory basename is available. A detached current worktree is identified by
  the graph `@` alone and has no redundant `@directory` label. Identical labels
  are deduplicated.
- An unselected commit checked out by any foreign worktree gives only its title
  a dark-gray background, whether that worktree is attached or detached and even
  when reference labels are hidden. The current worktree's reverse-video title
  takes precedence when both point to the same commit; selection clears either
  title emphasis.
- When reference labels are hidden, worktree labels are visible only on the
  selected row. Stale, malformed, unborn, and otherwise unreadable worktree
  entries are skipped without failing history loading.
- The selected row uses `>` at the left. If the displayed worktree block is dirty,
  `🫟` is shown at the `HEAD` row instead; a separately selected row retains `>`.
- A selected row at the current worktree is inverted from its left edge through
  the final displayed title character. Any other selected row is inverted through
  its non-title metadata, leaving the final space and title uninverted. The graph
  background is derived from the commit-marker color. The selected row's
  right-hand tail and contextual information remain separate, have blank margins,
  and never invert an adjacent character.
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
- `n g` edits the selected commit's note in `core.notesRef`, or
  `refs/notes/commits` when it is unset. The action is available on every
  selected commit, including immutable hidden boundaries. Empty content removes
  the note and unchanged content is a no-op.
- Every operation that actually rewrites a commit copies its default-ref Git
  note to the successor while retaining the predecessor note. Notes that converge
  through a squash are concatenated in source order with a blank line. A split
  copies the source note only to its rewritten lower identity; inserted and
  dropped commits do not propagate notes. The notes ref changes atomically with
  the other rebase refs and participates in rollback.
- Tix enrichments are stored separately as Git notes headed at the worktree-local
  `refs/worktree/tix/enrich` ref. Enrichments are keyed by the commit's effective
  change ID and use human-readable Git config. Independent `[commit]` keys store
  `todo = true` and an optional multiline `note` value.
  Consequently, rewritten commits retain their metadata and commits sharing a
  change ID share it as well. Malformed enrichments are ignored for display and
  diagnosed, while mutation refuses to overwrite them.
- Tree enrichments use the same human-readable Git config format in notes headed
  at `refs/worktree/tix/enrich-tree`. They are keyed directly by tree object ID;
  `[tree] checks-pass = true` therefore applies to every commit with that exact
  tree and naturally disappears when a rewrite changes the tree.
- Todo, note, and checks-pass enrichments receive a leading `🚧`, `📝`, and `✔️`
  respectively before
  the graph, with no gap between them or the following status field. The dedicated
  field remains visible alongside selection, dirty-worktree, and conflict markers.
  `tix show` emits the same field and aligns unmarked rows when any displayed
  commit has either enrichment.
- Only the selected history row prefixes its commit title with its note title,
  using black text on a yellow background followed by one unstyled space.
  Unselected rows and `tix show` retain only the commit title.
- Commit and selected-note titles render Markdown styling. Block-shaped title
  output is flattened onto the single history row; plain command output retains
  the rendered text without terminal styling.

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

### Ref-tree overviews

- After traversal completes, `t` toggles history and a rounded rail ref-tree.
  `Escape` also returns directly to history. The ref-tree cursor and viewport are
  independent of the history selection and panes. The direct ref-tree action
  appears in the `?` information group.
  “Tree” without the `ref-` prefix refers to Git tree objects and tree diffs.
- Entering the overview expands its completed graph with every successfully
  resolved main and linked worktree `HEAD` plus every valid symbolic
  `refs/worktree/tix/pins/HEAD` target. This does not add those commits to the
  history view. Special refs are excluded. First-parent paths form a
  forest whose referenced commits, forks, roots, shallow boundaries, and raw
  tips remain as nodes while linear runs are contracted. `Shift-T` toggles
  tags; when hidden, tag labels and tag-only anchors are removed before this
  projection.
- The component containing `HEAD` sorts first. Children sort by their smallest
  reference label and then object ID. Initial selection is `HEAD`, then a raw
  tip, then the first node; refresh and re-entry preserve the ref-tree cursor when
  its commit remains available.
- The selected node shows the exact number of commits reachable
  through all parents. Other reference and raw-tip nodes show the number reachable
  from them but not from the selection as `N•`; multiple labels at one commit
  share one count. Space fixes this count anchor at the selected node or clears it
  when pressed there again, so cursor navigation can reuse reachability and layout
  caches. Selected first-parent ancestry is emphasized, other reachable history is
  dimmed, and exclusive history remains normal. A non-selectable `●` splits a
  contracted edge where it becomes reachable. Exact exclusive counts are computed
  and cached only for reference rows visible in the viewport.
- The ref-tree orders tips above roots and renders one retained or boundary
  node per row. Rounded ancestry lanes precede aligned counts and labels; their
  `●` disk is the node marker, while the smaller `•` is the commit-count unit.
  Referenced or raw-tip nodes whose commits are present in history use the
  current-history cyan; other linked-worktree nodes use dark green.
  Selection inverts both the node disk and its label, including synthetic nodes
  whose disk is otherwise unlabelled.
- Rendering clips lanes and node labels to the viewport.
- Plain directions choose the nearest node in the requested screen direction.
  Shift-directions instead navigate topologically: Up moves toward leaves, Down
  toward roots, and Left/Right chooses a remembered child; `i/n` and an
  emphasized edge always show the choice.
- `g` selects the top ref-tree node, and `Shift-G` selects the root of the current
  component. Unshifted mouse pans the viewport, while Shift-mouse moves to the
  nearest node. Unshifted full- or half-page Ctrl/Page input moves the cursor by
  the corresponding viewport distance and keeps it visible; shifted Ctrl/Page
  input pans the viewport without moving the cursor.
- `e` opens node-level reference editing. `d` deletes every eligible local branch
  immediately. `e r` is offered only when selected remote-tracking references
  map uniquely through a named remote's fetch refspecs; it deletes every resolved
  remote reference, grouped into one Git push per remote. Pushes continue after
  individual failures and run with the terminal suspended for output and authentication.
- `<enter>` on a node with visible references creates or reuses symbolic
  current-worktree pins for every displayed local branch, tag, remote-tracking
  reference, or review reference at that commit. It returns to history and
  selects the pinned commit in the first refreshed frame, with its cached
  ancestry and hidden merge-base boundary already projected. Synthetic nodes, raw tips, detached-worktree labels,
  and stash associations have no Enter action.
- Worktree branch labels keep the history view's `@branch`, `branch@`, and
  `★branch` forms at the branch's actual tip. A detached current worktree is
  additionally shown with one `📌`; a detached foreign worktree instead uses
  `directory@` at its actual `HEAD`. Ordinary tix pins neither anchor nor
  decorate the ref-tree.
- After deleting selected local or remote references, refresh keeps the commit
  when it remains a ref-tree node, otherwise selects the next surviving node row or
  the nearest previous row when nothing below survives.

### Ref-tree diagnostics

- `tix ref-tree` prints the non-hidden reference projection to standard output without
  terminal colors, selection state, counts, or viewport clipping. A detached
  current worktree renders as `[pin]` in ASCII output and `📌` with `--unicode`.
  It traverses all normal references by default; positional revisions scope
  traversal. Hidden reference labels and traversal tips are omitted, including
  local defaults inferred from remote HEADs unless `--no-auto-hide` is given.
- Worktree traversal is always enabled. `--no-tags` and repeatable
  `-x/--hide <revision>` match the corresponding ref-tree inputs. Output uses
  ASCII lines and `o` nodes by default; `--unicode` uses the interactive
  ref-tree's rounded line and node glyphs.

## Interaction

### Navigation and display controls

| Key | Behavior |
| --- | --- |
| `j`/Down, `k`/Up | Move one selectable row or changed path. Shift follows the first parent or chosen child. |
| Mouse/trackpad vertical scroll | Pan history by the coalesced scroll distance without moving its cursor; Shift moves the cursor instead. Mouse input continues to move paths when a changes block is focused. |
| `h`/`l` | Pan history or the focused changes block horizontally. Shift chooses a remembered topological child. Shift-horizontal mouse input does the same. |
| `Ctrl-u`/`Ctrl-d` | Move the cursor half a page; Shift pans the viewport half a page. |
| `Ctrl-b`/`Ctrl-f`, `PageUp`/`PageDown` | Move the cursor a page; Shift pans the viewport a page. Both forms scroll an overflowing commit message when applicable. |
| `g`/Home, `G`/End | Select the newest/top or oldest/bottom selectable item. |
| `?` | Toggle the information key group. |
| `t` | Toggle the rounded ref-tree overview. |
| `[` | Cycle viewport-local title alignment, full-column alignment, no alignment, and compressed history. |
| `v` | Toggle the history-display key group. Pressing `v` again closes it. |
| `v d` | Cycle author dates, committer dates, and no dates. |
| `v i` | Cycle commit IDs, change IDs, and no explicit IDs. |
| `v s` | Toggle full actors/emails and titles. |
| `v e` | Cycle all attribution, author only, and no names, skipping inert states. |
| `v t` | Toggle attribution trailers. |
| `v m` | Toggle mailmap resolution. |
| `v r` | Cycle all, normal, and no reference labels. |
| `v h` | Show or hide configured hidden ancestry. |
| `r` | Hide reference labels or restore the mode visible when they were hidden. |
| `m`/`]` | Toggle the commit-message view. |
| `p` | Open the command menu from history or a focused changes block. |
| `Shift-P` | Cycle the comparison parent while Tree has focus. |
| `? e` | Cycle the tree/worktree changes display. |
| `Shift-R` | Explicitly refresh the revision view and visible worktree status. |
| `y` | Copy the selected commit ID, or the selected raw path when a changes block is focused. |
| `Shift-y`/`Y` | Copy the selected author as `Name <email>`. |
| `s` | Verify signed, unverified commits currently visible on screen. |
| `@` | Time-travel to the selected commit, or return through its tix pin. Terminals reporting the base key as `Shift-2` are also accepted. |
| `x` | Select the next visible commit with the same change ID, wrapping at the end. |

Alignment uses only rows in the current viewport to determine widths and starts
in title mode. Horizontal navigation pans the complete padded row in title and
full-column alignment so clipped fields can be reached.

Topological navigation treats the displayed history as a first-parent forest.
Down moves toward roots, Up moves toward leaves, and Left/Right or `h`/`l`
select among a fork's children in display order. The choice is remembered per
fork and shown as `i/n` beside the selected row without extending its inverted
selection style. Movement stops at missing roots or leaves and never follows a
merge's secondary parents. A viewport panned away with the mouse or page keys
stays detached until the next cursor movement makes its destination visible.

Compressed history keeps the visible reference, pin, and worktree tips, the
commit selected when compression begins, every graph endpoint or junction, and
every hidden boundary as full, selectable commit rows. Each remaining maximal
linear segment of at least two commits is represented by a selectable hollow
node followed by its exact commit count, such as `○ [12]`; a singleton remains
a full commit row. Pressing Enter on a summary expands that one segment in place;
expansions accumulate until the display is the ordinary title-aligned history.
Topological navigation peels one connected commit from a segment per step.
Moving toward a summary exposes and selects its connected boundary commit.
Starting on a summary instead exposes the boundary in the requested direction
and keeps the remaining summary selected, so repeated steps progressively open
it; when only one member remains, that ordinary commit inherits the selection.
Filtered target selection still exposes ineligible boundary commits one at a
time without selecting them.

Modal review and rewrite-target pickers retain the
compressed projection so its points of interest remain available as targets,
while conflict selection continues to show the full history. Leaving and
re-entering compressed mode through the `[` cycle, or performing a full history
reload, discards accumulated expansions.

The display group remains open for consecutive display changes and closes on
navigation or another recognized command. The `?` group similarly remains open
for signature verification, alignment, message, and changes actions. The
footer keeps every prefix compact. Opening one reverses its label and shows its available
items in a reversed popout immediately above and connected to that label. The
actions popout has commit operations in its first logical section and general
actions in its second. The `?` popout likewise has information actions first,
then the command-menu shortcut, pane switching, and keyboard navigation through
`<enter> diff`; other groups use one logical section. The popout has horizontal
padding and shifts left at the terminal edge. Complete items that would cross
the edge spill into additional rows while preserving the declared row order; an
individual item wider than the terminal is clipped. The whole popout is omitted
when its label, the required rows above the footer, or space needed to preserve
a protected message is not visible. It does not reserve history rows and may
cover history, but message and changes panes, their status lines, and transient
notices shift upward to reserve all of its rows and are never occluded. Closing
behavior and shortcut availability are unchanged, and direct status actions and quit remain in the
footer. The history status starts with the history position, then the `p`
command entry and the `v` and `a` prefixes when they are addressable. Remaining history-level
actions end at the information prefix while it is closed. An available direct
time-travel action follows the shortcut groups, and duplicate cycling follows it when
the selected commit has duplicates, and copy follows these actions; the reference toggle immediately precedes
the `?` group; quit is always last.
All status lines embed and underline a shortcut character in its action label when
possible; keys that cannot be expressed naturally in the label remain explicit.
The Enter key is written as `<enter>` throughout.

### Command menu

- Bare `p` opens a centered command menu from the main history UI, including
  while a changes block has focus. The ref-tree retains `p` for Pin, and `a p`
  remains Split.
- The menu contains the currently available executable entries from the Actions,
  View, Enrich, and Information groups. Each entry retains its exact contextual
  identity, so Stash and Unstash, Review and Finish Review, and Pin and Unpin are
  distinct commands rather than interchangeable labels for one action.
- A single-line input filters entries by a case-insensitive ordered-subsequence
  match against the command label or displayed prefix-group name. Up and Down move the selection,
  `<enter>` executes it, Escape closes the menu, and pasted text edits the query
  instead of invoking history paste behavior.
- A displayed prefix key followed by an ASCII space scopes the menu to that
  group: `v ` selects View, `a ` Actions, `n ` Enrich, and `? ` Information.
  With no suffix every available entry in the group matches; further text
  fuzzy-filters command labels within that group. The literal query remains in
  the input, and an invalid or unavailable scope has no matches.
- The unfiltered catalog interleaves View, Actions, Enrich, and Information
  entries while preserving each prefix group's own order, so the first screen
  represents every available group instead of being filled by one prefix.
- At most nine matching entries are visible and numbered `1` through `9`;
  pressing a displayed number executes that entry. The first opening has no
  default selection, so `<enter>` alone does nothing. On later openings, the
  last exact command submitted through this menu is preselected when it is still
  available; an available contextual opposite is not substituted. Typing a query
  replaces that recalled selection with the first matching entry.

### Time-travel

- On a completed, focused history in a worktree repository, `@` on a non-`HEAD`
  row runs `git checkout --detach <commit>` without forcing local changes.
- `a h` is available while `HEAD` is detached with a valid symbolic HEAD pin.
  It atomically moves the remembered local branch to the current `HEAD` commit
  and attaches `HEAD` without changing the index or worktree. The symbolic HEAD
  pin remains and follows the moved branch. The branch's previous tip receives
  an ordinary pin only when no other normal view tip still reaches it; an
  ordinary destination pin is consumed normally.
- Attach refuses a remembered branch checked out by another worktree. It is
  unavailable during conflicts or incomplete history, but otherwise permits
  dirty index and worktree state because the operation changes only refs.
- When tix detaches an attached local branch, it records that branch symbolically
  in the singleton `refs/worktree/tix/pins/HEAD` ref. Git stores this HEAD pin
  privately for the current worktree, and later branch advances move its tip.
  Further detached travel preserves the singleton. Landing on an available
  remembered branch tip uses that branch as the checkout target directly,
  reattaches `HEAD` in one checkout, and removes the HEAD pin; explicitly
  attaching another branch also removes it.
  Attach is the exception: it deliberately retains the symbolic HEAD pin
  after attaching its remembered branch.
  A failed automatic reattachment leaves both detached `HEAD` and the HEAD pin
  intact and reports a warning. External Git checkouts do not reconcile it.
- Other departures are provisionally retained with ordinary
  `refs/worktree/tix/pins/<suffix>` refs. An already detached `HEAD` receives a
  direct pin. After a successful checkout tix removes that pin when the old
  `HEAD` remains reachable from another view tip, and retains it otherwise. Pins
  use at least four alphanumeric characters; generated pins start with eight
  hexadecimal characters from the saved commit.
- When a rebase rewrites a detached departure, checkout applies the rebase mapping
  before deciding whether to pin it. A departure rewritten into the selected `@`
  successor is not pinned; a distinct departure preserves its rewritten identity.
- While `HEAD` is detached, every valid pin, including the HEAD pin, from the
  current worktree augments implicit and explicit revision tips. While it is
  attached, every ordinary pin away from `HEAD` does so while the HEAD pin is
  inactive. This lets explicitly pinned references and unrelated retained trees
  remain in history. Pins from other worktrees, dangling,
  malformed, and non-commit pins do not enter the view or its decorations.
  Normal hidden-revision exclusions still apply.
- One or more worktree pins at a commit are shown as a single blue `📌`
  resource marker immediately after the hash and outside ordinary reference
  decorations. It remains visible when references are hidden, and internal pin
  names are omitted from history rows. `@` on a pinned
  tip checks out its underlying branch, or its direct commit in detached mode,
  then removes that one pin. Multiple matching pins prefer symbolic targets and
  then lexical ref-name order.
- The HEAD pin instead marks its target branch as `★branch` in the local-branch
  style. It has no `📌`, is never selected as a return destination, and does not
  offer `unpin`; its branch keeps normal tracking-relation behavior.
- The edit menu offers `pin` on an unpinned row and `unpin` on a pinned row,
  both on `a i`. Pin creates or reuses a direct current-worktree pin for the
  selected commit. Unpin atomically removes every non-HEAD pin for that commit;
  both operations retain that row's selection.
- `tix pin <REVSPEC>...` resolves every argument before writing and deduplicates
  pin targets in argument order. A direct reference name creates or reuses a
  symbolic current-worktree pin so it follows later reference updates; derived
  revisions and object IDs remain fixed direct pins. Each unique target prints
  as `pin:<suffix> <short-id>`, and targets at the same commit remain distinct.
- Checkout failures retain the original `HEAD`, remove only a newly created
  source or HEAD pin, and leave destination pins intact. Successful travel
  consumes a destination pin and applies the same source-pin reconciliation for
  ancestor, descendant, and sideways moves. Conflict acceptance, history-rebase
  checkout, and automatic fork travel use this same primitive. Successful travel
  preserves the selected row, refreshes history directly, and invalidates
  worktree status.
- Active review commits define review trees containing all of their descendants.
  Time travel within one review tree keeps ordinary checkout behavior and never
  creates or restores a stash. Crossing out of a dirty review tree saves tracked,
  staged, unstaged, and untracked state with Git under
  `refs/worktree/tix/review/stashes/N`; ignored files remain untouched. Crossing
  into any commit in that review tree restores the state with `git stash apply
  --index` and always removes the companion ref after Git returns. Apply conflicts
  remain in the ordinary index/worktree conflict workflow. Leaving a review tree
  retains its leaf with the normal direct departure pin even after returning to
  attached history; returning through that pin consumes it. Nested trees use the
  nearest review-root ancestor.
- When loaded worktree status shows staged, unstaged, or untracked changes without
  conflicts, the actions menu offers `z stash` at the selected `@` entry. Missing or
  stale worktree status hides the action instead of performing another status
  query. Saving uses Git with `--include-untracked`, leaves ignored files alone,
  preserves the ordinary stash stack, and records the stash commit at
  `refs/tix/stash/<full-commit-id>`. A commit can retain only one such stash.
  `tix stash` performs this operation directly at `HEAD` with the same checks.
- A commit stash is shown as a bright `🎁` beside any `📌`, directly after the
  hash and outside reference visibility. Time travel back to that exact commit
  restores it with `git stash apply --index` and consumes its companion ref after
  Git returns, including when application leaves conflicts to resolve. Manual
  commit stashes use the same plumbing during reviews, while automatic review
  stashes retain their review-tree identity and namespace. An active automatic
  review stash likewise shows `🎁` on the review leaf whose worktree state it
  saved, without exposing its internal reference or stash commit to traversal.
- At a selected `@` with a commit stash, the actions menu offers `z unstash` even
  when other worktree changes are present. It applies and consumes the stash in
  place through the same path used when time travel returns to that commit.
- Rewriting a commit atomically renames its commit-stash association alongside
  other reference updates. Dropping a stashed commit, converging multiple stashes
  onto one result, or overwriting an existing destination stash is rejected before
  prepared objects or references are persisted.

## Overlay views

Overlay views paint over history without changing metadata alignment. Selection
is bounded above the top-most changes block: moving down at that boundary scrolls
history so the selected row stays visible. Shrinking a changes block does not
pull history back into the freed rows. The commit view reserves right-side
space first; changes blocks adapt within the remaining history width.

### Commit message

- `m` or `]` toggles the commit view on the right. It uses at most half the
  terminal and reserves 80 content columns when space permits.
- Its history-status action says `message`, avoiding confusion with the edit
  group's commit-creation action.
- The panel has a minimally shaded background derived from the detected terminal
  background, with the default background as fallback. Its content has two
  columns and one row of margin; an overflow status uses the bottom margin.
- A note renders its bold Markdown title and body first without a separate
  background, followed by a horizontal rule and the commit's bold Markdown title
  and body. Standard Git notes retain their bold purple `Notes`
  prefix and render their content as Markdown. Heading markers and code fences
  are hidden, fenced code uses generic styling without syntax highlighting, and
  commit trailers remain plain and aligned last.
- Overflow is page-scrollable and gets a distinct pane status line only when
  scrolling is possible.

### Tree and worktree changes

- Changes start enabled as `Tree + Worktree`. `? e` cycles `Tree + Worktree` →
  `Tree` → hidden. Bare repositories omit the worktree mode.
- Each block has a top border carrying its compact summary. Tree summaries show
  the selected short hash; worktree summaries distinguish staged and unstaged
  counts. Kind totals, total files when non-redundant, and non-zero line totals
  are color-coded. Empty enabled Tree and Worktree blocks remain visible, say
  `empty` and `clean` respectively in green, and are not focusable.
- Tree paths preserve tree-diff order. Worktree paths show staged entries first in
  green and unstaged/untracked/conflicted entries second in bright red, sorted by
  raw path within each group. When both groups exist, a non-selectable `↑ index ↑`
  divider scrolls between them; its dimmed label aligns with the path-kind letters
  and a green horizontal rail fills the inset content width to its right.
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
- `Shift-P` cycles the comparison parent while Tree has focus. Merge commits are
  compared to one parent at a time; root commits compare against an empty tree.
- Repeated history keys, including printable `j`/`k` reported through enhanced
  keyboard input, and vertical mouse bursts temporarily hide changes
  overlays. They return after 75 ms of navigation idle, with the same path
  selection and viewport where possible.
- Tree diff results, detached diff resources, and line counts use a bounded MRU
  while changes remain enabled. Worktree results are cached separately and
  invalidated by relevant filesystem events.
- Per-file line information is computed once in a lazily activated
  `available_parallelism` worker pool. One repository is opened per activation
  and cheaply cloned into thread-local worker handles; per-batch diff platforms
  are discarded after use. Ten seconds without a completed line-count batch
  joins the workers and releases their repositories. The next uncached diff
  reactivates the pool, while hiding changes drops it immediately.

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

- If excluding hidden history leaves no visible commit, each current view tip is
  shown as a selectable boundary without exposing its ancestry. An unborn
  `HEAD` instead falls back to the configured hidden branch tips. A born base
  supports creating the first stack commit and editing an empty rebase todo;
  rebase-update can advance it to a newer hidden tip without requiring a commit.
  Creating on an unborn base creates the branch there without moving the hidden branch.
- `a w` creates a child of the selected commit from tracked changes, or a root
  commit for an unborn `HEAD`. A changed index wins; otherwise, tracked worktree
  changes are used. Untracked files never enter an implicit new commit and remain
  untracked. It is available only with a live worktree, after history completion,
  and when the selected parent has no known merge descendant.
- `a n` creates an explicit empty commit which reuses the selected parent's tree,
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

- `a f` creates an independent child of any selected commit, including a hidden
  boundary or merge commit. It requires completed history and a live,
  conflict-free worktree, but unlike `a w` it is not restricted by descendants
  because it rewrites none of them. It is unavailable for unborn history.
- Fork preparation reuses the new-commit editor, candidate-tree, identity,
  enrichment, and signing rules. Empty-delta children are allowed so historical
  commits can be forked without borrowing the current worktree's changes.
- Saving writes only the new commit and a temporary direct
  `refs/worktree/tix/pins/*` ref; existing refs, descendants, indexes, and
  worktrees do not move during creation.
- Tix immediately time-travels to the new fork. A successful checkout consumes
  its temporary pin and reconciles the departed `HEAD` through the standard pin
  primitive. If checkout fails, the fork remains pinned and visible.

### Amend, spill, and split

- `a e` amends the current worktree's `@` commit with the changed index, or
  worktree changes when the index already matches `HEAD`. `a l` spills that
  commit's tree delta into the worktree by replacing its tree with its first
  parent's tree, or the empty tree for a root commit. Clean operations are
  unavailable and report a no-op through `tix amend|spill`.
- Command-line `tix amend --index` disables the worktree fallback. It amends
  staged index content when present and reports `nothing to amend` when the
  index matches `HEAD`, even if tracked worktree changes exist. This option does
  not alter the history-view amend action.
- Command-line edits use the same default HEAD, applicable pin, and review tips
  as the history view. Unrelated refs do not broaden their descendant rewrite
  scope, while mutable refs pointing into that scope are still retargeted.
- After any command-line amend, spill, split, reword, new, rebase, or pending
  time-travel replay, successfully retargeted commit refs are printed after the
  command's existing result as sorted `full/ref/name: old-id -> new-id` lines.
  IDs use the same seven-character display as other command results. Ref
  creations, deletions, unchanged refs, and unreferenced replayed commits add no
  mapping line.
- With a path selected in the focused tree-changes block, the main `a` prefix
  offers `spill` and `a l` spills only that path against the displayed parent.
  `tix spill PATH...` atomically spills the named paths against the first
  parent; omitting paths keeps the whole-commit behavior.
- With a path selected in the focused worktree-changes block, the main `a`
  prefix offers `amend` and `a e` amends only that path. A staged row uses its
  index version; an unstaged row uses its filtered worktree version. If both
  rows exist for one path, the selected row determines the version. Review
  commits accept only staged rows, and unresolved indexes cannot be amended.
  Unrelated staged entries retain their index state. The CLI intentionally
  supports only whole-commit amending.
- `a p` is offered at `@` only when both staged and unstaged changes exist. It
  amends the unstaged changes into the source commit, then creates a new upper
  commit from the staged delta using the standard Markdown editor buffer. Both
  deltas are three-way applied in memory before the editor opens, so overlapping
  changes abort without writing objects or changing refs, the index, or files.
- `tix split [--todo]` performs the same split at `HEAD`: worktree changes are amended
  into the source commit and staged index changes become the new commit on top.
  Its upper-commit editor uses the same enrichment headers as the new-commit
  editor; `--todo` enables its Todo header. Existing source enrichments stay
  with the rewritten lower commit.
- A successful split leaves the worktree bytes untouched and resets the index to
  the new upper commit. The rewritten source retains its message and ancestry;
  the upper commit receives the edited message. Their final trees and ancestry
  need no replay marker; rewritten descendants use the same lazy rebase as amend
  and spill.
- All three operations leave worktree files untouched and cheaply rewrite linear
  descendants. Whole-commit edits reset the affected
  worktree's index to the rewritten commit; selected-path amend synchronizes only
  its destination and renamed source. A directly amended commit already has its
  final tree and unchanged parent, so it is signed immediately when configured
  and is never pending. A zero-delta commit immediately adopts and is signed
  against its rewritten parent tree whenever that parent is final; it remains
  lazy only behind a pending parent. Other reparented descendants carry
  `tix-rebase-parent`, retaining the original parent needed for later replay.
  Pending forms use a grey commit marker so they remain distinct from unsigned
  blue. A final descendant whose effective parents did not change retains its
  exact commit instead of being replayed merely because it is checked out.
- Edit graph discovery follows refs that point to commits and ignores refs whose
  targets are trees, blobs, or other non-commit objects.
- Time travel toward a pending destination cherry-picks and signs only the pending
  ancestry through that destination. Later non-empty descendants become or
  remain lazy and unsigned; zero-delta descendants finalize immediately while
  their parent is final and remain lazy behind a pending parent. Traveling toward
  a non-pending ancestor leaves the entire pending region untouched. A completed
  final replay does not reload history;
  another pass loads only the rewritten path and never unrelated references.
  A conflict retains the ours tree, exact merge-result
  tree, conflict stages, prepared commits, and in-memory objects without changing
  the repository. The actual conflicting row is selected and centered with normal
  history-boundary clamping and shows a steady red conflict marker; `<enter>` persists
  the prepared rebase, leaves later descendants lazy, checks out the conflicting
  commit at the ours tree, then checks out the merge result and derives the
  unmerged index from it. `Esc` discards the suspended operation; navigation and
  other read-only actions leave the choice armed, while repository-changing actions
  and refresh are blocked. Key-release events are not actions and leave it armed.
  Diagnostics warn when a conflict suspends the rebase and record whether it is
  accepted, discarded, or fails during checkout.
- A checked-out unresolved index keeps `C` at `@`, overrides dirty `🫟`, and
  disables time travel until all conflict stages are resolved. The worktree
  changes block is shown for resolution.
- Accepting a conflict remembers the materialized commit, HEAD attachment,
  parents, and accumulated reference changes. If the conflict is resolved and
  amended outside tix, refresh recognizes completion only when HEAD remains
  attached the same way, moves to a same-parent replacement, and the
  conflict-free index exactly matches that replacement's tree. Tix then removes
  any pending-rebase marker preserved by Git's amend, appends both reference
  transitions to the same undo operation, and clears the mandatory prompt.
  Staging a resolution without amending remains incomplete. An unrelated HEAD
  move stays blocked with a diagnostic and can still be left with normal `q`.
  Tix's own `<enter>` amend also completes an identical-tree resolution so no
  pending marker can survive merely because the tree did not change.
- A materialized todo conflict keeps a high-contrast `REBASE PAUSED` attention notice until
  its in-memory continuation is consumed. The notice changes when the index is
  resolved but always advertises `<enter>` to continue and `Esc` to stop. History,
  changes-pane navigation, display toggles, copying, and path-diff inspection stay
  available; repository-changing actions and refresh are blocked. Pane-local
  `<enter>`, `Esc`, and `q` retain their inspection and focus behavior. Stopping
  forgets only the in-memory continuation and leaves the partially applied
  repository untouched; Ctrl-C still exits immediately.

### Reviews

- `a r` starts a review from any non-boundary commit without merge descendants.
  If exactly one selectable strict ancestor can be the review base, review starts
  with it immediately. Otherwise tix limits navigation to the selected commit's
  ancestry; the connected hidden base remains selectable, `<enter>` confirms it,
  and Escape cancels before any repository change.
- Starting requires a completely clean index and worktree, including no untracked
  files, and non-pending reviewed-tip and base commits. Only after confirmation,
  tix creates the first unused direct `refs/worktree/tix/review/N` ref at the
  reviewed tip and an unsigned ordinary `review` commit at the base with
  `tix-rebase: onto refs/worktree/tix/review/N`. Starting always creates a
  dedicated worktree-local tix pin for the departure, symbolic for an attached
  branch and direct for a detached checkout, and names it in the
  `tix-review-return-to` header. HEAD is detached at the review commit,
  its base tree fills the index, and the reviewed tip tree remains in the worktree
  as unstaged changes. The pin keeps the departure and its ancestry visible.
  Reviews never share return pins, even when they depart from the same ref or
  commit, so finishing one cannot consume another review's return path.
  Finishing maps the recorded return target through the rewrite and uses normal
  time-travel checkout semantics to restore attached or detached HEAD and consume
  its pin. Existing symbolic review refs remain readable.
- Review refs are resources, not traversal tips; pins alone retain history. They
  remain visible in every ref mode: one active ref is shown as `review`, while
  multiple refs are shown as `review:N`. Review
  commits show a filled diamond as the first resource marker, before pin and stash
  markers, while retaining the normal signature disc or `@` at `HEAD`. Ordinary
  edits preserve the review header and otherwise keep
  their normal signing and lazy-rebase behavior.
- At a checked-out review commit, amend is offered only for staged changes and
  consumes only the index tree. It leaves worktree bytes and the review header
  intact, removes signatures, and marks only affected descendants for lazy replay.
- `a r` finishes a selected review when status is completely clean and the current
  worktree HEAD is the review commit or one of its successors. The
  review commit is inserted after its reviewed tip with its exact tree, review
  header removed, updated committer, and configured signature. Review-side
  descendants retain exact trees and are signed without pending markers. With one
  review-side leaf, the reviewed tip's prior descendants are lazily reparented
  after it; with multiple leaves they branch directly after the finished review.
  The review ref is deleted in the same atomic ref/worktree transaction.
- If the recorded review return ref is missing, finishing leaves the repository
  untouched and limits navigation to visible non-review commits descended from
  the reviewed tip. The reviewed tip is selected initially when visible;
  otherwise the nearest eligible row is selected. `<enter>` finishes the review,
  maps the chosen commit through that rewrite, and checks it out detached, while
  Escape cancels recovery.
  Hidden, unrelated, and review commits are not selectable return targets.
- Forget is unavailable for a review commit with descendants. Forgetting a review
  leaf cancels the review: tracked review changes are discarded, its recorded
  return checkout is restored, and the departure pin is consumed. Finishing a
  review or dropping one through a rebase todo also deletes its review ref and
  optional saved-worktree ref atomically; reordering or rewriting it preserves the
  headers and resources. Review stash refs are internal: they are not traversal
  tips or named decorations, but their saved review leaf carries a `🎁` marker.

### Forget commits

- `e`, then `d`, is available after history completion for a selected non-merge
  commit with no known merge descendant. The first `d` arms a
  yellow notice asking for `d` again; the second performs it. Navigation, refresh,
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
  when later replay needs it; and `CherryPick` transplants each tree delta.
  Any edit that rewrites the current worktree's checked-out ancestry eagerly
  cherry-picks that affected path before committing the operation. The edited
  root of a direct amend or spill already has its final tree and does not receive
  a redundant worktree transition. Descendants on unrelated branches and in
  other worktrees remain lazy unless their delta is empty and their parent is
  final. A successful repeated rebase clears the marker
  through its checkout destination.
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
  while awaiting an explicit `<enter>` or `Esc` choice. Dropping it writes nothing;
  accepting it consumes the repository immediately after persisting the commit at
  the ours tree and materializing the retained merge result in the worktree and index.
  Forget, reword, commit insertion, review finishing, and other shared-rebase
  callers propagate this same suspended result instead of completing their ref
  transaction first. Thus a checkout-path conflict is reported by the initiating
  edit itself, and `Esc` leaves its repository snapshot unchanged.

### History rebase editor

- Selecting an eligible hidden boundary and pressing `a b` opens a Markdown
  `.md` todo. Its editable plan is read bottom-to-top like the history view:
  newest commands and refs are highest, and each stack ends in a centered
  `──── fork <id> ────` separator below its oldest command.
  IDs are shortened through repository configuration; metadata is loaded across
  the complete todo scope, repeats the full information visible in history, and
  always includes the subject. Base-level
  stacks end with `fork <id> (base) <title>` in the separator, using the title
  exactly as displayed in history without Markdown escaping. Fork points within the editable tree
  remain plain `fork <id>` separators. Every separator is centered with at least
  four `─` characters per side, and all span the widest editable line.
- When that boundary shows `⇣N`, `a u` opens the same editor with each base-level
  stack rooted at the corresponding hidden branch tip. Its otherwise unfamiliar
  separator is `fork <id> (updated-base) <title>`, with the raw title exactly as
  shown in history, including `[A]` and `[N]`. The hidden branch
  itself is not moved.
- Pick lines may be reordered or removed. `squash <id>` folds an existing
  non-merge commit into the following `pick` or `empty` below it in the same
  fork; it may carry `@`, and fork separators naming any folded ID resolve to
  the combined result. A fork cannot begin with `squash` when read bottom-to-top.
  Fork separators may otherwise target a pick below or any existing commit, so
  adding and removing separators creates
  and joins branches. `empty <title>` inserts an empty commit. Markdown code
  spans and equivalent plain commands are accepted; display text after an ID is
  informational and emitted verbatim without Markdown escaping.
- Squash groups are materialized eagerly on every fork by applying their source
  deltas in bottom-to-top todo order. The result retains the first member's author, author
  time, encoding, extra headers, and message, receives the operation's committer,
  and is signed once. Before every later full message, a permanent
  `# <short-id> <subject>` line identifies its source. Distinct raw authors of
  later commits are appended in first-seen order as `Co-authored-by` trailers,
  excluding the first author and identities already named by a valid such
  trailer in any source message. Name and email pairs are compared without
  mailmap. All folded IDs and mutable refs map to the one resulting commit;
  resources owned by a later folded review commit are removed.
- The first line points to complete self-documenting help after the editable
  todo. All instructions are enclosed in Markdown comments so only separators,
  reference lines, and command lines participate in the editable plan.
- A versioned Markdown state comment makes the document independently
  applicable in a later process. It records full base, target, scope and tip IDs,
  checkout requirements, and compare-and-swap state for mutable refs. Ref names
  use Git-compatible C-style quoting so arbitrary ref bytes round-trip. Missing
  state cancels; present invalid state never reaches repository mutation. The
  state comment follows the complete help at the end of the document. Bottom-up
  todos use `tix-rebase-state-v2`; older state versions are rejected rather than
  interpreted with the opposite command order.
- Standalone `(ref, ref)` lines place direct mutable refs at the following fork
  separator or command result below them. Multiple consecutive lines share that
  destination.
  Commit command metadata omits ref decorations because these lines are their
  sole editable representation.
  Existing displayed names may be moved or removed, and new unqualified names
  create local branches; explicit editable `refs/...` names are also accepted.
  Existing editable direct refs outside the generated todo are imported with
  their current target as compare-and-swap state and may be placed the same way.
  Short names follow the history display, ambiguous names expand to full names,
  and Git quoting preserves arbitrary bytes. Tags, remote-tracking refs, general
  symbolic refs, tix pins, stashes, and review resources remain hidden and
  unchanged.
- Pick lines use display-only state symbols documented in the footer: `↻` for a
  lazy rebase, `◌` for an invalidated signature awaiting signing, `◐` for an
  unverified signature, and `○` for an unsigned commit. Applicable states may be
  combined without changing plan semantics. Applicable `🚧`, `📝`, and `✔️`
  enrichment gutter symbols appear before the signature-state disk as metadata.
- `@pick`, `@squash`, or `@empty` chooses the post-rebase commit. A generated
  todo keeps this marker even when `HEAD` is attached, but shows its branch as an
  ordinary ref. Versioned state remembers that attachment while the ref stays
  at the marked result. Moving it elsewhere detaches `HEAD`; adding `@` to one
  editable ref explicitly attaches it and is valid only at the marked result.
  The ref may be imported from outside the generated todo and is moved before
  `HEAD` is made symbolic to it. Removing the name deletes the ref. Checkout
  markers are invalid without a worktree. Todo generation and application
  reject an unborn `HEAD`.
- Within the ancestry ending at `@`, unchanged picks whose original parent is
  still their planned parent retain their IDs. Eager cherry-picking and re-signing
  starts at the first pending or structurally changed commit. Any descendants
  above `@` and other resulting stacks retain their trees, receive pending-rebase
  markers, and invalidate old signatures for later time travel. With no explicit
  `@`, the current attached branch's resulting destination is inferred and its
  ancestry is replayed eagerly; a detached checkout is not inferred. Other
  ordinary steps remain lazy while squash groups are still materialized.
  Any conflict while applying a history todo first remains entirely in memory.
  The TUI projects the partial result, selects and centers the actual conflicting
  result with normal history-boundary clamping, and marks it with a steady red
  conflict marker; predicted ref decorations remain at their
  repository positions. Repository-backed overlay content is hidden while these
  candidate objects exist only in memory. History and pane navigation and other
  read-only actions leave the preview armed; repository-changing actions and
  refresh are blocked. `<enter>` accepts the partial result,
  moves already-final refs, records the ours tree in the conflicting commit, and
  checks out the retained merge result with an unmerged index,
  and retains an in-memory continuation plan. Only `Esc` discards the preview
  without writes. On continuation, `<enter>` stages paths that still have unresolved
  index entries, refuses to proceed if any unresolved stages remain, and amends the
  current conflicting commit from the complete staged index, including any additional
  staged changes. Unrelated unstaged changes remain untouched. Another conflict
  repeats the same explicit choice.
- Command-line apply, including todo `--edit-and-apply`, reports a conflict
  without changes unless `--materialize-conflicts` was explicitly supplied. Its
  continuation document uses the full null object ID for the command whose tree
  must come from the resolved index. Already produced commits use their new IDs,
  completed drops and squash sources disappear, unapplied squash sources remain,
  and the remaining
  todo stays editable. Applying it
  requires only that `HEAD` names a commit and the index has no unresolved stages;
  the index tree, including additional staged changes, becomes the resolved tree.
  There is no hidden sequencer state or separate continue/abort command.
- Every interactive operation that rewrites the stack below `HEAD`, including
  todo application, runs on a scoped worker and shows its modal gauge after
  300 ms. TUI time travel also runs on a scoped worker and follows completed
  pending rebases on the destination ancestry. The history selection traverses
  each fixed viewport from bottom to top; crossing its top jumps the viewport
  by one page and places the selection at the bottom again. Compressed history
  temporarily uses its canonical rows for these frames and is restored afterward.
  The first and latest rows are drawn even for fast operations, with intermediate
  rows coalesced to at most 60 fps. Command-line time travel does not animate.
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
  the file or removing the `tix-rebase-state-v2` comment cancels. Continuation
  todos likewise state that saving unchanged continues the materialized rebase.

### Commit and action shortcuts

- `a` toggles a two-line shortcut group with commit operations above general
  actions. `a o` rewords, `a w` creates a rebased child, `a n` creates an empty
  child, `a e` amends `@`, `a l` spills `@`, `a p` splits staged from unstaged
  changes, and `a d` forgets a top commit when each action is
  available. `a b` rebases an eligible hidden base,
  `a u` rebases it onto the newer hidden branch tip when available, `a r` starts
  or finishes a review, `a s` squashes the selected commit, `a z` stashes or
  restores changes at `@`, `a y` copy-inserts
  current `HEAD` above the selected commit, `a m` move-inserts it, `a t` starts
  stack-insert for the linear ancestry from the selected commit through `HEAD`,
  `a f` creates and travels to a standalone child of the selected commit, and
  `a h` attaches the remembered branch at detached `HEAD` when available.
- Squash accepts any visible strict ancestor whose affected descendants contain no merges. With one eligible
  target it applies immediately; otherwise navigation is limited to eligible ancestors, `<enter>` confirms,
  and Escape cancels. A non-adjacent source is folded next to the target while intervening commits and sibling
  forks remain above the combined result. Squash uses the history-todo rebase, conflict, and continuation rules.
- Copy-insert requires a non-root, single-parent source that is not an active
  review commit. `a y` copies current `HEAD`; `tix copy-insert C I` accepts any
  resolvable source `C`. It inserts another occurrence of its change above the
  target without removing the source occurrence, including when the target is
  the source's current parent. The new copy becomes detached `HEAD`; the branch
  checked out before the operation remains visible through the ordinary HEAD
  pin. If the target is an ancestor of the source, that source occurrence is
  retained in its original logical position while its branch follows the
  necessary rewrite. Git notes are copied to the new occurrence. Copy-insert
  uses the history-todo conflict and continuation rules.
- Bracketed paste in the history view trims surrounding whitespace and accepts
  one uniquely resolvable hexadecimal object-ID prefix. If that object is a
  commit, its change is copy-inserted above the commit at the cursor using the
  same progress, conflict, checkout, and undo behavior as `a y`. Other text,
  ambiguous or missing IDs, non-commit objects, and unavailable targets produce
  an attention message without changing the repository.
- Move-insert requires a non-root, single-parent `HEAD`. It removes `HEAD` from
  its old position, reconnects its former children to its parent, inserts its
  rewritten change above the selected target, and reparents every former direct
  child of the target above it. The target may be an ancestor, descendant, or in
  unrelated history; selecting `HEAD` or its current parent is a no-op. An
  unchanged merge target is permitted, but any move that would rewrite a merge
  is unavailable. Mutable refs, pins, Git notes, enrichments, review resources,
  and attached or detached checkout state follow their rewritten commits.
  Move-insert uses the history-todo conflict and continuation rules.
- Stack-insert requires the selected commit to be an inclusive base in the linear
  ancestry of `HEAD`. It then limits navigation to eligible insertion targets;
  `<enter>` moves the complete inclusive base-through-`HEAD` stack as a unit above the selected
  target, and Escape cancels. The stack follows the same eligibility, rewrite,
  no-op, metadata, checkout, and conflict rules as move-insert.
- `@` invokes time travel directly, outside the group. Invoking it leaves an
  already expanded actions group open.
- Commit and action shortcuts keep the actions group open. Navigation or
  another recognized command closes it, matching the `v` display shortcut group.
  Plain `r` does not mutate the repository, and plain `t` has no action.
- The footer underlines `a` in `actions`; its expanded commit and action lines
  contain only the operations available for the current selection. An empty
  line says `no actions`.
- The top-level `v`, `a`, `n`, and `?` keys are reserved for their groups.
  Pressing one while another group is open switches directly to that group;
  `? e` cycles the changes panes.
- While the `v` group is open, `d`, `i`, `s`, `e`, `m`, `t`, `r`, and `h` control
  dates, IDs, emails, names, mailmap, trailers, references, and hidden commits.
- The `n` in `enrich` toggles its shortcut group. On any commit eligible for rewording,
  `n t` toggles `[commit] todo`, preserving a saved note, and `n o` opens
  `[commit] note` in Git's editor as Markdown. Saving or removing a note preserves
  the todo flag, and toggling todo preserves the note. `n e` toggles
  `[tree] checks-pass` for any selected commit, including immutable boundaries.
  `n g` edits the real Git note and remains available when the commit-specific
  Tix actions are not. The group is mutually
  exclusive with the view, commit, actions, and information groups and otherwise follows
  their closing behavior.

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
- A filesystem history refresh is presented immediately as one complete frame,
  without animating or retaining intermediate history layouts.
- While the terminal is unfocused, filesystem-attributed redraws replace footer
  separators with persistent orange discs. Focus restores normal separators.
- Filesystem responses receive correlated IDs in daily tracing logs, including
  semantic trigger, coalesced paths, phases, presentation count, elapsed time,
  and outcome. Logs use the platform application-log directory, retain seven
  days, and are best-effort. Failure to create or open the log is silently
  ignored and never prevents either command-line or interactive operation.
- After every event-loop wait, tix assumes that the original worktree and process
  working directory may have disappeared. Before processing filesystem events or
  redrawing, it lexically normalizes and enters the common repository, reopens it
  as bare, drops worktree state, keeps tree/history views live, and reports recovery
  in the attention notice. If recovery fails, terminal state is restored and the
  contextual error is returned.

## Resource and responsiveness invariants

- No `gix::Repository`, commit-graph, object platform, notes platform, or other
  repository-owning value may remain in idle application/event-loop state,
  except line-diff worker repositories during their bounded ten-second reuse
  window.
- Hidden-revision startup validation returns only detached revision and warning
  data; its temporary repository is dropped before terminal initialization and
  the event loop.
- View population opens a fresh non-isolated repository so mailmap, notes, diff
  drivers, pagers, signing, and other Git configuration are current. It starts
  without an object cache; bounded diff operations may enable one temporarily and
  disable it again before any navigation reuse. Detached display data is retained.
- One fill repository may be shared by commit, tree, worktree, and metadata loads
  during continuous key-repeat or mouse navigation. It is dropped after the
  75 ms idle boundary.
- Traversal and incremental refresh workers may use a bounded object cache and
  must drop their repository when finished. Lane and verification workers exist
  only for active work. Line-diff workers may remain for ten seconds after their
  latest batch, then are joined together and release their shared repository
  resources.
- Change IDs are scanned only while configured hidden tips are actively excluded.
  Unrestricted and explicitly expanded views perform no scan. A refresh keeps
  the current projection's IDs until it has synchronously scanned the replacement,
  then publishes rows, IDs, duplicate markers, and gutter width together.
- Redraw is reactive and capped at approximately 60 frames per second while
  streaming. Mouse events are drained and coalesced in bounded batches so input
  storms cannot starve the main loop.
- Main status remains readable regardless of pane focus. Errors are surfaced in
  the nearest relevant status line; diagnostics never replace user-visible
  errors.
- Global command and recovery feedback uses one transient notice channel in the
  history and ref-tree views. It reserves a wrapped, content-height block above
  worktree changes until the next recognized user action, inset by two columns
  within that pane when visible and otherwise within the main view. It never
  covers the main or pane-local status lines. Green indicates success, yellow
  indicates attention, no-op, recovery, partial success, or an armed prompt, and
  red indicates failure. While undo or redo feedback retains its queue position,
  the notice becomes a two-tone progress bar: the applied share is bright on the
  left and the redo share is dim on the right. A fully applied queue is entirely
  bright, while its start and an empty queue are entirely dim; attention and
  failure notices retain the same progress in their respective hues. Forget,
  review selection and recovery, suspended
  conflicts, and paused rebases retain their notice until resolved; pane-specific
  errors remain in their pane status line.
- Closing the new-commit editor without changing its prepared buffer leaves the
  repository untouched and reports `no commit created: no input was provided`.

## Regression coverage

- Unit tests cover navigation, projections, pane layout, status summaries,
  selection restoration, watcher classification, cached graph walks, diff
  preparation, signatures, rewording, and terminal rendering.
- Behavior changes to this specification require corresponding tests and an
  update to this document in the same semantic patch.
