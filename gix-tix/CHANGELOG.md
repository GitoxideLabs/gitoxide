

## 0.4.0 (2026-09-25)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release over the course of 31 calendar days.
 - 32 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Merge pull request #2847 from GitoxideLabs/gix-error-completion ([`6356013`](https://github.com/GitoxideLabs/gitoxide/commit/6356013bca0987c6c97ad7ba9d5347271979b51e))
    - Merge pull request #2989 from GitoxideLabs/error-conversion-review ([`4b9ff51`](https://github.com/GitoxideLabs/gitoxide/commit/4b9ff511a49f7963e97a669ca82c6f6e833d8ea2))
    - Merge pull request #2971 from GitoxideLabs/diff-nullid-fix ([`d7551f1`](https://github.com/GitoxideLabs/gitoxide/commit/d7551f1593ad1e5897ab637a930f0d185310238b))
    - Merge pull request #2963 from GitoxideLabs/gix-notes-perf ([`4a870be`](https://github.com/GitoxideLabs/gitoxide/commit/4a870be7db38a3fde68db3774fa7d7f2000c3d68))
    - Merge pull request #2949 from GitoxideLabs/error-conversion-review ([`a095334`](https://github.com/GitoxideLabs/gitoxide/commit/a0953348e4d27f59222c1782119d2539a778cd4d))
    - Merge pull request #2944 from GitoxideLabs/error-conversion-review ([`e3a6fa1`](https://github.com/GitoxideLabs/gitoxide/commit/e3a6fa1516481ec69ab00cddcca081ecdc52b4ca))
    - Only `gix-tix` should auto-commit, with intermediate WIP commits ([`a6ca49f`](https://github.com/GitoxideLabs/gitoxide/commit/a6ca49fba89d37ba97481e7b5cf3325dc7eedb98))
    - Merge pull request #2942 from GitoxideLabs/error-conversion-review ([`a1d5a55`](https://github.com/GitoxideLabs/gitoxide/commit/a1d5a5520d597bdc33c1cf84c1d061b5bc1e382e))
    - Merge pull request #2940 from GitoxideLabs/vendor-bisync ([`dda600d`](https://github.com/GitoxideLabs/gitoxide/commit/dda600d7ee29a6bda4cf1047d0d1782e76b16f98))
</details>

## 0.3.0 (2026-08-24)

### Changed

 - <csr-id-58fb29b52f187e6e7075b34ae3394f02c3dc0546/> remove WIP author choices from commit editors
   Todo and Message enrichment headers supersede the commented provisional-author choice. Remove the option from the shared document writer and parser so new, fork, and split editors expose only their configured author, and update the specification and regression coverage accordingly.
 - <csr-id-30b4948e06722b96d87d8149dbf7402ce43b9566/> show raw display metadata in tix todos
   Todo metadata is informational, but Markdown escaping made subjects and anchor labels harder to compare with the history view and added noise to otherwise editable lines.
   
   Emit commit metadata, continuation and squash titles, and annotated fork labels verbatim. Command parsing still stops at the command span, so raw Markdown punctuation remains display-only.
 - <csr-id-24996c6525a4374dcb0d1520cdf61e15ba0238f6/> widen the default tix commit panel
   Reserve eighty content columns for commit messages on sufficiently wide terminals, in addition to the panel border and horizontal margins. This prevents conventionally wrapped commit text from orphaning its final word.
 - <csr-id-342e18218965bab400575763182e2350d56a01b1/> brighten red UI elements and omit zero diff counts
   Use the terminal bright-red color for deletions, unstaged changes, failed signatures, errors, behind counts, and graph rails so red remains legible against dark backgrounds.
   
   Render insertion and removal counts only when non-zero in history selection information, changes summaries, and selected changed paths. Keep clean worktree blocks visible by their title.

### New Features

 - <csr-id-b72684362014c96b0b6f57a94a1436de1476736e/> peel commits from collapsed history
   Topological navigation previously treated compressed summaries as indivisible nodes. Peel one boundary commit per Shift-j/k step so movement toward a summary exposes and selects the connected commit, while movement from a selected summary progressively opens it without selecting the peeled commit. Preserve target-picker eligibility and document the behavior.
 - <csr-id-480fb0adf6241af320b2060eac11280f77fffddd/> spill individual paths from the CLI
   Accept zero or more literal paths on tix spill, preserving whole-commit behavior when none are given. Resolve and validate all paths against the first-parent tree diff before applying them through one atomic history edit and undo entry.
 - <csr-id-560d81fe2a1e6d05484606bef5e4a8243ead538a/> add a fuzzy command menu
   Add a reusable text-input picker opened with p and populate it from the same contextual command catalog as the existing shortcut popups. Filter commands by ordered subsequence across labels and displayed prefix-group names, support prefix-key-and-space group scopes with fuzzy suffixes, interleave groups on the initial page, and recall only the exact previously submitted command while it remains available.
   
   Keep paste input inside the picker, clamp selection to rendered rows, and prevent closing-key repeats or asynchronous ref-tree entry from leaking stale modal state. Move comparison-parent cycling to Shift-P and retain p as the ref-tree pin shortcut.
   
   Render shortcut-prefix popouts by spilling complete items across as many rows as fit while preserving their logical sections and protected pane and notice layout. Split the information help into action and keyboard sections, advertise the command menu there, and omit mouse-input documentation.
 - <csr-id-42c89992cf7313f910ef275eeff0909e81f44b20/> copy-insert pasted commits
 - <csr-id-b7abf00018181166d603685d9a59d0c711c535bc/> show progress during stack edits
   Stack edits below HEAD now eagerly replay the checked-out path, so operations such as rewording can spend noticeable time cherry-picking and signing descendants.
   
   Run every interactive stack rewrite through the existing delayed rebase-progress modal, and extend ordinary edit rebases to report commit, cherry-pick, and signature progress.
 - <csr-id-0a0cef8d1e807452d0a3adf8d95d8fa47844816a/> add no-alt-screen diagnostics
   Render the interactive UI in a full-height inline viewport when requested so its frame and panic output remain visible. Restore terminal input modes, cursor state, and raw mode on normal, error, and panic paths.
 - <csr-id-502423b1542440793da481bd0645f74dc219977a/> add momentary topological navigation
   Use Shift-modified directions for first-parent traversal and remembered branch choices in history and ref-tree views. Keep mouse and page input viewport-only by default, with Shift moving the cursor, and show fork choices without persistent motion modes.
   
   Always invert selected ref-tree node markers.
 - <csr-id-2b338883a2daf48cf391db10abc6bfee847e0041/> restore paged time-travel animation
   Animate completed pending rebases through fixed history viewports, jumping by a full page when the selection reaches the top.
   
   Temporarily expand compressed history, keep cached panes tied to the animation origin, and preserve completed mutations if frame rendering fails.
 - <csr-id-b6096cf525412640b0d54272be2dfb4e464f3e9c/> replay diagnostic navigation inputs
   Accept an optional keyboard-input string on --quit-on-finish and replay read-only actions after completed lanes have established the final layout. Keep the retained frame diagnostic safe by ignoring repository-changing, external-program, copy, and quit actions.
 - <csr-id-f50bba83ace8dcc1c84c1168751d85ce626cdfad/> add copy-insert command
 - <csr-id-9ad195fa7166c607395c0bc23d3cb17d363291c7/> add tix admin clear-undo
   Expose the existing atomic undo-reference cleanup for recovery when the worktree-local queue should be discarded without replaying it.
   
   Delete only this worktree’s tip and cursor refs. Repeated calls are no-ops; recorded refs and other worktrees remain untouched.
 - <csr-id-0579a7b0337cbbb128682e72e841433c73598e2a/> add ref-backed undo and redo
   Store per-worktree operation history as config-serialized commits, restore refs and affected worktrees with checked transactions, and expose undo/redo through the TUI.
   
   Show the current queue position as two-tone progress in the transient message line, while keeping internal queue refs out of history and revision selection.
 - <csr-id-abb9ce7180664c1d77792c2f426fadea685b6c1f/> make compressed Tix history interactive
   Retain graph endpoints, junctions, and hidden boundaries as selectable commits. Let synthetic linear segments participate in navigation and expand in place while preserving modal target eligibility and refresh state.
 - <csr-id-ec71813dfeb5fad1f965e51e1cdb8b3adb60f8a9/> add a compressed Tix history view
   Collapse non-tip history segments into nonselectable ring rows while preserving visible tips, selection, and graph topology.
   
   Keep rewrite and conflict selection workflows on the full history.
 - <csr-id-8636739abeff3ff1ecd29936e3756d46bc443859/> add relative tix travel destinations
   Add first, parent, child, and tip destinations resolved within HEAD's visible default-view component. Report ambiguous endpoints with directly usable commit and change IDs while preserving the existing travel pipeline.
 - <csr-id-8a900927885081fb504550bc957c18abe0ca107e/> highlight foreign worktree heads in tix
   Shade titles at attached and detached foreign worktree HEADs while keeping the current HEAD and selection more prominent. Preserve both attached-checkout and remembered-branch decorations when they share a branch.
 - <csr-id-acc25697211ff829cc71f6a4a47e868b08d6a2a9/> show dirty HEAD with 🫟
   Replace the ASCII dirty-worktree marker with the requested two-column glyph while preserving the existing status gutter, selection styling, and conflict-marker precedence.
   
   Update the rendering regression and behavioral specification so deletion statistics continue to use D independently.
 - <csr-id-f7b4ae0f1f3ae37a91080da2602fd2ed72f37757/> add copy-insert to tix
   Add copy-insert beside the renamed move-insert and stack-insert actions, using a dedicated copied plan step so the destination receives HEAD’s change without consuming the source occurrence or its resources.
   
   Select the new copy as detached HEAD, retain source refs and notes, carry copy conflicts through the existing continuation flow, reject review-source duplication, and expose the actions as a c, a m, and a t.
 - <csr-id-b05b54762c7e4fdb5ce14be9690ef68bb1d98578/> add cherry-move-stack to tix
   Move rebase commands into the actions prefix and allow a selected linear HEAD ancestry to move as one stack while preserving refs, checkout state, and cycle safety.\n\nAdapt the rebased signing API and fixture snapshots needed by the feature tests.
 - <csr-id-3f3d5c30058a00d534f105a3990e87e7a8dda41c/> show prefix actions in floating menus
   Keep prefix labels compact in the footer and render active choices in a connected one-line overlay above it. Clamp the overlay to the terminal, draw it above other content, and preserve the existing interaction behavior.
 - <csr-id-7fc36ab141b848a26941915300601d9ff321f8b5/> add enrich subcommands
   Expose commit todo, Tix note, Git note, and tree checks-pass enrichments through nested CLI commands. Targets default to HEAD, accept change IDs, and boolean commands support idempotent --clear updates.
 - <csr-id-b32ffbc8512e349945b07c41493166d84ee94b49/> add checks-pass tree enrichments
   Store checks-pass metadata as worktree-local Git notes keyed by tree ID. Show the marker in the TUI and plain status output, and expose it through the enrich shortcut group.
 - <csr-id-628481ac7af57d45bfd50d4f3525669ce18c9b33/> add status as an alias of tix show
   The non-interactive history projection is useful as a repository status view, but was only discoverable under the show command. Advertise status as a visible Clap alias and route it through the exact same parser and implementation.
 - <csr-id-0ce4d8ed4129486b411d58895def3cf7adaed8ab/> round history graph connections
   The regular history graph used square box-drawing corners while the ref-tree rounded the same lane transitions afterward. Render the four unambiguous turns as rounded corners in the shared lane primitive so TUI history, tix show, and the ref-tree remain visually consistent.
   
   Merge tees and crossings stay orthogonal because they represent three- or four-way junctions and have no rounded Unicode equivalent. This preserves the existing one-commit-per-row layout, paging, and alignment while removing the ref-tree conversion pass.
 - <csr-id-c7c183c7305dda32e41d4b4a64996755caa27c37/> add a TUI cherry-move action
   The actions prefix can now move the current HEAD change directly above any visible selected commit instead of requiring an ancestral squash target. The planner removes HEAD from its former position, reconnects both sides of the edit, and reparents every target child so forks retain their shape.
   
   Run the operation through the existing history-todo engine so attached and detached checkouts, mutable refs, pins, notes, enrichments, review state, lazy descendants, and materialized conflict continuation keep their established transactional behavior. Root sources, current-position no-ops, and moves that would rewrite merges are rejected.
 - <csr-id-9e4763c88ef4ae917e58b0a461d7fa904e22bfc0/> add a ref-only TUI forkpoint action
   Forkpoint replaces the former attached-travel shortcut as the only TUI action that moves the remembered branch while attaching HEAD. The actions prefix establishes the current detached HEAD as that forkpoint without checking out files.
   
   Move the remembered branch and attach HEAD atomically while preserving the index and worktree, including dirty state. Keep the symbolic HEAD pin to identify the remembered branch, retain an otherwise-unreachable former tip with an ordinary pin, and reject branches owned by another worktree.
 - <csr-id-37ee0b895ad8addf3af77241ba2bf07de5e1dd6e/> add two-step TUI squash
   Add an edit shortcut that selects an eligible ancestor and folds the current commit into it through the existing rebase-plan machinery. Preserve intermediate forks and attached branches, and reuse materialized conflict continuation.
 - <csr-id-0dd8c28c0b324796cde1b07389b33b2670d90260/> add attached TUI time travel
 - <csr-id-d6584ce234f999382eb906dd4763669fbb587223/> toggle pins from the TUI edit menu
   Make e i create a direct current-worktree pin for an unpinned selection and remove every ordinary pin when the selection is already pinned. Preserve the special HEAD pin, retain selection across refresh, and report the operation through the existing TUI notices.
   
   The edit menu now always exposes the context-sensitive pin or unpin action, making pinned comparison histories available without leaving the TUI.
 - <csr-id-482270ffbf6908f569eac92fe71b82f9fca079dc/> print change IDs with tix commit hashes
   Make primary command output identify commits by both their abbreviated object ID and equally sized reverse-hex change ID. This covers mutation results, rewritten refs, pins, notices, raw ref-tree tips, and CLI rebase documents while leaving diagnostics, serialized todo state, and TUI editor output unchanged.
 - <csr-id-eed98fdcf935ff1d51aac2529517e9c3d63f66fe/> edit and preserve Git notes in tix
   Expose the configured default Git note in the enrichment menu for every selected commit, including immutable history boundaries. Empty editor content removes the note and unchanged content leaves the repository untouched.
   
   Record only actual predecessor-to-successor rewrites so notes follow rewritten commits without leaking through synthetic insert or drop mappings. Concatenate converging squash notes, keep split notes on the lower identity, and update the notes ref atomically with the existing rebase transaction and rollback.
 - <csr-id-97e262c8223c0851d22d32b17b48dd555c7dcdf1/> add --todo to tix new and split
   Command-line commit creation previously required editing the Todo header manually, and split had no option for marking its new upper commit.
   
   Seed the existing editable enrichment header from --todo for both commands. Explicit new messages apply it directly, while editor users can still comment it out; split keeps pre-existing source enrichments on the rewritten lower identity.
 - <csr-id-ee156520a37145505efb4b245d523a5ab253b4e7/> cycle through duplicate commits in history
   Show a next-duplicate action for commits sharing an effective change ID and move selection through the group with wraparound.
 - <csr-id-bb064d688b601afd83a9f3c300cb4aa677ebe3f9/> make history counts relative to visible bases
   Start each visible commit tree at zero and count descendants by their displayed row distance, choosing the nearest root for merges. Render visible roots as centered metadata-bearing base separators in tix show.
 - <csr-id-ad229980ff3fc1179f4bbeb61c674dd04e1cbb20/> mark ambiguous change IDs without hiding them
   Keep colliding short change IDs visible in tix show and mark affected rows with a bomb gutter. Mark duplicate TUI change IDs with a twin gutter while retaining the full short change ID. Replace the worktree conflict C with a bomb gutter so selection and dirty markers remain visible.
 - <csr-id-db5c2af45e1bc4d7e5c396450e660242d263659a/> let edited rebase todos materialize conflicts
   Allow tix rebase todo --edit-and-apply to opt into the same conflict materialization and continuation-document workflow already available through tix rebase apply. Require immediate editing when the option is present and retain the existing safe default of leaving conflicts unmaterialized.
 - <csr-id-4763b8ec4dcf9e04eef4c525f3125c9fd5f9125c/> add tix rebase todo --update-base
   Expose the TUI rebase-update behavior through command-line todo generation. Reuse the same hidden local branch update calculation for the uniquely derived fork point, label the selected target as updated-base, and keep unchanged plans actionable while rejecting explicit --onto combinations or missing update targets.
 - <csr-id-dacf744022f8e70e654dc049a872abb4f66260a2/> show and resolve abbreviated change IDs
   Expose unambiguous seven-character change IDs beside commit hashes in plain history output and accept reverse-hex prefixes in travel and reword commands. Resolve Git syntax first and scope change-ID fallback to the default Tix view so unrelated repository history cannot silently broaden mutation targets.
 - <csr-id-14e18ed857b7d2262038d70549b795306ff347d9/> report rewritten refs after tix mutations
   A mutation can print the commit it edited before lazy descendant replay has moved the final branch tip, leaving scripts and users without the resulting ref target.
   
   Capture old-to-new commit targets from the successful shared ref transaction and propagate them through command-line amend, spill, split, reword, new, rebase, and pending time-travel replay. Existing primary output remains first, followed by deterministic full ref names and abbreviated commit mappings. TUI edits continue to consume selections without printing command output.
 - <csr-id-f361fa63b1dff4676d1e8c63a5e5d28c3f1313ab/> include untracked files in tix new
 - <csr-id-ea5ba54a2b38102b35465e765dea68ac2a4946d1/> add tix new command
 - <csr-id-c544a7527161e3e0281d698cab38f72e6ab6b2f5/> edit tix enrichments with commit messages
 - <csr-id-89bc8532ba50fc0a1adf20c465d2451c8c252ea4/> improve tix travel, stash, and scrolling
   Animate TUI time-travel by following rebased commits at up to 60 fps, add the command-line stash operation, and make title-aligned history rows horizontally scrollable.
 - <csr-id-9c72c9a7fb3280d62be672ec466a118c05a28576/> italicize attached tix HEAD markers
   Attached and detached HEAD rows previously used the same graph marker styling, leaving attachment visible only through reference labels.
   
   Use the existing current-worktree branch decoration to italicize only an attached graph @. Preserve signature colors, selection reversal, descendant emphasis, and upright detached markers.
 - <csr-id-7324aa4de97f9ce557aefe99a12626235a967f3b/> align visible tix history columns
   History alignment previously fixed the entire metadata stream after the graph, making dates and authors align even when only commit messages needed a stable starting point.
   
   Make the bracket shortcut cycle title, full-column, and unaligned modes using widths from the current viewport. Full-column rows scroll as one padded line so clipped metadata remains reachable, while flat command and todo output stays unchanged.
 - <csr-id-e0ad9c084e5f56ebc904153fb9047b16b76710a0/> separate staged and unstaged tix changes
   Worktree paths previously relied on color and summary counts to distinguish index entries from unstaged changes, leaving the boundary unclear when both groups were visible.
   
   Insert a centered, scrolling index divider between the groups. Treat it as a non-selectable display row so cursor actions retain path indices, page movement skips the divider, and overflow counts include only hidden paths.
 - <csr-id-f7b6b5a736697ce861683a5e9378f5173b1ddff9/> add per-worktree commit enrichments
   Store human-readable todo and note metadata in refs/worktree/tix/enrich, keyed by effective change IDs so enrichments survive rewrites while remaining private to each worktree.
   
   Expose independent 🚧 todo and 📝 note controls under the enrich prefix. Render history titles, commit messages, enrichment notes, and Git notes as Markdown while keeping trailers structured and the commit-panel background uniform.
   
   Pad the commit view and inset transient notices so enrichment content and application feedback remain readable without reducing the normal content width.
 - <csr-id-9aee7b1f656b01735f82fdb840ac4e8576ea5911/> show progress while time travel signs commits
   Interactive time travel previously blocked the terminal while pending commits were replayed and signed. Reuse the todo-application worker and delayed modal, with shared replay progress that aggregates commit, cherry-pick, signature, and timing counters across discovered batches.
 - <csr-id-5ebe4eb643db542243105461d23901c538c66ec3/> show application notices above changes
   Operation feedback previously replaced the main footer, hiding status and controls while also truncating longer messages. Render shared severity-colored notices in reserved wrapped space aligned with worktree changes, and use the same surface for persistent confirmations in history and ref-tree views.
 - <csr-id-79d046a6b2cb3273972d4f28485b2285c53505d0/> stabilize tix history with change IDs
   Hide history IDs by default and add v i to cycle commit IDs, change IDs, and automatic display. Scan complete scenes in the background so duplicate identities use mixed change/commit prefixes without affecting cursor rendering.
   
   Persist canonical change-id headers only when commits are rewritten, preserving identity across descendants, squashes, reviews, and splits. Keep CLI show, rebase todos, and copy operations commit-ID based.
 - <csr-id-d7c421f27463eb5a028436dd892313a8b197b813/> infer hidden branches from remote HEADs
   Reverse-map symbolic remote HEAD targets through their fetch refspecs for tix show and tix rebase todo. Add --no-auto-hide while leaving interactive history and ref-tree hiding explicit.
 - <csr-id-c8f4337276347ceb23a1c2161a2964af0d4e6a3d/> add non-interactive history display
   Add tix show with a mandatory hidden boundary and optional tips. Reuse the TUI history traversal, lanes, decorations, pins, mailmap, and default metadata while emitting complete plain-text output.
 - <csr-id-da6d0e80cd9b89735bdb73127cdfefb819554c3c/> add index-only command-line amend
   The default amend command intentionally falls back to tracked worktree changes when the index matches HEAD, but scripts sometimes need to guarantee that only explicitly staged content is consumed.
   
   Add tix amend --index as a command-line-only strict mode. It uses staged content when present and reports the existing no-op result when the index is unchanged, leaving worktree-only changes untouched; TUI and default amend behavior remain unchanged.
 - <csr-id-34df4a2988376606edb20f4488ad9a56a88cfa72/> recover reviews with missing return pins
   Older reviews can retain a return-pin name after another review has consumed the shared pin. Finishing previously stopped at an unrecoverable missing-reference error even though the reviewed history remained valid.
   
   Treat only an absent recorded return ref as a request for recovery. Constrain history navigation to visible non-review descendants of the reviewed tip, then let Enter finish onto the selected commit with detached checkout semantics; Escape leaves the review untouched.
 - <csr-id-5f20c48f7f4c26e2999cd7a79ed3ee90bcf8c00d/> show saved review state in history
   Active review auto-stashes were deliberately kept out of traversal, but that also hid whether a review leaf had worktree state waiting to be restored.
   
   Resolve each internal review stash commit to its first parent and render the existing stash marker on that saved leaf. The internal reference name and stash commit remain absent from history topology.
 - <csr-id-b78ba47c8ee992aaabaf81d80c071251d00e6d74/> color ref-tree nodes by history visibility
   Track the completed history projection separately from the expanded ref-tree graph. Referenced commits already present in history use the current-history cyan, while linked-worktree nodes outside history use dark green.
   
   Update the cached color state only when lane computation finishes so cursor drawing remains allocation-free, and cover the visibility precedence in the interactive renderer test.
 - <csr-id-e6cdd8e4d2f3849a2e3e02932ec1f15812ea9bcd/> distinguish ref-tree counts with bullets
   Append U+2022 BULLET to exact commit counts in the interactive ref-tree, producing compact labels such as `500•`. Keep U+25CF BLACK CIRCLE exclusively for actual topology nodes so the two visual roles remain distinct.
   
   Update the rendering assertions and behavioral specification to record the count-unit convention.
 - <csr-id-b14f003d79496d18a7a26d7ca1e99c49d1963746/> show author dates by default
   Retain the author timestamp while decoding commit metadata and make it the history row default. The view date action now cycles author date, committer date, and no date, preserving the prior ability to hide the column while exposing both Git timestamps.
   
   The footer identifies the active date source, deferred metadata carries both timestamps, and rendering tests use distinct dates to protect the cycle.
 - <csr-id-c7815ec4708e6f5f17dfa10786320ca674f16f7a/> pin selected ref-tree references into history
   Make Enter on a reference-backed ref-tree node create or reuse symbolic current-worktree pins for every displayed local, remote, tag, or review reference. Synthetic topology nodes and detached worktree labels remain inert.
   
   Symbolic pins now augment attached history regardless of topology, unlike direct preservation pins which remain limited to descendants. Returning from the ref-tree refreshes history and selects the pinned commit, so unrelated worktree branches can be brought into the editable view deliberately.
 - <csr-id-1afaf20c6835afd5d12cc58ad4107ce19608d674/> show foreign detached worktrees in the ref-tree
   Keep each worktree checkout name separate from its symbolic branch label. With --worktrees, retain both the actual HEAD and the valid symbolic HEAD-pin target as traversal tips, and track the target branch so later movement refreshes the view.
   
   Render a detached foreign checkout as directory@ at its actual HEAD and its remembered branch as a star-prefixed label at the branch tip in both history and ref-tree. Reserve the pin marker for a detached current worktree, and fall back to the worktree administration name when no directory basename is available.
 - <csr-id-3ef6807527962a530c07ef238d91d15942ecc155/> move remote tree deletion to e r
   Keep remote-reference deletion inside the tree node edit prefix alongside local branch deletion. After e resolves uniquely reverse-mapped remote references, r immediately submits the grouped remote deletions and the footer advertises that action.
   
   Remove the Shift-D shortcut instead of retaining a compatibility alias, and update the behavioral specification and key-path regression test to match.
 - <csr-id-0bf7b83fa147a73002efcf85b6562086a1e804b2/> visualize worktrees in the tix tree
   Read each worktree's symbolic HEAD or private HEAD pin so remembered branch labels stay at the branch's actual tip while detached checkout commits receive their own marker. Attached and fallback labels retain the history view's current- and linked-worktree forms.
   
   Remove tree pin creation and removal along with ordinary pin anchors. The tree and its ASCII diagnostics now reserve the pin marker for detached worktrees, making detached checkout state visible without exposing internal pin references.
 - <csr-id-4c5be5241669347e92642438ced86aa58b6c34a2/> delete remote references from the tix tree
   Resolve selected remote-tracking references through their configured fetch mappings before offering deletion. Shift-D pushes exact deletion refspecs per remote with an interactive terminal, continues across failures, and refreshes successful removals without disturbing the existing local delete action.
 - <csr-id-5065f5926148f82f21123d37e9b0851fd6f09e2a/> manage pins and count anchors in the tix tree
   Make Space toggle an explicit reachability-count anchor so cursor navigation can reuse its overlay and placement instead of recomputing counts on every move. Preserve a visible anchor across refreshes and clear or relocate it when topology changes, including tag filtering.
   
   Render ordinary pins as one marker without exposing their internal names, and let `p` remove pins or create one at a visible reference or raw tip. Unique visible references produce symbolic pins while ambiguous targets remain fixed; selection falls to the nearest surviving tree node when pin or branch removal eliminates the current node.
 - <csr-id-827c1d126a541743d00c269fb5990f2ec4263749/> delete local branches from the tix tree
   Add an edit prefix to tree nodes that gathers every eligible local branch at the selected commit and submits one multi-reference `Repository::delete_local_branches()` operation immediately with `e d`. Checked-out worktree branches and the branch remembered by the HEAD pin remain protected, and deletion deliberately performs no merged-state check.
   
   Keep the tree interactive around the operation: Escape cancels only the armed edit prefix, success schedules a refresh, and cleanup failures also refresh when references may already have changed. Status and footer messages name the affected branches without exposing full ref paths.
 - <csr-id-78dde41fc198926db7f9d976c4a220b2e6287fd7/> add a reference tree and broaden rebase ref editing
   Add a rounded first-parent reference projection over the already loaded history graph. It contracts linear runs while retaining references, forks, roots, boundaries, and raw tips, uses all-parent reachability for selection counts, and provides independent spatial or topological navigation plus complete ASCII and Unicode `tix tree` diagnostics.
   
   Reuse the loaded graph and reference snapshot for tree toggling and tag, hidden-history, and worktree filtering so cursor movement never reopens the repository or traverses commits. Extend rebase todo editing to import existing direct refs outside the generated stack and permit attaching HEAD to an imported local branch under the existing compare-and-swap and worktree preflight rules.
 - <csr-id-94ffd843b8800abe0c19f6c5a47c790cee6e03d7/> remember detached HEAD branches with a HEAD pin
   Record the local branch tix detaches from in a singleton worktree-local symbolic pin. Keep it as a reachability tip without exposing ordinary return, reuse, or unpin behavior, and reattach HEAD automatically when travel reaches the branch tip.
   
   Render the remembered branch as ★branch, retain the pin when reattachment fails, and cover moving branch targets, repeated travel, explicit attachment, ordinary pin isolation, fork travel, and failure recovery.
 - <csr-id-33e6c66753cf34737c611753bc822db6fa9a3c18/> offer a WIP author in commit editors
   Show a commented 🚧WIP🚧 author directly below the configured author in new, empty, fork, and split commit editor documents. Uncommenting that single line now overrides the configured author while leaving the existing author date intact.
   
   Keep reword documents unchanged, document the opt-in, and cover both the default and selected-author behavior along with the shared new-commit and split templates.
 - <csr-id-639865115e042b41d4cbf88fe835f0e046c4aeb1/> finish reviews from checked-out successors
   Review-side commits may be created above the dedicated review commit while inspecting and organizing changes. Finishing was nevertheless offered only when HEAD pointed directly at the review root, forcing users to travel backward before integrating work that was already part of the same review line.
   
   Recognize the selected review as finishable when the completed in-memory history shows it in HEAD's ancestry and the worktree is clean. Revalidate the relationship against the edit operation's HistoryGraph, then reuse the existing finish machinery so review successors, the recorded return checkout, and resource cleanup retain their established behavior.
 - <csr-id-6bf8b38e3eb246ec203e100556e1ab32166b9c02/> let tix reword set the author actor
   The non-interactive reword command could replace a message but offered no equivalent way to change the author identity. Agents and scripts therefore had to launch and modify the full editor document even when the desired actor was already known.
   
   Add --author with the same Name <email> validation used by the reword document. Preserve the original author date, continue using the current repository committer, and combine the override with -m or -f without opening an editor. When no explicit message is supplied, prefill the ordinary Markdown document and apply the requested identity even if the editor makes no additional change.
   
   Cover clap parsing, editor-prefill behavior, explicit-message operation, identity validation, and preservation of the message and author date. Document the new command form in the tix specification.
 - <csr-id-e2c45d0256161d19c96d21a54db729a6e3b29fad/> start unambiguous reviews immediately
   Review setup previously entered base-selection mode even when the selected commit had only one selectable strict ancestor. In a two-commit range this required an extra navigation and confirmation step despite there being no decision to make.
   
   Detect that sole candidate from the same reachable and selectable rows used by review navigation, then start the review directly. Preserve the guided selection flow whenever multiple bases remain possible, and document the distinction in the tix behavior specification.
 - <csr-id-152ee6bb5909352ad50e21cf243e2991bbf95b9d/> emphasize active tix shortcut prefixes
   Expanded shortcut groups previously differed from their collapsed form primarily through parentheses and an underlined key. That cue was easy to miss in the dense history footer, particularly when deciding whether the next key would invoke a direct action or a prefixed one.
   
   Reverse every cell belonging to the open view, edit, or information group while preserving its existing colors, dimmed toggles, and underlines. Keep separators and neighboring top-level actions outside the reversed range so the active mode has a strong, theme-independent boundary.
 - <csr-id-2bd9185b8cd15a9f19615425e0d64e08aad29018/> reword commits without opening an editor
   Command-line rewording previously always opened Git's configured editor, even when an agent or script already had the complete replacement message. That made otherwise automatable history edits depend on terminal editor setup and temporary files.
   
   Add mutually exclusive -m/--message and -f/--file inputs. Repeated messages form paragraphs, while -f - consumes standard input. Explicit messages bypass editor discovery and suggested trailers, retain comment-looking content, and reuse the existing signing, lazy-rebase, mutable-reference, pin, and worktree-safety path.
   
   Normalize explicit messages with the same whitespace rules as editor input, but without editor-only comment stripping. Avoid rewriting an unchanged message so its committer timestamp and identity remain stable, and document and test file, stdin, repeated-message, no-op, and validation behavior.
 - <csr-id-5cf72b5e86eb17122853f621877e6fbf03661a56/> reword pinned history from the tix command line
   The TUI can reword any safely scoped commit, but command-line users and agents could edit only HEAD-oriented operations. Rewording an arbitrary revision without a visible descendant scope would risk leaving required forks behind or broadening the rewrite through unrelated refs.
   
   Add tix reword REVSPEC with eligibility checks before editor launch. An attached HEAD may edit itself directly; every other target needs a current-worktree pin at that commit or a descendant. Load HEAD and all pin tips into the edit graph, then reuse the existing Markdown editor, signing, lazy-rebase, ref transaction, and worktree preflight paths so covered forks move together.
 - <csr-id-092b3af20bd6dabfd622b406b6f90b47317b61fc/> add safe command-line time travel to tix
   Tix time travel was available only from the interactive history, even though scripts and agents need the same pending-rebase replay, stash handling, and pin-safe checkout behavior for a resolved revision. Calling git checkout directly would bypass those invariants and could lose sight of detached descendants.
   
   Add tix travel REVSPEC as a thin wrapper around the shared time-travel primitive. Attached departures use normal symbolic pins, while detached past or sideways travel requires an existing pin at the source lineage; forward travel and same-HEAD no-ops remain frictionless. Replay conflicts stay unobservable unless --materialize-conflicts explicitly accepts the standard conflicting checkout and index.
 - <csr-id-4ffedacf8b1f2f580b56fd46aa0a761ec5f51845/> make @ the direct tix time-travel shortcut
   Time travel had become an edit-prefix action even though it is navigation, and removing Shift tracking left no direct key that matched the HEAD marker shown in history.
   
   Handle `@` independently of shortcut prefixes, remove the old edit-group binding, and place the contextual travel or return hint between edit and copy. An expanded edit group remains open so direct navigation does not unexpectedly change modes.
 - <csr-id-dad687747f836a6bcb029929db2c7fc671da2995/> restore tix commit stashes in place
   Commit-associated worktree state was restored automatically only after travelling away and back. A user who chose to remain at the commit had no symmetric way to recover that state, particularly after deciding not to travel.
   
   Offer `unstash` on the selected `@` whenever a commit stash exists, even alongside unrelated worktree changes. Restore through the same indexed apply path used by time travel, consume the companion ref, retain unrelated changes, and refresh cached status.
 - <csr-id-ff55875c479d6112dc270045b95b5c85b3f49777/> stash worktree changes at tix commits
   Time travel and review need a way to preserve staged, unstaged, and untracked work without surrendering the index layout or disturbing the ordinary Git stash stack.
   
   Offer `stash` only from cached, unconflicted status at the selected `@`. Save through Git with untracked files, associate the result with the full commit ID, expose a persistent gift marker, and restore and consume that state when time travel returns, including when apply leaves conflicts for normal resolution.
 - <csr-id-cd6af74283d976734eb25c158466957788770659/> group tix navigation hints under help
   Pane switching and navigation hints occupied the footer continuously even though they describe stable, discoverable behavior. On narrower terminals they crowded out stateful actions that mattered more.
   
   Move switch, move, pan, and diff hints behind the `?` information prefix and reveal them only while that group is expanded. Keep quit outside the group and show transient focus feedback there only when the information menu is visible.
 - <csr-id-1fe3709af46880c31274e2159a72e0ebaf6978b9/> show pinned tix commits with a pushpin
   Generated pin suffixes are implementation details and consumed valuable horizontal space without explaining why the commit mattered. Multiple pins at one commit made that noise worse.
   
   Represent pin presence with one blue pushpin in history metadata, regardless of how many pins point there, and omit their internal names. The row communicates retained history while leaving exact pin identities to rebase todos and ref plumbing.
 - <csr-id-e1fc093498a0956b5d3a75b22a40695aa2fa641a/> follow references pinned by tix
   A direct pin created from a branch name became stale as soon as another process advanced that branch. Conversely, derived revisions such as `main~1` must continue to identify the commit resolved at creation time.
   
   Retain the parsed revspec reference and create a symbolic pin only when the argument directly names the peeled commit. Keep expressions and object IDs as direct pins, deduplicate by target semantics, and verify that symbolic pins follow later branch updates.
 - <csr-id-fa709da3ad45098d200dfa70639845aac88d187d/> pin arbitrary commits from the tix CLI
   History edits and detached travel sometimes need an explicit durable tip, but pin creation was available only through interactive TUI flows. Scripts and agents could not prepare the same safety boundary before invoking edit commands.
   
   Add `tix pin` for one or more revspecs, resolving the complete request before writing so an invalid argument leaves no partial pins. Preserve first-seen order, deduplicate commit IDs, reuse matching direct pins without collapsing distinct symbolic ones, and print each label with its repository-shortened ID.
 - <csr-id-c39f936bb4f9df0198b649c34ec9b9143f3ef4ec/> remove pins from selected tix commits
   Pins deliberately keep otherwise unreachable leaves visible, but repeated reviews and time travel can leave several pins at the same commit. Without an in-app cleanup action, users had to locate and delete implementation refs manually.
   
   Offer `unpin` in the edit menu for a pinned row and delete every matching pin in the current worktree through one reference transaction. Preserve unrelated pins, retain the selected commit across refresh, and report whether zero, one, or multiple pins were removed.
 - <csr-id-f973000ede925f6109426572edd0bee50f352f7d/> restore the reviewed branch after finishing
   Store the branch checked out at review start as the symbolic target of the review ref when it points to the reviewed commit. The ref naturally follows rewrites, and finishing reattaches HEAD after verifying that it reached the finished review commit.
   
   Keep direct review refs for detached HEAD or reviews started away from the checked-out branch, and delete symbolic review resources without dereferencing them during setup rollback.
 - <csr-id-88ed4f09692ff76244ca03cd56d6836d35c6b4a7/> edit references in tix rebase todos
   Render mutable local references as standalone todo lines so rebases can move, create, detach, or delete them explicitly. Keep the commit checkout marker alongside an attached branch marker and require both to agree.
   
   Apply checked-out reference moves through the existing worktree transition path, reject deletion from linked worktrees, and defer deleting the current branch until checkout succeeds. Preserve these semantics across suspended conflict continuations and document the todo contract.
 - <csr-id-4f8124822738d3878b096b509ac036fbb629a36d/> continue tix rebase todos through conflicts
   Keep todo conflicts fully in memory by default, while allowing the TUI and an explicit CLI flag to materialize the partial result with an ordinary unmerged index.
   
   Serialize the remaining editable operation as a self-contained continuation todo, retain unapplied squash sources, move only final references, and resume from the resolved index without hidden sequencer state.
 - <csr-id-2fbe08f29abf19a99ca39df715f6b890693eadee/> show progress while applying tix rebase todos
 - <csr-id-e66aa54120cbcc19d9c878c8d45bf3ebb774ca80/> squash commits in tix rebase todos
 - <csr-id-166fa19a5a3d46f78386a79c361694620c85fd3f/> add self-contained rebase todo commands
 - <csr-id-aa3e6ef2c552dc42aa110d0da58fc2f756932895/> add a top-level tix split command
   Run the existing transactional split flow outside the TUI by sharing its commit-message editor routine. The command amends worktree changes into HEAD and creates the new top commit from staged index changes.
   
   Document the behavior in clap short help and the tix specification, with command-level coverage for the resulting trees and clean index.
 - <csr-id-2944880fa8212a94e925d0e78d5d09e8ce93ef3a/> group tix view status behind question mark
   Replace the history footer actions after the reference toggle with a compact
   question-mark group. Expanding it reveals signature verification state,
   alignment, commit-message visibility, and changes visibility while leaving
   movement, focus, diff, and quit guidance at the top level.
   
   Keep the existing action keys direct so this is only an information-density
   change. The group remains open across those actions, closes on navigation or
   unrelated commands, and is mutually exclusive with the view and edit groups.
   
   Cover collapsed and expanded rendering, colored signature success and failure
   states, prefix lifecycle, and direct question-mark input, and record the new
   footer contract in the tix specification.
 - <csr-id-c1b0d8ef45fd3a055c80f3c6c33a9f6738ab0eee/> amend selected worktree paths in tix
   Offer amend from the edit prefix while a worktree-change path is selected.
   Use the selected staged or unstaged version of that path to rewrite HEAD,
   while leaving every unrelated path out of the commit.
   
   Synchronize only the amended destination and any renamed source back into
   the index after the ref transaction. This retains unrelated staged changes,
   keeps the worktree untouched, and participates in the existing rollback if
   the index cannot be locked or written.
   
   Keep review commits restricted to staged path amendments and reject scoped
   amend while the index contains unresolved conflicts. Cover staged and
   unstaged content as well as additions, deletions, renames, copies, and index
   rollback, and document the behavior in the tix specification.
 - <csr-id-97208f653f8659e05abf9e55287417e40acaab5e/> label external anchors in tix rebase todos
   Fork headings previously made the selected hidden boundary look identical to a
   fork created within the editable history, while rebase-update identified its
   unfamiliar target only through an unlabelled title.
   
   Render the selected boundary as `(base)` and a rebase-update target as
   `(updated-base)`, followed by the same marker-aware title shown in history.
   Keep descendant fork headings terse and rely on the existing display-only tail
   syntax so the annotations cannot change the parsed plan.
   
   Document both external anchors in the generated Markdown help and behavioral
   specification, and test their escaped titles, labels, and parsing semantics.
 - <csr-id-d00c3291140f5e39edc987b0a084b20b0b28cb9f/> distinguish the current symbolic branch in tix
   The commit marker identifies the current checkout, but a plain local branch
   decoration cannot show whether HEAD is attached to that branch or merely
   detached at the same commit.
   
   Classify the current worktree's exact symbolic target from the existing
   worktree snapshot and render it as @branch in the normal local-reference color.
   Keep branches checked out in other worktrees as branch@, and leave coincident
   branch decorations unchanged while the current HEAD is detached.
   
   Preserve reference visibility and upstream lookup semantics, and document and
   test attached, detached, current, and linked-worktree labels.
 - <csr-id-5d4fe474d556fc251a219bda044ac5dfe0c0c84d/> preserve review changes across time travel
   Treat each active review commit and its descendants as one review tree. Keep
   ordinary checkout behavior while moving inside that tree, but use Git's stash
   machinery when time travel crosses its boundary so staged, unstaged, and
   untracked review work can safely follow the reviewer back to the review.
   
   Store the stash under a review-specific companion ref without disturbing an
   existing refs/stash entry. Restore the index with the worktree, consume the
   companion ref after Git returns even when applying reports conflicts, and let
   the existing conflict workflow present any unresolved index.
   
   Keep saved review state internal to history traversal and decorations. Remove
   it together with the review ref when a review finishes, is forgotten, or is
   dropped by a rebase todo.
   
   Document the review-tree boundary behavior and cover same-tree travel,
   boundary crossings, nested reviews, pre-existing stashes, apply conflicts,
   fatal apply results, history visibility, and resource cleanup.
 - <csr-id-f9152fe0f9952f1b54f2eb27b95729cbea206735/> add tix review workflows
   Let a review expose the difference between a selected commit and an ancestor as worktree changes backed by an ordinary review commit and a worktree-local refs/worktree/tix/review/N reference.
   
   Offer ancestor selection through the edit prefix, preserve hidden fork points as selectable review bases, mark review commits with a stable star, and show review decorations independently of ordinary ref visibility. Review commits amend staged changes only and can finish only from a clean worktree.
   
   Finish reviews by moving the reviewed tree onto its original tip, preserving exact trees for review-side descendants, and lazily reparenting natural descendants without unnecessary cherry-picks. Keep fork topology intact and delete review references atomically when finishing, forgetting, or dropping review commits from a rebase todo.
   
   Document the lifecycle and cover start, finish, navigation, rendering, and resource cleanup.
 - <csr-id-80bedc35342f180e81394b19360174d65f9d7341/> make tix rebase todos self-documenting
   Put a brief help pointer on the first line of every history-rebase todo and move complete editing guidance below the plan. Keep every instruction inside Markdown comments so help remains visible to editors without participating in parsing.
   
   Document picking, dropping, reordering, forking, joining, empty commits, checkout selection, lazy replay, reference updates, pins, and conflict behavior alongside the editable plan.
 - <csr-id-92fc1e807c4a64c6ca73d272e0bae2590df80d28/> create explicit empty commits in tix
   Distinguish commits created from tracked index or worktree changes from deliberately empty commits. Advertise both choices from cached worktree status without opening a repository merely to expand the edit menu, while retaining authoritative validation when no cache is available.
   
   Keep untracked paths out of implicit new commits, preserve the complete index and worktree when creating an explicit empty commit, and reject either operation while the index is conflicted. Document the resulting edit semantics in the tix specification.
 - <csr-id-12902f2f34e53a34624d292725110baa09ef7461/> rebase tix history onto updated hidden branches
   Offer rebase-update when a hidden local branch has advanced beyond the selected fork point. Seed the existing history rebase editor with that current branch tip while preserving the visible stacks, transaction rules, lazy replay, conflict handling, and the hidden reference itself.
   
   Retain the hidden tip alongside the displayed behind count and label its otherwise unfamiliar fork heading with the title shown in history.
 - <csr-id-2fd1600e2759e56ce243a095a9e884f4c258b513/> fork commits from any tix history entry
   Allow hidden boundaries, merges, and ordinary commits to become parents of an independent commit without rewriting their descendants. Reuse new-commit preparation and signing, retain the new leaf with a temporary tix pin, and automatically time-travel to it.
   
   Centralize HEAD movement and pin reconciliation so ordinary time travel, post-rebase checkout, fork checkout, and conflict acceptance preserve departures, consume destination pins, and clean up redundant pins consistently.
 - <csr-id-abe38b7e9e17b214036a05fb7d1791a2ba649319/> edit forked history from hidden bases
   Offer a Markdown rebase todo when a selectable hidden boundary anchors merge-free visible descendants. Preserve forks while allowing picks to be reordered or dropped, new forks and empty commits to be added, and the worktree checkout marker to move.
   
   Replay only the ancestry ending at the checkout marker eagerly. Keep other stacks lazy, retain unreferenced leaves with tix pins, and update the pre-editor mutable-reference snapshot atomically so concurrent ref changes remain authoritative.
 - <csr-id-3e2a260669be84d2361012c922520452a49c9b8d/> inspect hidden branch bases in tix
   Make hidden ancestry boundaries selectable for read-only inspection. Reuse the cached history graph to identify boundaries with one visible descendant leaf, then use the existing tree-diff pipeline for base-to-tip stats, changed paths, and full diffs.
   
   Keep ambiguous multi-leaf boundaries on their ordinary commit diff and continue excluding hidden rows from editing, signature verification, time travel, and Shift reachability navigation.
 - <csr-id-a422c769f3809d558050ea746ab6913094def6ea/> order tix history status actions
 - <csr-id-5681a6940697327d71d740a4c1a7b771522b524e/> resume conflicting lazy rebases in tix
   Keep all history edits lazy and suspend unresolved time-travel cherry-picks in memory. Let the user materialize the prepared conflict for normal index-based resolution, while preventing further time travel until the index is resolved.
 - <csr-id-360d096b3da4c4a3b6b4f7546c33ffcecb05ae75/> streamline tix history actions
   Make the history footer read in action order: view and edit prefixes, copy, focus and display actions, reference and signature controls, navigation, then quit.
   
   Use m for the commit-message view, shorten the changes label, and render the Tab key consistently. Add a plain r reference-visibility toggle that restores the exact display mode it hid while leaving v r as the independent mode cycle.
   
   Update the behavioral specification and focused UI and input tests.
 - <csr-id-931ed71a5b6b2ec2fc9d0698dd037618691c2de1/> mark reword rebases for lazy replay
   Use Tree::LeaveAsIsAndMark when rewording so the changed commit and its linear descendants retain their trees while carrying the pending rebase marker. Time travel can then replay the marked region with normal cherry-picking and signing.
 - <csr-id-9fae9998afaf84cfc7dd2ceb4149de57dc41e119/> split staged and unstaged HEAD changes
   Offer e p only at the current worktree HEAD when staged and unstaged changes coexist. Amend the unstaged delta into the source commit, then cherry-pick the staged delta into a newly edited upper commit.
   
   Prepare both tree applications in memory before opening the editor, and reuse the rebase transaction for signature handling, mutable-ref updates, and index reset without touching worktree files.
 - <csr-id-a81651e5d4379057c4bd88de388255ab379f662b/> expose hidden branch divergence in tix
   Use named hidden local refs and the persistent history graph to find their best common bases with the visible view. Render the view-relative missing count as a right-aligned behind marker with stable margins, retain normal reference display, and tolerate unavailable hidden revisions when another requested hidden revision resolves.
 - <csr-id-abda17ec079a35f6280ec2359379391b7761a23e/> spill selected tree paths
   Keep the history edit prefix available while the tree-changes block has
   focus and scope spill to its selected path. Restore that path from the
   currently displayed parent while retaining every other tree change, then
   reuse the existing lazy rebase transaction to preserve worktree files,
   reset the index, invalidate signatures, and update mutable refs.
   
   Leave the command-line edit interface unchanged so tix edit spill continues
   to spill the complete commit.
 - <csr-id-483f248a3926ce07e713983f77e29c5821358778/> amend and spill commits with lazy rebases
   Add edit actions and CLI commands for amending HEAD from the index or
   worktree and spilling its tree delta back into the worktree. Preserve
   worktree files, reset the affected index, and atomically retarget mutable
   references through the shared rebase transaction.
   
   Mark cheaply reparented commits as pending with their original parent so
   time travel can cherry-pick the complete deferred region correctly. Show
   pending commits in cyan, invalidate stale signatures, and redo configured
   signatures when the deferred rebase is completed.
 - <csr-id-52340bfa84fe2d5fbf7da709c74d25559dedabf5/> trace tix edit durations
   Instrument reword, create, forget, rebase, editor, and time-travel operations at both the user-action and internal phase boundaries. Exclude repositories, graphs, terminals, and message buffers from span fields while retaining commit and mode context.
   
   Emit span close events through the existing diagnostic logger so each completed phase records tracing-subscriber busy and idle timing.
 - <csr-id-2df159a2dd59eca8b13509840ed8465125bb1283/> rebase linear descendants for tix history edits
   Allow reword, commit insertion, and forgetting from arbitrary points in a linear history instead of limiting edits to the newest commit.
   
   Introduce one transactional edit::rebase primitive that prepares all rewritten commits and cherry-picked trees in object memory, preserves forks, rejects descendant merges, and aborts the complete operation on conflicts. Mutable local references across the rewritten set move in one compare-and-swap transaction; tags and remote-tracking refs remain immutable.
   
   Model tree handling explicitly with LeaveAsIs, LeaveAsIsAndMark, and CherryPick. The marker form writes tix-rebase: pending, and Repeat resumes from a marked base, cherry-picks the marked range, and clears the markers. Model signatures with RedoIfNeeded and InvalidateExisting, respecting repository signing configuration while applying one current committer identity and timestamp to automatically rebased descendants.
   
   Preflight every affected accessible worktree through Git before refs move. Apply checkout transitions only after all preparation succeeds, and roll back already-updated worktrees and refs if a later checkout fails. Commit insertion preserves worktree bytes while updating affected indexes to the new committed tree. Stale unrelated linked worktrees are ignored.
   
   Route reword, create, and forget through this primitive and remove the former parallel MutableRefs implementation. Permit edits with linear descendants while retaining merge safety, and update the behavioral specification.
   
   Add scenario fixtures and snapshots for middle-stack rewrites, removal with tree transplanting, deferred rebase replay, and conflict atomicity.
 - <csr-id-6588c60621accee918d6163817d624e6793f0c9f/> streamline tix status shortcuts
   Replace redundant prefix-key-plus-verb labels with compact action labels whose shortcut characters are underlined. Apply the convention consistently to the history footer, expanded view and edit groups, changes blocks, and commit-message status, while keeping navigation chords and other keys explicit when they cannot be embedded naturally.
   
   Order the history footer as position, display and edit groups, top-level viewport actions, and finally navigation. Name the commit-message viewport toggle open message or close message so it cannot be confused with creating a commit in the edit group.
   
   Show active prefix choices directly in parentheses, making the former active suffix unnecessary. Render the Enter key consistently as <enter> throughout history, changes, and diff status bars. Update the behavioral specification and terminal-buffer assertions to cover the labels, styling, ordering, and disabled state.
 - <csr-id-95e22e1146d2ab20f8b8774b1b28b39c28d77a3f/> prioritize tix prefixes in history status
   Place the view prefix immediately after the history position and the edit prefix immediately after it, before navigation and other direct shortcuts. Keep active prefix groups in the same priority position so their available commands remain visible on narrow terminals.
   
   Do not advertise the edit prefix while the view prefix is active because `e` then toggles email display. Preserve all key handling and contextual availability semantics.
   
   Update the behavioral specification and footer regressions for inactive, active, deferred-progress, and narrow-history layouts.
 - <csr-id-f3ddb9412e5069e5ea3af4dc08ad15288418045a/> standardize transient tix messages
   Make App own the single path for global user feedback instead of allowing event-loop and rendering code to write its footer notice storage directly. Global command results and worktree-disappearance recovery now use `leave_message()`, while pane-specific errors stay with their panes and the next recognized action clears the message centrally.
   
   Explain an unchanged new-commit editor with `no commit created: no input was provided` instead of silently returning to history. Preserve empty edited messages as errors because those represent invalid supplied input.
   
   Document the transient-message lifetime and cover footer replacement, the no-input text, and clearing on the next action.
 - <csr-id-77ff0f941b372149f790e9ee172e76a4bce449f9/> clarify active tix shortcut prefixes
   Make the main footer show when the display or edit prefix is active instead of replacing the prefix label with an unbounded run of shortcuts. Enclose the applicable commands in a bold `v active (...)` or `e active (...)` group so direct shortcuts remain visibly outside it.
   
   Keep display-toggle state dimming, omit edit operations that cannot act on the current selection, and show `no actions` when an edit prefix has no applicable command. Document the footer contract and cover grouping, emphasis, contextual edit actions, hidden-history availability, and the empty state.
 - <csr-id-dadaa554ec39206bab5152962453d58a60d79263/> show net lines in tix diffstats
   Keep Git-compatible per-file churn totals and scaled insertion/deletion bars, then append a right-aligned signed net line count computed as additions minus deletions. Color positive deltas green, negative deltas bright red, and zero neutrally; retain binary rows as Bin without inventing line information.
   
   Use the shared summary renderer so whole-commit built-in views, pager input, external-diff summaries, and commented new-commit editor diffstats agree. Preserve parent and aggregate insertion/deletion summaries.
   
   Cover positive, negative, zero, differently sized bars, binary files, streamed pager bytes, and the commit-editor template. Update the tix behavioral specification.
 - <csr-id-adb4eccf9836a17dd712b87903b073b4a1840c1f/> forget top commits from tix
   Offer `e d d` only for a completed, selected non-merge commit with no known descendants. Make the first `d` arm an explicit status-line confirmation, and cancel it on navigation, refresh, cancellation, selection changes, or another command.
   
   Reuse the shared all-mutable-ref transaction used by reword and create. Retarget local branches, custom refs, direct pins, and detached HEAD to the sole parent; delete matching refs for a root while excluding tags and remote-tracking refs. Preserve attached HEAD, reject affected branches checked out in another worktree, support ref-only operation without a worktree, and leave an attached root branch unborn.
   
   When the selected commit is current worktree HEAD, use a temporary-index `git read-tree` preflight and a two-tree update to discard only the tracked delta introduced by the commit. Reject overlapping tracked, staged, index, or untracked conflicts while preserving unrelated untracked files; roll references back if checkout application fails. Refresh cached history and retain the parent selection.
   
   Add full-state scenarios for ordinary tips, conflict isolation, root-to-unborn behavior, bare repositories, detached-root rejection, multi-ref retargeting, and immutable tags/remotes. Update the tix specification and shortcut coverage.
 - <csr-id-cd7e0434c361e54d6eae8dd70116693eb7cdc095/> create commits from tix
   Restrict rewording and commit creation to commits without descendants in the completed in-memory history graph. Offer `e n` for a live worktree, including unborn repositories, without retaining a repository in the UI.
   
   Resolve identities, signing configuration, every mutable direct ref, linked-worktree safety, index conflicts, filters, the candidate tree, and its per-path diffstat before opening the editor. Keep provisional objects in object memory so cancellation and preflight failures leave refs, index, object storage, and worktree untouched.
   
   Let a changed index win over unstaged files; otherwise snapshot worktree changes only when HEAD is based on the selected parent, falling back to the parent or empty tree. Present a Markdown what/why template with optional attribution trailers and a commented Git-style diffstat.
   
   After editing, revalidate all destinations, sign when configured, persist prepared objects, and atomically advance every mutable direct ref pointing at the parent, including local branches, custom refs, direct pins, and detached HEAD. Preserve attached HEAD and unrelated worktrees, exclude tags and remote-tracking refs, reject branches checked out elsewhere, and align the current checkout without running hooks. Add scenario coverage for staged precedence, worktree snapshots, unborn roots, multi-ref updates, and repository-state isolation.
 - <csr-id-ff8a7ac919ea634604e139a4e12b657d26f2093c/> focus and flag non-tip HEAD in tix
   Select the current worktree HEAD as soon as its history row becomes available during startup, while allowing user navigation to cancel the pending jump and retaining the normal fallback when HEAD is outside the view.
   
   Warn when the current HEAD has visible descendants by underlining its unselected history row and bolding the @ marker without changing signature, selection, or other row colors.
 - <csr-id-797d6ddc2c01b00b532dcf425c7da19579020fc2/> group tix editing shortcuts
   Put commit rewording and time-travel checkouts behind an e prefix, matching the existing view shortcut group's toggle and dismissal behavior. Keep the overlapping e, r, and t display commands unchanged while the view group is active, and remove the former direct mutation shortcuts.
   
   Advertise the edit group in the history footer, expand it to only the actions available for the current selection, and document and test the key-routing and group-state contract.
 - <csr-id-ff25141fa396d5179425cc6058e57e1be39ca780/> show other worktree checkouts in tix
   Decorate commits checked out by other main or linked worktrees with light-blue name@ labels, replacing the ordinary branch label. Keep the checkout from which tix was opened represented by the graph @ marker and its ordinary local reference, and use directory basenames for other detached worktrees. Keep other-worktree labels visible on the selected row when references are hidden.
   
   Add -w/--worktrees to include every successfully resolved worktree HEAD, including the current checkout, alongside implicit or explicit traversal tips without weakening hidden-revision exclusions. Discover worktrees from their private Git metadata so stale checkout directories remain useful, while malformed, unborn, or inaccessible entries are logged and skipped.
   
   Watch linked HEAD and worktree membership changes so decorations and optional tips stay current, while ignoring unrelated linked indexes, logs, and metadata. Document the behavior and cover current, attached, detached, stale, malformed, hidden-reference, CLI, and watcher cases.
 - <csr-id-74898b5bc23d54422e568d1fe2e09f1b4fea868d/> add time-travel checkouts to tix
   Let t detach HEAD at the selected commit through git checkout while preserving descendants that would otherwise disappear behind namespaced refs/tix/pins references.
   
   Discover applicable pins for detached HEAD even with explicit revisions, follow symbolic branch targets as they advance, and return through a selected pin by restoring its branch or detached commit before removing it. Use the cached history graph to decide whether the old tip needs protection and discard provisional pins when existing view tips already retain it.
   
   Keep malformed, dangling, and non-commit pins out of history, surface checkout failures without forcing local changes, refresh history and worktree state after success, and document the lifecycle and UI contract.
 - <csr-id-8aa8235dbea4c5b55c0e6d8a46adb349190a94ef/> use a Markdown buffer for tix rewording
   Give the temporary commit-edit document an .md suffix so editors can select Markdown syntax highlighting while retaining automatic cleanup and the existing Git-selected editor flow.
   
   Record the filename contract in the tix specification.
 - <csr-id-b7cdebc331b29d217fa5d55fa8ec5e5e8e8d1862/> mark HEAD and dirty worktrees in tix
   Replace the HEAD commit disk with @ while retaining signature-state and selection coloring, so its position remains visible independently of reference label settings.
   
   When displayed worktree changes are non-empty, place D in the left marker column on HEAD. Keep > on a separately selected commit and restore the normal marker when the worktree pane is not rendered.
 - <csr-id-834ffff798f728de9a70f029d4185095e42076d7/> reword the newest commit in tix
   Allow r to edit the top-most selectable commit using the Git-selected editor and a structured author, committer, date, comment-prefix, and message document. Offer missing GPT 5.6 Assisted-by and Co-authored-by trailers as semicolon-prefixed comments that users may opt into by removing the prefix, without repeating trailer keys already present regardless of their values.
   
   Apply Git-compatible comment and whitespace cleanup after editing, with a configurable non-empty line prefix that defaults to a semicolon and only matches at column zero.
   
   Recreate the commit with configured signing when enabled, then atomically retarget direct references that still point at the old commit while excluding tags and remote-tracking references. A detached HEAD is updated as well.
   
   Keep repositories short-lived around editor invocation so tix retains no object database handles while idle.
 - <csr-id-b3e49b50a2d1341745cf860c0b43d41c16ffed1a/> shade the tix commit panel
   Replace the commit message panel border with a subtle background derived from the terminal theme. Query the terminal background once at startup, shift it by one sixteenth toward the opposite luminance extreme, and retain the default background when detection is unavailable.
 - <csr-id-94147bbad023ca9ebed0ce9e64a5341af2884a71/> highlight unseen filesystem redraws in tix
   Replace the main status separators with prominent orange commit-style discs when a filesystem-attributed frame is presented while the terminal is unfocused. Keep the indication across later redraws and restore the normal separators immediately when terminal focus returns.
 - <csr-id-c0baddade03ebbba1d3a5614f05e9d995594a913/> open whole-commit diffs from tix history
   Make Enter on a history row display the selected commit against its active comparison parent. Reuse the existing file-diff preparation so attributes, binary handling, external diff commands, configured pagers, and the built-in viewer behave consistently with changed-path diffs.
   
   Prefix whole-commit output with the selected row identity and a Git-style diffstat in tree order. Show each changed path with its line total and a scaled additions/deletions graph, followed by the comparison parent, per-kind file totals, and aggregate line counts. Mirror the history mailmap and email display, and derive all line counts from the diff already being prepared.
   
   Show the summary and internal patch before per-path external diff drivers. Allow Enter to continue into those drivers while q or Escape returns directly to history.
 - <csr-id-47e949735556738cecd83713dd449d010d8a173b/> correlate tix filesystem responses in diagnostics
   Assign a monotonically increasing response ID when the first actionable reference or relevant worktree notification arrives. Coalesce subsequent events under that ID until the existing debounce expires, and record batch counts, event-kind counts, rescans, and semantic triggers for HEAD, index, packed refs, loose refs, Git metadata, and worktree paths.
   
   Include up to sixteen deduplicated trigger paths and report how many additional unique paths were omitted. Recognize transaction lock files as the corresponding HEAD, index, or packed-refs trigger so the initiating Git operation remains clear.
   
   Carry response IDs through worktree invalidation, reference comparison, history refresh, lane computation, delayed status display, and history emphasis. Attribute every filesystem-caused presentation to all accumulated reasons, allowing multiple responses and phases to identify a single coalesced frame.
   
   Finish each response with its elapsed time, presentation count, and outcome, including superseded or interrupted emphasis and watcher failures. Keep keyboard and mouse redraws out of the trace except when they terminate an active filesystem emphasis.
   
   Preserve existing refresh and redraw behavior so diagnostics expose the current cause-and-effect chain without changing it. Cover trigger classification, batch coalescing, bounded path collection, rescans, new response IDs after debounce, and overlapping causes in one frame.
 - <csr-id-4e60df723118f72b792791f7e7e917aef52f7f81/> emphasize tix filesystem history changes
   Present completed filesystem refreshes immediately instead of interpolating between terminal buffers. Briefly bold only new or replaced visible history rows, then settle to the ordinary target after 180ms.
   
   Match rows by commit ID first and tree ID second so amended messages and authors retain their visual identity while a new object ID is emphasized. Keep duplicate-tree matches in display order, and load tree IDs only for changed visible commit sequences through a fresh cacheless repository.
   
   Leave removals and unrelated branch replacements immediate because neither has a useful on-screen anchor. Make show-hidden, hide-hidden, manual refresh, worktree-only updates, navigation, and pane changes immediate as well.
   
   Do not block refreshes, redraws, or input while emphasis is active. Any interaction clears the temporary styling, and a subsequent filesystem refresh replaces it from the last logical target.
   
   Snapshot every distinct frame for reword and new-top-commit changes with insta. Omit unchanged hold ticks and document the cargo-insta review workflow.
 - <csr-id-c193fdc52a318ee18d3446e9ec64decf2192a5e6/> persist tix diagnostics to OS log storage
   Write daily, non-ANSI tracing logs to the platform-standard application log directory so watcher and refresh failures can be diagnosed after the terminal UI exits. Keep seven days of logs and install the subscriber only for the calling thread so embedding tix cannot replace an application-wide tracing subscriber.
   
   Logging is best-effort: initialization failures are reported before terminal setup and do not prevent tix from starting.
 - <csr-id-27a0a3c293bca62f59a9f5690289b0903d8eb0fd/> copy selected changed paths in tix
   Make the existing y shortcut copy the selected path when either changes block has focus. Preserve raw Git path bytes and retain commit-id copying when history has focus.
 - <csr-id-bd3a6d0edde5c8701fe789dfc2bceaebf8b83a1a/> group tix history display shortcuts
   Collapse history presentation controls behind a `v view` prefix to keep the
   main status concise. Expand date, actor, mailmap, trailer, reference, and hidden
   history controls on demand while leaving alignment and overlay panes direct.
   
   Keep the display group open for consecutive presentation changes and collapse
   it after navigation or any other recognized command.
 - <csr-id-2b2afd13d0274ed1b97d4d66bf615454c2dc247d/> coordinate tix overlay pane layout
   Lay out the commit message and change blocks within a shared overlay region so
   they no longer paint over each other. Reserve the commit message width first
   and let tree and worktree changes adapt within the remaining space.
   
   Delineate the commit message with a left border and move its title onto the
   first pane row while retaining its padding and scrolling status.
 - <csr-id-cd79529ac0b0ffff1059efa17b5062b8a27d482c/> show the selected history row in tix
   Keep the live commit count in the history status bar while traversal, cancellation, and lane computation are active. Once the completed graph is displayed, replace it with a reverse row number for the current selection.
   
   Number displayed rows from the bottom so the oldest row is #1 and the top-most row is the total number of commits. Retain the commit count for empty histories where no row can be selected.
 - <csr-id-be77515141960c7f2cbf0f17d80f45423e33efda/> navigate tix history with mouse scrolling
   Capture terminal mouse input while tix is active and restore it across screen transitions and exit. Map vertical wheel and trackpad events to history movement and horizontal events to the existing pan actions.
   
   Treat vertical scrolling over the history like repeated keyboard navigation: hide the changes blocks, retain the temporary fill repository, and restore both after the existing 75 ms idle window. Keep scrolling within a focused changes block visible, and ignore clicks, drags, and pointer movement.
   
   Apply every scroll event faithfully while rate-limiting redraws to the existing frame interval. This lets fast wheel bursts drain from the terminal queue without rendering and refilling the selected view after every event.
 - <csr-id-5a7ed55d9c11fc1c69b1b1b487c0038b75afa7d8/> show tree and worktree changes together
   Show Tree and Worktree changes together by default and cycle c from Both to
   Tree to Hidden. Collect staged, unstaged, untracked, and conflicted paths with
   the cancellable status iterator, preserve Git-like ordering and colors, and
   reuse computed per-path line counts for summaries, selected rows, and the
   existing external or built-in diff pipeline.
   
   Watch the worktree and index only while needed, debounce updates for 75ms, and
   retain independent caches, selection, scrolling, and errors for both sources.
   Represent conflicts, submodules, unavailable diffs, and an enabled clean
   worktree without launching inappropriate pagers.
   
   Render both blocks over the full-height history, side by side when their
   condensed summaries fit and stacked otherwise. Join unequal borders, cap their
   height, keep history visible beside shorter blocks, and bound navigation above
   the top-most block so advancing scrolls history while the selected row stays
   fixed. Cycle focus in visual order and keep merge-parent controls on Tree.
 - <csr-id-191c4a501ae9a1cc26cec39dbbcce2d1185e2b47/> show contextual information beside the tix selection
   Reserve space before the history selection tail for compact information about
   the selected commit. Reuse bright tree-change line counts, and for commits
   pointed to by refs show one deterministic upstream ahead/behind relation or a
   visible-ancestry count when hidden history exists.
   
   Resolve fetch tracking branches through gix, use commit-graph-aware counts,
   cache results by commit and upstream targets, invalidate them with reference
   or projected-history changes, and reuse the navigation repository and object
   cache during repeated movement.
   
   Keep blank margins around contextual information and the right-hand selection
   marker even when clipped. Clear the marker cell before applying inversion so
   row text is never inverted accidentally.
 - <csr-id-b0d36b6a7c81b9f006388d32dae45eee995cb4f7/> refresh tix history when input references move
   Watch the repository reference stores with native filesystem notifications and extract every direct or symbolic reference used by the view and hidden revspecs. Re-evaluate notifications lazily: traversal-tip changes refresh history, while unrelated reference changes are ignored and visible decoration changes update without retraversal. Defer tip refreshes until the active traversal and lane computation finish, and retain Shift-R only as a fallback when a watcher cannot be established.
   
   Keep an append-only cache of discovered commit rows and their complete parent topology. Incremental walks stop at cached commits, decode only newly encountered ODB commits, and derive the current visible and hidden-boundary projection in memory without pruning commits that leave the view. Recompute lanes off-thread and atomically replace the rendered graph so the old frame and selection remain stable until the refreshed graph is ready.
   
   Reopen isolated repositories with a small object cache for incremental traversal, preserve notes refresh behavior for manual view toggles, and re-evaluate inline versus alternate-screen sizing after the projected history changes. Treat refs that disappear between filesystem enumeration and reading as transient, while continuing to report malformed or inaccessible refs. Add coverage for symbolic reference discovery, missing-ref races, and cached fast-forward, rewind, and restoration.
 - <csr-id-f259b6017b0a158948ccb1208f542013fe334fdc/> enrich commit details in tix
   Show Git notes in history with a bright-purple [N] marker and render their
   contents before trailers in the commit view. Load notes lazily for visible
   commits through the repository notes platform and refresh them with the view.
   
   Make overflowing commit messages page-scrollable with PgUp/PgDn and Ctrl-b/
   Ctrl-f, clamp offsets when content changes, and show pane-specific navigation
   status only while scrolling is possible and the pane is focused. Give pane
   status bars a distinct background without changing the main status line.
 - <csr-id-f6c1d5e006d6936c7ee3220d8e02c5da3ac14f09/> inspect tree changes in tix
   Add a default-open, focusable bottom panel for changed paths in the selected
   commit. Preserve diff order, summarize color-coded change kinds and line
   counts, support merge-parent cycling and path navigation, cap the panel at
   half the screen, and hide it during repeated history navigation. Distinguish
   inactive panels and history, expose focus feedback, and return with q/Escape.
   
   Compute line statistics in a temporary available-parallelism worker pool and
   use a short-lived cached repository for tree changes. Highlight compared
   parents, reuse computed line counts, and open selected file diffs through the
   built-in viewer or Git-compatible external diff and core.pager pipeline,
   preserving output from immediately closing pagers.
   
   Keep aligned metadata stable while panning and resizing panels. Avoid letting
   the default changes view force inline startup into the alternate screen, and
   delay empty loading frames to prevent scrollback residue and startup flashes.
   On exit, retain only the left selection marker in static frames, including
   when leaving an alternate screen, and let Ctrl-C terminate from any focus.
 - <csr-id-2f50f983681ebced61140a1deb3f73115bb24f80/> clarify filtered history in tix
   Retain the selected commit when toggling hidden history, falling back to the
   top only when the commit is no longer present. Hide references together with
   ancestry so filtered rows remain visually aligned.
   
   Retain excluded parents directly connected to visible commits as dimmed,
   terminal-colored boundary rows that show where graph lanes terminate without
   restoring full hidden ancestry. Keep these rows outside selection, paging,
   Shift navigation, signature verification, and selection restoration.
 - <csr-id-1d14fce933c500d0931a08a34e3535f092ddb404/> navigate merge ancestry with Shift in tix
 - <csr-id-7cc4c195739151cb683e7628d93b7e28b965cf93/> present commit authorship in tix
   Improve attribution display while retaining access to complete actor identities.
   Treat every Assisted-by trailer value as an agent, toggle full actors and
   emails while hiding attribution comments, omit classified agent emails,
   italicize GitHub noreply actors, and group attribution keys whose displayed
   values are identical.
   
   Detect agent markers in commit messages and prefix generated commit titles
   with a bright-purple [A] marker shared with the Git notes marker.
 - <csr-id-71273251c11aadb8dfef523eaa96f44362c1b885/> verify visible commit signatures in tix

### Bug Fixes

 - <csr-id-75dca4fbe01cfbfa31625d41cd30be6548644576/> make tests portable to Windows
   Windows runners exposed platform-specific snapshot metadata and filenames, and could not let external editors replace open temporary files. Use portable snapshots and paths, close editor tempfiles before invoking PowerShell-backed replacement editors, and mark shell fixtures executable.
   
   Keep exact-byte and index tests deterministic by disabling automatic line-ending conversion only in their temporary repositories, removing the override before configuration snapshots. Identify the current linked worktree by its canonical per-worktree Git directory so undo and redo update its index when worktree paths have different representations.
 - <csr-id-a7214c49209b9b22fe1892d19505e7bfff55bb71/> keep history position when changes shrink
   Allow a changes pane to push the selected history row upward without pulling it back down when a later pane needs less space. Keep explicit viewport panning bounded and cover the tall-to-short transition.
 - <csr-id-b07cf5ee175c621e037de6e428551ab04d98043a/> keep linked worktrees clean across undo
   Recognize the current linked worktree through repository identity instead of lexical git-dir paths, so direct HEAD undo/redo transitions also update its index and files.
 - <csr-id-8c25e9322e5b8c016c55d8666d25118abd558ea2/> show hidden bases for unborn branches
 - <csr-id-4a4e718d0e8ed68082dcba3b50a32e70b2361bdf/> keep history paging on the cursor
   Restore cursor movement for plain page and Ctrl-page navigation in the history view. Reserve the Shift variants for panning the viewport without changing the selection.
 - <csr-id-2d876ccf7f267a28c4d2b34a7e45fc23fdd992bf/> keep first travel inside visible history
   Infer the default hidden base for relative travel targets and exclude its boundary ancestry from navigation. This makes --to first select the oldest displayed commit above the base.
 - <csr-id-e87b250a7d01702555b104889fe88610919cef9e/> keep time-travel animation on its path
   Animate only commits on the completed destination ancestry while still rewriting lazy sibling stacks. Defer time travel during lane computation so stale refresh results cannot displace the selection.
 - <csr-id-884b6141cffbc58752bbc9f37efdd9779a8abf59/> make ref-tree paging move the cursor
   Make unshifted Page and Ctrl-page input move the ref-tree selection, while Shift variants pan the viewport without moving it. Keep mouse and history navigation unchanged, and update the ref-tree hints and specification.
 - <csr-id-68abfcdb4b86feec6563ef30afbdaf63053b57a3/> retain history selection after time travel
   Restore the animation origin and its viewport row, and retain the mapped time-travel destination through refreshes.
   
   When a refresh no longer contains the selected object ID, match the displayed entry number within its visual base, then keep the nearest prior row before falling back to the top. Schedule refreshes when lazy finalization changes refs without producing a checkout notice.
 - <csr-id-f49faf1998d91eb83faccc237fbc53200446c838/> avoid pins during rebase continuation
   Record externally amended conflict resolutions in the rewrite mapping and preserve that mapping across intermediate materialization. Use it for final continuation checkout so superseded resolution commits are recognized as rewritten instead of pinned as departures.
 - <csr-id-2f7bde8628ae32fb05f1c2b97173d63a7c976598/> keep singleton history rows uncompressed
   Emit ordinary commit rows for one-member linear groups so compressed mode always saves vertical space.
   
   Keep multi-segment expansion and reset coverage with longer fixtures, and document the minimum segment size.
 - <csr-id-0e8c249f051ccd8eb6806517f88e3cecae0ee17b/> finalize amended commits immediately
   Re-sign direct amend roots immediately so signed checked-out commits are never left pending. Keep reparented descendants lazy and cover the signed worktree fallback.
 - <csr-id-1e6ed75804d01f9ab8dc2f6503fc3101daa1ee38/> align side-by-side change panes
   Give tree and worktree panes a shared maximum height so the shorter pane covers history rows beneath it instead of exposing title fragments during alignment changes.
 - <csr-id-980ce79f458a5c874b51d3d15161d314598dfc06/> retain metadata and the final diagnostic frame
   Preserve every asynchronously loaded metadata field when lane computation replaces provisional rows, avoiding epoch dates and lost review state. Render --quit-on-finish inline on the normal screen after the completed frame so its result remains inspectable.
 - <csr-id-3912b9e811d88b17aceb308a7b343b16cf9322a6/> preserve unaffected commits and invert selected rows
   Skip commit writing for final descendants whose effective parents did not change while retaining eager conflict detection for affected checkout history.
   
   Render every selected history row as one inversion through its title and remove the obsolete selection-tail state.
 - <csr-id-c00b72c1c0cc6f667b8bdae784e0d69693a12021/> eagerly finalize empty rebased commits
   Finalize zero-delta commits as soon as their rewritten parent is final, and keep linked worktree indexes synchronized without touching worktree bytes.
 - <csr-id-e4a4dd2eed64a1cf09220944ec234a4822fff7ff/> defer tree changes while scrolling
   Temporarily hide enabled changes panes during repeated history navigation and restore them after the existing idle deadline. Keep this debounce independent from the removed frame animation machinery.
 - <csr-id-b7ad89079cf118379f5bdfafa6c5429c29690739/> distinguish off-worktree selections
   Keep the compact selection treatment for the current worktree, while reversing an off-worktree selection from the row start through its non-title metadata. Preserve an uninverted separator and title.
 - <csr-id-a991892021d6638aa078f32787ef74c9e0befa1e/> keep commit titles beside selected notes
   Prefix the selected commit title with its highlighted note title and an unstyled separator instead of replacing the commit title.
 - <csr-id-6c1eccc228cabfc384fe929a5ddb40becc0754c3/> resolve rewritten checkout conflicts eagerly
   Replay the affected checkout ancestry before committing history edits so forget, reword, insertion, review completion, and todo rebases surface conflicts immediately while unrelated branches remain lazy.
   
   Keep suspended conflicts transactional, recognize valid external conflict amendments, preserve undo bookkeeping, and leave normal history quit available during resolution.
 - <csr-id-2ee1a55dfe28dbe73c2fc0a4f42035168b73b9a5/> call empty tree changes empty
 - <csr-id-7fe620548af77e845d3421d56f3b40c8120c757c/> stop writing after leaving the alternate screen
 - <csr-id-b73b40425fb1f2c07b991b2b07cd6ec8724282b5/> remove Tix UI animations
   Present history refreshes and time travel as complete frames so viewport-dependent alignment is not redrawn through intermediate layouts.
   
   Keep changes panes visible during navigation and make conflict markers steady, leaving Ratatui responsible for normal buffer diffing.
 - <csr-id-32eb3ebdbf47ee93e60e5faaaa37fdbafb94ab8a/> avoid duplicate checkout when returning from Tix travel
   Reuse an available symbolic HEAD pin as the checkout target when returning to its branch, avoiding an intermediate detached checkout and its extra worktree traversal.
   
   Complete pending rebases using only the affected rewritten path so unrelated history cannot delay or block time travel.
 - <csr-id-c9875ff71e1ba766604c7d503fae1e54b7cba20b/> keep Tix edits within the current history view
   Exclude obsolete commits retained by the append-only history cache from descendant and merge queries used by edits. Preserve the ordinary history scope across ref-tree refreshes that temporarily load foreign worktree tips.
 - <csr-id-c326831070561f151554d27c64b2839e26baea7c/> stabilize Tix history gutter layout
 - <csr-id-0d7b860866a71b5d987d10558527dc2f2bd1b29c/> preserve reword edits across concurrent amends
   Reload the active view after an editor returns and relocate the target by its full change ID. This preserves concurrent tree amendments while failing safely if the identity disappeared or became ambiguous.
 - <csr-id-41c4e3332fce09b1c223eb5e9ff69d61c3906a67/> make Tix prefix menus switchable
   Reserve top-level prefix keys across every submenu, move stash into actions, and underline stack-insert at its actual shortcut.
 - <csr-id-b6351386fac12089017b131ae51bcc520bef8edd/> preserve Tix history position across compression
   Preserve the selected entry viewport row while cycling between canonical and compressed projections, clamping only when the new history cannot keep it. Cover entering on a commit and leaving from a compressed segment.
 - <csr-id-5e148e57172d6c5e0a9bf5224ee4da1c4d721a9a/> keep Tix history lanes stable across refreshes
   Rank refreshed first-parent descendants by their current displayed branch before lane computation, so advancing a tip does not swap independent lanes. Cover successive refreshes and compressed history.
 - <csr-id-f75f93caadc42147cc33d3b175d81269c7719ef3/> keep ref-tree pins symbolic and connected
   Preserve selected symbolic ref names before peeling, and continue incremental refresh through cached ancestry when a pin makes it visible.
 - <csr-id-8f11c13315f600138fe71f00c4dfc429a3a3b44b/> resolve tix change IDs only in visible history
   Exclude tracking-only graph nodes from reverse-hex change-ID resolution so command selectors match rendered history rows. Cover the duplicated-upstream case that made tix travel report false ambiguity.
 - <csr-id-ee6fc7556594ba91e0ded5e823a9c3bfe824e631/> keep change panes behind prefix popups
   Keep pane content above the floating popup while extending each cleared overlay to its original bottom, so no history row shows through.
 - <csr-id-ee62e414949a876677d211bbfd83f3641349e553/> keep tix popouts clear of messages
   Shift notices and pane status rows above visible prefix popouts, suppressing a popout only when a protected row has no space to move. Reverse only the title of an unselected HEAD row.
 - <csr-id-5db2e5e416512833706fb81078ab7241f477fde9/> keep tix rebase editors complete
   Keep conflict previews armed while users navigate, center the conflicting row, require Enter or Escape, and absorb the resolved index before continuing.\n\nLoad metadata for the full editable todo scope and render commit/tree enrichment markers before signature state.
 - <csr-id-271353a85935941bbeb5f5aac894c678dfc2b27b/> hide non-visible references from tix ref-tree
   The diagnostic ref-tree accepted hidden revisions while also adding every normal reference as a visible tip, so hiding a branch could leave the output unchanged. Filter hidden reference tips and labels in the diagnostic renderer, while retaining visible aliases that share their commit.
   
   Infer hidden local defaults from remote HEADs like tix show and rebase todo, with --no-auto-hide as the explicit opt-out. Interactive ref-tree behavior remains unchanged.
 - <csr-id-ad9aeb49ef55742d83ce2955b876f21f85c6a2f6/> update history change IDs atomically
   Scan change IDs after lane computation and before publishing replacement rows. Keep the current projection metadata visible during refresh, then install rows and duplicate markers together.
 - <csr-id-127d20a43d9b4ec9aa924f386f3a3cdfab5329fb/> distinguish lazy-rebased commits with grey markers
   Pending lazy-rebase commits previously used bright cyan, which could be confused with the blue unsigned state. Render them grey and document the palette distinction.
 - <csr-id-363bbc303210aebf06aa65eaa1188b40e9bb246a/> limit change ID scans to filtered history views
   Lane completion previously started a full change-ID scan even for unrestricted histories, keeping large repositories busy after their graph was ready.
   
   Start the worker only when hidden tips are currently excluded, cancel it immediately on reload, and keep expanded or otherwise unrestricted views scan-free.
 - <csr-id-4c81533baf90085cac84342b922c6e3ebb834ae7/> release startup repository before the TUI event loop
   Hidden-revision validation previously left its repository alive for the entire interactive session even though only detached revision and warning data were needed afterward.
   
   Confine validation to a helper whose return type cannot retain repository state, and document the startup lifetime boundary.
 - <csr-id-7111ee31992fbc0e787812123ed136bce63d2296/> avoid redundant pins during conflict continuation
   Skip provisional departure pins when the destination already retains the mapped departure. Exercise two successive materialized conflicts and verify that continuation leaves no pins.
 - <csr-id-7ad2b0ab349d1c361f899dcd537fe189e0fe1408/> persist rebase objects before writing continuation todos
   Flush candidate replay objects to the repository before formatting a conflict continuation, so generated short IDs resolve through the persistent ODB. Keep an empty object-memory transaction active until conflict materialization finishes.
 - <csr-id-3b574c455943a060074fcfe16c1db4808a9ebfcf/> move the ref-tree cursor page-wise
   Full- and half-page keys previously panned only the ref-tree viewport, unlike the same navigation in history where selection advances.
   
   Choose the nearest selectable node at the requested vertical viewport distance and keep it visible. Page and Ctrl aliases share the behavior, while mouse scrolling and explicit Shift-direction panning remain cursor-independent.
 - <csr-id-7cbddf6d717858a576790ebddf62298309028d59/> ignore unavailable tix diagnostic logs
   Diagnostic logging is optional, but command-line mutations previously failed when their platform log destination could not be opened. Interactive startup also emitted a warning for the same best-effort facility.
   
   Treat initialization failure as diagnostics being disabled. Both command-line and interactive operation now continue silently while successful initialization retains the existing daily tracing behavior.
 - <csr-id-90ead856bf871ae2cd1c02fce9f66c980d57aa66/> align tix index divider with change kinds
   The staged-content divider was centered across the changes pane, which disconnected its index marker from the file-status rows it separates.
   
   Start the dimmed index label in the same column as path-kind letters and fill only the remaining width with the green rail. Clip the label in panes too narrow to show it completely.
 - <csr-id-708abd263ede7aec489630c2053df5e9e696686b/> remove the tix enrichment gutter margin
   Enrichment markers included a trailing space in their shared text representation, separating them from the selection status in the TUI and from graph lanes in plain history output.
   
   Remove that embedded margin while retaining fixed-width gutter alignment for rows without markers, and assert direct adjacency for combined, note-only, and plain-output cases.
 - <csr-id-901e69ee401dc5f8fe494860fb5ea1715ba1e00a/> keep empty tree changes visible
   Tree changes disappeared when their loaded diff had no parent, range, or paths, unlike the clean worktree block. Separate pane rendering from focusability so an available empty Tree block remains visible, reports clean, and stays out of Tab navigation.
 - <csr-id-389492d6402a86693dd9ac362fd8b896e4f53bd9/> give reviews independent return pins
   Review startup reused any existing pin with the same target. Multiple reviews could therefore record one return ref, and finishing either review consumed the shared resource.
   
   Split out an always-create pin path and use it for review departures only. Ordinary time travel retains pin reuse, while each review now owns a return pin with an independent lifetime.
 - <csr-id-9eabb28da88073489f5cc9b55b0dc042ac53120c/> retain direct pins while HEAD is attached
   Treat every ordinary worktree-local pin away from the current attached HEAD as a history tip. Previously, unrelated direct pins were filtered out until HEAD detached, which made a stashed review tree disappear after travelling back to its branch even though its departure pin still existed.
   
   Extend the review-boundary test through a fresh attached-HEAD snapshot and verify that returning consumes the retained pin and restores review state.
 - <csr-id-088b825ba99fdb174bec729fd0136e7aa1b477a2/> omit redundant detached-worktree labels
   Suppress the current detached worktree’s textual `@directory` decoration in history rows because the graph marker already identifies HEAD with `@`.
   
   Keep foreign detached worktree labels, detached-worktree discovery, dirty-state display, and ref-tree decorations unchanged.
 - <csr-id-3fe20e4036cbca2388b12d8717a44fd122d651bd/> keep selected ref-tree disks uninverted
   Preserve the selected reference label’s reverse-video emphasis while leaving its disk marker unchanged. This keeps the rounded rail visually stable and avoids turning ordinary referenced commit disks into filled selection blocks.
   
   Synthetic topology nodes have no reference label to carry that emphasis, so their selected disk remains inverted. Rendering assertions cover both styles and the ref-tree specification documents the distinction.
 - <csr-id-77869c386e9f6c436b81a954dfd1e5560762b2bc/> keep materialized rebases visibly paused
   A todo rebase continuation previously lived only in the event loop after its conflict was checked out. Its instruction was a transient message, so filesystem redraws and navigation could hide the required Enter action while ordinary edits remained available to invalidate the retained plan.
   
   Mirror the continuation in application state and give it a persistent high-contrast footer that distinguishes unresolved and resolved indexes. Reserve history Enter for continuing and Escape for explicitly stopping without rollback, allow read-only inspection in the meantime, and reject repository-changing actions until the continuation is consumed.
 - <csr-id-3527ceb1b64f72f95c33d02cd1694cf5184f3248/> keep the HEAD marker on review commits
   Review commits previously replaced the graph disc with a diamond before HEAD rendering was considered. This made a checked-out review look unlike every other checked-out commit and obscured the repository position.
   
   Keep graph markers solely responsible for signature and HEAD state. Render the review diamond as the first resource marker after the hash, ahead of pin and stash markers, so review identity remains prominent without competing with @.
 - <csr-id-0b408dd2bd36cd61e41fe1c018c987042928580f/> let pins alone retain review history
   Review references served two unrelated purposes: they identified the commit being reviewed and also acted as unconditional traversal tips. Once review setup learned to preserve the departure through normal pins, the second role made review resources retain history independently of the standard pin lifecycle.
   
   Always create or reuse a departure pin, including when attached HEAD points directly at the reviewed commit, and store that pin in tix-review-return-to. Remove review refs from history tip and watched reference-chain construction while retaining their review decorations and lifecycle role.
   
   Capture symbolic return targets before peeling their pins so finishing and cancelling exact-tip reviews reliably reattach the original branch. Cover the separation between review resources and traversal tips and document pins as the sole retention mechanism.
 - <csr-id-1e8c6f477d57ebeef893485f42e6f520745d42ab/> make deleting a review return to its departure
   Deleting a review commit removed its resource refs but left HEAD at the review base with the review checkout still present. In a narrow view this could leave no useful history visible and provided no complete cancellation path.
   
   Capture the recorded return action before the review resources are deleted, discard the tracked review checkout, and return through the same branch-and-pin machinery used by time travel. Detached reviews now preserve an exact-tip departure as a direct pin as well, so cancellation restores both attached and detached starting states and consumes the temporary pin.
   
   Cover both forms with a repository-state regression and document deletion of a review leaf as review cancellation.
 - <csr-id-ccc48a488d11e46f3e4caadad3c7b6a8df4f5bc6/> return to the original checkout after reviews
   A review ref cannot simultaneously anchor a commit below the checked-out tip and remember the branch that should be restored. Using it symbolically worked only when the branch pointed directly at the reviewed commit, leaving descendant reviews detached after completion.
   
   Record the intended action as tix-review-return-to while keeping new review refs direct. Map that destination through review insertion, return through the standard time-travel checkout path, and remove the metadata from the finished commit. This also consumes the departure pin and keeps HEAD, index, and worktree synchronized with the rewritten branch tip.
   
   Continue accepting symbolic review refs created by older tix versions, and distinguish a completed review whose return checkout failed from an unapplied review.
 - <csr-id-3f747ca40cb0da97f21f55f7607806117d92f2e3/> preserve the history tip when starting reviews
   Starting a review performed a private detached checkout instead of preserving the position being left. When the reviewed commit was below the checked-out tip and that tip was not otherwise part of the requested view, refreshing from the review ref made the descendant history disappear.
   
   Create or reuse the same worktree-local pins used by time travel before leaving HEAD. Attached departures use symbolic pins so they continue following their branch, detached departures use direct pins, and failed review setup removes only a pin created by that attempt.
   
   Keep the regression's descendant tip visible and document that review setup retains the departure independently of the review anchor.
 - <csr-id-1a7f93398983d93343e06db57c718184e84fd6aa/> use the current committer for rewritten commits
   Rewording previously retained the selected commit's old or editor-provided committer while only some descendant rewrite paths installed the repository identity. Inserted, split, and review-related paths also depended on their callers remembering to update the actor before signing.
   
   Make the configured operation committer a required argument of the shared commit writer and assign it before signature handling. This gives every newly written rewrite the same current identity and date, ensures signatures cover those final fields, and leaves truly untouched objects alone. Reword templates still expose committer fields for context, but initialize them from repository configuration and cannot use them to override the write-time identity.
 - <csr-id-25732f32471d4c3d4eb7f63484940d25ac247ffb/> accept Shift-2 for tix time travel
   Enhanced keyboard reporting is not consistent across terminals when the user types an at sign. Some report the resulting '@' character, while others preserve the physical '2' key and attach the Shift modifier. Tix previously recognized only the first form, making the documented shortcut inert in the latter terminals.
   
   Recognize Shift-2 alongside the direct at-sign event without restoring any global Shift tracking. Keep plain 2 and standalone Shift inert, and document both terminal encodings under the same @ action.
 - <csr-id-5a7d01392831227f81fb77c914825b9e967086b9/> avoid pinning superseded tix checkout commits
   Todo checkout considered the pre-rewrite detached HEAD when preserving the departure. If that commit was rewritten into the selected successor, tix retained an obsolete pin beside the commit it actually checked out.
   
   Carry complete and conflicted rebase mappings into the shared checkout path and map the departure before deciding whether it needs a pin. Skip preservation when it maps to the destination, and otherwise retain only the rewritten identity that still anchors visible history.
 - <csr-id-d0d6d321fd0e4e5cc551a62d77a54a1d327a6d2f/> preserve clean prefixes in tix rebases
   Applying a todo eagerly recreated every commit through `@`, even when the early picks and their parentage were unchanged. Large clean prefixes were needlessly re-signed, changed identity, and inflated rebase time.
   
   Map an unchanged, non-pending pick without squashes to itself when its planned parent is still its actual parent. Eager replay now starts at the first pending or structurally changed commit, while descendants and other stacks retain the established lazy-rebase behavior.
 - <csr-id-cefb6d775d3e4548f891804f3c2e070d4b0b1a5c/> retain tix stashes across commit rewrites
   A commit stash is associated through the commit ID embedded in its ref name. Rewording or rebasing that commit otherwise orphaned the saved state under an ID that history no longer displayed.
   
   Prepare stash-ref moves from the rewrite map and include their forward and rollback edits in the same reference transaction as the history update. Reject drops, converging associations, malformed IDs, and occupied destinations before prepared objects or refs become observable.
 - <csr-id-7f1788f3a886403a257e1a04b28b065f9f150c0a/> keep tix resource markers visible in history
   The pushpin was rendered as an ordinary reference decoration, so hiding refs also hid the evidence that a commit was preserving history. Resource state should not depend on the chosen ref-detail mode.
   
   Render pin state immediately after the hash, outside the ordinary decoration list, while continuing to collapse multiple pins and hide their internal names. The marker now remains visible in every reference mode and has a stable position beside other commit resources.
 - <csr-id-6ad55212e444c4ee1790303cf5db2e2cc8d0bfce/> detect repeated tix character navigation
   The terminal enhancement requested repeat events, but printable `j` and `k` could still arrive through legacy character input without event types. Holding those keys therefore looked like isolated presses and recomputed the changes pane for every row.
   
   Request escape-code reporting for all keys together with event types and disambiguation. Printable navigation now participates in the existing repeat suppression path, matching page keys and restoring responsive held-key scrolling.
 - <csr-id-e248933fdde76090a1857d45bd0e2b3e0e363686/> don't replay clean checkout ancestry in tix todos
   An unchanged todo became actionable when any visible fork contained a pending commit. A pending sibling could therefore force the clean `@` ancestry through needless cherry-picking and signing.
   
   Inspect only the first-parent path ending at the checkout marker when deciding whether the history editor must apply an unchanged document. Pending work on other forks remains visible and lazy, while explicit command-line apply keeps its intentional always-apply semantics.
 - <csr-id-c9312dfc2034bcf75acd7f4c1de81bb251e42d4a/> center the initial HEAD selection in tix
   Startup selected the current worktree HEAD as soon as it appeared, but merely kept it visible at the viewport edge. When HEAD had descendants, the first frame concealed the surrounding history and made the detached position look like a tip.
   
   Retain the pending initial selection until the real viewport height is known, then center it with normal boundary clamping. Continue following HEAD while history streams unless the user navigates first, so startup reveals useful context without overriding interaction.
 - <csr-id-c80117f289e2b1c259b815e746d83466a0ecbc9e/> show refs only once in tix rebase todos
   References had become first-class editable lines in rebase todos, but the display metadata copied from history still repeated the same decorations beside each commit. The duplicate presentation obscured which representation actually controlled ref movement.
   
   Format todo commit metadata without reference decorations in both TUI and CLI preparation. Standalone ref lines remain the sole editable source of ref placement, while dates, authors, attributions, and subjects retain the familiar history presentation.
 - <csr-id-02561319a9309e487b19a723493dcb7b03d89aba/> explain when unchanged rebase todos apply
   Saving an unchanged rebase document can mean a true no-op, replay pending commits, update the base, or continue a materialized conflict. The generic opening comment gave no indication whether leaving the document untouched would mutate history.
   
   Generate a mode-specific first Markdown comment for every combination of base update and pending work, and a distinct continuation notice. Each explains the unchanged behavior and the common cancellation rule of emptying the file or removing the versioned state comment.
 - <csr-id-5911366863499baf035f929946740ba29c63e868/> keep conflict-marker trees out of rebased commits
   Record the cherry-pick ours tree in suspended conflict commits while retaining the synthetic merge-result tree solely for materializing conflict markers and unmerged index stages. Reuse the existing two-tree worktree transition and roll it back if the final index write fails.
 - <csr-id-a33c0749fb335df47c3aaa18fbdd0732ee77f04b/> emphasize tix review commits
   Replace the undersized star marker with a filled diamond whose visual weight matches ordinary commit discs while remaining distinct and one cell wide.
 - <csr-id-de86482cdf1d0752dedf853405971803b74f0299/> keep in-memory rebase conflicts drawable
   Hide repository-backed changes and defer notes or message loading while a suspended conflict exists only in the rebase operation's object memory. Restore overlays as soon as the user accepts or cancels the preview.
 - <csr-id-7069acc29fdeac2a43a50a63e119fef4f896c299/> make the tix information menu reachable
   Stop requesting and interpreting standalone Shift events, removing the held-Shift ancestry and author-preview mode that could intercept Shift-/ before it became ?. Accept both enhanced-keyboard encodings of that shortcut while preserving ordinary shifted commands.
   
   Keep all status actions following ? inside its expanded group through commit diff navigation, with quit alone outside the group. Update the behavioral specification and cover collapsed and expanded footer layouts.
 - <csr-id-472d81e78ae7c84fef14b8b7158dfacaba84dfce/> place tix rebase state after todo help
 - <csr-id-0f3eb475f6f68e159516e33048c14a6db194f1fa/> keep tix references at rewritten tips
 - <csr-id-7ff44c80957261b602e9b46b3f209710e42be284/> time-travel through mixed pending tix history
 - <csr-id-6cb2f3c4b4360b701ec403aa1956973d41279995/> keep pending tix rebases unsigned
 - <csr-id-562fa1493cf58beaf5c9b2091fcba5b43435b2df/> keep rebased tix descendants visible
 - <csr-id-43084549db74fd48b238d9a6d3891420810afd16/> materialize only selected pending ancestry in tix
   Treat empty signature fields as pending without attaching an original-parent marker to commits whose tree and parent are already final.
   
   Replay and sign only the ancestry through the time-travel destination while reparenting later descendants as lazy commits. Cover signed spill traversal, conflict handling, and the resulting history state.
 - <csr-id-ac19c036d3168b1a9abc08ad7d1b648fa4aa6d65/> scope tix command edits to the visible history
   Build the amend and spill graph from the same default HEAD, pin, and review tips as the history view. Keep the all-reference graph for rebase primitives that intentionally rewrite multiple mutable lines, while preventing unrelated descendant merges from blocking HEAD-only command edits.
 - <csr-id-41b3cbfb9afaae4f1dc684114934b8781a616005/> apply tix rebase updates from hidden anchors
   Resolve rebase todo anchor titles from the freshly opened repository instead
   of requiring the updated hidden tip to have a visible history row. This keeps
   the `(updated-base)` heading available for tips outside the displayed view and
   retains the same agent, notes, and subject markers as history titles.
   
   Treat `base != onto` as an actionable unchanged todo, just like pending lazy
   rebases. Rebase-update now applies when the user saves the generated document
   without editing its picks, because moving the fork point is itself the edit.
   
   Extend the update regression to use an unlisted, annotated hidden anchor, save
   the todo unchanged, execute the plan, and assert that the first rewritten
   commit is parented onto the new hidden tip. Update the tix specification to
   record this behavior.
 - <csr-id-e20cd28bdf89cd1a454b4b981bd8faca5942ea1d/> replay pending commits from unchanged tix rebase todos
   Saving an unchanged rebase todo previously returned before planning, even when
   the listed commits carried deferred tree work. This left pending rebases in
   place despite an explicit request to replay the stack.
   
   Treat an unchanged todo as actionable when its scope contains pending commits,
   then run it through the ordinary todo plan so only the checked-out ancestry is
   eagerly cherry-picked and other forks remain lazy.
   
   Use tix-rebase-parent as the sole pending marker, including a hash-kind-specific
   null object ID for roots, and preserve the originally recorded parent across
   additional lazy rewrites so later replay applies the intended delta.
   
   Document and test unchanged replay, pending roots, marker-driven display, and
   the resulting conflict snapshot.
 - <csr-id-a619ba6c6941b6740c20d1b6f1ebf530c5f6b073/> preserve and expose suspended tix conflicts
   Enhanced keyboard reporting emits a release event after the key press that
   starts time travel. A pending lazy rebase can suspend on a cherry-pick
   conflict, but the release was treated as the next user action and immediately
   discarded that state. The red conflict marker and its checkout instruction
   therefore disappeared before the user could respond.
   
   Distinguish actionable key presses and repeats from key releases so conflicts
   remain visible until Enter accepts them or another key explicitly cancels.
   Emit a prominent warning when a rebase suspends and record acceptance,
   cancellation, or checkout failure to make the conflict lifecycle clear in
   diagnostic logs.
   
   Document the input and diagnostic boundary and cover press, repeat, and release
   events.
 - <csr-id-31cafb87606a7a34c6cd89d40ff967a8d1009607/> keep tix review tips private to their worktree
   Store time-travel pins under the Git worktree-private refs namespace and include every valid private pin while HEAD is detached. This keeps sibling review forks visible without leaking pins between tix instances in other worktrees.
   
   Leave legacy shared pins untouched and ignored, and teach reference watching and regression coverage about private pin isolation.
 - <csr-id-46c243fd075e0a9d7a9167c7308c938ce04618db/> retain tix selection across commit rewrites
   Carry the successor ID returned by reword, create, split, amend, spill, and time-travel operations through the asynchronous history refresh instead of selecting the newest row.
   
   Follow external rewrites when the selected commit is HEAD or a stable reference such as a StGit patch ref moves to its successor. Preserve the prior commit ID for ordinary refreshes and fall back only when no successor remains visible.
   
   Document the selection identity rules and cover edit, HEAD, and reference-based successor tracking.
 - <csr-id-07f62618cac73bf7758727120b3bc064e0aba4de/> ignore non-commit refs when editing HEAD
   Build the edit rebase graph only from refs whose direct targets are commits. This keeps tree refs such as Codex checkpoints from being passed to the commit traversal while preserving normal branches, patch refs, pins, and detached HEAD.
 - <csr-id-3659d06a1ead201e26da29981a2a2e4eda841f43/> retain history selection across filesystem refreshes
   Preserve the selected commit by object ID when reference notifications refresh the visible history. Fall back to the first selectable row only when that commit is no longer visible, while keeping explicit edit operations free to request their new top commit.
 - <csr-id-018a20337eeee11aade6c986b6dbcb2e805baae4/> recover tix before processing filesystem events
   Treat the event-loop head as the repository-lifecycle boundary because the linked worktree and process working directory may disappear while terminal or filesystem input is pending. Recover to the lexically normalized common repository before watcher handling, cache invalidation, or redraw can open a short-lived fill repository.
   
   Switch the surviving common repository to bare mode, discard worktree-only watchers, caches and diff workers, rebuild reference watching, and schedule history refresh from the surviving graph. If a repository vanishes in the narrow interval before reference inspection, return to the boundary instead of aborting.
   
   Document the lifecycle invariant for future event-loop work and extend the isolated recovery regression to cover disappearance during event processing.
 - <csr-id-df4bef9915a2c894aba443aed47d0988698ae5fe/> ignore incomplete Git lock notifications in tix
   Ignore filesystem batches that only create, write, or remove Git lock files. Keep completed rename notifications actionable even when a backend reports only the lock path, and keep events naming the actual index, HEAD, or reference target, so atomic updates still refresh reliably without read-only Git commands causing needless history and status work.
   
   Label an empty worktree changes block as clean in green so its otherwise empty status remains explicit.
 - <csr-id-f9d9990452f4fa3519828f62c3782193e17a9ed7/> avoid watching ignored tix worktree directories
   Use gix directory walking to register non-recursive watches only for directories that Git status would traverse. This prevents ignored build output such as target/ from repeatedly invalidating and recomputing an unchanged worktree status.
   
   Refresh the directory watch set when directory topology, ignore rules, or the index changes, while retaining only paths and native watcher handles between refreshes.
 - <csr-id-965f604546a31ae5849a85e5cacb1dc11b3144df/> refresh tix after reference transactions
   Wait for a short quiet period after filesystem notifications before reading traversal tips, so multi-step ref updates are observed after their final rename instead of at the temporary lock-file event. This makes newly created or pushed commits appear automatically. Keep Shift+R available as an unconditional explicit refresh, and cover loose-ref notification delivery with a real watcher test.
 - <csr-id-bb01e9a7fa3a771abd67ad2475f6c685cc14b9c4/> delay background refresh status
   Keep the completed history footer stable for the first 500 ms of filesystem-triggered and manual refreshes. Reveal loading or computing state only when the combined traversal and lane work remains active beyond that threshold.
   
   Initial history loading keeps its immediate progress behavior, and the existing event-loop deadline mechanism provides the delayed redraw without polling.
 - <csr-id-e33c30061cb99ff34497edec8957bd9dc8e93699/> retain history selection for worktree-only changes
   Decide whether a filesystem-triggered history refresh selects the top row only after comparing the watched traversal refs. Index and worktree notifications may still invalidate status data, but no longer move the history selection when the view and hidden refs are unchanged.
   
   Also avoid marking an already-running refresh for top selection before the pending notification has been classified.
 - <csr-id-a638caacc44fedfe09fb7b372517f570ef1a4a30/> keep mouse navigation responsive in large tix histories
   Avoid scanning every history row when no failed signature state needs resetting, and coalesce queued vertical mouse-scroll events into a single bounded selection move. This keeps terminal momentum from monopolizing the event loop on million-commit histories while preserving the total requested movement.
 - <csr-id-8b0a4583105ecbb6b4cf92aa62b7230d4537da73/> keep tix ancestry comparisons in its cached graph
   Replace the main history rev-walk with a traversal that retains detached parent, generation, and commit-time data while streaming visible rows. Reverse-index local branches and, when their visible target is encountered, resolve configured upstreams and schedule both sides of each tracking relationship as internal-only traversal tips.
   
   Computing both sides through hidden frontiers ensures ahead/behind painting always sees complete ancestry. Compute ahead/behind and hidden-history counts with a bounded two-color paint over this in-memory graph instead of opening a repository and traversing object history whenever the selection changes. Cache completed relationships and preserve shallow, hidden-boundary, commit-graph, and deferred-metadata behavior.
   
   Move the graph through filesystem refresh workers and stop new walks at any complete cached ancestry, including upstream-only history. Give explicit hidden-history expansion its own traversal state so topology-only cached commits become display rows until already-materialized ancestry is reached, without adding commits reachable only from a hidden tip.
   
   This lets ref changes append to the existing graph without retraversing known commits, makes show-hidden restore the original view, and still drops every repository and commit-graph handle when its worker finishes.
 - <csr-id-637f7a705cab6c945fa28dc0d8a0a3068da1a0aa/> release idle tix repository handles
   Remove the long-lived view repository and notes platform from the event loop because both retained object database pack handles while tix was idle. Open repositories only for bounded view population and watcher setup, then retain detached mailmap, note, and reference data.
   
   Document repository lifetime as a local gix-tix invariant so future panes and platforms do not accidentally reintroduce persistent repository ownership.
 - <csr-id-a58a2e5e764a2b646e2685f355dc1612931d2e28/> retain changed-path selection across refreshes
   Preserve the selected path and its relative viewport position when filesystem notifications reload tree or worktree changes. Fall back to the previous numeric position, clamped by layout, only when the selected path disappeared.
 - <csr-id-3bc22e4908239a6591161969ac22cf382fe4f13a/> reliably refresh tix after repository changes
   Start worktree observation whenever the default combined changes view is active, including at startup, so edits made before cycling the panel are not missed. Treat ref events as potential worktree-status changes as well, which covers checked-out branch movement changing both HEAD and the index/worktree comparison.
   
   Bound notification batches and use a fixed 75ms coalescing window so event storms cannot starve terminal input. Honor backend rescan requests, retain the native event-driven design, and retry failed ref or worktree watchers every five seconds while they are needed.
   
   Add diagnostic tracing around watcher roots, event decisions, cache invalidation, snapshot comparisons, refresh workers, retries, and worktree-removal recovery.
 - <csr-id-5dfc82cbd0128c8be17acb0cc306d1dd22d9793e/> select the newest tix row after watched refs change
   Filesystem-driven reference updates can insert or replace traversal tips, so a
   preserved selection may leave the refreshed view positioned in stale context.
   Track the refresh origin through asynchronous traversal and lane computation,
   then select the first selectable row only for watched-reference refreshes.
   Manual and visibility reloads continue preserving their selection.
 - <csr-id-2c2ed16582fab47bb376b16bb7f13af567d1f18e/> keep tix open after its worktree is removed
   A deleted linked worktree invalidates the per-worktree Git directory watched by tix, even on systems where the process current directory still resolves. Normalize the common repository path lexically at startup so it no longer traverses the removable .git/worktrees/<name> directory.
   
   Recover through the common repository both when a running view observes removal and when the worktree is already gone between repository discovery and the initial history metadata open. In the startup case, replace the stale traversal repository before any frame is retained, so the view starts directly from the common repository without attempting an animation from unavailable state.
   
   Move into the common directory and reopen it with core.bare enabled, then restart reference watching there so history remains live. Drop worktree-derived state during runtime recovery and keep the changes view limited to tree changes. Temporary repositories and diff workers retain the bare mode, and the line-diff pool avoids creating worktree resources even when gix still retains an inferred main-worktree path internally.
   
   Show successful recovery in the main status line until the next user action. If changing into or opening the common repository fails, restore the terminal and return the contextual error instead of silently quitting.

### Performance

 - <csr-id-cad0ae874d55e0590115cd8509af36d6c95dcd4a/> refresh worktree state incrementally
 - <csr-id-56b1bbf4de376632a4836228b7456c4d2449c379/> avoid status refreshes for unrelated refs
   Keep cached worktree status when a reference event leaves the logical HEAD referent and target unchanged. Let the worktree watcher exclusively handle index events so they don't also trigger history work.
 - <csr-id-fdd92c920780e3d610786ff35775ae6c1ed865ca/> retire idle line-diff workers
   Open one repository per lazily activated worker set and share it among thread-local worker handles. Retain the workers for ten seconds after a completed batch, then wake the event loop to join the pool and release its repository resources. Rebuild diff platforms for each batch so retained repositories do not preserve stale diff state.
 - <csr-id-5ebfbc413bd38cd8f7be35a6a649359526374ca8/> cache recently viewed tree changes
   Keep a bounded MRU of immutable commit and merge-parent diff results while the changes view is open. Reuse changed paths, detached diff resources, and computed line counts when revisiting history without retaining unbounded data.
   
   Worktree changes remain separately cached and invalidated by filesystem notifications.
 - <csr-id-c8c38721c75dd523b92a439f5aa99b9f0bbba442/> compact tix history storage
   Store persistent ancestry once as index-addressed commits with flat u32 parent edges instead of repeating object IDs in cached parent lists. Use compact vector walk state for refresh and ahead/behind calculations, and share immutable display rows between the active view, append-only cache, and lane worker.
   
   Keep lane-local parent pruning out of the append-only cache so later view projections retain off-screen ancestry needed for expansion.
   
   This preserves fast incremental refreshes and navigation while reducing peak memory on the 1.35-million-commit Linux history from 1.45 GB to 796 MB, with startup changing from 3.35s to 3.49s.

### Changed (BREAKING)

 - <csr-id-17ce36794d2e19d6b12382caded2c34a0a114e74/> forget commits without confirmation
   Execute the forget action immediately when the selected commit is eligible. Remove the obsolete confirmation state and prompt, and update the behavioral specification.
 - <csr-id-9941554c92aca5e360353ae74a90002a280c7a0d/> merge commit shortcuts into actions
   Replace the separate commit prefix with a two-line actions menu. Keep general action keys stable and move new-empty and spill to a n and a l.
 - <csr-id-cfe26ba3a0362a48002439897dbc4474fa8429bd/> replace tix forkpoint with attach and fork
   Rename the ref-only forkpoint action to attach at a h. Restore a f as an independent commit fork that reuses new-commit preparation, preserves enrichments, pins the new leaf atomically, and travels to it without rewriting the source stack.
 - <csr-id-dbef92a6135e353f1ec84503440c0dd489ef9c77/> reorganize TUI commit and action prefixes
   Use c for commit-producing and commit-editing operations, and move review and squash into a separate a actions group. Scope the changes toggle to the information prefix and remove the obsolete independent fork-commit action and its unused implementation.
 - <csr-id-b17e3139d5e798508544d74fb4e6ee38de33b107/> restore standard help flags
   Tix previously reserved -h for --hide and manually restored only long help on selected parsers. Leaf commands such as reword consequently treated --help as their required revision instead of displaying usage.
   
   Let Clap provide -h/--help for every standalone command, and move the repeatable hide option to -x/--hide consistently. Remove the custom help fields and cover every command path with parser tests.
 - <csr-id-af062fab60816d0905f7297dd6bbd03d40ced45b/> orient tix rebase todos bottom-to-top
   History grows upward from older commits, while rebase todos previously presented and interpreted stacks in the opposite direction. This made visual comparison and editing unnecessarily disorienting.
   
   Render the complete editable plan in reverse and parse it bottom-to-top, including squash and reference placement. Replace fork headings with centered content-width separators, and advance the embedded state marker to v2 so saved v1 documents cannot be misinterpreted.
 - <csr-id-b83758dec85c5dbce45f6e8b7a97906d2bdf1415/> always include worktrees in ref-tree traversal
   Remove the opt-in worktree flags from both the interactive and diagnostic commands. Entering the interactive ref-tree now expands the persistent graph with every known worktree HEAD and remembered branch tip, while projecting the history rows from their original tips so the history view remains scoped.
   
   The standalone ref-tree uses the same automatic worktree traversal. Existing cached commits and refresh workers are reused so expansion stays asynchronous and does not introduce a second graph.
 - <csr-id-5e5c227b5296b6dba0ac858e6b67981c6f254f2c/> make existing HEAD attachment implicit in tix todos
   Generated todos marked the currently attached branch with `@`, making unchanged checkout state look like an explicit ref edit. Removing or moving that marker also overloaded detachment, deletion, and checkout selection in ways that were difficult to reason about.
   
   Record the attached local branch in versioned todo state and render it as an ordinary ref. Preserve attachment automatically while that ref remains at the `@` command; reserve an explicit `@ref` for requesting or enforcing attachment, and validate that it is one editable local branch at the selected result.
 - <csr-id-ab03c098a8f4908c7c5dd6bebbbcb212f570b027/> promote tix head edits to top-level commands
   Replace the standalone manual parser with one Clap-derived command platform. Keep bare tix launching the history view, expose amend and spill directly, retain literal command-name revisions after --, and document that the experimental interface does not carry compatibility shims.
 - <csr-id-ef93be6d4e938b500cfb22cd468a19187ab9d8da/> remove gix tix screen selection
   Remove the plumbing CLI screen-mode option now that the interactive history always owns the alternate screen. The automatic and half-screen modes depended on inline rendering behavior that is being retired from gix-tix.
   
   Stop accepting --screen for gix tix and its aliases, and remove the mode validation and translation into gix-tix. Construct the reduced gix-tix options directly from the remaining quit and hidden-revision arguments. Keep a command-line regression assertion so the removed option cannot silently return as an ignored argument.
 - <csr-id-0b7c1e900460868b42e6e5c83b3dd2f681478ac3/> always run tix in the alternate screen
   Make the alternate screen the only terminal lifecycle for tix. Retain the legacy Screen option temporarily as an ignored compatibility field so workspace consumers continue to compile; the following change removes the public selection API and adapts them.
   
   Delete the preliminary screen-size revision walk, inline height prediction, half-screen sizing, runtime transitions between terminal surfaces, inline resize handling, and the reduced static exit frame. Keep the existing alternate-screen rendering, external pager suspension, input setup, refresh animation, and terminal restoration paths unchanged.

### Other (BREAKING)

 - <csr-id-307b7fa7485738525436a26d7a40fa7d8be9b385/> call the tix reference view ref-tree
   Rename the experimental reference projection to ref-tree across the command line, terminal UI, specification, module, actions, helpers, and tests. The diagnostic command is now tix ref-tree, while plain tree remains reserved for Git tree objects and diffs.
   
   Do not retain a command alias: the old word is parsed only as an ordinary revision, preserving the existing ability to inspect a branch named tree.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 286 commits contributed to the release over the course of 1 calendar day.
 - 2 days passed between releases.
 - 266 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release gix-packetline v0.22.2, gix-worktree-stream v0.36.1, gix-archive v0.36.1, gix-diff v0.67.1, gix-blame v0.17.1, gix-dir v0.29.1, gix-mailmap v0.34.1, gix-revision v0.49.1, gix-merge v0.20.1, gix-negotiate v0.35.1, gix-note v0.1.1, gix-pack v0.74.2, gix-macros v0.1.6, gix-refspec v0.45.1, gix-transport v0.59.1, gix-protocol v0.65.1, gix-status v0.34.1, gix-worktree-state v0.34.1, gix v0.87.1, gix-fsck v0.25.1, gitoxide-core v0.61.1, gix-tix v0.3.0, gitoxide v0.58.0, safety bump gitoxide v0.58.0 ([`3ebca8b`](https://github.com/GitoxideLabs/gitoxide/commit/3ebca8b66017ab2dd02a38f75f78f485bee1ded8))
    - Merge pull request #2842 from GitoxideLabs/tix-improvements ([`fbebed7`](https://github.com/GitoxideLabs/gitoxide/commit/fbebed746e296be64d5607e0a75b5a4e08cf4413))
    - Make tests portable to Windows ([`75dca4f`](https://github.com/GitoxideLabs/gitoxide/commit/75dca4fbe01cfbfa31625d41cd30be6548644576))
    - Peel commits from collapsed history ([`b726843`](https://github.com/GitoxideLabs/gitoxide/commit/b72684362014c96b0b6f57a94a1436de1476736e))
    - Document provisional agent commits ([`bfd395d`](https://github.com/GitoxideLabs/gitoxide/commit/bfd395dcd09150f7f7ac0edf6ebde168f30d748b))
    - Off-changelog rename `note::Platform::add_to_ref()` to `::replace_at_ref()` ([`2c8ddd4`](https://github.com/GitoxideLabs/gitoxide/commit/2c8ddd405bf9b3ca44743900972c99a4ddfe87c5))
    - Spill individual paths from the CLI ([`480fb0a`](https://github.com/GitoxideLabs/gitoxide/commit/480fb0adf6241af320b2060eac11280f77fffddd))
    - Use direct fundamental-type comparisons in tests ([`83d3896`](https://github.com/GitoxideLabs/gitoxide/commit/83d3896d88f6378bc255c72ae233fe8f6d7b1b91))
    - Keep history position when changes shrink ([`a7214c4`](https://github.com/GitoxideLabs/gitoxide/commit/a7214c49209b9b22fe1892d19505e7bfff55bb71))
    - Forget commits without confirmation ([`17ce367`](https://github.com/GitoxideLabs/gitoxide/commit/17ce36794d2e19d6b12382caded2c34a0a114e74))
    - Add a fuzzy command menu ([`560d81f`](https://github.com/GitoxideLabs/gitoxide/commit/560d81fe2a1e6d05484606bef5e4a8243ead538a))
    - Keep linked worktrees clean across undo ([`b07cf5e`](https://github.com/GitoxideLabs/gitoxide/commit/b07cf5ee175c621e037de6e428551ab04d98043a))
    - Refresh worktree state incrementally ([`cad0ae8`](https://github.com/GitoxideLabs/gitoxide/commit/cad0ae874d55e0590115cd8509af36d6c95dcd4a))
    - Avoid status refreshes for unrelated refs ([`56b1bbf`](https://github.com/GitoxideLabs/gitoxide/commit/56b1bbf4de376632a4836228b7456c4d2449c379))
    - Show hidden bases for unborn branches ([`8c25e93`](https://github.com/GitoxideLabs/gitoxide/commit/8c25e9322e5b8c016c55d8666d25118abd558ea2))
    - Copy-insert pasted commits ([`42c8999`](https://github.com/GitoxideLabs/gitoxide/commit/42c89992cf7313f910ef275eeff0909e81f44b20))
    - Merge commit shortcuts into actions ([`9941554`](https://github.com/GitoxideLabs/gitoxide/commit/9941554c92aca5e360353ae74a90002a280c7a0d))
    - Show progress during stack edits ([`b7abf00`](https://github.com/GitoxideLabs/gitoxide/commit/b7abf00018181166d603685d9a59d0c711c535bc))
    - Keep history paging on the cursor ([`4a4e718`](https://github.com/GitoxideLabs/gitoxide/commit/4a4e718d0e8ed68082dcba3b50a32e70b2361bdf))
    - Keep first travel inside visible history ([`2d876cc`](https://github.com/GitoxideLabs/gitoxide/commit/2d876ccf7f267a28c4d2b34a7e45fc23fdd992bf))
    - Keep time-travel animation on its path ([`e87b250`](https://github.com/GitoxideLabs/gitoxide/commit/e87b250a7d01702555b104889fe88610919cef9e))
    - Make ref-tree paging move the cursor ([`884b614`](https://github.com/GitoxideLabs/gitoxide/commit/884b6141cffbc58752bbc9f37efdd9779a8abf59))
    - Retain history selection after time travel ([`68abfcd`](https://github.com/GitoxideLabs/gitoxide/commit/68abfcdb4b86feec6563ef30afbdaf63053b57a3))
    - Avoid pins during rebase continuation ([`f49faf1`](https://github.com/GitoxideLabs/gitoxide/commit/f49faf1998d91eb83faccc237fbc53200446c838))
    - Retire idle line-diff workers ([`fdd92c9`](https://github.com/GitoxideLabs/gitoxide/commit/fdd92c920780e3d610786ff35775ae6c1ed865ca))
    - Add no-alt-screen diagnostics ([`0a0cef8`](https://github.com/GitoxideLabs/gitoxide/commit/0a0cef8d1e807452d0a3adf8d95d8fa47844816a))
    - Add momentary topological navigation ([`502423b`](https://github.com/GitoxideLabs/gitoxide/commit/502423b1542440793da481bd0645f74dc219977a))
    - Keep singleton history rows uncompressed ([`2f7bde8`](https://github.com/GitoxideLabs/gitoxide/commit/2f7bde8628ae32fb05f1c2b97173d63a7c976598))
    - Restore paged time-travel animation ([`2b33888`](https://github.com/GitoxideLabs/gitoxide/commit/2b338883a2daf48cf391db10abc6bfee847e0041))
    - Finalize amended commits immediately ([`0e8c249`](https://github.com/GitoxideLabs/gitoxide/commit/0e8c249f051ccd8eb6806517f88e3cecae0ee17b))
    - Align side-by-side change panes ([`1e6ed75`](https://github.com/GitoxideLabs/gitoxide/commit/1e6ed75804d01f9ab8dc2f6503fc3101daa1ee38))
    - Replay diagnostic navigation inputs ([`b6096cf`](https://github.com/GitoxideLabs/gitoxide/commit/b6096cf525412640b0d54272be2dfb4e464f3e9c))
    - Retain metadata and the final diagnostic frame ([`980ce79`](https://github.com/GitoxideLabs/gitoxide/commit/980ce79f458a5c874b51d3d15161d314598dfc06))
    - Preserve unaffected commits and invert selected rows ([`3912b9e`](https://github.com/GitoxideLabs/gitoxide/commit/3912b9e811d88b17aceb308a7b343b16cf9322a6))
    - Eagerly finalize empty rebased commits ([`c00b72c`](https://github.com/GitoxideLabs/gitoxide/commit/c00b72c1c0cc6f667b8bdae784e0d69693a12021))
    - Defer tree changes while scrolling ([`e4a4dd2`](https://github.com/GitoxideLabs/gitoxide/commit/e4a4dd2eed64a1cf09220944ec234a4822fff7ff))
    - Distinguish off-worktree selections ([`b7ad890`](https://github.com/GitoxideLabs/gitoxide/commit/b7ad89079cf118379f5bdfafa6c5429c29690739))
    - Keep commit titles beside selected notes ([`a991892`](https://github.com/GitoxideLabs/gitoxide/commit/a991892021d6638aa078f32787ef74c9e0befa1e))
    - Resolve rewritten checkout conflicts eagerly ([`6c1eccc`](https://github.com/GitoxideLabs/gitoxide/commit/6c1eccc228cabfc384fe929a5ddb40becc0754c3))
    - Call empty tree changes empty ([`2ee1a55`](https://github.com/GitoxideLabs/gitoxide/commit/2ee1a55dfe28dbe73c2fc0a4f42035168b73b9a5))
    - Stop writing after leaving the alternate screen ([`7fe6205`](https://github.com/GitoxideLabs/gitoxide/commit/7fe620548af77e845d3421d56f3b40c8120c757c))
    - Add copy-insert command ([`f50bba8`](https://github.com/GitoxideLabs/gitoxide/commit/f50bba83ace8dcc1c84c1168751d85ce626cdfad))
    - Add tix admin clear-undo ([`9ad195f`](https://github.com/GitoxideLabs/gitoxide/commit/9ad195fa7166c607395c0bc23d3cb17d363291c7))
    - Remove Tix UI animations ([`b73b404`](https://github.com/GitoxideLabs/gitoxide/commit/b73b40425fb1f2c07b991b2b07cd6ec8724282b5))
    - Avoid duplicate checkout when returning from Tix travel ([`32eb3eb`](https://github.com/GitoxideLabs/gitoxide/commit/32eb3ebdbf47ee93e60e5faaaa37fdbafb94ab8a))
    - Keep Tix edits within the current history view ([`c9875ff`](https://github.com/GitoxideLabs/gitoxide/commit/c9875ff71e1ba766604c7d503fae1e54b7cba20b))
    - Stabilize Tix history gutter layout ([`c326831`](https://github.com/GitoxideLabs/gitoxide/commit/c326831070561f151554d27c64b2839e26baea7c))
    - Preserve reword edits across concurrent amends ([`0d7b860`](https://github.com/GitoxideLabs/gitoxide/commit/0d7b860866a71b5d987d10558527dc2f2bd1b29c))
    - Make Tix prefix menus switchable ([`41c4e33`](https://github.com/GitoxideLabs/gitoxide/commit/41c4e3332fce09b1c223eb5e9ff69d61c3906a67))
    - Add ref-backed undo and redo ([`0579a7b`](https://github.com/GitoxideLabs/gitoxide/commit/0579a7b0337cbbb128682e72e841433c73598e2a))
    - Preserve Tix history position across compression ([`b635138`](https://github.com/GitoxideLabs/gitoxide/commit/b6351386fac12089017b131ae51bcc520bef8edd))
    - Keep Tix history lanes stable across refreshes ([`5e148e5`](https://github.com/GitoxideLabs/gitoxide/commit/5e148e57172d6c5e0a9bf5224ee4da1c4d721a9a))
    - Make compressed Tix history interactive ([`abb9ce7`](https://github.com/GitoxideLabs/gitoxide/commit/abb9ce7180664c1d77792c2f426fadea685b6c1f))
    - Keep ref-tree pins symbolic and connected ([`f75f93c`](https://github.com/GitoxideLabs/gitoxide/commit/f75f93caadc42147cc33d3b175d81269c7719ef3))
    - Add a compressed Tix history view ([`ec71813`](https://github.com/GitoxideLabs/gitoxide/commit/ec71813dfeb5fad1f965e51e1cdb8b3adb60f8a9))
    - Resolve tix change IDs only in visible history ([`8f11c13`](https://github.com/GitoxideLabs/gitoxide/commit/8f11c13315f600138fe71f00c4dfc429a3a3b44b))
    - Keep change panes behind prefix popups ([`ee6fc75`](https://github.com/GitoxideLabs/gitoxide/commit/ee6fc7556594ba91e0ded5e823a9c3bfe824e631))
    - Add relative tix travel destinations ([`8636739`](https://github.com/GitoxideLabs/gitoxide/commit/8636739abeff3ff1ecd29936e3756d46bc443859))
    - Highlight foreign worktree heads in tix ([`8a90092`](https://github.com/GitoxideLabs/gitoxide/commit/8a900927885081fb504550bc957c18abe0ca107e))
    - Replace tix forkpoint with attach and fork ([`cfe26ba`](https://github.com/GitoxideLabs/gitoxide/commit/cfe26ba3a0362a48002439897dbc4474fa8429bd))
    - Show dirty HEAD with 🫟 ([`acc2569`](https://github.com/GitoxideLabs/gitoxide/commit/acc25697211ff829cc71f6a4a47e868b08d6a2a9))
    - Add copy-insert to tix ([`f7b4ae0`](https://github.com/GitoxideLabs/gitoxide/commit/f7b4ae0f1f3ae37a91080da2602fd2ed72f37757))
    - Keep tix popouts clear of messages ([`ee62e41`](https://github.com/GitoxideLabs/gitoxide/commit/ee62e414949a876677d211bbfd83f3641349e553))
    - Keep tix rebase editors complete ([`5db2e5e`](https://github.com/GitoxideLabs/gitoxide/commit/5db2e5e416512833706fb81078ab7241f477fde9))
    - Add cherry-move-stack to tix ([`b05b547`](https://github.com/GitoxideLabs/gitoxide/commit/b05b54762c7e4fdb5ce14be9690ef68bb1d98578))
    - Show prefix actions in floating menus ([`3f3d5c3`](https://github.com/GitoxideLabs/gitoxide/commit/3f3d5c30058a00d534f105a3990e87e7a8dda41c))
    - Add enrich subcommands ([`7fc36ab`](https://github.com/GitoxideLabs/gitoxide/commit/7fc36ab141b848a26941915300601d9ff321f8b5))
    - Add checks-pass tree enrichments ([`b32ffbc`](https://github.com/GitoxideLabs/gitoxide/commit/b32ffbc8512e349945b07c41493166d84ee94b49))
    - Clarify tix stash help ([`254f935`](https://github.com/GitoxideLabs/gitoxide/commit/254f935878fa90ab5c8e46eaf7e287dc21e501d5))
    - Add status as an alias of tix show ([`628481a`](https://github.com/GitoxideLabs/gitoxide/commit/628481ac7af57d45bfd50d4f3525669ce18c9b33))
    - Hide non-visible references from tix ref-tree ([`271353a`](https://github.com/GitoxideLabs/gitoxide/commit/271353a85935941bbeb5f5aac894c678dfc2b27b))
    - Round history graph connections ([`0ce4d8e`](https://github.com/GitoxideLabs/gitoxide/commit/0ce4d8ed4129486b411d58895def3cf7adaed8ab))
    - Add a TUI cherry-move action ([`c7c183c`](https://github.com/GitoxideLabs/gitoxide/commit/c7c183c7305dda32e41d4b4a64996755caa27c37))
    - Add a ref-only TUI forkpoint action ([`9e4763c`](https://github.com/GitoxideLabs/gitoxide/commit/9e4763c88ef4ae917e58b0a461d7fa904e22bfc0))
    - Reorganize TUI commit and action prefixes ([`dbef92a`](https://github.com/GitoxideLabs/gitoxide/commit/dbef92a6135e353f1ec84503440c0dd489ef9c77))
    - Update history change IDs atomically ([`ad9aeb4`](https://github.com/GitoxideLabs/gitoxide/commit/ad9aeb49ef55742d83ce2955b876f21f85c6a2f6))
    - Add two-step TUI squash ([`37ee0b8`](https://github.com/GitoxideLabs/gitoxide/commit/37ee0b895ad8addf3af77241ba2bf07de5e1dd6e))
    - Add attached TUI time travel ([`0dd8c28`](https://github.com/GitoxideLabs/gitoxide/commit/0dd8c28c0b324796cde1b07389b33b2670d90260))
    - Toggle pins from the TUI edit menu ([`d6584ce`](https://github.com/GitoxideLabs/gitoxide/commit/d6584ce234f999382eb906dd4763669fbb587223))
    - Distinguish lazy-rebased commits with grey markers ([`127d20a`](https://github.com/GitoxideLabs/gitoxide/commit/127d20a43d9b4ec9aa924f386f3a3cdfab5329fb))
    - Print change IDs with tix commit hashes ([`482270f`](https://github.com/GitoxideLabs/gitoxide/commit/482270ffbf6908f569eac92fe71b82f9fca079dc))
    - Avoid parallel editor collisions in the --todo test ([`c7f9c6f`](https://github.com/GitoxideLabs/gitoxide/commit/c7f9c6fb78820a5be2680babc13baad068b81703))
    - Edit and preserve Git notes in tix ([`eed98fd`](https://github.com/GitoxideLabs/gitoxide/commit/eed98fdcf935ff1d51aac2529517e9c3d63f66fe))
    - Add --todo to tix new and split ([`97e262c`](https://github.com/GitoxideLabs/gitoxide/commit/97e262c8223c0851d22d32b17b48dd555c7dcdf1))
    - Limit change ID scans to filtered history views ([`363bbc3`](https://github.com/GitoxideLabs/gitoxide/commit/363bbc303210aebf06aa65eaa1188b40e9bb246a))
    - Release startup repository before the TUI event loop ([`4c81533`](https://github.com/GitoxideLabs/gitoxide/commit/4c81533baf90085cac84342b922c6e3ebb834ae7))
    - Cycle through duplicate commits in history ([`ee15652`](https://github.com/GitoxideLabs/gitoxide/commit/ee156520a37145505efb4b245d523a5ab253b4e7))
    - Make history counts relative to visible bases ([`bb064d6`](https://github.com/GitoxideLabs/gitoxide/commit/bb064d688b601afd83a9f3c300cb4aa677ebe3f9))
    - Avoid redundant pins during conflict continuation ([`7111ee3`](https://github.com/GitoxideLabs/gitoxide/commit/7111ee31992fbc0e787812123ed136bce63d2296))
    - Mark ambiguous change IDs without hiding them ([`ad22998`](https://github.com/GitoxideLabs/gitoxide/commit/ad229980ff3fc1179f4bbeb61c674dd04e1cbb20))
    - Persist rebase objects before writing continuation todos ([`7ad2b0a`](https://github.com/GitoxideLabs/gitoxide/commit/7ad2b0ab349d1c361f899dcd537fe189e0fe1408))
    - Let edited rebase todos materialize conflicts ([`db5c2af`](https://github.com/GitoxideLabs/gitoxide/commit/db5c2af45e1bc4d7e5c396450e660242d263659a))
    - Add tix rebase todo --update-base ([`4763b8e`](https://github.com/GitoxideLabs/gitoxide/commit/4763b8ec4dcf9e04eef4c525f3125c9fd5f9125c))
    - Show and resolve abbreviated change IDs ([`dacf744`](https://github.com/GitoxideLabs/gitoxide/commit/dacf744022f8e70e654dc049a872abb4f66260a2))
    - Remove WIP author choices from commit editors ([`58fb29b`](https://github.com/GitoxideLabs/gitoxide/commit/58fb29b52f187e6e7075b34ae3394f02c3dc0546))
    - Move the ref-tree cursor page-wise ([`3b574c4`](https://github.com/GitoxideLabs/gitoxide/commit/3b574c455943a060074fcfe16c1db4808a9ebfcf))
    - Report rewritten refs after tix mutations ([`14e18ed`](https://github.com/GitoxideLabs/gitoxide/commit/14e18ed857b7d2262038d70549b795306ff347d9))
    - Ignore unavailable tix diagnostic logs ([`7cbddf6`](https://github.com/GitoxideLabs/gitoxide/commit/7cbddf6d717858a576790ebddf62298309028d59))
    - Include untracked files in tix new ([`f361fa6`](https://github.com/GitoxideLabs/gitoxide/commit/f361fa63b1dff4676d1e8c63a5e5d28c3f1313ab))
    - Add tix new command ([`ea5ba54`](https://github.com/GitoxideLabs/gitoxide/commit/ea5ba54a2b38102b35465e765dea68ac2a4946d1))
    - Edit tix enrichments with commit messages ([`c544a75`](https://github.com/GitoxideLabs/gitoxide/commit/c544a7527161e3e0281d698cab38f72e6ab6b2f5))
    - Improve tix travel, stash, and scrolling ([`89bc853`](https://github.com/GitoxideLabs/gitoxide/commit/89bc8532ba50fc0a1adf20c465d2451c8c252ea4))
    - Italicize attached tix HEAD markers ([`9c72c9a`](https://github.com/GitoxideLabs/gitoxide/commit/9c72c9a7fb3280d62be672ec466a118c05a28576))
    - Align visible tix history columns ([`7324aa4`](https://github.com/GitoxideLabs/gitoxide/commit/7324aa4de97f9ce557aefe99a12626235a967f3b))
    - Align tix index divider with change kinds ([`90ead85`](https://github.com/GitoxideLabs/gitoxide/commit/90ead856bf871ae2cd1c02fce9f66c980d57aa66))
    - Separate staged and unstaged tix changes ([`e0ad9c0`](https://github.com/GitoxideLabs/gitoxide/commit/e0ad9c084e5f56ebc904153fb9047b16b76710a0))
    - Remove the tix enrichment gutter margin ([`708abd2`](https://github.com/GitoxideLabs/gitoxide/commit/708abd263ede7aec489630c2053df5e9e696686b))
    - Restore standard help flags ([`b17e313`](https://github.com/GitoxideLabs/gitoxide/commit/b17e3139d5e798508544d74fb4e6ee38de33b107))
    - Add per-worktree commit enrichments ([`f7b6b5a`](https://github.com/GitoxideLabs/gitoxide/commit/f7b6b5a736697ce861683a5e9378f5173b1ddff9))
    - Show progress while time travel signs commits ([`9aee7b1`](https://github.com/GitoxideLabs/gitoxide/commit/9aee7b1f656b01735f82fdb840ac4e8576ea5911))
    - Keep empty tree changes visible ([`901e69e`](https://github.com/GitoxideLabs/gitoxide/commit/901e69ee401dc5f8fe494860fb5ea1715ba1e00a))
    - Show application notices above changes ([`5ebe4eb`](https://github.com/GitoxideLabs/gitoxide/commit/5ebe4eb643db542243105461d23901c538c66ec3))
    - Stabilize tix history with change IDs ([`79d046a`](https://github.com/GitoxideLabs/gitoxide/commit/79d046a6b2cb3273972d4f28485b2285c53505d0))
    - Infer hidden branches from remote HEADs ([`d7c421f`](https://github.com/GitoxideLabs/gitoxide/commit/d7c421f27463eb5a028436dd892313a8b197b813))
    - Add non-interactive history display ([`c8f4337`](https://github.com/GitoxideLabs/gitoxide/commit/c8f4337276347ceb23a1c2161a2964af0d4e6a3d))
    - Add index-only command-line amend ([`da6d0e8`](https://github.com/GitoxideLabs/gitoxide/commit/da6d0e80cd9b89735bdb73127cdfefb819554c3c))
    - Show raw display metadata in tix todos ([`30b4948`](https://github.com/GitoxideLabs/gitoxide/commit/30b4948e06722b96d87d8149dbf7402ce43b9566))
    - Orient tix rebase todos bottom-to-top ([`af062fa`](https://github.com/GitoxideLabs/gitoxide/commit/af062fab60816d0905f7297dd6bbd03d40ced45b))
    - Recover reviews with missing return pins ([`34df4a2`](https://github.com/GitoxideLabs/gitoxide/commit/34df4a2988376606edb20f4488ad9a56a88cfa72))
    - Give reviews independent return pins ([`389492d`](https://github.com/GitoxideLabs/gitoxide/commit/389492d6402a86693dd9ac362fd8b896e4f53bd9))
    - Show saved review state in history ([`5f20c48`](https://github.com/GitoxideLabs/gitoxide/commit/5f20c48f7f4c26e2999cd7a79ed3ee90bcf8c00d))
    - Retain direct pins while HEAD is attached ([`9eabb28`](https://github.com/GitoxideLabs/gitoxide/commit/9eabb28da88073489f5cc9b55b0dc042ac53120c))
    - Omit redundant detached-worktree labels ([`088b825`](https://github.com/GitoxideLabs/gitoxide/commit/088b825ba99fdb174bec729fd0136e7aa1b477a2))
    - Color ref-tree nodes by history visibility ([`b78ba47`](https://github.com/GitoxideLabs/gitoxide/commit/b78ba47c8ee992aaabaf81d80c071251d00e6d74))
    - Distinguish ref-tree counts with bullets ([`e6cdd8e`](https://github.com/GitoxideLabs/gitoxide/commit/e6cdd8e4d2f3849a2e3e02932ec1f15812ea9bcd))
    - Document stderr for tix diagnostics ([`db31fdc`](https://github.com/GitoxideLabs/gitoxide/commit/db31fdc8fc4aecff2d079ab10526b21d01f5c9c1))
    - Keep selected ref-tree disks uninverted ([`3fe20e4`](https://github.com/GitoxideLabs/gitoxide/commit/3fe20e4036cbca2388b12d8717a44fd122d651bd))
    - Show author dates by default ([`b14f003`](https://github.com/GitoxideLabs/gitoxide/commit/b14f003d79496d18a7a26d7ca1e99c49d1963746))
    - Pin selected ref-tree references into history ([`c7815ec`](https://github.com/GitoxideLabs/gitoxide/commit/c7815ec4708e6f5f17dfa10786320ca674f16f7a))
    - Always include worktrees in ref-tree traversal ([`b83758d`](https://github.com/GitoxideLabs/gitoxide/commit/b83758dec85c5dbce45f6e8b7a97906d2bdf1415))
    - Show foreign detached worktrees in the ref-tree ([`1afaf20`](https://github.com/GitoxideLabs/gitoxide/commit/1afaf20c6835afd5d12cc58ad4107ce19608d674))
    - Call the tix reference view ref-tree ([`307b7fa`](https://github.com/GitoxideLabs/gitoxide/commit/307b7fa7485738525436a26d7a40fa7d8be9b385))
    - Move remote tree deletion to e r ([`3ef6807`](https://github.com/GitoxideLabs/gitoxide/commit/3ef6807527962a530c07ef238d91d15942ecc155))
    - Visualize worktrees in the tix tree ([`0bf7b83`](https://github.com/GitoxideLabs/gitoxide/commit/0bf7b83fa147a73002efcf85b6562086a1e804b2))
    - Delete remote references from the tix tree ([`4c5be52`](https://github.com/GitoxideLabs/gitoxide/commit/4c5be5241669347e92642438ced86aa58b6c34a2))
    - Manage pins and count anchors in the tix tree ([`5065f59`](https://github.com/GitoxideLabs/gitoxide/commit/5065f5926148f82f21123d37e9b0851fd6f09e2a))
    - Delete local branches from the tix tree ([`827c1d1`](https://github.com/GitoxideLabs/gitoxide/commit/827c1d126a541743d00c269fb5990f2ec4263749))
    - Add a reference tree and broaden rebase ref editing ([`78dde41`](https://github.com/GitoxideLabs/gitoxide/commit/78dde41fc198926db7f9d976c4a220b2e6287fd7))
    - Remember detached HEAD branches with a HEAD pin ([`94ffd84`](https://github.com/GitoxideLabs/gitoxide/commit/94ffd843b8800abe0c19f6c5a47c790cee6e03d7))
    - Offer a WIP author in commit editors ([`33e6c66`](https://github.com/GitoxideLabs/gitoxide/commit/33e6c66753cf34737c611753bc822db6fa9a3c18))
    - Change reword shortcut to e o ([`6d721af`](https://github.com/GitoxideLabs/gitoxide/commit/6d721af1b23514f9255652045b2c42e4354e0ba5))
    - Keep materialized rebases visibly paused ([`77869c3`](https://github.com/GitoxideLabs/gitoxide/commit/77869c386e9f6c436b81a954dfd1e5560762b2bc))
    - Finish reviews from checked-out successors ([`6398651`](https://github.com/GitoxideLabs/gitoxide/commit/639865115e042b41d4cbf88fe835f0e046c4aeb1))
    - Keep the HEAD marker on review commits ([`3527ceb`](https://github.com/GitoxideLabs/gitoxide/commit/3527ceb1b64f72f95c33d02cd1694cf5184f3248))
    - Require agents to author commits as themselves ([`ac761b0`](https://github.com/GitoxideLabs/gitoxide/commit/ac761b0b56dfab479bba271ecc940938d1cb6691))
    - Let tix reword set the author actor ([`6bf8b38`](https://github.com/GitoxideLabs/gitoxide/commit/6bf8b38e3eb246ec203e100556e1ab32166b9c02))
    - Let pins alone retain review history ([`0b408dd`](https://github.com/GitoxideLabs/gitoxide/commit/0b408dd2bd36cd61e41fe1c018c987042928580f))
    - Make deleting a review return to its departure ([`1e8c6f4`](https://github.com/GitoxideLabs/gitoxide/commit/1e8c6f477d57ebeef893485f42e6f520745d42ab))
    - Return to the original checkout after reviews ([`ccc48a4`](https://github.com/GitoxideLabs/gitoxide/commit/ccc48a488d11e46f3e4caadad3c7b6a8df4f5bc6))
    - Preserve the history tip when starting reviews ([`3f747ca`](https://github.com/GitoxideLabs/gitoxide/commit/3f747ca40cb0da97f21f55f7607806117d92f2e3))
    - Start unambiguous reviews immediately ([`e2c45d0`](https://github.com/GitoxideLabs/gitoxide/commit/e2c45d0256161d19c96d21a54db729a6e3b29fad))
    - Centralize isolated tix test repositories ([`1140bd6`](https://github.com/GitoxideLabs/gitoxide/commit/1140bd6ebad9d2b0c3ed488369c30e81e0cecd56))
    - Cover review splicing below existing successors ([`8ab9eb9`](https://github.com/GitoxideLabs/gitoxide/commit/8ab9eb90cae8200c799d256b896d224bd9af7a8d))
    - Emphasize active tix shortcut prefixes ([`152ee6b`](https://github.com/GitoxideLabs/gitoxide/commit/152ee6bb5909352ad50e21cf243e2991bbf95b9d))
    - Use the current committer for rewritten commits ([`1a7f933`](https://github.com/GitoxideLabs/gitoxide/commit/1a7f93398983d93343e06db57c718184e84fd6aa))
    - Reword commits without opening an editor ([`2bd9185`](https://github.com/GitoxideLabs/gitoxide/commit/2bd9185b8cd15a9f19615425e0d64e08aad29018))
    - Reword pinned history from the tix command line ([`5cf72b5`](https://github.com/GitoxideLabs/gitoxide/commit/5cf72b5e86eb17122853f621877e6fbf03661a56))
    - Add safe command-line time travel to tix ([`092b3af`](https://github.com/GitoxideLabs/gitoxide/commit/092b3af20bd6dabfd622b406b6f90b47317b61fc))
    - Accept Shift-2 for tix time travel ([`25732f3`](https://github.com/GitoxideLabs/gitoxide/commit/25732f32471d4c3d4eb7f63484940d25ac247ffb))
    - Document contextual tix commit messages ([`00600a7`](https://github.com/GitoxideLabs/gitoxide/commit/00600a7939c248fef6be0e6a239c80e144a70b26))
    - Make @ the direct tix time-travel shortcut ([`4ffedac`](https://github.com/GitoxideLabs/gitoxide/commit/4ffedacf8b1f2f580b56fd46aa0a761ec5f51845))
    - Avoid pinning superseded tix checkout commits ([`5a7d013`](https://github.com/GitoxideLabs/gitoxide/commit/5a7d01392831227f81fb77c914825b9e967086b9))
    - Preserve clean prefixes in tix rebases ([`d0d6d32`](https://github.com/GitoxideLabs/gitoxide/commit/d0d6d321fd0e4e5cc551a62d77a54a1d327a6d2f))
    - Restore tix commit stashes in place ([`dad6877`](https://github.com/GitoxideLabs/gitoxide/commit/dad687747f836a6bcb029929db2c7fc671da2995))
    - Retain tix stashes across commit rewrites ([`cefb6d7`](https://github.com/GitoxideLabs/gitoxide/commit/cefb6d775d3e4548f891804f3c2e070d4b0b1a5c))
    - Stash worktree changes at tix commits ([`ff55875`](https://github.com/GitoxideLabs/gitoxide/commit/ff55875c479d6112dc270045b95b5c85b3f49777))
    - Keep tix resource markers visible in history ([`7f1788f`](https://github.com/GitoxideLabs/gitoxide/commit/7f1788f3a886403a257e1a04b28b065f9f150c0a))
    - Share tix stash plumbing across edit workflows ([`c9622d3`](https://github.com/GitoxideLabs/gitoxide/commit/c9622d3ae528482698c6455aa43dab1e6ce995e5))
    - Detect repeated tix character navigation ([`6ad5521`](https://github.com/GitoxideLabs/gitoxide/commit/6ad55212e444c4ee1790303cf5db2e2cc8d0bfce))
    - Group tix navigation hints under help ([`cd6af74`](https://github.com/GitoxideLabs/gitoxide/commit/cd6af74283d976734eb25c158466957788770659))
    - Show pinned tix commits with a pushpin ([`1fe3709`](https://github.com/GitoxideLabs/gitoxide/commit/1fe3709af46880c31274e2159a72e0ebaf6978b9))
    - Don't replay clean checkout ancestry in tix todos ([`e248933`](https://github.com/GitoxideLabs/gitoxide/commit/e248933fdde76090a1857d45bd0e2b3e0e363686))
    - Follow references pinned by tix ([`e1fc093`](https://github.com/GitoxideLabs/gitoxide/commit/e1fc093498a0956b5d3a75b22a40695aa2fa641a))
    - Center the initial HEAD selection in tix ([`c9312df`](https://github.com/GitoxideLabs/gitoxide/commit/c9312dfc2034bcf75acd7f4c1de81bb251e42d4a))
    - Pin arbitrary commits from the tix CLI ([`fa709da`](https://github.com/GitoxideLabs/gitoxide/commit/fa709da3ad45098d200dfa70639845aac88d187d))
    - Show refs only once in tix rebase todos ([`c80117f`](https://github.com/GitoxideLabs/gitoxide/commit/c80117f289e2b1c259b815e746d83466a0ecbc9e))
    - Make existing HEAD attachment implicit in tix todos ([`5e5c227`](https://github.com/GitoxideLabs/gitoxide/commit/5e5c227b5296b6dba0ac858e6b67981c6f254f2c))
    - Remove obsolete shared-pin compatibility coverage ([`fb74a1a`](https://github.com/GitoxideLabs/gitoxide/commit/fb74a1ac2aaec372d087e3021b26adc88578d727))
    - Remove pins from selected tix commits ([`c39f936`](https://github.com/GitoxideLabs/gitoxide/commit/c39f936bb4f9df0198b649c34ec9b9143f3ef4ec))
    - Explain when unchanged rebase todos apply ([`0256131`](https://github.com/GitoxideLabs/gitoxide/commit/02561319a9309e487b19a723493dcb7b03d89aba))
    - Document semantic commit boundaries for gix-tix agents ([`1b3f965`](https://github.com/GitoxideLabs/gitoxide/commit/1b3f965a7902dc52280895ba737255f9cf8f1c09))
    - Keep conflict-marker trees out of rebased commits ([`5911366`](https://github.com/GitoxideLabs/gitoxide/commit/5911366863499baf035f929946740ba29c63e868))
    - Restore the reviewed branch after finishing ([`f973000`](https://github.com/GitoxideLabs/gitoxide/commit/f973000ede925f6109426572edd0bee50f352f7d))
    - Emphasize tix review commits ([`a33c074`](https://github.com/GitoxideLabs/gitoxide/commit/a33c0749fb335df47c3aaa18fbdd0732ee77f04b))
    - Keep in-memory rebase conflicts drawable ([`de86482`](https://github.com/GitoxideLabs/gitoxide/commit/de86482cdf1d0752dedf853405971803b74f0299))
    - Edit references in tix rebase todos ([`88ed4f0`](https://github.com/GitoxideLabs/gitoxide/commit/88ed4f09692ff76244ca03cd56d6836d35c6b4a7))
    - Make the tix information menu reachable ([`7069acc`](https://github.com/GitoxideLabs/gitoxide/commit/7069acc29fdeac2a43a50a63e119fef4f896c299))
    - Continue tix rebase todos through conflicts ([`4f81248`](https://github.com/GitoxideLabs/gitoxide/commit/4f8124822738d3878b096b509ac036fbb629a36d))
    - Show progress while applying tix rebase todos ([`2fbe08f`](https://github.com/GitoxideLabs/gitoxide/commit/2fbe08f29abf19a99ca39df715f6b890693eadee))
    - Place tix rebase state after todo help ([`472d81e`](https://github.com/GitoxideLabs/gitoxide/commit/472d81e78ae7c84fef14b8b7158dfacaba84dfce))
    - Squash commits in tix rebase todos ([`e66aa54`](https://github.com/GitoxideLabs/gitoxide/commit/e66aa54120cbcc19d9c878c8d45bf3ebb774ca80))
    - Add self-contained rebase todo commands ([`166fa19`](https://github.com/GitoxideLabs/gitoxide/commit/166fa19a5a3d46f78386a79c361694620c85fd3f))
    - Keep tix references at rewritten tips ([`0f3eb47`](https://github.com/GitoxideLabs/gitoxide/commit/0f3eb475f6f68e159516e33048c14a6db194f1fa))
    - Time-travel through mixed pending tix history ([`7ff44c8`](https://github.com/GitoxideLabs/gitoxide/commit/7ff44c80957261b602e9b46b3f209710e42be284))
    - Keep pending tix rebases unsigned ([`6cb2f3c`](https://github.com/GitoxideLabs/gitoxide/commit/6cb2f3c4b4360b701ec403aa1956973d41279995))
    - Keep rebased tix descendants visible ([`562fa14`](https://github.com/GitoxideLabs/gitoxide/commit/562fa1493cf58beaf5c9b2091fcba5b43435b2df))
    - Add a top-level tix split command ([`aa3e6ef`](https://github.com/GitoxideLabs/gitoxide/commit/aa3e6ef2c552dc42aa110d0da58fc2f756932895))
    - Materialize only selected pending ancestry in tix ([`4308454`](https://github.com/GitoxideLabs/gitoxide/commit/43084549db74fd48b238d9a6d3891420810afd16))
    - Scope tix command edits to the visible history ([`ac19c03`](https://github.com/GitoxideLabs/gitoxide/commit/ac19c036d3168b1a9abc08ad7d1b648fa4aa6d65))
    - Promote tix head edits to top-level commands ([`ab03c09`](https://github.com/GitoxideLabs/gitoxide/commit/ab03c098a8f4908c7c5dd6bebbbcb212f570b027))
    - Apply tix rebase updates from hidden anchors ([`41b3cbf`](https://github.com/GitoxideLabs/gitoxide/commit/41b3cbfb9afaae4f1dc684114934b8781a616005))
    - Group tix view status behind question mark ([`2944880`](https://github.com/GitoxideLabs/gitoxide/commit/2944880fa8212a94e925d0e78d5d09e8ce93ef3a))
    - Amend selected worktree paths in tix ([`c1b0d8e`](https://github.com/GitoxideLabs/gitoxide/commit/c1b0d8ef45fd3a055c80f3c6c33a9f6738ab0eee))
    - Label external anchors in tix rebase todos ([`97208f6`](https://github.com/GitoxideLabs/gitoxide/commit/97208f653f8659e05abf9e55287417e40acaab5e))
    - Distinguish the current symbolic branch in tix ([`d00c329`](https://github.com/GitoxideLabs/gitoxide/commit/d00c3291140f5e39edc987b0a084b20b0b28cb9f))
    - Replay pending commits from unchanged tix rebase todos ([`e20cd28`](https://github.com/GitoxideLabs/gitoxide/commit/e20cd28bdf89cd1a454b4b981bd8faca5942ea1d))
    - Preserve and expose suspended tix conflicts ([`a619ba6`](https://github.com/GitoxideLabs/gitoxide/commit/a619ba6c6941b6740c20d1b6f1ebf530c5f6b073))
    - Preserve review changes across time travel ([`5d4fe47`](https://github.com/GitoxideLabs/gitoxide/commit/5d4fe474d556fc251a219bda044ac5dfe0c0c84d))
    - Add tix review workflows ([`f9152fe`](https://github.com/GitoxideLabs/gitoxide/commit/f9152fe0f9952f1b54f2eb27b95729cbea206735))
    - Make tix rebase todos self-documenting ([`80bedc3`](https://github.com/GitoxideLabs/gitoxide/commit/80bedc35342f180e81394b19360174d65f9d7341))
    - Create explicit empty commits in tix ([`92fc1e8`](https://github.com/GitoxideLabs/gitoxide/commit/92fc1e807c4a64c6ca73d272e0bae2590df80d28))
    - Rebase tix history onto updated hidden branches ([`12902f2`](https://github.com/GitoxideLabs/gitoxide/commit/12902f2f34e53a34624d292725110baa09ef7461))
    - Keep tix review tips private to their worktree ([`31cafb8`](https://github.com/GitoxideLabs/gitoxide/commit/31cafb87606a7a34c6cd89d40ff967a8d1009607))
    - Fork commits from any tix history entry ([`2fd1600`](https://github.com/GitoxideLabs/gitoxide/commit/2fd1600e2759e56ce243a095a9e884f4c258b513))
    - Edit forked history from hidden bases ([`abe38b7`](https://github.com/GitoxideLabs/gitoxide/commit/abe38b7e9e17b214036a05fb7d1791a2ba649319))
    - Inspect hidden branch bases in tix ([`3e2a260`](https://github.com/GitoxideLabs/gitoxide/commit/3e2a260669be84d2361012c922520452a49c9b8d))
    - Order tix history status actions ([`a422c76`](https://github.com/GitoxideLabs/gitoxide/commit/a422c769f3809d558050ea746ab6913094def6ea))
    - Resume conflicting lazy rebases in tix ([`5681a69`](https://github.com/GitoxideLabs/gitoxide/commit/5681a6940697327d71d740a4c1a7b771522b524e))
    - Retain tix selection across commit rewrites ([`46c243f`](https://github.com/GitoxideLabs/gitoxide/commit/46c243fd075e0a9d7a9167c7308c938ce04618db))
    - Streamline tix history actions ([`360d096`](https://github.com/GitoxideLabs/gitoxide/commit/360d096b3da4c4a3b6b4f7546c33ffcecb05ae75))
    - Mark reword rebases for lazy replay ([`931ed71`](https://github.com/GitoxideLabs/gitoxide/commit/931ed71a5b6b2ec2fc9d0698dd037618691c2de1))
    - Ignore non-commit refs when editing HEAD ([`07f6261`](https://github.com/GitoxideLabs/gitoxide/commit/07f62618cac73bf7758727120b3bc064e0aba4de))
    - Retain history selection across filesystem refreshes ([`3659d06`](https://github.com/GitoxideLabs/gitoxide/commit/3659d06a1ead201e26da29981a2a2e4eda841f43))
    - Split staged and unstaged HEAD changes ([`9fae999`](https://github.com/GitoxideLabs/gitoxide/commit/9fae9998afaf84cfc7dd2ceb4149de57dc41e119))
    - Expose hidden branch divergence in tix ([`a81651e`](https://github.com/GitoxideLabs/gitoxide/commit/a81651e5d4379057c4bd88de388255ab379f662b))
    - Spill selected tree paths ([`abda17e`](https://github.com/GitoxideLabs/gitoxide/commit/abda17ec079a35f6280ec2359379391b7761a23e))
    - Amend and spill commits with lazy rebases ([`483f248`](https://github.com/GitoxideLabs/gitoxide/commit/483f248a3926ce07e713983f77e29c5821358778))
    - Trace tix edit durations ([`52340bf`](https://github.com/GitoxideLabs/gitoxide/commit/52340bfa84fe2d5fbf7da709c74d25559dedabf5))
    - Rebase linear descendants for tix history edits ([`2df159a`](https://github.com/GitoxideLabs/gitoxide/commit/2df159a2dd59eca8b13509840ed8465125bb1283))
    - Streamline tix status shortcuts ([`6588c60`](https://github.com/GitoxideLabs/gitoxide/commit/6588c60621accee918d6163817d624e6793f0c9f))
    - Prioritize tix prefixes in history status ([`95e22e1`](https://github.com/GitoxideLabs/gitoxide/commit/95e22e1146d2ab20f8b8774b1b28b39c28d77a3f))
    - Standardize transient tix messages ([`f3ddb94`](https://github.com/GitoxideLabs/gitoxide/commit/f3ddb9412e5069e5ea3af4dc08ad15288418045a))
    - Clarify active tix shortcut prefixes ([`77ff0f9`](https://github.com/GitoxideLabs/gitoxide/commit/77ff0f941b372149f790e9ee172e76a4bce449f9))
    - Recover tix before processing filesystem events ([`018a203`](https://github.com/GitoxideLabs/gitoxide/commit/018a20337eeee11aade6c986b6dbcb2e805baae4))
    - Show net lines in tix diffstats ([`dadaa55`](https://github.com/GitoxideLabs/gitoxide/commit/dadaa554ec39206bab5152962453d58a60d79263))
    - Forget top commits from tix ([`adb4ecc`](https://github.com/GitoxideLabs/gitoxide/commit/adb4eccf9836a17dd712b87903b073b4a1840c1f))
    - Create commits from tix ([`cd7e043`](https://github.com/GitoxideLabs/gitoxide/commit/cd7e0434c361e54d6eae8dd70116693eb7cdc095))
    - Move tix editing into a dedicated module ([`2f15cc9`](https://github.com/GitoxideLabs/gitoxide/commit/2f15cc93ecd64546d8d9e07725bdf2d27395521f))
    - Focus and flag non-tip HEAD in tix ([`ff8a7ac`](https://github.com/GitoxideLabs/gitoxide/commit/ff8a7ac919ea634604e139a4e12b657d26f2093c))
    - Group tix editing shortcuts ([`797d6dd`](https://github.com/GitoxideLabs/gitoxide/commit/797d6ddc2c01b00b532dcf425c7da19579020fc2))
    - Show other worktree checkouts in tix ([`ff25141`](https://github.com/GitoxideLabs/gitoxide/commit/ff25141fa396d5179425cc6058e57e1be39ca780))
    - Add time-travel checkouts to tix ([`74898b5`](https://github.com/GitoxideLabs/gitoxide/commit/74898b5bc23d54422e568d1fe2e09f1b4fea868d))
    - Use a Markdown buffer for tix rewording ([`8aa8235`](https://github.com/GitoxideLabs/gitoxide/commit/8aa8235dbea4c5b55c0e6d8a46adb349190a94ef))
    - Document the gix-tix behavioral contract ([`61d946b`](https://github.com/GitoxideLabs/gitoxide/commit/61d946b80ab5a45b7e233462fb0a222d7e9062bf))
    - Mark HEAD and dirty worktrees in tix ([`b7cdebc`](https://github.com/GitoxideLabs/gitoxide/commit/b7cdebc331b29d217fa5d55fa8ec5e5e8e8d1862))
    - Reword the newest commit in tix ([`834ffff`](https://github.com/GitoxideLabs/gitoxide/commit/834ffff798f728de9a70f029d4185095e42076d7))
    - Shade the tix commit panel ([`b3e49b5`](https://github.com/GitoxideLabs/gitoxide/commit/b3e49b50a2d1341745cf860c0b43d41c16ffed1a))
    - Highlight unseen filesystem redraws in tix ([`94147bb`](https://github.com/GitoxideLabs/gitoxide/commit/94147bbad023ca9ebed0ce9e64a5341af2884a71))
    - Open whole-commit diffs from tix history ([`c0badda`](https://github.com/GitoxideLabs/gitoxide/commit/c0baddade03ebbba1d3a5614f05e9d995594a913))
    - Ignore incomplete Git lock notifications in tix ([`df4bef9`](https://github.com/GitoxideLabs/gitoxide/commit/df4bef9915a2c894aba443aed47d0988698ae5fe))
    - Avoid watching ignored tix worktree directories ([`f9d9990`](https://github.com/GitoxideLabs/gitoxide/commit/f9d9990452f4fa3519828f62c3782193e17a9ed7))
    - Correlate tix filesystem responses in diagnostics ([`47e9497`](https://github.com/GitoxideLabs/gitoxide/commit/47e949735556738cecd83713dd449d010d8a173b))
    - Emphasize tix filesystem history changes ([`4e60df7`](https://github.com/GitoxideLabs/gitoxide/commit/4e60df723118f72b792791f7e7e917aef52f7f81))
    - Refresh tix after reference transactions ([`965f604`](https://github.com/GitoxideLabs/gitoxide/commit/965f604546a31ae5849a85e5cacb1dc11b3144df))
    - Remove gix tix screen selection ([`ef93be6`](https://github.com/GitoxideLabs/gitoxide/commit/ef93be6d4e938b500cfb22cd468a19187ab9d8da))
    - Always run tix in the alternate screen ([`0b7c1e9`](https://github.com/GitoxideLabs/gitoxide/commit/0b7c1e900460868b42e6e5c83b3dd2f681478ac3))
    - Delay background refresh status ([`bb01e9a`](https://github.com/GitoxideLabs/gitoxide/commit/bb01e9a7fa3a771abd67ad2475f6c685cc14b9c4))
    - Retain history selection for worktree-only changes ([`e33c300`](https://github.com/GitoxideLabs/gitoxide/commit/e33c30061cb99ff34497edec8957bd9dc8e93699))
    - Cache recently viewed tree changes ([`5ebfbc4`](https://github.com/GitoxideLabs/gitoxide/commit/5ebfbc413bd38cd8f7be35a6a649359526374ca8))
    - Compact tix history storage ([`c8c3872`](https://github.com/GitoxideLabs/gitoxide/commit/c8c38721c75dd523b92a439f5aa99b9f0bbba442))
    - Keep mouse navigation responsive in large tix histories ([`a638caa`](https://github.com/GitoxideLabs/gitoxide/commit/a638caacc44fedfe09fb7b372517f570ef1a4a30))
    - Keep tix ancestry comparisons in its cached graph ([`8b0a458`](https://github.com/GitoxideLabs/gitoxide/commit/8b0a4583105ecbb6b4cf92aa62b7230d4537da73))
    - Document agent authorship for tix changes ([`bd0f8f8`](https://github.com/GitoxideLabs/gitoxide/commit/bd0f8f892db1bc85a3cc7e0750809648e5382c7a))
    - Release idle tix repository handles ([`637f7a7`](https://github.com/GitoxideLabs/gitoxide/commit/637f7a705cab6c945fa28dc0d8a0a3068da1a0aa))
    - Retain changed-path selection across refreshes ([`a58a2e5`](https://github.com/GitoxideLabs/gitoxide/commit/a58a2e5e764a2b646e2685f355dc1612931d2e28))
    - Reliably refresh tix after repository changes ([`3bc22e4`](https://github.com/GitoxideLabs/gitoxide/commit/3bc22e4908239a6591161969ac22cf382fe4f13a))
    - Persist tix diagnostics to OS log storage ([`c193fdc`](https://github.com/GitoxideLabs/gitoxide/commit/c193fdc52a318ee18d3446e9ec64decf2192a5e6))
    - Widen the default tix commit panel ([`24996c6`](https://github.com/GitoxideLabs/gitoxide/commit/24996c6525a4374dcb0d1520cdf61e15ba0238f6))
    - Copy selected changed paths in tix ([`27a0a3c`](https://github.com/GitoxideLabs/gitoxide/commit/27a0a3c293bca62f59a9f5690289b0903d8eb0fd))
    - Brighten red UI elements and omit zero diff counts ([`342e182`](https://github.com/GitoxideLabs/gitoxide/commit/342e18218965bab400575763182e2350d56a01b1))
    - Group tix history display shortcuts ([`bd3a6d0`](https://github.com/GitoxideLabs/gitoxide/commit/bd3a6d0edde5c8701fe789dfc2bceaebf8b83a1a))
    - Select the newest tix row after watched refs change ([`5dfc82c`](https://github.com/GitoxideLabs/gitoxide/commit/5dfc82cbd0128c8be17acb0cc306d1dd22d9793e))
    - Coordinate tix overlay pane layout ([`2b2afd1`](https://github.com/GitoxideLabs/gitoxide/commit/2b2afd13d0274ed1b97d4d66bf615454c2dc247d))
    - Keep tix open after its worktree is removed ([`2c2ed16`](https://github.com/GitoxideLabs/gitoxide/commit/2c2ed16582fab47bb376b16bb7f13af567d1f18e))
    - Show the selected history row in tix ([`cd79529`](https://github.com/GitoxideLabs/gitoxide/commit/cd79529ac0b0ffff1059efa17b5062b8a27d482c))
    - Navigate tix history with mouse scrolling ([`be77515`](https://github.com/GitoxideLabs/gitoxide/commit/be77515141960c7f2cbf0f17d80f45423e33efda))
    - Show tree and worktree changes together ([`5a7ed55`](https://github.com/GitoxideLabs/gitoxide/commit/5a7ed55d9c11fc1c69b1b1b487c0038b75afa7d8))
    - Show contextual information beside the tix selection ([`191c4a5`](https://github.com/GitoxideLabs/gitoxide/commit/191c4a501ae9a1cc26cec39dbbcce2d1185e2b47))
    - Refresh tix history when input references move ([`b0d36b6`](https://github.com/GitoxideLabs/gitoxide/commit/b0d36b6a7c81b9f006388d32dae45eee995cb4f7))
    - Enrich commit details in tix ([`f259b60`](https://github.com/GitoxideLabs/gitoxide/commit/f259b6017b0a158948ccb1208f542013fe334fdc))
    - Inspect tree changes in tix ([`f6c1d5e`](https://github.com/GitoxideLabs/gitoxide/commit/f6c1d5e006d6936c7ee3220d8e02c5da3ac14f09))
    - Clarify filtered history in tix ([`2f50f98`](https://github.com/GitoxideLabs/gitoxide/commit/2f50f983681ebced61140a1deb3f73115bb24f80))
    - Navigate merge ancestry with Shift in tix ([`1d14fce`](https://github.com/GitoxideLabs/gitoxide/commit/1d14fce933c500d0931a08a34e3535f092ddb404))
    - Present commit authorship in tix ([`7cc4c19`](https://github.com/GitoxideLabs/gitoxide/commit/7cc4c195739151cb683e7628d93b7e28b965cf93))
    - Verify visible commit signatures in tix ([`7127325`](https://github.com/GitoxideLabs/gitoxide/commit/71273251c11aadb8dfef523eaa96f44362c1b885))
    - Merge pull request #2933 from GitoxideLabs/report-august ([`b8914ff`](https://github.com/GitoxideLabs/gitoxide/commit/b8914ffda5bc8f6ea851aaf1f720140acfe96dbb))
</details>

## 0.2.0 (2026-08-22)

### Chore

 - <csr-id-bf68622e2883a1559bdb64bb5c91f695e0c273aa/> add `gix-tix` archive ignore; make mtime sensitive test always work.

### New Features

 - <csr-id-fae66224013bee9c7c108cb9f45e8d30ff03dfe9/> separate the inline history status with a spacer
 - <csr-id-ad5d5277d21665181b1118ecb22bb29f61dd032f/> reuse the history-view repository during sustained scrolling
 - <csr-id-5c7cf3e715464955974cf5c1c8415d6544c8e1a7/> lazily load commit metadata from commit graphs
 - <csr-id-3e14d256ae0dfbafca1f410ddb6645094c048842/> open the tix commit pane with o
   Accept `o` as the conventional commit-pane toggle alongside `]`, and advertise the easier binding in the footer.
 - <csr-id-c4f62d95366b81d71fa17c9360b7d2848c71bdbc/> abbreviate matching tix attributions
   Render an attribution as `*` when its parsed identity is identical to the primary author, avoiding repetition while preserving distinct and merely mailmapped identities.
 - <csr-id-f380f8f0fc2303572cf225a233dc3e5a1e9ba876/> show Assisted-by agents in tix
   Classify Assisted-by identities from commit trailers and render them as a distinct As attribution group, so agent assistance is visible without assigning co-authorship.
 - <csr-id-6c033fd328ae07859d9e57464a3b57419685f5a5/> style tix commit message metadata
 - <csr-id-f7cd04c856b5e1329a5d380b54168e195b5b8d55/> group tix commit message trailers
 - <csr-id-200dc704c6cd9c18e16b3b47fc6c8be908ca7329/> show tix copy feedback
 - <csr-id-f71e3ebcd524add5d1a526ebbdaa21b432591832/> align tix commit message trailers
 - <csr-id-4c05d6cfb6bf162d770cc46fdf12aa87e5e510bb/> copy tix commit authors with Y
 - <csr-id-124dd4e2ff90063ba7ff7ee20d3223d6ddc95ac9/> use alternate screen for the tix commit pane
 - <csr-id-425bbbb038e0613333e1ab36b5c6c5ec451110e0/> cycle tix author names with n
 - <csr-id-2c2bd9cd58bb5b8bdc84a926af0fd875488cb2de/> keep tix graph browsing memory-efficient on large histories
 - <csr-id-f4c8f0a2c088f3df5356cf05560ff00c0c26d91b/> choose which references tix displays
 - <csr-id-4e16ae61acdbb4d1235a3a6e272832b626a91d6b/> estimate graph width before lanes are ready
 - <csr-id-28fa84d280ec89fc8da48e43051db9a1ed57d018/> toggle align mode with a single key
 - <csr-id-ba30a934d1655010f760a9320efccd5013e7c854/> mark the selected line at its right edge
 - <csr-id-ba8163a0cd9580481fc8eb5f9f9df4ab53f7c374/> clear the inline status line on exit

### Bug Fixes

 - <csr-id-53fdba1a28dabd107d1f654e74030d8f73811543/> recognise GPT as an assisting agent
 - <csr-id-64b29d1371c1a744cb338a3e306d936d635c3db3/> hide the trailing selection marker on exit
 - <csr-id-1d86733b9c44299caff9be2615c76653cf991b1e/> avoid a lane-width shift when graph rendering completes
 - <csr-id-f20701a7962208eb88d025e34f3d525cc8dcce30/> switch screens after hidden history reloads
 - <csr-id-b1f6624ca0d4806cf111d069054328da9c1e9af3/> restore Shift-G navigation to the first commit
 - <csr-id-b2f8783b06e2eb04fadcae3050f22cb2064ca559/> skip the inert tix name mode
   When visible commits have no trailer attributions, switching from all names to author-only changes nothing on screen. Skip directly to hidden names so each press of `n` has an observable effect.
 - <csr-id-f6ee2381ca883846ea4ec8287fbb03306e7a8585/> copy parsed tix authors without re-validation
   History parsing accepts author names containing `>` and emails containing `<`, but IdentityRef serialization rejects them and made the TUI exit when copying. Assemble the displayed identity directly from its already-parsed bytes.
 - <csr-id-581ea82a0562ea08633cad3d529c3ae3c62e3a6c/> apply mailmap to trailer attributions
   The mailmap toggle resolved only the primary commit author, leaving stale
   aliases in Co-authored-by, Reviewed-by, and the other supported attribution
   trailers. Route both primary and trailer authors through the same name resolver
   so the toggle consistently controls every displayed identity.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 42 commits contributed to the release over the course of 30 calendar days.
 - 30 days passed between releases.
 - 28 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update manifests prior to release ([`ebe9095`](https://github.com/GitoxideLabs/gitoxide/commit/ebe9095f2888d3c12447ea5eed9d0afdb0fd5aeb))
    - Merge pull request #2839 from GitoxideLabs/tix-improvements ([`4b3bf5a`](https://github.com/GitoxideLabs/gitoxide/commit/4b3bf5a12ea3fcaeb15e5ac22f8f4c2188699327))
    - Recognise GPT as an assisting agent ([`53fdba1`](https://github.com/GitoxideLabs/gitoxide/commit/53fdba1a28dabd107d1f654e74030d8f73811543))
    - Hide the trailing selection marker on exit ([`64b29d1`](https://github.com/GitoxideLabs/gitoxide/commit/64b29d1371c1a744cb338a3e306d936d635c3db3))
    - Avoid a lane-width shift when graph rendering completes ([`1d86733`](https://github.com/GitoxideLabs/gitoxide/commit/1d86733b9c44299caff9be2615c76653cf991b1e))
    - Switch screens after hidden history reloads ([`f20701a`](https://github.com/GitoxideLabs/gitoxide/commit/f20701a7962208eb88d025e34f3d525cc8dcce30))
    - Separate the inline history status with a spacer ([`fae6622`](https://github.com/GitoxideLabs/gitoxide/commit/fae66224013bee9c7c108cb9f45e8d30ff03dfe9))
    - Reuse the history-view repository during sustained scrolling ([`ad5d527`](https://github.com/GitoxideLabs/gitoxide/commit/ad5d5277d21665181b1118ecb22bb29f61dd032f))
    - Restore Shift-G navigation to the first commit ([`b1f6624`](https://github.com/GitoxideLabs/gitoxide/commit/b1f6624ca0d4806cf111d069054328da9c1e9af3))
    - Lazily load commit metadata from commit graphs ([`5c7cf3e`](https://github.com/GitoxideLabs/gitoxide/commit/5c7cf3e715464955974cf5c1c8415d6544c8e1a7))
    - Merge pull request #2830 from GitoxideLabs/fix-jj-test-on-windows ([`82711e1`](https://github.com/GitoxideLabs/gitoxide/commit/82711e17b5529fe9d0cb52c9c211127900e83c15))
    - Add `gix-tix` archive ignore; make mtime sensitive test always work. ([`bf68622`](https://github.com/GitoxideLabs/gitoxide/commit/bf68622e2883a1559bdb64bb5c91f695e0c273aa))
    - Merge pull request #2834 from GitoxideLabs/tix-improvements ([`2fadbc7`](https://github.com/GitoxideLabs/gitoxide/commit/2fadbc79986d84520ffb7f4ef85dabd0a7469d16))
    - Open the tix commit pane with o ([`3e14d25`](https://github.com/GitoxideLabs/gitoxide/commit/3e14d256ae0dfbafca1f410ddb6645094c048842))
    - Abbreviate matching tix attributions ([`c4f62d9`](https://github.com/GitoxideLabs/gitoxide/commit/c4f62d95366b81d71fa17c9360b7d2848c71bdbc))
    - Show Assisted-by agents in tix ([`f380f8f`](https://github.com/GitoxideLabs/gitoxide/commit/f380f8f0fc2303572cf225a233dc3e5a1e9ba876))
    - Skip the inert tix name mode ([`b2f8783`](https://github.com/GitoxideLabs/gitoxide/commit/b2f8783b06e2eb04fadcae3050f22cb2064ca559))
    - Copy parsed tix authors without re-validation ([`f6ee238`](https://github.com/GitoxideLabs/gitoxide/commit/f6ee2381ca883846ea4ec8287fbb03306e7a8585))
    - Style tix commit message metadata ([`6c033fd`](https://github.com/GitoxideLabs/gitoxide/commit/6c033fd328ae07859d9e57464a3b57419685f5a5))
    - Group tix commit message trailers ([`f7cd04c`](https://github.com/GitoxideLabs/gitoxide/commit/f7cd04c856b5e1329a5d380b54168e195b5b8d55))
    - Show tix copy feedback ([`200dc70`](https://github.com/GitoxideLabs/gitoxide/commit/200dc704c6cd9c18e16b3b47fc6c8be908ca7329))
    - Align tix commit message trailers ([`f71e3eb`](https://github.com/GitoxideLabs/gitoxide/commit/f71e3ebcd524add5d1a526ebbdaa21b432591832))
    - Copy tix commit authors with Y ([`4c05d6c`](https://github.com/GitoxideLabs/gitoxide/commit/4c05d6cfb6bf162d770cc46fdf12aa87e5e510bb))
    - Use alternate screen for the tix commit pane ([`124dd4e`](https://github.com/GitoxideLabs/gitoxide/commit/124dd4e2ff90063ba7ff7ee20d3223d6ddc95ac9))
    - Cycle tix author names with n ([`425bbbb`](https://github.com/GitoxideLabs/gitoxide/commit/425bbbb038e0613333e1ab36b5c6c5ec451110e0))
    - Merge pull request #2831 from GitoxideLabs/tix-improvements ([`9a9a166`](https://github.com/GitoxideLabs/gitoxide/commit/9a9a166f551b187ef953b788538870496d7f6a56))
    - Keep tix graph browsing memory-efficient on large histories ([`2c2bd9c`](https://github.com/GitoxideLabs/gitoxide/commit/2c2bd9cd58bb5b8bdc84a926af0fd875488cb2de))
    - Lower peak memory while completing tix histories ([`fe1de43`](https://github.com/GitoxideLabs/gitoxide/commit/fe1de43f1202bd29646e3f08b03c0c35050dd140))
    - Reduce tix memory use on large histories ([`f878906`](https://github.com/GitoxideLabs/gitoxide/commit/f878906bd1d2ecc85681fb033f9b7a68faf43cd4))
    - Keep tix history loading from caching one-shot objects ([`aec3c35`](https://github.com/GitoxideLabs/gitoxide/commit/aec3c354e65c4f763c72774d8c4def6400ecac33))
    - Show on-demand commit messages in a tix side pane ([`e1daced`](https://github.com/GitoxideLabs/gitoxide/commit/e1daced17b08ff54a8918ced9e4a999375285ec0))
    - Keep tix responsive while computing graph lanes ([`5a8a94b`](https://github.com/GitoxideLabs/gitoxide/commit/5a8a94b651d86eb2ad88bc7b7175823940db56e9))
    - Hide redundant tix status labels ([`e6e56e3`](https://github.com/GitoxideLabs/gitoxide/commit/e6e56e35cbb4a761411402e84529d7deb866720e))
    - Make agent attribution blend with authors in tix ([`2ca0d95`](https://github.com/GitoxideLabs/gitoxide/commit/2ca0d95d3c973b3f54aabf05e85bc7f8306de39a))
    - Choose which references tix displays ([`f4c8f0a`](https://github.com/GitoxideLabs/gitoxide/commit/f4c8f0a2c088f3df5356cf05560ff00c0c26d91b))
    - Merge pull request #2829 from GitoxideLabs/tix-improvements ([`530a399`](https://github.com/GitoxideLabs/gitoxide/commit/530a39900f2fcd760fef275ab40ab84b7c4884d5))
    - Estimate graph width before lanes are ready ([`4e16ae6`](https://github.com/GitoxideLabs/gitoxide/commit/4e16ae61acdbb4d1235a3a6e272832b626a91d6b))
    - Toggle align mode with a single key ([`28fa84d`](https://github.com/GitoxideLabs/gitoxide/commit/28fa84d280ec89fc8da48e43051db9a1ed57d018))
    - Mark the selected line at its right edge ([`ba30a93`](https://github.com/GitoxideLabs/gitoxide/commit/ba30a934d1655010f760a9320efccd5013e7c854))
    - Clear the inline status line on exit ([`ba8163a`](https://github.com/GitoxideLabs/gitoxide/commit/ba8163a0cd9580481fc8eb5f9f9df4ab53f7c374))
    - Merge pull request #2812 from GitoxideLabs/report-july ([`ae8845a`](https://github.com/GitoxideLabs/gitoxide/commit/ae8845a47c4c87e0996a119822106cf09036340b))
    - Apply mailmap to trailer attributions ([`581ea82`](https://github.com/GitoxideLabs/gitoxide/commit/581ea82a0562ea08633cad3d529c3ae3c62e3a6c))
</details>

## 0.1.0 (2026-07-23)

### Chore

 - <csr-id-3e05ca352597ef5966fa4dc4f52456c2424cddad/> add package.include directives to control which files are packaged.
 - <csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/> bump `rust-version` to 1.70
   That way clippy will allow to use the fantastic `Option::is_some_and()`
   and friends.
 - <csr-id-3bd09ef120945a9669321ea856db4079a5dab930/> change `rust-version` manifest field back to 1.65.
   They didn't actually need to be higher to work, and changing them
   unecessarily can break downstream CI.
   
   Let's keep this value as low as possible, and only increase it when
   more recent features are actually used.
 - <csr-id-aea89c3ad52f1a800abb620e9a4701bdf904ff7d/> upgrade MSRV to v1.70
   Our MSRV follows the one of `helix`, which in turn follows Firefox.

### Documentation

 - <csr-id-64ff0a77062d35add1a2dd422bb61075647d1a36/> Update gitoxide repository URLs
   This updates `Byron/gitoxide` URLs to `GitoxideLabs/gitoxide` in:
   
   - Markdown documentation, except changelogs and other such files
     where such changes should not be made.
   
   - Documentation comments (in .rs files).
   
   - Manifest (.toml) files, for the value of the `repository` key.
   
   - The comments appearing at the top of a sample hook that contains
     a repository URL as an example.
   
   When making these changes, I also allowed my editor to remove
   trailing whitespace in any lines in files already being edited
   (since, in this case, there was no disadvantage to allowing this).
   
   The gitoxide repository URL changed when the repository was moved
   into the recently created GitHub organization `GitoxideLabs`, as
   detailed in #1406. Please note that, although I believe updating
   the URLs to their new canonical values is useful, this is not
   needed to fix any broken links, since `Byron/gitoxide` URLs
   redirect (and hopefully will always redirect) to the coresponding
   `GitoxideLabs/gitoxide` URLs.
   
   While this change should not break any URLs, some affected URLs
   were already broken. This updates them, but they are still broken.
   They will be fixed in a subsequent commit.
   
   This also does not update `Byron/gitoxide` URLs in test fixtures
   or test cases, nor in the `Makefile`. (It may make sense to change
   some of those too, but it is not really a documentation change.)

### New Features

 - <csr-id-21401427be1873e72dce7a802587571543e529e7/> add configurable terminal screen modes.
 - <csr-id-e8a9649f01f6d99d2fff5d11885d32b5eabb7727/> limit commit selection highlighting.
 - <csr-id-7118a844ff07f6be2396fa7b85943551a7ebc6d0/> mailmap support with 'm' toggle.
   Additionally, dim disabled toggles.
 - <csr-id-cdbe465db8828a412164873242db8214f5b43baa/> show bots and commit attributions in tix
   Highlight authors identified by the Codex and Claude email addresses, and expose recognized contribution trailers alongside the primary author. Group actors by trailer kind and keep trailer metadata enabled by default behind a dedicated toggle.
   
   The regression fixture covers bot authors, bot co-authors, mixed-case trailer tokens, all recognized attribution kinds, repeated kinds, and malformed actor values. UI coverage verifies grouping, colors, bracketed bot names, and both metadata toggles.
   
   Validated with:
   - GIX_TEST_IGNORE_ARCHIVES=1 cargo test -p gix-tix --features sha1
   - cargo check -p gix-tix --no-default-features --features sha256
   - cargo clippy -p gix-tix --all-targets --features sha1 --no-deps
   - cargo fmt --all -- --check
 - <csr-id-fed0051609450abf49c6aeebdfa36e191f4292c7/> hide revision ancestry in tix
 - <csr-id-6e8f282c0e79dad8f759af04cd82d861bf62cc0d/> keep tix available as a standalone binary
 - <csr-id-31a94aa8e268fb9e3442ce786788624938fce275/> add tix to the gix CLI
 - <csr-id-da73da10fd22581be22c890a130bfded6a781376/> use tig colors in tix
 - <csr-id-dcfe626bf13245aeb5bc3b5e82cbca187075cc87/> add horizontal paging to tix
 - <csr-id-f9482d5dadcdd5abd57a1d8ab445ce1cf04a98dc/> toggle special references in tix
 - <csr-id-e63ffb7fe68829f0534e8d3706e358cf70b6f53a/> show commit dates and author names in tix
 - <csr-id-74784783c2557450d48578cd7472fe5f9ad28e8d/> draw commit graph lanes in tix
 - <csr-id-1cec12b3192445c53e2fe7716d71c262139f764c/> add Vim-style page scrolling to tix
 - <csr-id-5deecf27c4f6fe2d7e6479f5ab3ea39251157109/> let tix quit when history loading completes
 - <csr-id-5704260ee85a6ede0326ebb18fbaf34dfd361a8b/> add the Ratatui tix binary
   Turn gix-tix into the installed tix executable and render its streaming history with Ratatui. The terminal loop accepts multiple revision tips, defaults to HEAD, keeps input responsive with bounded event draining, supports paging/tail navigation and cancellation, and copies full object IDs through OSC52.
   
   Use the latest Rust version requested for this binary. Ratatui TestBackend verifies row, decoration, selection, and footer rendering; key mapping and model/history tests cover the remaining behavior.
   
   Post-implementation Linux checkout observations: first paint and quit completed within 0.11s; the history walker visited 1,352,640 commits in 7.24s with 1,148,321,792 bytes maximum RSS. For reference, git rev-list --count took 0.61s and git log --oneline --decorate took 14.55s. These are observations only, with no optimization threshold.
   
   Validated with cargo test -p gix-tix, cargo clippy -p gix-tix --all-targets -- -D warnings, cargo check -p gix-tix, and cargo build --release -p gix-tix.

### Bug Fixes

 - <csr-id-b5e7a45a4b86ba00670ec3b1fdf8905a9eb6ad00/> better memory handling (use less)
 - <csr-id-3502897677d75a5900f88595c9e47f7a8c313cf3/> ignore broken references in tix decorations
 - <csr-id-ceabbbccf7850c89767ea3b41378f36ddf470967/> require explicit hash selection for gix-tix
 - <csr-id-77876668e5b3944d55cdd2d90a69382443eed3bf/> keep tix metadata visible on wide graphs

### Performance

 - <csr-id-40e105647b970e312dff92b677f3f77be14d2bdc/> render tix reactively at most 60 fps
 - <csr-id-48c925ad849911ac21cad2e569899e397cca4216/> intern author names in tix
 - <csr-id-def11571c40b3c94472e9fc88db46fa8dab3f611/> store commit titles in an arena
 - <csr-id-640af5d456b7b834365fc06328e62a80c6666edb/> speed up tix on million-commit histories
 - <csr-id-db9b2a1ce4a4c844c193fe2732c32e80c27610be/> avoid waiting and formatting hidden tix rows

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 49 commits contributed to the release.
 - 1071 days passed between releases.
 - 29 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#325](https://github.com/GitoxideLabs/gitoxide/issues/325)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#325](https://github.com/GitoxideLabs/gitoxide/issues/325)**
    - Add the tix history and application model ([`774f856`](https://github.com/GitoxideLabs/gitoxide/commit/774f856f32d626506deda2917a974191ec69f32e))
 * **Uncategorized**
    - Update changelogs prior to release ([`cb6ec7d`](https://github.com/GitoxideLabs/gitoxide/commit/cb6ec7dce283943d811b1600b577f586d7a13e1f))
    - Release gix-trace v0.1.21, gix-validate v0.11.3, gix-path v0.12.3, gix-utils v0.3.5, gix-config-value v0.19.0, gix-prompt v0.16.0, gix-sec v0.14.2, gix-url v0.37.0, gix-credentials v0.39.0, safety bump 18 crates ([`f0ec710`](https://github.com/GitoxideLabs/gitoxide/commit/f0ec71076aa1cef3181b77946ee556a89c651b8e))
    - Add configurable terminal screen modes. ([`2140142`](https://github.com/GitoxideLabs/gitoxide/commit/21401427be1873e72dce7a802587571543e529e7))
    - Limit commit selection highlighting. ([`e8a9649`](https://github.com/GitoxideLabs/gitoxide/commit/e8a9649f01f6d99d2fff5d11885d32b5eabb7727))
    - Better memory handling (use less) ([`b5e7a45`](https://github.com/GitoxideLabs/gitoxide/commit/b5e7a45a4b86ba00670ec3b1fdf8905a9eb6ad00))
    - Mailmap support with 'm' toggle. ([`7118a84`](https://github.com/GitoxideLabs/gitoxide/commit/7118a844ff07f6be2396fa7b85943551a7ebc6d0))
    - Merge pull request #2813 from GitoxideLabs/tix-authors ([`dbe7bb6`](https://github.com/GitoxideLabs/gitoxide/commit/dbe7bb6ff0ac53a3056f81cc181411d0d3bfbc3f))
    - Show bots and commit attributions in tix ([`cdbe465`](https://github.com/GitoxideLabs/gitoxide/commit/cdbe465db8828a412164873242db8214f5b43baa))
    - Merge pull request #2809 from GitoxideLabs/gix-tix-mvp ([`443b401`](https://github.com/GitoxideLabs/gitoxide/commit/443b401730e91503666192f502556f334049fbc0))
    - Explain `gix tix` and that it's an experiment ([`b9c0c80`](https://github.com/GitoxideLabs/gitoxide/commit/b9c0c80976da1ec5dd36c7f385d45146eb068947))
    - Ignore broken references in tix decorations ([`3502897`](https://github.com/GitoxideLabs/gitoxide/commit/3502897677d75a5900f88595c9e47f7a8c313cf3))
    - Require explicit hash selection for gix-tix ([`ceabbbc`](https://github.com/GitoxideLabs/gitoxide/commit/ceabbbccf7850c89767ea3b41378f36ddf470967))
    - Hide revision ancestry in tix ([`fed0051`](https://github.com/GitoxideLabs/gitoxide/commit/fed0051609450abf49c6aeebdfa36e191f4292c7))
    - Render tix reactively at most 60 fps ([`40e1056`](https://github.com/GitoxideLabs/gitoxide/commit/40e105647b970e312dff92b677f3f77be14d2bdc))
    - Intern author names in tix ([`48c925a`](https://github.com/GitoxideLabs/gitoxide/commit/48c925ad849911ac21cad2e569899e397cca4216))
    - Keep tix available as a standalone binary ([`6e8f282`](https://github.com/GitoxideLabs/gitoxide/commit/6e8f282c0e79dad8f759af04cd82d861bf62cc0d))
    - Store commit titles in an arena ([`def1157`](https://github.com/GitoxideLabs/gitoxide/commit/def11571c40b3c94472e9fc88db46fa8dab3f611))
    - Add tix to the gix CLI ([`31a94aa`](https://github.com/GitoxideLabs/gitoxide/commit/31a94aa8e268fb9e3442ce786788624938fce275))
    - Use tig colors in tix ([`da73da1`](https://github.com/GitoxideLabs/gitoxide/commit/da73da10fd22581be22c890a130bfded6a781376))
    - Add horizontal paging to tix ([`dcfe626`](https://github.com/GitoxideLabs/gitoxide/commit/dcfe626bf13245aeb5bc3b5e82cbca187075cc87))
    - Toggle special references in tix ([`f9482d5`](https://github.com/GitoxideLabs/gitoxide/commit/f9482d5dadcdd5abd57a1d8ab445ce1cf04a98dc))
    - Keep tix metadata visible on wide graphs ([`7787666`](https://github.com/GitoxideLabs/gitoxide/commit/77876668e5b3944d55cdd2d90a69382443eed3bf))
    - Show commit dates and author names in tix ([`e63ffb7`](https://github.com/GitoxideLabs/gitoxide/commit/e63ffb7fe68829f0534e8d3706e358cf70b6f53a))
    - Speed up tix on million-commit histories ([`640af5d`](https://github.com/GitoxideLabs/gitoxide/commit/640af5d456b7b834365fc06328e62a80c6666edb))
    - Avoid waiting and formatting hidden tix rows ([`db9b2a1`](https://github.com/GitoxideLabs/gitoxide/commit/db9b2a1ce4a4c844c193fe2732c32e80c27610be))
    - Draw commit graph lanes in tix ([`7478478`](https://github.com/GitoxideLabs/gitoxide/commit/74784783c2557450d48578cd7472fe5f9ad28e8d))
    - Add Vim-style page scrolling to tix ([`1cec12b`](https://github.com/GitoxideLabs/gitoxide/commit/1cec12b3192445c53e2fe7716d71c262139f764c))
    - Let tix quit when history loading completes ([`5deecf2`](https://github.com/GitoxideLabs/gitoxide/commit/5deecf27c4f6fe2d7e6479f5ab3ea39251157109))
    - Apply repository formatting to tix ([`d928f86`](https://github.com/GitoxideLabs/gitoxide/commit/d928f8673784ac0b756680ed27bbbd7830b6f60e))
    - Add the Ratatui tix binary ([`5704260`](https://github.com/GitoxideLabs/gitoxide/commit/5704260ee85a6ede0326ebb18fbaf34dfd361a8b))
    - Merge pull request #2568 from GitoxideLabs/dependabot/cargo/cargo-56d6b174d8 ([`ab2fee1`](https://github.com/GitoxideLabs/gitoxide/commit/ab2fee14651202fcb7b3d8178932090c73492014))
    - Update crates to Rust 2024 edition ([`2cb17b2`](https://github.com/GitoxideLabs/gitoxide/commit/2cb17b2e7f6009693a55af907614f705a29d8c29))
    - Remove rust_2018_idioms lint declarations ([`e10d5f6`](https://github.com/GitoxideLabs/gitoxide/commit/e10d5f662df2ee05f973a3167ad215a330ee74e1))
    - Raise MSRV for hash dependency updates ([`3675a8d`](https://github.com/GitoxideLabs/gitoxide/commit/3675a8d61b17845a783bc27912a3f52ac273a4af))
    - Merge pull request #2518 from GitoxideLabs/improvements ([`444a92b`](https://github.com/GitoxideLabs/gitoxide/commit/444a92b0fa1df406cf2f36f8dbe82c2859e04e0b))
    - Add package.include directives to control which files are packaged. ([`3e05ca3`](https://github.com/GitoxideLabs/gitoxide/commit/3e05ca352597ef5966fa4dc4f52456c2424cddad))
    - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
    - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
    - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
    - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
    - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
    - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
    - Merge branch 'global-lints' ([`37ba461`](https://github.com/GitoxideLabs/gitoxide/commit/37ba4619396974ec9cc41d1e882ac5efaf3816db))
    - Workspace Clippy lint management ([`2e0ce50`](https://github.com/GitoxideLabs/gitoxide/commit/2e0ce506968c112b215ca0056bd2742e7235df48))
    - Merge branch 'msrv' ([`8c492d7`](https://github.com/GitoxideLabs/gitoxide/commit/8c492d7b7e6e5d520b1e3ffeb489eeb88266aa75))
    - Change `rust-version` manifest field back to 1.65. ([`3bd09ef`](https://github.com/GitoxideLabs/gitoxide/commit/3bd09ef120945a9669321ea856db4079a5dab930))
    - Merge branch 'maintenance' ([`4454c9d`](https://github.com/GitoxideLabs/gitoxide/commit/4454c9d66c32a1de75a66639016c73edbda3bd34))
    - Upgrade MSRV to v1.70 ([`aea89c3`](https://github.com/GitoxideLabs/gitoxide/commit/aea89c3ad52f1a800abb620e9a4701bdf904ff7d))
</details>

## v0.0.0 (2023-08-17)

### Chore

 - <csr-id-f7f136dbe4f86e7dee1d54835c420ec07c96cd78/> uniformize deny attributes
 - <csr-id-533e887e80c5f7ede8392884562e1c5ba56fb9a8/> remove default link to cargo doc everywhere

### New Features (BREAKING)

 - <csr-id-3d8fa8fef9800b1576beab8a5bc39b821157a5ed/> upgrade edition to 2021 in most crates.
   MSRV for this is 1.56, and we are now at 1.60 so should be compatible.
   This isn't more than a patch release as it should break nobody
   who is adhering to the MSRV, but let's be careful and mark it
   breaking.
   
   Note that `git-features` and `git-pack` are still on edition 2018
   as they make use of a workaround to support (safe) mutable access
   to non-overlapping entries in a slice which doesn't work anymore
   in edition 2021.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 23 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#325](https://github.com/GitoxideLabs/gitoxide/issues/325), [#691](https://github.com/GitoxideLabs/gitoxide/issues/691)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#325](https://github.com/GitoxideLabs/gitoxide/issues/325)**
    - Update changelog ([`7882203`](https://github.com/GitoxideLabs/gitoxide/commit/7882203f558c98b18a381ec763ff1242c396046e))
    - Empty crate for 'tix' tool ([`2efed08`](https://github.com/GitoxideLabs/gitoxide/commit/2efed088c572380a152f75dc7200f13fe3b055ad))
 * **[#691](https://github.com/GitoxideLabs/gitoxide/issues/691)**
    - Set `rust-version` to 1.64 ([`55066ce`](https://github.com/GitoxideLabs/gitoxide/commit/55066ce5fd71209abb5d84da2998b903504584bb))
 * **Uncategorized**
    - Release gix-tix v0.0.0, gix-note v0.0.0, gix-lfs v0.0.0, gix-fetchhead v0.0.0, gix-sequencer v0.0.0, gix-rebase v0.0.0 ([`0199927`](https://github.com/GitoxideLabs/gitoxide/commit/019992765d3cfc2627cf57e82771c006726c8fbc))
    - Update license field following SPDX 2.1 license expression standard ([`9064ea3`](https://github.com/GitoxideLabs/gitoxide/commit/9064ea31fae4dc59a56bdd3a06c0ddc990ee689e))
    - Merge branch 'corpus' ([`aa16c8c`](https://github.com/GitoxideLabs/gitoxide/commit/aa16c8ce91452a3e3063cf1cf0240b6014c4743f))
    - Change MSRV to 1.65 ([`4f635fc`](https://github.com/GitoxideLabs/gitoxide/commit/4f635fc4429350bae2582d25de86429969d28f30))
    - Merge branch 'main' into auto-clippy ([`3ef5c90`](https://github.com/GitoxideLabs/gitoxide/commit/3ef5c90aebce23385815f1df674c1d28d58b4b0d))
    - Merge branch 'blinxen/main' ([`9375cd7`](https://github.com/GitoxideLabs/gitoxide/commit/9375cd75b01aa22a0e2eed6305fe45fabfd6c1ac))
    - Include license files in all crates ([`facaaf6`](https://github.com/GitoxideLabs/gitoxide/commit/facaaf633f01c857dcf2572c6dbe0a92b7105c1c))
    - Merge branch 'rename-crates' into inform-about-gix-rename ([`c9275b9`](https://github.com/GitoxideLabs/gitoxide/commit/c9275b99ea43949306d93775d9d78c98fb86cfb1))
    - Adjust to renaming of `git-tix` to `gix-tix` ([`531003b`](https://github.com/GitoxideLabs/gitoxide/commit/531003bb03dd83e8870643bbf114008a367c6599))
    - Rename `git-tix` to `gix-tix` ([`5cd02dc`](https://github.com/GitoxideLabs/gitoxide/commit/5cd02dcc56eda79888f1d1344031744457fc04fa))
    - Merge branch 'main' into http-config ([`bcd9654`](https://github.com/GitoxideLabs/gitoxide/commit/bcd9654e56169799eb706646da6ee1f4ef2021a9))
    - Merge branch 'version2021' ([`0e4462d`](https://github.com/GitoxideLabs/gitoxide/commit/0e4462df7a5166fe85c23a779462cdca8ee013e8))
    - Upgrade edition to 2021 in most crates. ([`3d8fa8f`](https://github.com/GitoxideLabs/gitoxide/commit/3d8fa8fef9800b1576beab8a5bc39b821157a5ed))
    - Merge branch 'main' into index-from-tree ([`bc64b96`](https://github.com/GitoxideLabs/gitoxide/commit/bc64b96a2ec781c72d1d4daad38aa7fb8b74f99b))
    - Merge branch 'main' into remote-ls-refs ([`e2ee3de`](https://github.com/GitoxideLabs/gitoxide/commit/e2ee3ded97e5c449933712883535b30d151c7c78))
    - Merge branch 'docsrs-show-features' ([`31c2351`](https://github.com/GitoxideLabs/gitoxide/commit/31c235140cad212d16a56195763fbddd971d87ce))
    - Uniformize deny attributes ([`f7f136d`](https://github.com/GitoxideLabs/gitoxide/commit/f7f136dbe4f86e7dee1d54835c420ec07c96cd78))
    - Remove default link to cargo doc everywhere ([`533e887`](https://github.com/GitoxideLabs/gitoxide/commit/533e887e80c5f7ede8392884562e1c5ba56fb9a8))
    - Merge branch 'main' into repo-status ([`4086335`](https://github.com/GitoxideLabs/gitoxide/commit/40863353a739ec971b49410fbc2ba048b2762732))
    - Release git-tix v0.0.0 ([`31d1882`](https://github.com/GitoxideLabs/gitoxide/commit/31d18826514c65e281f13986123df7c58b3f88b4))
</details>

