# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.24.0 (2026-11-22)

### New Features

- <csr-id-1bf13466d1092b5aec815ff58debd01a06d89d2a/> expose external driver commands in error metadata
  <!-- agent -->
  Move the prepared external merge-driver command's debug representation from
  diagnostic prose into `command` metadata for launch failures and failed exits,
  alongside the existing program and exit-status metadata.
  
  Document the field and update the exit-failure regression test to assert the
  command metadata and the diagnostic message without command details.

### Bug Fixes

- <csr-id-5f2af64257d4f6c93ea334eb895514443f013634/> keep ancestor directories out of symlink conflict paths
  A tiny fix that limits a base to leafs, avoiding directories, alongside a test.
  This will also have no impact on performance.
  
  <!-- agent -->
  Selecting the ancestor in a symlink conflict could emit its directory at
  a unique conflict path after rename resolution had already placed the
  children. That copied directory introduced resolved paths in the tree
  that were absent from the index plan.
  
  Treat a directory as an absent ancestor in leaf merges and let its
  children follow their own resolution. Recreate the reported trees and
  check the emitted tree and index stages in both merge directions.
- <csr-id-85476b3136cbf748e60e672e3ac533d3cba2a899/> match Git's text conflict regions and large-file handling
  This is a great simplification over the previous text-merge implementation, and
  and removes all known divergence from existing tests, while adding a few new ones.
  Great win.
  
  <!-- agent -->
  Fragmented hunk handling produced different conflict markers and forced/union
  contents from Git. Known deviations skipped meaningful output checks, and the
  histogram reference selected `diff.algorithm`, which `git merge-file` ignores.
  Repository merges also classified large text as binary through a diff-specific
  size threshold, preventing ordinary text merges.
  
  Represent merge regions in all three inputs and share side-to-side refinement,
  nearby-conflict simplification, and line-ending selection across marker rendering
  and content-favor policies. All known text deviations now match Git, so remove
  their exemptions and the custom tree expectations for text hunk boundaries.
  Every baseline case asserts exact bytes and the conflict verdict.
  
  Pass Git's explicit histogram option in the reference fixture and retain the
  non-histogram cases on older Git. Record its exit status separately so literal
  marker text cannot be mistaken for a conflict. Cover short, long, and
  punctuation-only gaps as well as line-ending boundaries.
  
  Stop using `core.bigFileThreshold` to classify repository merge inputs as binary.
  The plumbing pipeline keeps its explicit caller-controlled resource limit.
- <csr-id-62bae58d2eb246b9b20dccdb85b6643f1e693b9d/> renormalize files before declaring modify/delete conflicts
  Rubber-stamp the tests, which are good where they are (as opposed to the baseline).
  The change itself looked pretty minimal and sane.
  
  <!-- agent -->
  With `merge.renormalize`, a historical CRLF blob and its LF replacement can
  represent unchanged content. Git resolves deletion against that replacement
  cleanly, but the tree merger previously declared a conflict without passing
  those versions through the existing conversion pipeline.
  
  Compare normalized regular-file contents at their final merge path before
  recording modify/delete. Retain conflicts for executable-mode changes, real
  edits, and renamed identities. When normalization removes a file that would
  otherwise conflict with a directory, omit that obsolete structural conflict.
  
  All 24 Git-reference comparisons pass, covering both directions, enabled and
  disabled normalization, and attributes that do or do not match a relocated
  file/directory conflict path.
- <csr-id-2d8389f5c4da336b4b3a4735db07a60dbced592e/> preserve explicit renames blocked by directory relocation
  rubber-stamp basically, after all a rewrite is in the making as well.
  
  <!-- agent -->
  The local synthetic fuzz testcase
  `clusterfuzz-testcase-gix-merge-tree-4702732561481728` makes the tree harness
  panic after a valid merge returns `Can only diff blobs, not Commit`.
  An inferred `a -> c` directory rename relocates a gitlink rename into
  `c/a`, where the other side replaced a blob with a different gitlink.
  The modify/rename arm then attempts a content merge between commits.
  
  Extend the existing blocked-directory-relocation arm to explicit renames
  and run it before modify/rename matching. Both changes stay at their
  explicit paths, so neither gitlink is lost or passed to a blob merger.
  The regression states the reduced trees directly in an isolated in-memory
  object database and checks both merge directions with the default,
  ours, and ancestor tree policies. Its gitlink objects are intentionally
  unavailable, ensuring the merge never tries to read them as blobs.
  
  Git reference: `merge-ort.c::handle_path_level_conflicts()` at
  `6de20f6092dcf9bdb1c8efe03db4b70c82b423dd` skips an inferred relocation
  when its destination is occupied. The local Git binary
  `2.55.0.windows.4.17.g15c6308cf7.dirty` produces the same expected tree in
  both directions and reports an implicit-directory-rename conflict.
  This change follows the existing gix blocked-relocation behavior without
  adding a new conflict classification.
- <csr-id-532f5a40c7097c75244f8abd10cd0e7ac2713a5e/> preserve files at directory conflict destinations
  File/directory conflicts could reuse a side-qualified name already added
  by the opposite side, losing the relocated file or merging unrelated
  contents. Renames onto newly added directories could also be consumed
  without applying the rename or recording a conflict.
  
  Require both sides' pending paths in `unique_path_in_tree()` and update
  every caller. Match added directories at rename destinations through the
  existing directory-conflict handler, and only defer replacement children
  at the rename's source. Ignore added directories whose children were
  discarded by forced resolutions so they cannot block independent renames.
  
  Add 48 Git-reference cases covering additions, modifications, and renames,
  occupied names on either side, and both merge orders. Exercise forced
  ancestor/current-side selection with early conflict detection and preserve
  unrelated additions. Refresh both object-format fixture archives.
- <csr-id-9cd5c00409f7f5f592c844d5a412d644d8624d5f/> share entry and distinct-type conflict resolution
  The resolver's copies of distinct-type handling had drifted. Rename/add,
  rename/rename, and relocated add/add could reuse a pending path from the
  opposite side, merging an unrelated file into conflict markers instead of
  preserving it. Forced `ours` selection could choose an entry reordered by
  type, while ancestor selection could lose a rename source, especially
  after the rename had become a deferred addition.
  
  Use one placement implementation for all four distinct-type arms, checking
  both change trees and retaining the original side order for tree entries,
  deferred changes, and conflict stages. Keep original change indices across
  deferrals so shared forced-entry handling can restore cancelled sources.
  
  Route all six compatible entry merges through one helper for mode, content,
  ancestor compatibility, and conflict decisions. Carry the blob driver's
  selection instead of inferring it from object identity. Interpret virtual
  and user-requested tree policies centrally while preserving explicit option
  values; leave rename and directory routing in their respective arms.
  
  Extend the Git-reference baseline with the same occupied-path conflict
  across four pair variants, both side orders, and both regular-file sides.
  Update the older ancestor expectation that documented losing the source.
  Also cover forced early-exit behavior and independent virtual blob/tree
  policies. Refresh both fixture archives.
- <csr-id-cff89ce7dc399ba55305a4aee72bec9dfcca7229/> preserve renamed ancestors and binary versions
  Virtual merge bases applied whole-entry ancestor favor to rename/delete
  and rename/type-change conflicts. This left obsolete sources in the base
  and discarded the renamed entries. Later criss-cross merges could silently
  drop restored files, select the wrong entry type, or miss conflicts.
  
  Keep ancestor content and modes at the renamed destinations during virtual
  base construction, and retain a type replacement at the source. When a
  deletion also adds the destination, preserve the side contents for the
  add/add merge instead. Ordinary explicit tree-favor choices retain their
  existing behavior.
  
  An unresolved binary merge selects one input rather than producing merged
  content. For divergent renames, preserve each version at its own path and
  conflict-index stage, following Git's mode handling. Resolved file-favor
  choices continue to use the selected result at both destinations.
  
  Add 72 Git-reference cases covering both directions, recursive histories,
  content and executable-mode changes, destination collisions, and favor
  policies. All 57 `gix-merge` tests, including the 471-case baseline suite,
  pass with SHA-1 and SHA-256. All 64 CLI reproduction comparisons match Git.
  `cargo fmt --all --check` and
  `cargo clippy -p gix-merge --all-features --all-targets -- -D warnings` pass.
- <csr-id-301474e849a0146ee7876cc0872308cb984e4abe/> preserve renamed paths in recursive merges
  When both merge bases renamed the same source, ancestor tree favor could
  retain that source or discard both destinations during virtual-base
  construction. A later criss-cross merge could silently delete a restored
  source and report success where Git reports conflicts.
  
  Remove the source when both destinations agree, independently of tree
  favor. Retain all merged destinations and their conflict contents in a
  virtual base, while preserving explicit ancestor restoration for actual
  tree conflicts. Ancestor favor now leaves ordinary clean renames unchanged.
  
  For a shared destination, put the original ancestor and side blobs into
  the conflict index at that destination, including after directory
  relocation. Remove the fixture override that accepted the old source path
  and already-merged blobs in those stages.
  
  Add 18 Git-reference cases and assert that ancestor favor cannot change a
  clean merge. Validation: all 57 `gix-merge` tests, including the 399-case
  baseline suite, pass with SHA-1 and SHA-256. The CLI reproductions pass,
  as do `cargo fmt --all --check` and
  `cargo clippy -p gix-merge --all-features --all-targets -- -D warnings`.
- <csr-id-bf0b9359f14423aeeebf127c4f8fbce95ccca361/> preserve conflict policies across additions and renames
  Regular files added or renamed to the same destination can disagree on
  executable bits without sharing a regular-file ancestor. Prefer the current
  side's mode and report `ModeConflict` independently of content resolution.
  Share that decision with type replacements and ancestor-relative mode
  changes. Keep content-side mapping independent of the modes, including
  relocated rename/add pairs, and preserve original paths in conflict labels.
  
  Colliding renames from different sources must remove those sources when
  constructing a virtual merge base. Restoring them instead can make a later
  criss-cross merge silently drop surviving files. Retain text conflict markers
  in virtual bases, represent an absent binary ancestor with an empty blob,
  and apply early stopping only to the final merge. Distinct types leave the
  contested virtual destination absent, as in Git.
  
  Forced ancestor choices discard content merges for entries they reject.
  Whole-entry mode and add/add resolutions do likewise, while forced location
  choices that retain merged content still report its conflicts. This allows
  `--tree-favor ancestor --message` to commit the selected tree without hiding
  content conflicts in the other policies. Converging directory relocations
  also use the marker size for a shared destination.
  
  Add 50 Git-reference baseline cases and focused assertions for mode conflicts,
  content orientation, recursive bases, and forced resolutions in both merge
  directions. Remove obsolete mode/index deviations, retain the existing hunk
  splitting deviation with correct modes and labels, and refresh both fixture
  archives.
- <csr-id-3dff1b0f050d22a110bfa07fdbe2861348ab8afc/> preserve modes across rename/edit merges
  A two-sided mode comparison restores an executable bit removed by the renaming side. Resolve compatible modes against their shared ancestor, retaining the existing fallback for incompatible ancestor types.
- <csr-id-aa4956d78c1dcf6dbe4c6ae855e47c48eaaf75ca/> invalidate resources after failed setup
  Took a closer look, but didn't immediately see how that would
  avoid it to end up in an invalid state. Independently of that,
  it's definitely easier to follow now, and less duplication
  
  <!-- agent -->
  Reviewing type safety in the diff and merge stack exposed a recovery hazard:
  replacing a populated merge resource could clear its buffer before object
  lookup failed, leaving the previous ID and path attached to empty content.
  `prepare_merge()` still accepted that inconsistent resource.
  
  Take the selected resource out before setup and restore it only on success,
  reusing its buffer on the successful path. Validation, attribute lookup, and
  conversion failures now leave only that side unset, so callers must set it
  successfully before preparing another merge.
- <csr-id-d4ccfca493fa4c74e7ff98c5e21feb711007ddae/> treat merge driver exit codes 1 to 128 as conflicts, like Git does
  `gitattributes(5)` specifies that a merge driver leaves its result in `%A` and exits
  non-zero if conflicts remain, while statuses above 128 mean the driver crashed. Git
  records a content conflict for the former and fails only for the latter.
- <csr-id-e1a1001d0f53347a4cbbff15926511a401776437/> preserve overlapping edits when collecting text hunks
  <!-- agent -->
  Appending different content after repeated blank lines could merge cleanly
  and silently discard the conflict. Indentation-based diff placement moved
  one side's edit before an unchanged line and separated overlapping hunks.
  
  Use hunk postprocessing without the indentation heuristic, matching the
  merge behavior of Git's xdiff caller. Retain the reported EOF-append case
  in the existing `git merge-file` baseline: both directions and all eight
  merge variants compare the result and conflict classification with Git.
- <csr-id-fd9cf42a4a81d4cd3b95594d0ede6d7d938e960f/> preserve relocated merges and rename/delete base stages
  Mostly a rubber-stamp, knowing that less tests with deviations are better.
  
  <!-- agent -->
  Inferred directory relocation must not silently resolve a location decision
  or discard non-conflicting content. The provisional tree and conflict index
  must also describe the same destinations so callers can materialize conflicts.
  
  Report a file-location conflict when a deferred rename meets an identical
  addition at its inferred destination, even when the contents merge cleanly.
  Retain location conflicts alongside rename/delete and divergent-rename
  failures, and record those failures at the paths present in the provisional
  tree. This lets index materialization find the surviving entries.
  
  Keep tree and content choices independent: `TreeFavor::Ours` chooses the
  location while retaining `merged_blob_id` and the merged mode from both
  sides. Apply this to modify/rename and converging rewrites, and handle forced
  rewrite/add collisions without losing the selected source or retaining a
  rejected destination. Include `DirectoryRename::final_location` in the CLI's
  unresolved-path diagnostic when `--message` refuses to write a commit.
  
  Match Git's rename/delete index stages by retaining the original source blob
  and mode at stage 1 of the rename destination. Omit that stage when the deleting
  side independently adds or renames an entry to the destination: those entries
  have no common ancestor. Detect these collisions through the existing path index.
  
  Remove fourteen custom index overrides. Twelve now match Git with the base
  stage retained; the rename matrix's other two can use Git directly once both
  implementations use automatic directory relocation. Update the remaining
  fixtures for their separate merge differences.
  
  Extend Git baselines across all three directory modes and both merge directions,
  checking trees and index stages. Cover companion rename failures, early stopping,
  forced paths, preservation of merged content, and the CLI diagnostic.
- <csr-id-9430019b7eb2a9ae5a05e032b6422c905ec33fb2/> merge equal blob content with different modes
  <!-- agent -->
  Equal blob IDs with different executable modes fell through to `Unknown`:
  add/add lost the path, and modify/modify retained obsolete ancestor content.
  Allow these pairs into the existing content/mode merge arms. Pass original
  modes to the blob helper so its equal-content shortcut remains applicable.
  The existing mode merge handles both enabling and disabling execution.
  
  The Git tree baseline now retains add/add and modify/modify reproductions,
  including both merge directions and an executable-bit removal control.
  Both reported failures were observed before the fix. For add/add, preserve
  gix's existing clean-merge preference for executable mode, recording the
  intentional difference from Git's add/add mode conflict in the baseline.
  
  Git's `merge-ort.c::handle_content_merge()` mode and equal-ID handling was
  inspected at `v2.56.0-116-g6de20f6092`; Apple Git 2.54.0 generated local
  baselines. All 187 tree cases pass with SHA-1 and SHA-256, as do the full
  `gix-merge` test suite, strict all-target Clippy, and formatting.
  
  CIFuzz exposed a remaining invalid assertion in `perform_blob_merge()`:
  directory relocation can make identical additions meet even when their
  original paths differ, so equal blob IDs and equal modes are valid here.
  Remove the assertion and retain the existing equal-content resolution.
  The permanent Git baseline reproduces this with `a/h/a` moved to `d/h/a`
  and identical additions at `d/a` and `a/a`, in both merge directions.
  Git's `merge.directoryRenames=true` aligns its relocation policy with gix,
  so this reproduction compares both the merged tree and index directly.
  The new case failed at the assertion before the correction.
- <csr-id-fa9a6bd70a1d494e82fc810f64a4de466c2ac3c7/> avoid merging identical edits across a rename
  When one side renames and edits a file and the other makes the same edit
  in place, both entries already have the desired blob. Asking for a blob
  merge hits a debug assertion because their modes and IDs are identical.
  Skip that content merge while retaining mode and rename resolution.
  
  The `tree-baseline.sh` reproduction retains Git's `merge-tree --write-tree`
  result in both directions. It panicked before this fix and now matches Git's
  clean tree. Git's `merge-ort` is the behavior reference; inspected the local
  Git checkout at `v2.56.0-116-g6de20f6092`.

### Performance

- <csr-id-c5ef9eb78d819abffe0e5f9b30c664b274b2518d/> reuse matching entries and current-side trees
  Send unchanged entries directly to the common pool. When physical
  cursors have identical encoded prefixes, decode once and share the
  validated mode and ID while keeping each cursor's own filename slice.
  Preserve order checks, hidden duplicate checks and decoding errors.
  
  Compact immutable siblings to one directory liveness witness when
  the changed paths allow it. Seed the result editor from the current
  tree when safe, while keeping the index relative to the ancestor.
  Use ancestor seeding for malformed names and subtree replacements
  that could otherwise restore descendants discarded by resolution.
  
  Apply removals before insertions with the existing editor. Borrow
  cached directory paths so edits only allocate keys on cache misses.
- <csr-id-61f57b4afad391f99649eb20ce77e83eb6841000/> stream and compact three-way merge inputs
  Collect sibling identities directly from three tree iterators in Git's
  traversal order. Fall back to the buffered collector for duplicate,
  out-of-order, or malformed names so last-input ownership and rename
  destination order remain stable.
  
  Store paths in a shared byte buffer, bulk-build compact clean leaf
  decisions, and defer full stages until normalization needs them. Cache
  live descendant counts and index original leaf changes so directory
  liveness and one-sided renames avoid repeated scans.
  
  These changes address the collection and normalization overhead seen
  in Linux-sized merges, including merges without rename tracking.

### Changed (BREAKING)

- <csr-id-d75dce4cd4a240816eaf18846d178d36a5c3de4a/> build merges and indexes from normalized three-way paths
  Checked the baseline and many more divergent indices were removed.
  Rubber-stamped tests.
  
  The new merge implementation is a single rubber-stamp. Just looked at the API
  with cargo doc.
  
  Simplified `gix::Repository` api in favor of putting everything onto `Outcome`.
  
  <!-- agent -->
  Repeated fixes for rename/addition pairs left equivalent conflicts in other
  change combinations inconsistent. The pairwise scheduler consumed and rewrote
  changes while editing the result, so later decisions reconstructed identities,
  directory occupancy, and index paths from partially processed state. Conflict
  diagnostics doubled as an ordered recipe for changing the index: replaying that
  recipe could duplicate stages, while editing diagnostics could change whether
  the result appeared resolved.
  
  Collect immutable base/current/other entries, normalize renames by their source,
  and resolve the resulting paths from children to parents. Share entry resolution
  between normalization and the final path pass, carrying selected entries apart
  from optional blob metadata. Gitlinks never reach the blob merger or appear as
  merged blobs. Remove the pairwise scheduler, deferred change tracking, and
  conflict-index path inference.
  
  Each path owns its final unmerged stages. Relocation, type splitting, restoration,
  and deletion update that ownership; diagnostics explain decisions independently.
  Retain source-merge stages only when their contents survive a later choice.
  Forced tree choices restore connected rename sources and parent directories
  before retaining selected content, preserving unrelated additions and avoiding
  file/descendant collisions. Keep unchanged regions sparse and apply only the
  resolved differences to the base tree editor.
  
  Return an owned `IndexPlan` that constructs and validates a fresh index from the
  original tree and final path changes. Reject malformed emitted paths, duplicate
  entries, null IDs, and overlapping stage-zero files. Represent an absent symlink
  ancestor as absent, while retaining Git's valid unmerged rename-source stages
  below a resolved file. Validate surviving entries rather than rejected inputs.
  
  Breaking change: replace `apply_index_entries()`, `RemovalMode`,
  `Outcome::index_changed_after_applying_conflicts()`, and `Conflict`'s index-entry
  accessors with `Outcome::index` and `IndexPlan::to_index()`. The `gix` facade adds
  `Outcome::index()` and `Repository::index_from_merge()`. Tix retains the detached
  plan for deferred conflict materialization instead of retaining diagnostics.
  
  Explicit renames returned to their original source by directory relocation
  remain identities. Remove source stages only when their destination differs,
  and do not merge an opposite source again as a destination collision. This
  preserves rename/rename and rename/delete stages and the selected namespace.
  One regression deliberately differs from Git: `merge-ort` skips a rename-to-self
  before looking ahead for the opposite same-source rename. `A-B` loses `h/a`
  and reports clean; `B-A` preserves both destinations and their conflict stages.
  Unique basename matches and distinct-content controls rule out ambiguous
  pairing, and disabling directory relocation restores symmetry. Preserve Git's
  conservative result in both directions, exchanging side stages as needed.
  
  Regression coverage compares exact trees, index paths/stages/modes/IDs, and
  conflict verdicts with Git. It exercises Cartesian file/gitlink states, typed
  renames, forced policies, early stopping, recursive ancestors, detached and
  repeated index construction, diagnostic/tree mutation, and malformed inputs.
  The tree fuzzer checks emitted-tree/index correspondence and conflict-state
  independence instead of merely requiring a successful tree merge.
  
  Historical short Criterion runs on 94,852 files measured a single deep conflict
  at about 0.76 ms versus 0.15-0.17 ms before, because unchanged siblings still
  participate in the three-input namespace. Equal-tip merges improved from about
  154 microseconds to 1 microsecond; rename workloads changed by about +2% to +5%.
  These are bounded measurements, not a claim of sparse-conflict performance parity.
  
  CI follow-up: `CIFuzz` reproduced
  `crash-a3a5ca9afbb42f9f9aaadaa1dc0461ceca390524`. A directory relocation
  colliding with the opposite rename recorded a binary content conflict,
  then early stopping returned before restoring the selected source and
  its unresolved index stages. The verdict was unresolved while the index
  contained only the base tree. Stop scanning further collisions but finish
  the already-selected source restoration before returning. A reduced
  in-memory regression verifies the partial tree and both conflict stages.
  The merge test suite passes for both hash formats.
  
  CI follow-up: `tests@linux-x86` and `tests@linux-arm` use Git 2.47.3,
  which aborts in `process_renames()` while generating the returned-to-source
  rename fixture. Git 2.50 removes that assertion failure but still drops the
  required content stages. Require Git 2.51.1, which includes both upstream
  fixes, before generating this exact Git-reference fixture. Other merge
  fixtures continue to run on older Git versions.
- <csr-id-25401d01fb6dcfb57061c0d2cec4d77c098973f8/> preserve missing and unread merge resource states
  A change to how the merge-platform differentiates missing inputs from empty ones.
  It also removes the over-simplification when accessing data from the platform,
  an API to avoid copying data around just to get the result which could also be any
  of its input buffers.
  
  <!-- agent -->
  The null-ID audit reproduced false clean binary merges when large worktree
  files were skipped and their unread buffers compared as empty slices.
  A missing file could also appear identical to an existing file with the
  supplied ID, and selecting it could return that ID.
  
  Compare resource states before IDs, and compare buffers only when both are
  available. Unread content without two known IDs cannot prove equality.
  Replace `buffer_by_pick()` with `data_by_pick(pick, merged)` so callers retain
  `Data::Missing`, `Data::Buffer`, and `Data::TooLarge`.
  `Data::as_slice()` exposes only present buffers. The pipeline result, rather
  than buffer length, distinguishes absence, empty content, and skipped data.
  
  Keep missing-as-empty conversion private to text, union, and external
  driver preparation. Tree-level deletions remain the caller's responsibility.
  `id_by_pick()` returns `None` for a selected missing resource regardless of
  its supplied ID; `Pick::Buffer` always represents content, including an
  empty blob. Adapt the CLI to handle absence explicitly for byte output.
  
  Regressions cover every resource pick and state, missing versus empty
  worktree and object-database inputs, result IDs and blob writes, external
  driver tempfiles, and binary fallback. Extend the read-only fixture with
  an empty file and refresh its SHA-1 and SHA-256 archives.
  Recorded validation includes both fixture hashes, strict Clippy, rustdoc
  checks, and CLI compilation.

### New Features (BREAKING)

- <csr-id-a5055f4d3bc68d9b26504e7da69476603be5a6fb/> reuse existing index metadata in merge plans
  This commit is just the base for a follow-up that completes how the index is
  kept. For now it keeps only what's important for fast status baselines.
  
  In `gix`, IndexPlan::to_index() now takes an existing index to derive the
  extensions from. The `gix-index` itself now writes the UNTR cache unchanged
  and thus gains serialisation. `gix-index` also gains a method to copy
  metadata from another index, which is a key part in allowing merge indices
  to retain their metadata, with automatic pruning of conflicted paths or those
  that don't exist (in UNTR and TREE).
  
  This method is then merely used in `IndexPlan` when creating a new index and
  a metadata source is present.
  
  <!-- agent -->
  Replacing a repository's index with an `IndexPlan` currently discards all
  filesystem stats and caches. Git's `ort` merge instead carries unchanged
  entries and useful `TREE`/`UNTR` information forward from the existing index.
  Preserve that information when materializing a merge for the same worktree,
  without making detached, tree-only merges depend on a live index.
  
  Accept an optional borrowed `gix_index::State` in `IndexPlan::to_index()` and
  the `gix` outcome wrappers. A linear walk over sorted entries reuses stats
  and applicable flags only for unchanged stage-zero paths. Keep the source
  timestamp and in-memory version, and smudge racy cached sizes so rewriting
  the index cannot turn an unchecked content change into a clean stat match.
  Changed paths and conflict stages receive zero stats; checkout supplies
  stats for rewritten stage-zero files.
  
  Clone and selectively invalidate `TREE` and `UNTR`, keeping unaffected
  cached subtrees and directories. Add `UNTR` serialization so those caches
  survive writing the result. Discard resolve-undo, split-index, filesystem
  monitor, and decoded layout data that cannot be transferred as-is. Adapt
  all callers, and preserve Tix's existing cache-discarding behavior when
  resetting selected tracked paths.
- <csr-id-a8976b9f0a623fbba485016d22fd3e55aadb2b06/> fast-forward gitlink updates using submodule history
  A nice change that adds context to gix-merge which allows to answer the questions
  relevant to submodule conflicts, opening repos on the fly which on `gix` side
  are cached as well so will be opened only once.
  The implementation also handles submodules that aren't iterable, just like Git.
  Plenty of tests, rubber-stamped, all read-only.
  
  <!-- agent -->
  Two branches can update a submodule to different commits on the same
  line of history. Select the descendant when both commits descend from
  the recorded gitlink base and one contains the other. Divergent updates,
  rewinds, and unavailable history still follow the tree conflict policy.
  
  Query commit availability and ancestry at the original gitlink path,
  including during rename resolution and virtual merge-base construction.
  Direct callers of `gix_merge::tree()`, `gix_merge::commit()`, and
  `gix_merge::commit::virtual_merge_base()` now pass a `tree::Context`
  before options. Use `Context::default()` when no history provider is
  available.
  
  In `gix`, look for the submodule checkout before consulting
  `.gitmodules`, and cache the lookup for the merge. Read and decode the
  three gitlink commits through the replacement-aware object database.
  Use a compatible commit graph for reachability, respecting shallow
  boundaries and replacement refs.
  
  Add `try_path()` to skip missing path fields while reporting invalid
  values during merge discovery, status, and index listing.
  `gix_submodule::File::name_by_path()` now returns `Result<Option<&BStr>>`
  to make the same distinction.
  
  Compare trees and index stages with Git, including a missing tip still
  listed in the commit graph and a deinitialized submodule listed after
  a section without a path.
- <csr-id-e574d0edfc1d7d61cc775ff4189ff56466085d30/> make inferred directory renames configurable
  <!-- agent -->
  The identical-additions regression needed `merge.directoryRenames=true`
  in Git to match the unconditional directory relocation in `gix-merge`.
  Identical contents make the content merge clean, but do not establish that
  the inferred file location is correct. Git distinguishes these decisions
  and defaults to asking for confirmation of the location.
  
  Add `tree::Options::directory_renames` with three policies, matching Git:
  
  - `Disabled` leaves new paths at their explicit locations.
  - `Enabled` relocates additions and rename destinations automatically.
  - `Conflict` performs the relocation but records a file-location conflict,
  even when contents merge cleanly. This is the default.
  
  Retain structural directory boundaries when relocation is disabled without
  disabling explicit file-rename detection. When `rewrites` is `None`,
  directory renames cannot be inferred and every policy behaves as disabled.
  
  Record `ResolutionFailure::DirectoryRename` with the suggested destination
  and any content-merge outcome. Preserve unmerged index stages for relocated
  paths, including identical additions and renames whose destinations converge.
  Make location conflicts participate in early stopping and forced tree
  resolutions, applying ancestor/ours choices to the resulting tree rather
  than merely changing the conflict classification. Automatic relocation
  is no longer mistaken for an evasive unresolved rename.
  
  Disable inferred directory relocation while constructing virtual merge
  bases, matching `merge-ort` at nonzero call depth, while retaining explicit
  file-rename tracking.
  
  Expose the policy through `gix` and `with_directory_renames()`. Read
  `merge.directoryRenames` for repository-derived options, resolving absent
  configuration from the schema-declared `conflict` default.
  
  Extend read-only Git baselines to compare trees and index stages for all
  three modes in both merge directions, covering additions, rename destinations,
  source modifications, converging destinations, and disabled rewrite tracking.
  Add regression coverage for defaults, early stopping, forced resolutions,
  virtual merge bases, configuration validation, and the porcelain builder.
  Keep existing automatic-relocation fixtures explicitly in `Enabled` mode.
  
  ### Breaking changes
  
  - Exhaustive `gix_merge::tree::Options` struct literals must initialize the
  new `directory_renames` field.
  - Exhaustive `ResolutionFailure` matches must handle `DirectoryRename`.
  - The default policy now reports file-location conflicts, matching Git.
  Select `DirectoryRenames::Enabled` explicitly to retain automatic
  relocation without these conflicts when contents merge cleanly.

### Bug Fixes (BREAKING)

- <csr-id-456bc0e07cccb3eb8d7ca38998330af101dcd2bf/> preserve Git's traversal order for ambiguous renames
  gix-diff really only gets a path-order for renamed directories, and the code for
  that is simple, adding a `Context` arg as breaking change.
  Producing this `Context` is some new code in gix-merge, but most of it is tests as usual.
  One divergence gets removed.
  
  <!-- agent -->
  Repeated contents make rename identity depend on which destination consumes a
  source first. Two independent, path-sorted diffs cannot reproduce the order
  `merge-ort` derives from all three inputs, which changes the resulting conflict
  stages even when the merged tree is identical.
  
  Derive initial and deferred destination order from the existing immutable path
  state, including unchanged sibling placeholders. Retain Git's pending-directory
  hash order because it affects those identities. Pass a borrowed `Context` into
  the shared `gix-diff` matcher; ordinary callers keep their existing defaults and
  no second rename implementation is needed.
- <csr-id-c64b5531c5487acbda313156a9e5bc00679673a2/> distinguish mode conflicts and ancestor restorations
  This was purely a rubber-stamp unfortunately, and the merge code is deeply
  in the realm of AI controlled by now. It's too complex to figure out desirable
  results in a reasonable time, and... mergeORT is basically calling the shots.
  
  Maybe the only realistic way of ever complying is to implement the actual
  algorithm more closely, adjusting the underlying data structures as well
  the the introspectable, non-index results as much as possible.
  
  <!-- agent -->
  Replacing a non-blob ancestor with regular files can resolve the content
  while leaving conflicting executable bits. Retain the current side's mode
  and report `ResolutionFailure::ModeConflict` independently of content
  resolution. Forced tree choices restore the whole selected entry and omit
  the discarded content merge. Virtual merge bases retain the replacement
  file so later criss-cross merges preserve distinct-type conflicts.
  
  Represent symlink policies that restore the ancestor with
  `Resolution::OursModifiedTheirsModifiedThenAncestor`. The restored object
  need not be a blob, so preserve its mode and identity in the ancestor index
  entry instead of exposing it as a content merge. Unresolved checks classify
  this choice as a forced content resolution.
  
  Merge modes relative to the ancestor when both sides rename to the same
  destination, preserving executable-bit removal alongside content edits.
  
  Extend Git-reference fixtures for mode conflicts, criss-cross merges, and
  renamed executable files. Add synthetic assertions for forced resolutions
  and index stages, and refresh the SHA-1 and SHA-256 fixture archives.
- <csr-id-696e2183da8ed848fff4eb8be02dd465b13b888d/> preserve entries across directory and type conflicts
  While I looked at everything, it's mostly a rubber-stamp, the code has
  always been ridiculouly complex and is held together by AI logic
  (by now) and tests.
  Yet I think the individual pieces fit together well, and once the rename-tracker
  learned all the tricks it should be correct in more ambiguous cases as well.
  
  <!-- agent -->
  Structural merge conflicts must retain surviving entries, avoid relocating
  independent changes, and describe the same paths in the provisional tree and
  conflict index. This combines the related fixes from #3106, #3107, #3108, and
  #3109, using Git's `merge-ort` behavior and generated baselines as the reference.
  
  Ignore a directory/file overlap when every indexed descendant is a deletion.
  Check only the affected subtree, and retain deletions consumed without
  application when forced ancestor resolution may have kept their entries.
  Match explicit rename destinations against modified directories too, retaining
  both the directory and the relocated file or symlink. Remove the rename source
  only when the rename is applied, including forced tree resolutions.
  
  Retain both sides when regular files, symlinks, or gitlinks clash at one path.
  Move regular files aside, or both entries when neither is a regular file,
  checking both change indexes and the ancestor tree for occupied names. Keep
  base stages only beside compatible types and retry relocated additions so
  later collisions remain visible.
  
  Bound inferred parent-directory renames to child pairs with matching relative
  directory paths, as in Git's `update_dir_rename_counts()`. A move from `d/e` to
  `y/z` must leave an independent `d/c` addition in place; changing only the file
  basename may still imply a parent rename. Clear tracked child indices on reuse.
  
  Git-backed fixtures cover both merge directions, forced ancestor/ours choices,
  distinct types, occupied relocation paths, and directory-suffix controls. Three
  older tree deviations and two obsolete index overrides now use Git directly.
  The remaining ambiguous identical-blob fixture records the selected source's
  base mode at the paths present in the provisional tree.
  
  ### Breaking change
  
  Exhaustive matches on `gix_merge::tree::ResolutionFailure` must handle
  `DistinctTypes`, which exposes the resulting location of each retained side.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 51 commits contributed to the release.
- 31 days passed between releases.
- 27 commits were understood as [conventional](https://www.conventionalcommits.org).
- 3 unique issues were worked on: [#3091](https://github.com/GitoxideLabs/gitoxide/issues/3091), [#3097](https://github.com/GitoxideLabs/gitoxide/issues/3097), [#3110](https://github.com/GitoxideLabs/gitoxide/issues/3110)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#3091](https://github.com/GitoxideLabs/gitoxide/issues/3091)**
  - Avoid merging identical edits across a rename ([`fa9a6bd`](https://github.com/GitoxideLabs/gitoxide/commit/fa9a6bd70a1d494e82fc810f64a4de466c2ac3c7))
- **[#3097](https://github.com/GitoxideLabs/gitoxide/issues/3097)**
  - Merge equal blob content with different modes ([`9430019`](https://github.com/GitoxideLabs/gitoxide/commit/9430019b7eb2a9ae5a05e032b6422c905ec33fb2))
- **[#3110](https://github.com/GitoxideLabs/gitoxide/issues/3110)**
  - Preserve overlapping edits when collecting text hunks ([`e1a1001`](https://github.com/GitoxideLabs/gitoxide/commit/e1a1001d0f53347a4cbbff15926511a401776437))
- **Uncategorized**
  - Release gix-error v0.5.3, gix-macros v0.2.2, gix-date v0.19.0, gix-actor v0.45.0, gix-trace v0.3.2, gix-validate v0.13.2, gix-path v0.14.2, gix-utils v0.5.3, gix-quote v0.10.2, gix-command v0.12.2, gix-hash v0.29.0, gix-hashtable v0.19.0, gix-parallel v0.50.2, gix-fs v1.0.0, gix-tempfile v27.0.0, gix-object v0.67.0, gix-glob v0.30.0, gix-attributes v0.38.0, gix-zlib v0.4.1, gix-encoding v0.1.0, gix-packetline v0.24.2, gix-filter v0.37.0, gix-chunk v0.10.2, gix-commitgraph v0.43.0, gix-revwalk v0.39.0, gix-traverse v0.65.0, gix-worktree-stream v0.40.0, gix-archive v0.40.0, gix-bitmap v0.6.2, gix-lock v27.0.0, gix-index v0.59.0, gix-config-value v0.21.2, gix-pathspec v0.23.0, gix-ignore v0.25.0, gix-worktree v0.60.0, gix-imara-diff v0.3.4, gix-diff v0.71.0, gix-blame v0.21.0, gix-ref v0.70.0, gix-sec v0.17.0, gix-config v0.63.0, gix-prompt v0.19.2, gix-url v0.41.0, gix-credentials v0.43.0, gix-discover v0.58.0, gix-dir v0.33.0, gix-fsck v0.28.0, gix-notify v0.1.0, gix-fsmonitor v0.1.0, gix-mailmap v0.36.2, gix-revision v0.53.0, gix-merge v0.24.0, gix-negotiate v0.39.0, gix-note v0.4.0, gix-pack v0.78.0, gix-odb v0.88.0, gix-refspec v0.48.0, gix-shallow v0.16.0, gix-transport v0.63.0, gix-protocol v0.69.0, gix-status v0.38.0, gix-submodule v0.37.0, gix-worktree-state v0.38.0, gix v0.91.0, gitoxide-core v0.65.0, gix-tix v0.7.0, gitoxide v0.62.0, safety bump 44 crates ([`df27410`](https://github.com/GitoxideLabs/gitoxide/commit/df27410adf06a49413eb84b9d6bc0a633ff94b78))
  - Merge pull request #3228 from GitoxideLabs/tree-panic ([`aa9d8f1`](https://github.com/GitoxideLabs/gitoxide/commit/aa9d8f14ded749590bc9267b357b702837cc5035))
  - Keep ancestor directories out of symlink conflict paths ([`5f2af64`](https://github.com/GitoxideLabs/gitoxide/commit/5f2af64257d4f6c93ea334eb895514443f013634))
  - Merge pull request #3207 from GitoxideLabs/fix/gix-merge-0.23-correctness ([`3e89f3b`](https://github.com/GitoxideLabs/gitoxide/commit/3e89f3b56f88f7080d0b42e9619ae4b843ba8b5d))
  - Reuse matching entries and current-side trees ([`c5ef9eb`](https://github.com/GitoxideLabs/gitoxide/commit/c5ef9eb78d819abffe0e5f9b30c664b274b2518d))
  - Stream and compact three-way merge inputs ([`61f57b4`](https://github.com/GitoxideLabs/gitoxide/commit/61f57b4afad391f99649eb20ce77e83eb6841000))
  - Reuse existing index metadata in merge plans ([`a5055f4`](https://github.com/GitoxideLabs/gitoxide/commit/a5055f4d3bc68d9b26504e7da69476603be5a6fb))
  - Fast-forward gitlink updates using submodule history ([`a8976b9`](https://github.com/GitoxideLabs/gitoxide/commit/a8976b9f0a623fbba485016d22fd3e55aadb2b06))
  - Preserve Git's traversal order for ambiguous renames ([`456bc0e`](https://github.com/GitoxideLabs/gitoxide/commit/456bc0e07cccb3eb8d7ca38998330af101dcd2bf))
  - Match Git's text conflict regions and large-file handling ([`85476b3`](https://github.com/GitoxideLabs/gitoxide/commit/85476b3136cbf748e60e672e3ac533d3cba2a899))
  - Renormalize files before declaring modify/delete conflicts ([`62bae58`](https://github.com/GitoxideLabs/gitoxide/commit/62bae58d2eb246b9b20dccdb85b6643f1e693b9d))
  - Build merges and indexes from normalized three-way paths ([`d75dce4`](https://github.com/GitoxideLabs/gitoxide/commit/d75dce4cd4a240816eaf18846d178d36a5c3de4a))
  - Merge pull request #3214 from GitoxideLabs/fix-merge-tree ([`b8e1eb2`](https://github.com/GitoxideLabs/gitoxide/commit/b8e1eb2a3deaf67d82bdfec0ddad43b693639d8b))
  - Preserve explicit renames blocked by directory relocation ([`2d8389f`](https://github.com/GitoxideLabs/gitoxide/commit/2d8389f5c4da336b4b3a4735db07a60dbced592e))
  - Merge pull request #3211 from GitoxideLabs/jiff-upgrade ([`12eff45`](https://github.com/GitoxideLabs/gitoxide/commit/12eff45f0e16fa7a9f75086034ccb0198f55b9f3))
  - Merge pull request #3202 from PJUllrich/fix/gix-merge-0.23-correctness ([`a8eeec9`](https://github.com/GitoxideLabs/gitoxide/commit/a8eeec9516a55d713e9671da3cf7606f121b5e9d))
  - Preserve files at directory conflict destinations ([`532f5a4`](https://github.com/GitoxideLabs/gitoxide/commit/532f5a40c7097c75244f8abd10cd0e7ac2713a5e))
  - Share entry and distinct-type conflict resolution ([`9cd5c00`](https://github.com/GitoxideLabs/gitoxide/commit/9cd5c00409f7f5f592c844d5a412d644d8624d5f))
  - Preserve renamed ancestors and binary versions ([`cff89ce`](https://github.com/GitoxideLabs/gitoxide/commit/cff89ce7dc399ba55305a4aee72bec9dfcca7229))
  - Preserve renamed paths in recursive merges ([`301474e`](https://github.com/GitoxideLabs/gitoxide/commit/301474e849a0146ee7876cc0872308cb984e4abe))
  - Preserve conflict policies across additions and renames ([`bf0b935`](https://github.com/GitoxideLabs/gitoxide/commit/bf0b9359f14423aeeebf127c4f8fbce95ccca361))
  - Distinguish mode conflicts and ancestor restorations ([`c64b553`](https://github.com/GitoxideLabs/gitoxide/commit/c64b5531c5487acbda313156a9e5bc00679673a2))
  - Preserve modes across rename/edit merges ([`3dff1b0`](https://github.com/GitoxideLabs/gitoxide/commit/3dff1b0f050d22a110bfa07fdbe2861348ab8afc))
  - Merge pull request #3198 from GitoxideLabs/fix-long-paths ([`c2c487c`](https://github.com/GitoxideLabs/gitoxide/commit/c2c487c550f3e0a7a07fb1a37f9039fef2869dbf))
  - Keep Windows QA free of platform-specific warnings ([`9f331f0`](https://github.com/GitoxideLabs/gitoxide/commit/9f331f084ded92bb6e9dcf1d2dc677edc9f1cfa0))
  - Merge pull request #3162 from GitoxideLabs/nullid-is-option-id ([`8050181`](https://github.com/GitoxideLabs/gitoxide/commit/8050181d59102772e3f1148f28e140e1986d05ee))
  - Invalidate resources after failed setup ([`aa4956d`](https://github.com/GitoxideLabs/gitoxide/commit/aa4956d78c1dcf6dbe4c6ae855e47c48eaaf75ca))
  - Preserve missing and unread merge resource states ([`25401d0`](https://github.com/GitoxideLabs/gitoxide/commit/25401d01fb6dcfb57061c0d2cec4d77c098973f8))
  - Merge pull request #3171 from GitoxideLabs/make-merge-resolution-clearer ([`bc4d813`](https://github.com/GitoxideLabs/gitoxide/commit/bc4d8135306797b528170709b231faf3e6ebe170))
  - Document and test preservation of external merge driver results ([`a72c2a1`](https://github.com/GitoxideLabs/gitoxide/commit/a72c2a1f54428eb62db5562ac292467cc54f5aa7))
  - Merge pull request #3155 from arthurzam/fix-merge-driver-conflict-exit ([`89c3026`](https://github.com/GitoxideLabs/gitoxide/commit/89c302650b4ae97e8315b10067436f7056e5fc9b))
  - Expose external driver commands in error metadata ([`1bf1346`](https://github.com/GitoxideLabs/gitoxide/commit/1bf13466d1092b5aec815ff58debd01a06d89d2a))
  - Merge pull request #3152 from GitoxideLabs/even-faster-qa ([`2baa819`](https://github.com/GitoxideLabs/gitoxide/commit/2baa819ba6d4b0c6177e00902928ce94ee7f537e))
  - Treat merge driver exit codes 1 to 128 as conflicts, like Git does ([`d4ccfca`](https://github.com/GitoxideLabs/gitoxide/commit/d4ccfca493fa4c74e7ff98c5e21feb711007ddae))
  - Batch commit lookups in Cartesian merge fixtures ([`9c9b443`](https://github.com/GitoxideLabs/gitoxide/commit/9c9b443e4ba02bbcd90c7148c270b844799a7452))
  - Skip unused expected-tree lookups in merge fixtures ([`ddc63a3`](https://github.com/GitoxideLabs/gitoxide/commit/ddc63a3ec2a2c754b1e36c476ab1bdd537e7f894))
  - Merge pull request #3146 from GitoxideLabs/publish-gix-macros ([`9dd78c8`](https://github.com/GitoxideLabs/gitoxide/commit/9dd78c800d554c18fc99816bf922c93d71402881))
  - Release gix-macros v0.2.1 ([`16940c7`](https://github.com/GitoxideLabs/gitoxide/commit/16940c743ac2688de34fc854906469ed061d6a56))
  - Merge pull request #3143 from GitoxideLabs/fast-ci ([`c4a28f8`](https://github.com/GitoxideLabs/gitoxide/commit/c4a28f8c9331603ee2e2928974ee587a23026c49))
  - Discover QA work from manifests and share it between local runs and CI ([`8bf25a2`](https://github.com/GitoxideLabs/gitoxide/commit/8bf25a232d00f71f73b964f39d509ac9ae976efe))
  - Merge pull request #3061 from GitoxideLabs/stable-gix ([`5c88568`](https://github.com/GitoxideLabs/gitoxide/commit/5c88568c5ad8ab85ef4d60d64b4ab48958dc3226))
  - Merge pull request #3092 from GitoxideLabs/brace-brace ([`15b71a3`](https://github.com/GitoxideLabs/gitoxide/commit/15b71a3a0bfbab5e1667d26f2259805f4fecb216))
  - Preserve entries across directory and type conflicts ([`696e218`](https://github.com/GitoxideLabs/gitoxide/commit/696e2183da8ed848fff4eb8be02dd465b13b888d))
  - Preserve relocated merges and rename/delete base stages ([`fd9cf42`](https://github.com/GitoxideLabs/gitoxide/commit/fd9cf42a4a81d4cd3b95594d0ede6d7d938e960f))
  - Make inferred directory renames configurable ([`e574d0e`](https://github.com/GitoxideLabs/gitoxide/commit/e574d0edfc1d7d61cc775ff4189ff56466085d30))
  - Merge pull request #3117 from GitoxideLabs/release-testtools ([`8929480`](https://github.com/GitoxideLabs/gitoxide/commit/89294801971c59a8379344ca4404aa36f0305761))
  - Release gix-utils v0.5.2, gix-parallel v0.50.1, gix-hashtable v0.18.1, gix-sec v0.16.1, gix-config v0.62.2, gix-ignore v0.24.1, gix-imara-diff v0.3.3, gix-testtools v0.22.0 ([`584a602`](https://github.com/GitoxideLabs/gitoxide/commit/584a602385dd3ad4dacfdd9fd77a13c193a87dfe))
  - Merge pull request #3083 from GitoxideLabs/report-october ([`e87b6ee`](https://github.com/GitoxideLabs/gitoxide/commit/e87b6eefbcaa77156ddee85576c847e395ec0586))

</details>

## 0.23.0 (2026-10-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 5 commits contributed to the release over the course of 14 calendar days.
- 14 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.2, gix-date v0.18.1, gix-actor v0.44.1, gix-trace v0.3.1, gix-validate v0.13.1, gix-path v0.14.1, gix-quote v0.10.1, gix-command v0.12.1, gix-hash v0.28.1, gix-fs v0.24.1, gix-tempfile v26.0.1, gix-object v0.66.1, gix-glob v0.29.1, gix-attributes v0.37.1, gix-packetline v0.24.1, gix-filter v0.36.1, gix-chunk v0.10.1, gix-commitgraph v0.42.0, gix-revwalk v0.38.0, gix-traverse v0.64.0, gix-worktree-stream v0.39.0, gix-archive v0.39.0, gix-bitmap v0.6.1, gix-lock v26.0.1, gix-index v0.58.0, gix-config-value v0.21.1, gix-pathspec v0.22.1, gix-worktree v0.59.0, gix-imara-diff v0.3.2, gix-diff v0.70.0, gix-blame v0.20.0, gix-ref v0.69.1, gix-config v0.62.1, gix-prompt v0.19.1, gix-url v0.40.1, gix-credentials v0.42.1, gix-discover v0.57.1, gix-dir v0.32.0, gix-mailmap v0.36.1, gix-revision v0.52.0, gix-merge v0.23.0, gix-negotiate v0.38.0, gix-note v0.3.1, gix-zlib v0.4.0, gix-pack v0.77.0, gix-odb v0.87.0, gix-refspec v0.47.1, gix-shallow v0.15.1, gix-transport v0.62.0, gix-protocol v0.68.0, gix-status v0.37.0, gix-submodule v0.36.1, gix-worktree-state v0.37.0, gix v0.90.0, gix-fsck v0.27.1, gitoxide-core v0.64.0, gix-tix v0.6.0, gitoxide v0.61.0, safety bump 21 crates ([`dd39fb6`](https://github.com/GitoxideLabs/gitoxide/commit/dd39fb66e3b1551c9fe849043e39b357a9799ca1))
  - Merge pull request #3077 from GitoxideLabs/fix-merge-file ([`4439d41`](https://github.com/GitoxideLabs/gitoxide/commit/4439d41098e0b4a82f99f270fe92d0bd83965b3e))
  - Merge pull request #3053 from GitoxideLabs/various-improvements ([`2d8dcc4`](https://github.com/GitoxideLabs/gitoxide/commit/2d8dcc466750e474e6c51ac636c24259563a26cf))
  - Release gix-error v0.5.1, gix-utils v0.5.1 ([`6d1b686`](https://github.com/GitoxideLabs/gitoxide/commit/6d1b68632cec55fa5d3f752ca97daca4e5485b6c))
  - Merge pull request #3048 from GitoxideLabs/new-release ([`5fccccf`](https://github.com/GitoxideLabs/gitoxide/commit/5fccccfdedd17f7d4b0cc57039d118a3496aa8aa))

</details>

## 0.22.0 (2026-10-08)

### Bug Fixes

- <csr-id-18f476cdd3956f6c339d912646205ad1ca5c1ec7/> don't remove a directory when removing the file it replaced during tree merges
  One side of a merge can delete or rename away the file `a/b` and fill a new directory
  `a/b/` purely through renames. Rename tracking reports those renames before the unpaired
  deletion (or the rename away) of `a/b`, so the tree editor received `a/b/g` first and then
  removed `a/b`, dropping the whole new directory. The merge reported success, and the result
  silently lost every file under `a/b/`. It happened in both merge directions, whether or not
  the other side changed anything, and inside virtual merge bases, where it turned later edits
  below `a/b/` into add/add conflicts.
  
  Every removal in the tree merge now goes through `Editor::remove_leaf()` and leaves a tree at
  the removed path in place, since tree changes are never applied directly and the leaf changes
  below a tree carry every update to it. Three baselines cover a deletion and a rename of the
  replaced file, the latter with the other side's edit following the rename, and the shape
  with two merge bases. Git 2.51.0 `merge-tree` keeps `a/b/` in all of them.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 12 commits contributed to the release over the course of 13 calendar days.
- 13 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.0, gix-date v0.18.0, gix-actor v0.44.0, gix-trace v0.3.0, gix-validate v0.13.0, gix-path v0.14.0, gix-utils v0.5.0, gix-quote v0.10.0, gix-command v0.12.0, gix-hash v0.28.0, gix-hashtable v0.18.0, gix-parallel v0.50.0, gix-fs v0.24.0, gix-tempfile v26.0.0, gix-object v0.66.0, gix-glob v0.29.0, gix-attributes v0.37.0, gix-packetline v0.24.0, gix-filter v0.36.0, gix-chunk v0.10.0, gix-commitgraph v0.41.0, gix-revwalk v0.37.0, gix-traverse v0.63.0, gix-worktree-stream v0.38.0, gix-archive v0.38.0, gix-bitmap v0.6.0, gix-lock v26.0.0, gix-index v0.57.0, gix-config-value v0.21.0, gix-pathspec v0.22.0, gix-ignore v0.24.0, gix-worktree v0.58.0, gix-imara-diff v0.3.1, gix-diff v0.69.0, gix-blame v0.19.0, gix-ref v0.69.0, gix-sec v0.16.0, gix-config v0.62.0, gix-prompt v0.19.0, gix-url v0.40.0, gix-credentials v0.42.0, gix-discover v0.57.0, gix-dir v0.31.0, gix-mailmap v0.36.0, gix-revision v0.51.0, gix-merge v0.22.0, gix-negotiate v0.37.0, gix-note v0.3.0, gix-zlib v0.3.0, gix-pack v0.76.0, gix-odb v0.86.0, gix-refspec v0.47.0, gix-shallow v0.15.0, gix-transport v0.61.0, gix-protocol v0.67.0, gix-status v0.36.0, gix-submodule v0.36.0, gix-worktree-state v0.36.0, gix v0.89.0, gix-fsck v0.27.0, gitoxide-core v0.63.0, gix-tix v0.5.0, gitoxide v0.60.0, safety bump 62 crates ([`0115e90`](https://github.com/GitoxideLabs/gitoxide/commit/0115e902fd0334b8f370766993eb17babed3157f))
  - Prepare changelogs prior to release ([`794eeef`](https://github.com/GitoxideLabs/gitoxide/commit/794eeef835f8dde0fc8f4fd1d215f304d28d7f77))
  - Merge pull request #3032 from GitoxideLabs/sec-audit ([`1d7bac7`](https://github.com/GitoxideLabs/gitoxide/commit/1d7bac742f70b72ddda0c7294f4a97566b8db596))
  - Use `bstr` helpers for byte substring searches ([`21b210a`](https://github.com/GitoxideLabs/gitoxide/commit/21b210aab99b30c660d0de497bf2045aefa2fa09))
  - Merge pull request #3033 from GitoxideLabs/gix-cli-progress-cleanup ([`80f4b03`](https://github.com/GitoxideLabs/gitoxide/commit/80f4b03257da9a664468e7b75247023187565a34))
  - Merge pull request #3022 from GitoxideLabs/release-testtools ([`f819565`](https://github.com/GitoxideLabs/gitoxide/commit/f819565c2c4c56619c4888acef6cf3b8144cbccb))
  - Merge pull request #3027 from cruessler/remove-sha-1-default-in-pipeline ([`06b0098`](https://github.com/GitoxideLabs/gitoxide/commit/06b0098a65d66b9c7c9e652c6b17a231f65b1039))
  - Merge pull request #3024 from goldjacobe/fix-tree-merge-keep-dir-on-leaf-removal ([`3f1ef3a`](https://github.com/GitoxideLabs/gitoxide/commit/3f1ef3afd77e493276dac656b6c944aacd75f2ac))
  - Review ([`9ef2352`](https://github.com/GitoxideLabs/gitoxide/commit/9ef23525bb9098e1175bacb7882c9e40acacb15f))
  - Don't remove a directory when removing the file it replaced during tree merges ([`18f476c`](https://github.com/GitoxideLabs/gitoxide/commit/18f476cdd3956f6c339d912646205ad1ca5c1ec7))
  - Merge pull request #3020 from GitoxideLabs/report-september ([`5fb3dcf`](https://github.com/GitoxideLabs/gitoxide/commit/5fb3dcf6a86ac0c403776c8820bf5d23f187f7e1))
  - Fix CI ([`b3080a6`](https://github.com/GitoxideLabs/gitoxide/commit/b3080a63a0484b4880c527ed1bbc9665c385ccfe))

</details>

## 0.21.0 (2026-09-25)

### Bug Fixes

- <csr-id-2a910eeefb0dc3e1b81ff5670c993195067fc89b/> skip null-ID lookups in worktree filters
  <!-- agent -->
  Worktree merge resources naturally use null IDs when their content comes
  from a file path, as in `gix merge file`. Automatic text filters queried
  those IDs to inspect indexed line endings, causing guaranteed misses and
  unnecessary object-database refreshes.
  
  Treat the null sentinel as a missing index blob before querying objects.
  Clarify the resource contract and strengthen the existing `worktree_filter`
  test to reject object lookups for null-ID resources in both conversion
  modes. Existing assertions retain coverage for indexed CRLF content and
  renormalization.
- <csr-id-66d2daf44bccbadee661e6368d70daff915b1215/> ignore diff-local metadata for identical changes
  <!-- agent -->
  Tree diffs assign relation IDs independently. When both sides add the same
  entry under newly added directories, unrelated sibling changes can give those
  additions different `ChildOfParent` IDs. Comparing the complete `Change` values
  then routes an agreed tree effect to `ResolutionFailure::Unknown`.
  
  Compare the semantic tree effect instead: change kind, source and destination
  paths, modes, object IDs, and rewrite copy behavior. Ignore relation IDs and
  rewrite diff statistics, which do not affect the resulting tree. This also
  coalesces equivalent deletions and rewrites through the same matcher.
  
  Add bidirectional fixtures with an asymmetric earlier directory change so the
  equivalent target operations receive different diff-local relation IDs. The
  add/add fixture reproduces the pre-fix Unknown resolution, while deletion and
  rewrite fixtures verify the broader class against Git in both directions.
  
  Git baseline: `merge-ort.c` at `1630431f326e15fcde608827b5ff38422528eb59` sets
  `match_mask = 6` when both sides have the same mode and object ID, then resolves
  that entry cleanly.

### Changed (BREAKING)

- <csr-id-7d42d81eb3463b6e64142cb8306d8d176124b8d7/> migrate errors to gix-error
  <!-- Byron -->
  rubberstamp
- <csr-id-4b42e0ce80ae934cae4f102f44c392581758608f/> raise MSRV to Rust 1.88
  <!-- agent -->
  The newly published `dua-core` 3.3 release used by linked-worktree removal
  requires Rust 1.88, so raise every workspace crate and the advertised badge
  together.
  
  Keep the MSRV checks buildable by selecting the latest `sysinfo` and `rusqlite`
  release lines that support Rust 1.88.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 16 commits contributed to the release over the course of 31 calendar days.
- 32 days passed between releases.
- 4 commits were understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#2968](https://github.com/GitoxideLabs/gitoxide/issues/2968)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#2968](https://github.com/GitoxideLabs/gitoxide/issues/2968)**
  - Ignore diff-local metadata for identical changes ([`66d2daf`](https://github.com/GitoxideLabs/gitoxide/commit/66d2daf44bccbadee661e6368d70daff915b1215))
- **Uncategorized**
  - Release gix-error v0.4.0, gix-date v0.17.0, gix-actor v0.43.0, gix-trace v0.2.0, gix-validate v0.12.0, gix-path v0.13.0, gix-utils v0.4.0, gix-quote v0.9.0, gix-command v0.11.0, gix-features v0.50.0, gix-hash v0.27.0, gix-hashtable v0.17.0, gix-fs v0.23.0, gix-tempfile v25.0.0, gix-object v0.65.0, gix-glob v0.28.0, gix-attributes v0.36.0, gix-packetline v0.23.0, gix-filter v0.35.0, gix-chunk v0.9.0, gix-commitgraph v0.40.0, gix-revwalk v0.36.0, gix-traverse v0.62.0, gix-worktree-stream v0.37.0, gix-archive v0.37.0, gix-bitmap v0.5.0, gix-lock v25.0.0, gix-index v0.56.0, gix-config-value v0.20.0, gix-pathspec v0.21.0, gix-ignore v0.23.0, gix-worktree v0.57.0, gix-imara-diff v0.3.0, gix-diff v0.68.0, gix-blame v0.18.0, gix-ref v0.68.0, gix-sec v0.15.0, gix-config v0.61.0, gix-prompt v0.18.0, gix-url v0.39.0, gix-credentials v0.41.0, gix-discover v0.56.0, gix-dir v0.30.0, gix-mailmap v0.35.0, gix-revision v0.50.0, gix-merge v0.21.0, gix-negotiate v0.36.0, gix-note v0.2.0, gix-zlib v0.2.0, gix-pack v0.75.0, gix-odb v0.85.0, gix-macros v0.2.0, gix-refspec v0.46.0, gix-shallow v0.14.0, gix-transport v0.60.0, gix-protocol v0.66.0, gix-status v0.35.0, gix-submodule v0.35.0, gix-worktree-state v0.35.0, gix v0.88.0, gix-fsck v0.26.0, gitoxide-core v0.62.0, gix-tix v0.4.0, gitoxide v0.59.0, safety bump 60 crates ([`37860b3`](https://github.com/GitoxideLabs/gitoxide/commit/37860b34db26096c8187ef55bdf4b76705142733))
  - Merge pull request #2847 from GitoxideLabs/gix-error-completion ([`6356013`](https://github.com/GitoxideLabs/gitoxide/commit/6356013bca0987c6c97ad7ba9d5347271979b51e))
  - Add error context without preliminary erasure ([`9d0329a`](https://github.com/GitoxideLabs/gitoxide/commit/9d0329af119874c7441bafeac950166a1e13d899))
  - Use borrowed error inspection throughout the workspace ([`daf73b5`](https://github.com/GitoxideLabs/gitoxide/commit/daf73b5fe5a21e3ddcc58f0882c2d880f48b7860))
  - Migrate errors to gix-error ([`7d42d81`](https://github.com/GitoxideLabs/gitoxide/commit/7d42d81eb3463b6e64142cb8306d8d176124b8d7))
  - Merge pull request #2982 from mtsgrd/testtools-config-file-isolation ([`0cd9319`](https://github.com/GitoxideLabs/gitoxide/commit/0cd9319555ff2b30d8eca95eb3befba018b15de5))
  - Review ([`bc9185d`](https://github.com/GitoxideLabs/gitoxide/commit/bc9185d33bc24e85165b53b546d87aa6f7c0de95))
  - Merge pull request #2971 from GitoxideLabs/diff-nullid-fix ([`d7551f1`](https://github.com/GitoxideLabs/gitoxide/commit/d7551f1593ad1e5897ab637a930f0d185310238b))
  - Skip null-ID lookups in worktree filters ([`2a910ee`](https://github.com/GitoxideLabs/gitoxide/commit/2a910eeefb0dc3e1b81ff5670c993195067fc89b))
  - Merge pull request #2969 from GitoxideLabs/merge-fix ([`f0681ba`](https://github.com/GitoxideLabs/gitoxide/commit/f0681bae4db7c78d8d25f15594a8963e40d5817a))
  - Merge pull request #2949 from GitoxideLabs/error-conversion-review ([`a095334`](https://github.com/GitoxideLabs/gitoxide/commit/a0953348e4d27f59222c1782119d2539a778cd4d))
  - Raise MSRV to Rust 1.88 ([`4b42e0c`](https://github.com/GitoxideLabs/gitoxide/commit/4b42e0ce80ae934cae4f102f44c392581758608f))
  - Merge pull request #2955 from GitoxideLabs/transport-url-encoding ([`7e35849`](https://github.com/GitoxideLabs/gitoxide/commit/7e35849b36646cff9722f6906a4527d64818a374))
  - Release gix-path v0.12.6, gix-error v0.3.2, gix-command v0.10.1, gix-transport v0.59.2 ([`888677a`](https://github.com/GitoxideLabs/gitoxide/commit/888677ad2d63a2e3930a02add2de0b4b667a5581))
  - Merge pull request #2940 from GitoxideLabs/vendor-bisync ([`dda600d`](https://github.com/GitoxideLabs/gitoxide/commit/dda600d7ee29a6bda4cf1047d0d1782e76b16f98))

</details>

## 0.20.1 (2026-08-24)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 5 commits contributed to the release over the course of 1 calendar day.
- 2 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-packetline v0.22.2, gix-worktree-stream v0.36.1, gix-archive v0.36.1, gix-diff v0.67.1, gix-blame v0.17.1, gix-dir v0.29.1, gix-mailmap v0.34.1, gix-revision v0.49.1, gix-merge v0.20.1, gix-negotiate v0.35.1, gix-note v0.1.1, gix-pack v0.74.2, gix-macros v0.1.6, gix-refspec v0.45.1, gix-transport v0.59.1, gix-protocol v0.65.1, gix-status v0.34.1, gix-worktree-state v0.34.1, gix v0.87.1, gix-fsck v0.25.1, gitoxide-core v0.61.1, gix-tix v0.3.0, gitoxide v0.58.0, safety bump gitoxide v0.58.0 ([`3ebca8b`](https://github.com/GitoxideLabs/gitoxide/commit/3ebca8b66017ab2dd02a38f75f78f485bee1ded8))
  - Merge pull request #2932 from GitoxideLabs/fundamental-types-comp ([`6704303`](https://github.com/GitoxideLabs/gitoxide/commit/6704303ed5ef3403b129e2b6cc4a9214432ffd03))
  - Release gix-error v0.3.1, gix-hash v0.26.2, gix-object v0.64.1, gix-ref v0.67.1, gix-packetline v0.22.1, gix-pack v0.74.1, gix-testtools v0.20.0 ([`e52fe9d`](https://github.com/GitoxideLabs/gitoxide/commit/e52fe9d03e82437a25bdfb1098e7046ec7e1b558))
  - Use fundamental-type comparisons throughout tests ([`47536a5`](https://github.com/GitoxideLabs/gitoxide/commit/47536a5c2b22da3a9f4892c8af5e460c2d5bda0a))
  - Merge pull request #2933 from GitoxideLabs/report-august ([`b8914ff`](https://github.com/GitoxideLabs/gitoxide/commit/b8914ffda5bc8f6ea851aaf1f720140acfe96dbb))

</details>

## 0.20.0 (2026-08-22)

### Bug Fixes

- <csr-id-61b45a81911561761b1546e8bf74541263b964be/> track whether tree changes were actually applied
  <!-- agent -->
  
  Replace the ambiguous `was_written` flag with an explicit change state:
  
  - `Pending` means the change still needs processing.
  - `Processed` means conflict handling consumed the change without applying it.
  - `Applied` means its effect was written to the tree editor.
  
  This distinction matters for forced ancestor resolution. A deletion may participate
  in a conflict while the ancestor entry is deliberately retained. Treating that
  deletion as written caused later changes below the same path to assume the file
  had been removed and replace it with a directory.
  
  Only suppress subsequent conflicts for deletions that were actually applied.
  Keep applied state monotonic so later bookkeeping cannot downgrade it to merely
  processed.
  
  The expanded rename baselines exposed related ordering issues, to allow a fix:
  
  - Preserve trie children when removing a change attached to an interior node.
  - Associate deferred rewrite additions with the opposing change index so the
  same rename content is not merged twice.
  - Detect different source files renamed onto the same destination.
  - Merge converging rename contents as competing additions instead of silently
  allowing one rename to win.
  - Define forced resolution for converging renames: ancestor retains both source
  files, while ours applies only the selected side's rename.
  
  Add an inspectable rename/change matrix covering:
  
  - rename versus source modification
  - rename versus source deletion
  - rename versus destination addition
  - rename onto a modified destination
  - rename onto a deleted destination
  - the same source renamed to different destinations
  - additions below a renamed directory
  - both sides modifying and choosing the same rename
  - different sources renamed onto the same destination
  - rename/delete combined with file-to-directory replacement
- <csr-id-195f0b99a650c626d951ef7a902733a0e2ac44cc/> treat adjacent text changes as conflicting.
  <!-- agent -->
  
  The text merge previously considered changes from opposing sides to
  overlap only when the later hunk started inside the earlier hunk's
  half-open base range. For adjacent ranges `[a, b)` and `[b, c)`, the
  second hunk starts exactly at the end of the first and therefore wasn't
  considered to overlap.
  
  This caused gix to merge edits to neighboring lines cleanly even though
  there was no unchanged base line separating them. Git instead combines
  such touching changes into one conflicting region.
  
  Treat opposing hunks as disjoint only when there is an actual gap
  between their base ranges. A later hunk whose start is equal to the
  earlier hunk's end now intersects it, while same-side hunks and changes
  separated by an unchanged line retain their previous behavior.
  
  Add direct text-driver coverage for both merge directions. The tests
  verify that adjacent edits produce one conflict with the expected
  content and that edits separated by an unchanged base line still merge
  cleanly while preserving both changes.
- <csr-id-1ee3fc6adf44520ee3397982ec42dbf65a9126f8/> merge additions below jointly deleted files
  <!-- agent -->
  
  When both sides delete a file and one side replaces it with a directory,
  the tree merge matched the directory parent instead of the non-tree deletion
  and later treated a child addition as a conflict. In debug builds this reached
  an assertion for changes at different paths.
  
  Prefer non-tree changes when multiple changes occupy the same lookup path.
  Mark matching deletions as consumed so their child additions can be applied
  without colliding with stale conflict state. Add a bidirectional baseline
  covering the file-to-directory replacement.
  
  Git baseline: Git 2.50.1 merge-tree resolves the fixture cleanly in both
  directions and retains to-be-deleted/a. The reference checkout was at
  a23bace963.

### Changed (BREAKING)

- <csr-id-43cf77fdb52b547e727343644264fe60c05e6589/> Refactor tree-merge change matching and resolution.
  Breaking because of added `ResolutionFailure` variants.
  
  <!-- agent -->
  The tree-merge implementation previously combined side-diff collection,
  path matching, scheduling, and the complete conflict-resolution matrix in a
  single function. It also duplicated change collection for both sides and
  represented pair outcomes with independent boolean flags.
  
  Keep the public tree() entry point as a small facade and move the merge
  engine into focused private modules. Collect each ancestor-to-side diff
  through one helper that constructs a SideState containing the flat change
  list and its matching tree together. Isolate path and rename candidate
  matching, including identical-change suppression, from classification into
  pairs that the resolution matrix understands.
  
  Represent the result of handling each paired change explicitly as a
  ChangeDisposition. This preserves the important distinction between a
  change that was merely processed and one whose effect is present in the
  editor, without duplicating the final state transition in the scheduler.
  Replace the type-specific side-picking helpers with generic pick() and
  pick_mut() helpers as well.
  
  These boundaries make the state machine easier to review and reduce the
  chance that a future rename or forced-resolution fix accidentally changes
  collection, matching, and application at once. They also remove duplicated
  setup while keeping the exhaustive resolution match in one place, where
  its symmetry remains visible. The public API and all recorded merge results
  remain unchanged.
  
  Fixes and Improvements
  ----------------------
  
  Tree merging combines a flat change schedule with per-side path indexes. Valid
  Git operations can therefore arrive in different orders or expose structural
  relationships before the leaf changes that ultimately apply them. Several
  resolver branches treated those relationships as physical occupancy or as
  content changes for the same identity, leading to hangs, assertions, duplicate
  entries, lost siblings, or merge results that depended on diff and side order.
  
  Separate unique-path occupancy from PassedRewrittenDirectory scheduling so a
  side-qualified name can terminate below directory rewrites. Prune empty path
  nodes back to the root, and allow a deferred rewrite to insert only its new
  destination because its source is already indexed.
  
  Resolve the structural cases at their actual identity boundaries:
  - handle an added file blocking an added directory before mode-specific add/add
  resolution and defer early descendants until their parent deletion runs;
  - keep explicit file renames ahead of inferred directory renames, and keep
  directory replacements at their explicit sources;
  - treat file replacements of incompatible non-blob ancestors as additions with
  an empty compatible merge base;
  - pair shared deletions before descendants and allow file renames into paths
  vacated by directory renames;
  - preserve unrelated nested or overlapping rename destinations by keeping the
  directory in place and moving only the blocking file;
  - reject incompatible same-destination rewrites before blob merging, while
  collapsing identical rewrites to one clean shared destination; and
  - defer file-to-directory children until the parent rename/delete decision is
  made exactly once.
  
  Forced Ancestor and Ours resolution continues to apply only the selected side.
  Git-backed baselines cover both directions, forced policies, modes, symlinks,
  gitlinks, nested directories, and documented index-only deviations. The
  resulting suite contains 155 directional baseline cases, and the Cartesian
  model reaches 210/210 Git/gix agreement for trees and path/mode results.
  
  More Hardening
  --------------
  
  Deferred tree changes may be reconsidered after another conflict has already
  consumed or pruned the same path-tree node. This is valid when rename detection
  has ambiguous identical sources, when structural conflicts overlap, or when a
  change follows a detected directory rename. The editor and conflict records
  still contain the required state, but strict bookkeeping removals and older
  same-path assertions turned these schedules into debug panics, hangs, or
  side-order-dependent duplicate content.
  
  Make cleanup idempotent wherever absence is already the required end state:
  add/add type conflicts, same-source rewrites, blocking conflict destinations,
  delete/rewrite sources, and changes deferred through directory renames. Accept
  cross-path structural matches from ambiguous rewrite candidates and let the
  existing conservative unknown-conflict fallback handle them.
  
  Preserve each rewrite input mode when blob content is identical so executable
  mode changes remain visible to the merge. Make unique-path selection respect
  childless tracked directories and qualify the first blocking file component,
  which guarantees termination instead of varying an ineffective descendant
  suffix forever. Finally, when a deferred addition is relocated to a unique
  conflict path, remove its temporary original path from the side index before
  marking it processed so a later descendant cannot relocate the same content a
  second time.
  
  The minimized and accumulated fuzz inputs now complete without failure.
  Git-backed regressions cover ambiguous sources, consumed nodes, repeated
  rename/delete candidates, mode-only rewrite collisions, unique paths below
  files, and nested rename destinations in both side orderings. The final tree
  baseline contains 165 directional cases with 130 intentionally skipped forced
  resolution checks, and reversing the nested relocation case retains exactly
  `a/a/a` and `a~A` without inventing `a~A_0`.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 18 commits contributed to the release over the course of 30 calendar days.
- 30 days passed between releases.
- 4 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.3.0, gix-date v0.16.0, gix-actor v0.42.0, gix-validate v0.11.4, gix-path v0.12.5, gix-utils v0.3.6, gix-quote v0.8.0, gix-command v0.10.0, gix-features v0.49.1, gix-hash v0.26.1, gix-fs v0.22.1, gix-object v0.64.0, gix-glob v0.27.1, gix-attributes v0.35.0, gix-filter v0.34.0, gix-chunk v0.8.0, gix-commitgraph v0.39.0, gix-revwalk v0.35.0, gix-traverse v0.61.0, gix-worktree-stream v0.36.0, gix-archive v0.36.0, gix-bitmap v0.4.0, gix-index v0.55.0, gix-pathspec v0.20.0, gix-ignore v0.22.1, gix-worktree v0.56.0, gix-imara-diff v0.2.5, gix-diff v0.67.0, gix-blame v0.17.0, gix-ref v0.67.0, gix-config v0.60.0, gix-prompt v0.17.0, gix-url v0.38.0, gix-credentials v0.40.0, gix-discover v0.55.0, gix-dir v0.29.0, gix-mailmap v0.34.0, gix-revision v0.49.0, gix-merge v0.20.0, gix-negotiate v0.35.0, gix-note v0.1.0, gix-pack v0.74.0, gix-odb v0.84.0, gix-refspec v0.45.0, gix-transport v0.59.0, gix-protocol v0.65.0, gix-status v0.34.0, gix-submodule v0.34.0, gix-worktree-state v0.34.0, gix v0.87.0, gix-fsck v0.25.0, gitoxide-core v0.61.0, gix-tix v0.2.0, gitoxide v0.57.0 ([`d2af4ed`](https://github.com/GitoxideLabs/gitoxide/commit/d2af4ed5532ea660fbd643e48d8925cd88de5ee0))
  - Update manifests prior to release ([`ebe9095`](https://github.com/GitoxideLabs/gitoxide/commit/ebe9095f2888d3c12447ea5eed9d0afdb0fd5aeb))
  - Merge pull request #2905 from GitoxideLabs/various-improvements ([`f3bbfad`](https://github.com/GitoxideLabs/gitoxide/commit/f3bbfadd4b4f1d72c85c62eb3d7ae337c922f945))
  - Adapt to changes in `gix-testtools` ([`0cbe539`](https://github.com/GitoxideLabs/gitoxide/commit/0cbe53971687fb3b1959925aa9d8dc89deb5b474))
  - Merge pull request #2901 from cruessler/switch-to-gix-odb-at-opts ([`2a4d996`](https://github.com/GitoxideLabs/gitoxide/commit/2a4d996ca53bd38a5e9889da0b180580315d905f))
  - Introduce `Store::at()` where possible ([`17fea2a`](https://github.com/GitoxideLabs/gitoxide/commit/17fea2ab8a1c23f2e8bc50b78b90feb1e361f66a))
  - Merge pull request #2843 from GitoxideLabs/deleted-file-added-dir ([`f33e250`](https://github.com/GitoxideLabs/gitoxide/commit/f33e250d35391faf981cabc1ff5c7c1ab0af62a9))
  - Refactor tree-merge change matching and resolution. ([`43cf77f`](https://github.com/GitoxideLabs/gitoxide/commit/43cf77fdb52b547e727343644264fe60c05e6589))
  - Add Linux-sized tree merge benchmarks ([`7ccac01`](https://github.com/GitoxideLabs/gitoxide/commit/7ccac0132e48e23aaf56ad29eed9e9d35bff89cc))
  - Add structural tree-merge validation. ([`af2aa23`](https://github.com/GitoxideLabs/gitoxide/commit/af2aa2302506df476297a14db388fa4169a811ba))
  - Merge pull request #2867 from GitoxideLabs/fix-url-authority-parsing ([`cc3ee80`](https://github.com/GitoxideLabs/gitoxide/commit/cc3ee8060ad7a32ee8d2eb9139854be7f7561b70))
  - Release gix-path v0.12.4, gix-command v0.9.2, gix-config-value v0.19.1, gix-url v0.37.1, gix-credentials v0.39.1, gix-transport v0.58.1 ([`ab4fcb0`](https://github.com/GitoxideLabs/gitoxide/commit/ab4fcb0364ec4d01115595198f383b1ad9c29808))
  - Merge pull request #2826 from GitoxideLabs/deleted-file-added-dir ([`83e074c`](https://github.com/GitoxideLabs/gitoxide/commit/83e074c1e2d070059febaaab1022058948855101))
  - Add a Cartesian tree-merge correctness baseline. ([`df845cf`](https://github.com/GitoxideLabs/gitoxide/commit/df845cfedca98e9c9b52922223ebf4d77581af90))
  - Track whether tree changes were actually applied ([`61b45a8`](https://github.com/GitoxideLabs/gitoxide/commit/61b45a81911561761b1546e8bf74541263b964be))
  - Treat adjacent text changes as conflicting. ([`195f0b9`](https://github.com/GitoxideLabs/gitoxide/commit/195f0b99a650c626d951ef7a902733a0e2ac44cc))
  - Merge additions below jointly deleted files ([`1ee3fc6`](https://github.com/GitoxideLabs/gitoxide/commit/1ee3fc6adf44520ee3397982ec42dbf65a9126f8))
  - Merge pull request #2812 from GitoxideLabs/report-july ([`ae8845a`](https://github.com/GitoxideLabs/gitoxide/commit/ae8845a47c4c87e0996a119822106cf09036340b))

</details>

## 0.19.0 (2026-07-23)

### Bug Fixes

- <csr-id-cea5ea77b57f840de618e1d15508add8293f30f2/> prefer file-name matches when pairing rename candidates, like Git
  When several sources are equally viable for a destination - identical ids in the identity pass, or equal similarity in the similarity pass - prefer the one whose file name matches the destination's, in the spirit of Git's basename-driven matching. This makes the now-deterministic choice also the semantically better one, where previously the pairing among identical candidates was arbitrary.
  
  The tree-merge baseline had recorded the arbitrary pairings in its three deviating cases, along with their consequences. These expectations are updated to the improved outcomes:
  
  - rename-within-rename-2: both merge directions now produce the same cleanly merged tree - the result that was previously present but commented out in favor of the order-dependent conflict, making the merge reversible as intended. The expected-reversed workaround branch is no longer needed.
  - conflicting-rename-complex (both directions): contents follow their true renames now that files pair up by name, and the previously documented mismatch of finding a rename of a/w to a-renamed/z no longer happens. Forward and reversed merges produce the same tree and mirror-image conflict stages.
  
  All other 114 baseline cases are unchanged, in SHA-1 and SHA-256. Note that in the identity pass, when no file-name match exists, the scan now covers the whole same-id range before falling back to the first eligible candidate, where it stopped previously.
- <csr-id-ce9cff421c8b4f3f4876a801e75806ac819ab551/> keep file/directory merges independent of the object hash
  When one side edits a file and the other replaces it with a directory,
  the merge finds the replacement by checking only the next change after
  the deletion, assuming the dir's addition is adjacent to it.
  
  That adjacency is an accident of SHA-1: rename-detection orders changes
  by object-id and the addition lands next to the deletion.
  
  Switching to SHA-256 changes every id and reorders the diff, moving the
  addition elsewhere, so a merge that resolved cleanly under SHA-1 missed
  the replacement under SHA-256, left the file in place and panicked.
  
  In other words, what happens is that the same input gives a different
  merge result due to the hash algo.
  
  Instead of looking just at the next change, scan all of that side's
  changes so the outcome no longer depends on the object-id order.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 18 commits contributed to the release.
- 31 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#1832](https://github.com/GitoxideLabs/gitoxide/issues/1832)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#1832](https://github.com/GitoxideLabs/gitoxide/issues/1832)**
  - Prefer file-name matches when pairing rename candidates, like Git ([`cea5ea7`](https://github.com/GitoxideLabs/gitoxide/commit/cea5ea77b57f840de618e1d15508add8293f30f2))
- **Uncategorized**
  - Release gix-actor v0.41.2, gix-features v0.49.0, gix-hash v0.26.0, gix-hashtable v0.16.0, gix-object v0.63.0, gix-glob v0.27.0, gix-attributes v0.34.0, gix-packetline v0.22.0, gix-filter v0.33.0, gix-fs v0.22.0, gix-chunk v0.7.3, gix-commitgraph v0.38.0, gix-revwalk v0.34.0, gix-traverse v0.60.0, gix-worktree-stream v0.35.0, gix-archive v0.35.0, gix-bitmap v0.3.3, gix-tempfile v24.0.0, gix-lock v24.0.0, gix-index v0.54.0, gix-pathspec v0.19.0, gix-ignore v0.22.0, gix-worktree v0.55.0, gix-imara-diff v0.2.4, gix-diff v0.66.0, gix-blame v0.16.0, gix-ref v0.66.0, gix-config v0.59.0, gix-discover v0.54.0, gix-dir v0.28.0, gix-mailmap v0.33.2, gix-revision v0.48.0, gix-merge v0.19.0, gix-negotiate v0.34.0, gix-zlib v0.1.0, gix-pack v0.73.0, gix-odb v0.83.0, gix-refspec v0.44.0, gix-shallow v0.13.0, gix-transport v0.58.0, gix-protocol v0.64.0, gix-status v0.33.0, gix-submodule v0.33.0, gix-worktree-state v0.33.0, gix v0.86.0, gix-fsck v0.24.0, gitoxide-core v0.60.0, gix-tix v0.1.0, gitoxide v0.56.0, safety bump 40 crates ([`842bc44`](https://github.com/GitoxideLabs/gitoxide/commit/842bc447e3aeacf5d9d36f7f8a01068eda4b7999))
  - Update changelogs prior to release ([`cb6ec7d`](https://github.com/GitoxideLabs/gitoxide/commit/cb6ec7dce283943d811b1600b577f586d7a13e1f))
  - Release gix-trace v0.1.21, gix-validate v0.11.3, gix-path v0.12.3, gix-utils v0.3.5, gix-config-value v0.19.0, gix-prompt v0.16.0, gix-sec v0.14.2, gix-url v0.37.0, gix-credentials v0.39.0, safety bump 18 crates ([`f0ec710`](https://github.com/GitoxideLabs/gitoxide/commit/f0ec71076aa1cef3181b77946ee556a89c651b8e))
  - Merge pull request #2737 from GitoxideLabs/encoding-fallback-pony ([`2315ede`](https://github.com/GitoxideLabs/gitoxide/commit/2315ede714da6a43c885ed534f37901b2e1db687))
  - Adapt to changes in `gix-filter` ([`552402f`](https://github.com/GitoxideLabs/gitoxide/commit/552402f6147b8ed4f412b9d24bf8408320b4a5d8))
  - Merge pull request #2722 from GitoxideLabs/reasons ([`c16b5a1`](https://github.com/GitoxideLabs/gitoxide/commit/c16b5a1892704b7c72a253bdd74a6848dd61032a))
  - Replace lint allowances with expectations ([`43ff87a`](https://github.com/GitoxideLabs/gitoxide/commit/43ff87a73897b70313e3a58e7de82231be5b59ad))
  - Merge pull request #2714 from GitoxideLabs/fix-credentials-parsing ([`cf3053a`](https://github.com/GitoxideLabs/gitoxide/commit/cf3053a3c18e2de788cdaa9f41b5bd343bdc0091))
  - Release gix-path v0.12.2, gix-error v0.2.5, gix-utils v0.3.4, gix-date v0.15.6, gix-url v0.36.2, gix-credentials v0.38.2 ([`27aec47`](https://github.com/GitoxideLabs/gitoxide/commit/27aec474c113cc885d44631b329454dc1ad0fed2))
  - Merge pull request #2687 from ameyypawar/fix/1832-rename-tracker-order-independent ([`a82b492`](https://github.com/GitoxideLabs/gitoxide/commit/a82b492d189d2ac70e108a5a272f5c56461444a8))
  - Review ([`32cb1ad`](https://github.com/GitoxideLabs/gitoxide/commit/32cb1ad007d3a69e7c512fc08372ad13c72d8c98))
  - Use `gix_odb::memory::Proxy<gix_object::Never>` where applicable ([`3880993`](https://github.com/GitoxideLabs/gitoxide/commit/3880993161e2a8fe07500962019204e182c1cc7b))
  - Merge pull request #2677 from 10ne1/dev/aratiu/sha256-merge ([`63f86a4`](https://github.com/GitoxideLabs/gitoxide/commit/63f86a4cf5cbba6e38006e14fc6322e826474c28))
  - Review ([`3f8754c`](https://github.com/GitoxideLabs/gitoxide/commit/3f8754c138ab228568325cdef5162868c30b595b))
  - Make the tree-baseline tests hash-aware ([`02f2383`](https://github.com/GitoxideLabs/gitoxide/commit/02f238346605f071979d74b2c2d0dca88522122b))
  - Keep file/directory merges independent of the object hash ([`ce9cff4`](https://github.com/GitoxideLabs/gitoxide/commit/ce9cff421c8b4f3f4876a801e75806ac819ab551))
  - Merge pull request #2646 from GitoxideLabs/report ([`1b1541e`](https://github.com/GitoxideLabs/gitoxide/commit/1b1541ed7a457afd48385c1ee39113949a9f5263))

</details>

## 0.18.0 (2026-06-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 2 commits contributed to the release over the course of 27 calendar days.
- 27 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.15.5, gix-hashtable v0.15.2, gix-object v0.62.0, gix-attributes v0.33.2, gix-filter v0.32.0, gix-revwalk v0.33.0, gix-traverse v0.59.0, gix-worktree-stream v0.34.0, gix-archive v0.34.0, gix-tempfile v23.0.2, gix-index v0.53.0, gix-worktree v0.54.0, gix-imara-diff v0.2.3, gix-diff v0.65.0, gix-blame v0.15.0, gix-ref v0.65.0, gix-config v0.58.0, gix-discover v0.53.0, gix-dir v0.27.0, gix-revision v0.47.0, gix-merge v0.18.0, gix-negotiate v0.33.0, gix-pack v0.72.0, gix-odb v0.82.0, gix-refspec v0.43.0, gix-transport v0.57.2, gix-protocol v0.63.0, gix-status v0.32.0, gix-submodule v0.32.0, gix-worktree-state v0.32.0, gix v0.85.0, gix-fsck v0.23.0, gitoxide-core v0.59.0, gitoxide v0.55.0, safety bump 28 crates ([`6428edc`](https://github.com/GitoxideLabs/gitoxide/commit/6428edc82fc8a16d5ef34ca2d49aa6fdff3645fe))
  - Merge pull request #2618 from GitoxideLabs/report ([`f7d4f33`](https://github.com/GitoxideLabs/gitoxide/commit/f7d4f33b58503996ae90497b69ce4c3a757982ac))

</details>

## 0.17.0 (2026-05-26)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 12 commits contributed to the release over the course of 28 calendar days.
- 28 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.2.4, gix-date v0.15.4, gix-actor v0.41.1, gix-trace v0.1.20, gix-validate v0.11.2, gix-path v0.12.1, gix-utils v0.3.3, gix-features v0.48.1, gix-hash v0.25.1, gix-hashtable v0.15.1, gix-object v0.61.0, gix-glob v0.26.1, gix-quote v0.7.2, gix-attributes v0.33.1, gix-command v0.9.1, gix-packetline v0.21.4, gix-filter v0.31.0, gix-fs v0.21.2, gix-chunk v0.7.2, gix-commitgraph v0.37.1, gix-revwalk v0.32.0, gix-traverse v0.58.0, gix-worktree-stream v0.33.0, gix-archive v0.33.0, gix-bitmap v0.3.2, gix-tempfile v23.0.1, gix-lock v23.0.1, gix-index v0.52.0, gix-config-value v0.18.1, gix-pathspec v0.18.1, gix-ignore v0.21.1, gix-worktree v0.53.0, gix-imara-diff v0.2.2, gix-diff v0.64.0, gix-blame v0.14.0, gix-ref v0.64.0, gix-sec v0.14.1, gix-config v0.57.0, gix-prompt v0.15.1, gix-url v0.36.1, gix-credentials v0.38.1, gix-discover v0.52.0, gix-dir v0.26.0, gix-mailmap v0.33.1, gix-revision v0.46.0, gix-merge v0.17.0, gix-negotiate v0.32.0, gix-pack v0.71.0, gix-odb v0.81.0, gix-refspec v0.42.0, gix-shallow v0.12.1, gix-transport v0.57.1, gix-protocol v0.62.0, gix-status v0.31.0, gix-submodule v0.31.0, gix-worktree-state v0.31.0, gix v0.84.0, gix-fsck v0.22.0, gitoxide-core v0.58.0, gitoxide v0.54.0, safety bump 27 crates ([`10c58bb`](https://github.com/GitoxideLabs/gitoxide/commit/10c58bb56597d9335611da121aac21f9b09b6e5b))
  - Merge pull request #2573 from cruessler/run-gix-traverse-tests-with-sha-256 ([`278d7ec`](https://github.com/GitoxideLabs/gitoxide/commit/278d7ec395124b3ce00db6f3e029265bfec8ccd1))
  - `sha1` and `sha256` forwardings for all crates ([`09b982c`](https://github.com/GitoxideLabs/gitoxide/commit/09b982c97fd7802819e49e5613c588d9aab008bb))
  - Merge pull request #2568 from GitoxideLabs/dependabot/cargo/cargo-56d6b174d8 ([`ab2fee1`](https://github.com/GitoxideLabs/gitoxide/commit/ab2fee14651202fcb7b3d8178932090c73492014))
  - Update crates to Rust 2024 edition ([`2cb17b2`](https://github.com/GitoxideLabs/gitoxide/commit/2cb17b2e7f6009693a55af907614f705a29d8c29))
  - Remove rust_2018_idioms lint declarations ([`e10d5f6`](https://github.com/GitoxideLabs/gitoxide/commit/e10d5f662df2ee05f973a3167ad215a330ee74e1))
  - Raise MSRV for hash dependency updates ([`3675a8d`](https://github.com/GitoxideLabs/gitoxide/commit/3675a8d61b17845a783bc27912a3f52ac273a4af))
  - Merge pull request #2559 from GitoxideLabs/fix/symlink-prefix-reuse-worktree-escape-ghsa-f89h-2fjh-2r9q ([`3af9b4a`](https://github.com/GitoxideLabs/gitoxide/commit/3af9b4adcbca28e7072b96d266b716efb483893c))
  - Release gix-fs v0.21.1 ([`d3e4c17`](https://github.com/GitoxideLabs/gitoxide/commit/d3e4c176f52a685f0405427ad7070014a6baf92b))
  - Merge pull request #2543 from cruessler/run-gix-worktree-stream-tests-with-sha-256 ([`23af41a`](https://github.com/GitoxideLabs/gitoxide/commit/23af41ab13d4fa894c2c2774a096fef5438bef7d))
  - Adapt to changes in `gix_object::Data` ([`4309fa4`](https://github.com/GitoxideLabs/gitoxide/commit/4309fa4a98fd613f8e346ed0274d0edec8dfab1f))
  - Merge pull request #2546 from GitoxideLabs/fix-2545 ([`adb8328`](https://github.com/GitoxideLabs/gitoxide/commit/adb8328952478c443ead5f5a8c6851928b377b37))

</details>

## 0.16.0 (2026-04-28)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 2 commits contributed to the release over the course of 2 calendar days.
- 4 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.2.3, gix-date v0.15.3, gix-actor v0.41.0, gix-path v0.12.0, gix-features v0.48.0, gix-hash v0.25.0, gix-hashtable v0.15.0, gix-object v0.60.0, gix-glob v0.26.0, gix-attributes v0.33.0, gix-command v0.9.0, gix-filter v0.30.0, gix-fs v0.21.0, gix-commitgraph v0.37.0, gix-revwalk v0.31.0, gix-traverse v0.57.0, gix-worktree-stream v0.32.0, gix-archive v0.32.0, gix-tempfile v23.0.0, gix-lock v23.0.0, gix-index v0.51.0, gix-config-value v0.18.0, gix-pathspec v0.18.0, gix-ignore v0.21.0, gix-worktree v0.52.0, gix-imara-diff v0.2.1, gix-diff v0.63.0, gix-blame v0.13.0, gix-ref v0.63.0, gix-sec v0.14.0, gix-config v0.56.0, gix-prompt v0.15.0, gix-url v0.36.0, gix-credentials v0.38.0, gix-discover v0.51.0, gix-dir v0.25.0, gix-mailmap v0.33.0, gix-revision v0.45.0, gix-merge v0.16.0, gix-negotiate v0.31.0, gix-pack v0.70.0, gix-odb v0.80.0, gix-refspec v0.41.0, gix-shallow v0.12.0, gix-transport v0.57.0, gix-protocol v0.61.0, gix-status v0.30.0, gix-submodule v0.30.0, gix-worktree-state v0.30.0, gix v0.83.0, gix-fsck v0.21.0, gitoxide-core v0.57.0, gitoxide v0.53.0, safety bump 48 crates ([`53f880c`](https://github.com/GitoxideLabs/gitoxide/commit/53f880c7604232c367870088176e42efd8a5b783))
  - Merge pull request #2540 from GitoxideLabs/reporting ([`4d5ba23`](https://github.com/GitoxideLabs/gitoxide/commit/4d5ba231685e8ff36195603c57193aa1cd21fa8e))

</details>

## 0.15.0 (2026-04-24)

### Chore

- <csr-id-ed32b8af877ae273570e950886b1050fe2f8afaa/> make `fuzz` test myers algorithm resilient against pathological inputs.
  Achieved by removing Myers as it's way of operation can't really be fixed.
- <csr-id-3e05ca352597ef5966fa4dc4f52456c2424cddad/> add package.include directives to control which files are packaged.

### New Features

- <csr-id-1ec28125bc33b4725133f27648b0978a0353463e/> add `blob::text::PreparedMerge` to allow re-running with different Conflict resolutions.
  This is useful to quickly check different resolution results.

### Bug Fixes

- <csr-id-91feaf29334555c8287ae9f16a0bdf2be0b9ccc0/> enable diff-postprocessing in when merging blobs
  This can lead to cleaner, more Git-like merges.
- <csr-id-680dbb76266c8555324b89b47fac7dbeee04e67f/> coalesce split Myers hunks to prevent false merge conflicts
  When imara-diff's Myers algorithm diffs two files, it sometimes splits
  what is logically one change into a non-empty deletion hunk and a
  separate empty insertion hunk, with one unchanged base line between
  them. This is a valid minimal edit script, but it differs from the
  alignment that git's xdiff (also Myers-based) would choose.
  
  When the empty insertion lands at a base position that the other side
  of a 3-way merge also touches, `take_intersecting` reports an overlap
  and the merge produces a conflict — even though `git merge-file`
  resolves the same inputs cleanly.
  
  Fix this by adding a pre-processing step after sorting hunks: for each
  empty-range insertion hunk, look backwards for the nearest same-side
  non-empty hunk within a gap of at most one unchanged base line. If
  found, extend that hunk to cover the gap and the insertion point. This
  re-joins the split hunk, making the merge robust to different diff
  algorithm alignment choices.
  
  The coalescing is conservative: it only applies when (a) the insertion
  hunk has an empty before-range, (b) there is a same-side non-empty
  hunk nearby (gap ≤ 1), and (c) that hunk is the nearest same-side
  hunk. This avoids affecting cases like zdiff3-interesting where empty
  insertions are standalone and represent genuinely different changes.

### New Features (BREAKING)

- <csr-id-8094f5dcd4f24f4d54f7fbe7f716f80f2974b586/> Use `imara-diff-v2` with git sliders processing
  The slider post-processing imrpoves the diff quality for about 8% slower diffs.
  Line-counts, however, will be 50% faster to compute.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 20 commits contributed to the release over the course of 32 calendar days.
- 33 days passed between releases.
- 6 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Update changelogs prior to release ([`f9fbcba`](https://github.com/GitoxideLabs/gitoxide/commit/f9fbcba28278f3fb2ad7969c2d00ac6765165724))
  - Merge pull request #2534 from GitoxideLabs/fuzz-merge-timeout ([`2e8a90d`](https://github.com/GitoxideLabs/gitoxide/commit/2e8a90d5e70e18766d46f552c579ebd8d19b1140))
  - Address auto-review ([`94b1c4b`](https://github.com/GitoxideLabs/gitoxide/commit/94b1c4bf2fe53bf326ba8fc0ad69ee9cb65b0c83))
  - Make `fuzz` test myers algorithm resilient against pathological inputs. ([`ed32b8a`](https://github.com/GitoxideLabs/gitoxide/commit/ed32b8af877ae273570e950886b1050fe2f8afaa))
  - Add `blob::text::PreparedMerge` to allow re-running with different Conflict resolutions. ([`1ec2812`](https://github.com/GitoxideLabs/gitoxide/commit/1ec28125bc33b4725133f27648b0978a0353463e))
  - Merge pull request #2497 from cruessler/pass-hash-len-to-tree-ref-iter ([`7d50c30`](https://github.com/GitoxideLabs/gitoxide/commit/7d50c30040b8854a0129d86bd30535cb570ca75e))
  - Adapt to changes in `gix-object` ([`6df1d55`](https://github.com/GitoxideLabs/gitoxide/commit/6df1d553b03e2809aa4f651e3ec48aad4688696e))
  - Merge pull request #2476 from mtsgrd/fix/false-conflict-empty-insertion-overlap ([`172bd22`](https://github.com/GitoxideLabs/gitoxide/commit/172bd22b8241aba9187fc6e5134e3f454053c2be))
  - Review ([`7b82f3a`](https://github.com/GitoxideLabs/gitoxide/commit/7b82f3ab676a8f8d223f02b57d5badd0e4392618))
  - Enable diff-postprocessing in when merging blobs ([`91feaf2`](https://github.com/GitoxideLabs/gitoxide/commit/91feaf29334555c8287ae9f16a0bdf2be0b9ccc0))
  - Coalesce split Myers hunks to prevent false merge conflicts ([`680dbb7`](https://github.com/GitoxideLabs/gitoxide/commit/680dbb76266c8555324b89b47fac7dbeee04e67f))
  - Merge pull request #2513 from GitoxideLabs/v2-diff ([`2a5db88`](https://github.com/GitoxideLabs/gitoxide/commit/2a5db88d0330b0d125de4b6f3819f17a7f76f4b8))
  - Thanks clippy ([`e4f380e`](https://github.com/GitoxideLabs/gitoxide/commit/e4f380eff3b0440002f7e9b64a14ddcfbe63192a))
  - Use `imara-diff-v2` with git sliders processing ([`8094f5d`](https://github.com/GitoxideLabs/gitoxide/commit/8094f5dcd4f24f4d54f7fbe7f716f80f2974b586))
  - Merge pull request #2518 from GitoxideLabs/improvements ([`444a92b`](https://github.com/GitoxideLabs/gitoxide/commit/444a92b0fa1df406cf2f36f8dbe82c2859e04e0b))
  - Add package.include directives to control which files are packaged. ([`3e05ca3`](https://github.com/GitoxideLabs/gitoxide/commit/3e05ca352597ef5966fa4dc4f52456c2424cddad))
  - Merge pull request #2506 from GitoxideLabs/vendor-imara-diff ([`8f091d1`](https://github.com/GitoxideLabs/gitoxide/commit/8f091d108cd75371be2ed9de6e81f785cda53d92))
  - Add a test that reproduces a timeout issue in `gix-merge`. ([`3bf4bdc`](https://github.com/GitoxideLabs/gitoxide/commit/3bf4bdc8fafd0949e4fe9648972ee8fe3438088e))
  - Vendor `imara-diff` 0.1 and 0.2 ([`fd49295`](https://github.com/GitoxideLabs/gitoxide/commit/fd49295c5ed4a57bf5771e23c0f803435990ecfa))
  - Merge pull request #2480 from GitoxideLabs/report ([`98bae84`](https://github.com/GitoxideLabs/gitoxide/commit/98bae84fe534879899489c6f2c5e8cfcc863116d))

</details>

## 0.14.0 (2026-03-22)

### New Features

- <csr-id-383291689c659a2cc0bee7687f5a9b9f7a3659a4/> add `sha1` and `sha256` features to `gix`.
  This way one can control which hashes are compiled in exactly,
  while having reasonable defaults automatically.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 11 commits contributed to the release.
- 28 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.2.1, gix-date v0.15.1, gix-path v0.11.2, gix-features v0.46.2, gix-hash v0.23.0, gix-hashtable v0.13.0, gix-object v0.58.0, gix-packetline v0.21.2, gix-filter v0.28.0, gix-fs v0.19.2, gix-commitgraph v0.35.0, gix-revwalk v0.29.0, gix-traverse v0.55.0, gix-worktree-stream v0.30.0, gix-archive v0.30.0, gix-tempfile v21.0.2, gix-lock v21.0.2, gix-index v0.49.0, gix-pathspec v0.16.1, gix-ignore v0.19.1, gix-worktree v0.50.0, gix-diff v0.61.0, gix-blame v0.11.0, gix-ref v0.61.0, gix-sec v0.13.2, gix-config v0.54.0, gix-prompt v0.14.1, gix-credentials v0.37.1, gix-discover v0.49.0, gix-dir v0.23.0, gix-revision v0.43.0, gix-merge v0.14.0, gix-negotiate v0.29.0, gix-pack v0.68.0, gix-odb v0.78.0, gix-refspec v0.39.0, gix-shallow v0.10.0, gix-transport v0.55.1, gix-protocol v0.59.0, gix-status v0.28.0, gix-submodule v0.28.0, gix-worktree-state v0.28.0, gix v0.81.0, gix-fsck v0.19.0, gitoxide-core v0.55.0, gitoxide v0.52.0, safety bump 31 crates ([`c389a2c`](https://github.com/GitoxideLabs/gitoxide/commit/c389a2ccb32b36c1178a1352a2bb3229aef3b016))
  - Merge pull request #2472 from GitoxideLabs/improvements ([`8e47e0f`](https://github.com/GitoxideLabs/gitoxide/commit/8e47e0f00bed137db6231cf2ab327843ada0b2d2))
  - Add a test for mergiraf-style merge drivers specifically ([`abe8bbf`](https://github.com/GitoxideLabs/gitoxide/commit/abe8bbfb02dc864011f8a0ed518aaf6b0faf3301))
  - Merge pull request #2454 from GitoxideLabs/dependabot/cargo/cargo-da044b9bb0 ([`6183fd0`](https://github.com/GitoxideLabs/gitoxide/commit/6183fd092d7acd43763fe15be400ce81e7172775))
  - Bump the cargo group with 68 updates ([`6bdb331`](https://github.com/GitoxideLabs/gitoxide/commit/6bdb33145e8aa81ba0dae5caafc675c591569715))
  - Merge pull request #2441 from cruessler/remove-sha-1-from-default-features ([`e8bf096`](https://github.com/GitoxideLabs/gitoxide/commit/e8bf096c07205a41089a697a9726f075d3515643))
  - Add `sha1` and `sha256` features to `gix`. ([`3832916`](https://github.com/GitoxideLabs/gitoxide/commit/383291689c659a2cc0bee7687f5a9b9f7a3659a4))
  - Adapt to sha1 not being default feature of `gix-hash` ([`e71c703`](https://github.com/GitoxideLabs/gitoxide/commit/e71c703f0b8ca209f8aa912cbaf5aa26551496ef))
  - Merge pull request #2445 from GitoxideLabs/improvements ([`6a7287c`](https://github.com/GitoxideLabs/gitoxide/commit/6a7287c9247120167e49154463f7e86c25100649))
  - Add `cargo machete` CI job including exclusions ([`abd0724`](https://github.com/GitoxideLabs/gitoxide/commit/abd072444ff076557aa7e4c5b76ad7c47d488a4a))
  - Merge pull request #2442 from GitoxideLabs/report ([`f7277f3`](https://github.com/GitoxideLabs/gitoxide/commit/f7277f3c9e3e5130edb714ff5bd3db06b7f589b3))

</details>

## 0.13.0 (2026-02-22)

### New Features (BREAKING)

- <csr-id-231fda44d3de46776d19227100d52459e37bcaf5/> model merge-bases as a non-empty type in `gix-revision` and `gix-merge`
  Adapt `gix` accordingly (even though it's nonbreaking).

### Commit Statistics

<csr-read-only-do-not-edit/>

- 8 commits contributed to the release over the course of 31 calendar days.
- 31 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.2.0, gix-date v0.15.0, gix-actor v0.40.0, gix-object v0.57.0, gix-quote v0.7.0, gix-attributes v0.31.0, gix-command v0.8.0, gix-filter v0.27.0, gix-chunk v0.7.0, gix-commitgraph v0.34.0, gix-revwalk v0.28.0, gix-traverse v0.54.0, gix-worktree-stream v0.29.0, gix-archive v0.29.0, gix-bitmap v0.3.0, gix-index v0.48.0, gix-pathspec v0.16.0, gix-worktree v0.49.0, gix-diff v0.60.0, gix-blame v0.10.0, gix-ref v0.60.0, gix-config v0.53.0, gix-prompt v0.14.0, gix-url v0.35.2, gix-credentials v0.37.0, gix-discover v0.48.0, gix-dir v0.22.0, gix-mailmap v0.32.0, gix-revision v0.42.0, gix-merge v0.13.0, gix-negotiate v0.28.0, gix-pack v0.67.0, gix-odb v0.77.0, gix-refspec v0.38.0, gix-shallow v0.9.0, gix-transport v0.55.0, gix-protocol v0.58.0, gix-status v0.27.0, gix-submodule v0.27.0, gix-worktree-state v0.27.0, gix v0.80.0, gix-fsck v0.18.0, gitoxide-core v0.54.0, gitoxide v0.51.0, safety bump 42 crates ([`ecf90fc`](https://github.com/GitoxideLabs/gitoxide/commit/ecf90fccb9d43bff320c17f46fdc3f5832533a52))
  - Merge pull request #2433 from GitoxideLabs/codex/nonempty-rewrite ([`29040a8`](https://github.com/GitoxideLabs/gitoxide/commit/29040a8277735cbc9fcd0d80626c75d710d3da2a))
  - Model merge-bases as a non-empty type in `gix-revision` and `gix-merge` ([`231fda4`](https://github.com/GitoxideLabs/gitoxide/commit/231fda44d3de46776d19227100d52459e37bcaf5))
  - Merge branch 'release' ([`9327b73`](https://github.com/GitoxideLabs/gitoxide/commit/9327b73785227f1322a327cb48fbb0800e1286ae))
  - Release gix-error v0.1.0, gix-date v0.14.0, gix-actor v0.39.0, gix-trace v0.1.18, gix-path v0.11.1, gix-features v0.46.1, gix-hash v0.22.1, gix-object v0.56.0, gix-quote v0.6.2, gix-attributes v0.30.1, gix-command v0.7.1, gix-packetline v0.21.1, gix-filter v0.26.0, gix-fs v0.19.1, gix-chunk v0.6.0, gix-commitgraph v0.33.0, gix-revwalk v0.27.0, gix-traverse v0.53.0, gix-worktree-stream v0.28.0, gix-archive v0.28.0, gix-bitmap v0.2.16, gix-tempfile v21.0.1, gix-lock v21.0.1, gix-index v0.47.0, gix-config-value v0.17.1, gix-pathspec v0.15.1, gix-worktree v0.48.0, gix-diff v0.59.0, gix-blame v0.9.0, gix-ref v0.59.0, gix-sec v0.13.1, gix-config v0.52.0, gix-prompt v0.13.1, gix-url v0.35.1, gix-credentials v0.36.0, gix-discover v0.47.0, gix-dir v0.21.0, gix-mailmap v0.31.0, gix-revision v0.41.0, gix-merge v0.12.0, gix-negotiate v0.27.0, gix-pack v0.66.0, gix-odb v0.76.0, gix-refspec v0.37.0, gix-shallow v0.8.1, gix-transport v0.54.0, gix-protocol v0.57.0, gix-status v0.26.0, gix-submodule v0.26.0, gix-worktree-state v0.26.0, gix v0.79.0, safety bump 35 crates ([`d66ac10`](https://github.com/GitoxideLabs/gitoxide/commit/d66ac1057a5b7bfb608d4e6be585c69fb692bfee))
  - Merge pull request #2407 from GitoxideLabs/dependabot/cargo/cargo-fb4135702f ([`8bceefb`](https://github.com/GitoxideLabs/gitoxide/commit/8bceefbfc5f897517bfdd24744695a82cfa0d5be))
  - Bump the cargo group with 59 updates ([`7ce3c55`](https://github.com/GitoxideLabs/gitoxide/commit/7ce3c5587aec1ca813039c047783b9cb2a106826))
  - Merge pull request #2393 from GitoxideLabs/report ([`f7d0975`](https://github.com/GitoxideLabs/gitoxide/commit/f7d09758d245aaa89409e39bb6ba1ed6b7118ea5))

</details>

## 0.12.0 (2026-02-10)

## 0.11.0 (2026-01-22)

### New Features (BREAKING)

- <csr-id-5c1bd0387f98eee37265a42ba4b6624c783c9a71/> Use `std::ops::ControlFlow` where possible

### Commit Statistics

<csr-read-only-do-not-edit/>

- 9 commits contributed to the release over the course of 21 calendar days.
- 22 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 1 unique issue was worked on: [#2363](https://github.com/GitoxideLabs/gitoxide/issues/2363)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **[#2363](https://github.com/GitoxideLabs/gitoxide/issues/2363)**
  - Regenerate all changelogs with a more recent CSR version ([`cbbdef5`](https://github.com/GitoxideLabs/gitoxide/commit/cbbdef5095b894a944a526fb57dfebeb0f3ab5eb))
- **Uncategorized**
  - Release gix-error v0.0.0, gix-date v0.13.0, gix-actor v0.38.0, gix-validate v0.11.0, gix-path v0.11.0, gix-features v0.46.0, gix-hash v0.22.0, gix-hashtable v0.12.0, gix-object v0.55.0, gix-glob v0.24.0, gix-attributes v0.30.0, gix-command v0.7.0, gix-packetline v0.21.0, gix-filter v0.25.0, gix-fs v0.19.0, gix-chunk v0.5.0, gix-commitgraph v0.32.0, gix-revwalk v0.26.0, gix-traverse v0.52.0, gix-worktree-stream v0.27.0, gix-archive v0.27.0, gix-tempfile v21.0.0, gix-lock v21.0.0, gix-index v0.46.0, gix-config-value v0.17.0, gix-pathspec v0.15.0, gix-ignore v0.19.0, gix-worktree v0.47.0, gix-diff v0.58.0, gix-blame v0.8.0, gix-ref v0.58.0, gix-sec v0.13.0, gix-config v0.51.0, gix-prompt v0.13.0, gix-url v0.35.0, gix-credentials v0.35.0, gix-discover v0.46.0, gix-dir v0.20.0, gix-mailmap v0.30.0, gix-revision v0.40.0, gix-merge v0.11.0, gix-negotiate v0.26.0, gix-pack v0.65.0, gix-odb v0.75.0, gix-refspec v0.36.0, gix-shallow v0.8.0, gix-transport v0.53.0, gix-protocol v0.56.0, gix-status v0.25.0, gix-submodule v0.25.0, gix-worktree-state v0.25.0, gix v0.78.0, gix-fsck v0.17.0, gitoxide-core v0.53.0, gitoxide v0.50.0, safety bump 50 crates ([`562e684`](https://github.com/GitoxideLabs/gitoxide/commit/562e684319fa649db6a96c0a22d64bbe3c11e9e6))
  - Use `std::ops::ControlFlow` where possible ([`5c1bd03`](https://github.com/GitoxideLabs/gitoxide/commit/5c1bd0387f98eee37265a42ba4b6624c783c9a71))
  - Merge pull request #2364 from GitoxideLabs/changelogs ([`0a333e5`](https://github.com/GitoxideLabs/gitoxide/commit/0a333e5941a0a58727c694fcf7dc48f95d7481db))
  - Merge pull request #2346 from GitoxideLabs/release ([`c663b3f`](https://github.com/GitoxideLabs/gitoxide/commit/c663b3f05791db86d2e0a683e26e149f620bf2e4))
  - Release gix-trace v0.1.17, gix-features v0.45.2, gix-command v0.6.5, gix-hash v0.21.2, gix-date v0.12.1, gix-actor v0.37.1, gix-object v0.54.1, gix-filter v0.24.1, gix-fs v0.18.2, gix-tempfile v20.0.1, gix-lock v20.0.1, gix-traverse v0.51.1, gix-index v0.45.1, gix-diff v0.57.1, gix-pack v0.64.1 ([`7be8f90`](https://github.com/GitoxideLabs/gitoxide/commit/7be8f9068ab875ca4123300ba08df9d32fd63941))
  - Merge pull request #2333 from GitoxideLabs/improvements ([`9e1ba5e`](https://github.com/GitoxideLabs/gitoxide/commit/9e1ba5ee6b7e0e032f682097d48664a68891762d))
  - Release gix-command v0.6.4, gix-credentials v0.34.1, gix-transport v0.52.1 ([`3bc0c47`](https://github.com/GitoxideLabs/gitoxide/commit/3bc0c472a8395e9634dd602839e208501f841c11))
  - Merge pull request #2322 from GitoxideLabs/report ([`211b4fb`](https://github.com/GitoxideLabs/gitoxide/commit/211b4fb5a31792eda91191789f3656c217960986))

</details>

## 0.10.0 (2025-12-31)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 2 commits contributed to the release over the course of 9 calendar days.
- 9 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.12.0, gix-actor v0.37.0, gix-features v0.45.1, gix-hash v0.21.1, gix-object v0.54.0, gix-filter v0.24.0, gix-fs v0.18.1, gix-revwalk v0.25.0, gix-traverse v0.51.0, gix-worktree-stream v0.26.0, gix-archive v0.26.0, gix-index v0.45.0, gix-worktree v0.46.0, gix-diff v0.57.0, gix-blame v0.7.0, gix-ref v0.57.0, gix-config v0.50.0, gix-credentials v0.34.0, gix-discover v0.45.0, gix-dir v0.19.0, gix-mailmap v0.29.0, gix-revision v0.39.0, gix-merge v0.10.0, gix-negotiate v0.25.0, gix-pack v0.64.0, gix-odb v0.74.0, gix-refspec v0.35.0, gix-transport v0.52.0, gix-protocol v0.55.0, gix-status v0.24.0, gix-submodule v0.24.0, gix-worktree-state v0.24.0, gix v0.77.0, gix-fsck v0.16.0, gitoxide-core v0.52.0, gitoxide v0.49.0, safety bump 32 crates ([`115e208`](https://github.com/GitoxideLabs/gitoxide/commit/115e208b7bc7a96024e64ea872f2731b5125a6e0))
  - Merge pull request #2299 from GitoxideLabs/report ([`d6c5b9d`](https://github.com/GitoxideLabs/gitoxide/commit/d6c5b9d7843c24663ffcf20bd756ea3eb747ca0a))

</details>

## 0.9.0 (2025-12-22)

### Bug Fixes (BREAKING)

- <csr-id-1955b5b1a436518c036cfc9bf642170f9c7b6540/> expose raw commit/tag actor headers for round-tripping.
  Note that this means you have to call `CommitRef::commiter|author()?` and `TagRef::tagger()?` instead
  of assuming pre-parsed fields.
  
  This PR makes signature handling truly lossless for "creative" emails and other info.
  We now stash the raw name <email> slice on IdentityRef/SignatureRef and fall back to it when rewriting,
  so even commits with embedded angle brackets round-trip cleanly (might want to expand to other malformed characters before merging?
  Parsing and serialization honor that flag but still keep strict validation for normal input.
  I also added regression coverage for these scenarios.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 4 commits contributed to the release over the course of 21 calendar days.
- 30 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.11.1, gix-actor v0.36.1, gix-trace v0.1.16, gix-features v0.45.0, gix-hash v0.21.0, gix-hashtable v0.11.0, gix-object v0.53.0, gix-glob v0.23.0, gix-attributes v0.29.0, gix-filter v0.23.0, gix-fs v0.18.0, gix-commitgraph v0.31.0, gix-revwalk v0.24.0, gix-traverse v0.50.0, gix-worktree-stream v0.25.0, gix-archive v0.25.0, gix-tempfile v20.0.0, gix-lock v20.0.0, gix-index v0.44.0, gix-config-value v0.16.0, gix-pathspec v0.14.0, gix-ignore v0.18.0, gix-worktree v0.45.0, gix-diff v0.56.0, gix-blame v0.6.0, gix-ref v0.56.0, gix-config v0.49.0, gix-prompt v0.12.0, gix-url v0.34.0, gix-credentials v0.33.0, gix-discover v0.44.0, gix-dir v0.18.0, gix-mailmap v0.28.1, gix-revision v0.38.0, gix-merge v0.9.0, gix-negotiate v0.24.0, gix-pack v0.63.0, gix-odb v0.73.0, gix-refspec v0.34.0, gix-shallow v0.7.0, gix-transport v0.51.0, gix-protocol v0.54.0, gix-status v0.23.0, gix-submodule v0.23.0, gix-worktree-state v0.23.0, gix v0.76.0, gix-fsck v0.15.0, gitoxide-core v0.51.0, gitoxide v0.48.0, safety bump 43 crates ([`21fecdf`](https://github.com/GitoxideLabs/gitoxide/commit/21fecdf928336ac5fa3dd1402f92e8200d8aff62))
  - Merge pull request #2253 from Pingasmaster/raw-email-attempt-fix ([`f471ac5`](https://github.com/GitoxideLabs/gitoxide/commit/f471ac5fece0baf3766d80482400f6b37db8edd0))
  - Refactor ([`6f7b23a`](https://github.com/GitoxideLabs/gitoxide/commit/6f7b23a4b85eb7b8c922753aa86849893d2a089d))
  - Expose raw commit/tag actor headers for round-tripping. ([`1955b5b`](https://github.com/GitoxideLabs/gitoxide/commit/1955b5b1a436518c036cfc9bf642170f9c7b6540))

</details>

## 0.8.0 (2025-11-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 2 commits contributed to the release.
- 31 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.11.0, gix-actor v0.36.0, gix-path v0.10.22, gix-object v0.52.0, gix-packetline v0.20.0, gix-filter v0.22.0, gix-revwalk v0.23.0, gix-traverse v0.49.0, gix-worktree-stream v0.24.0, gix-archive v0.24.0, gix-index v0.43.0, gix-worktree v0.44.0, gix-diff v0.55.0, gix-blame v0.5.0, gix-ref v0.55.0, gix-config v0.48.0, gix-url v0.33.2, gix-credentials v0.32.0, gix-discover v0.43.0, gix-dir v0.17.0, gix-mailmap v0.28.0, gix-revision v0.37.0, gix-merge v0.8.0, gix-negotiate v0.23.0, gix-pack v0.62.0, gix-odb v0.72.0, gix-refspec v0.33.0, gix-transport v0.50.0, gix-protocol v0.53.0, gix-status v0.22.0, gix-submodule v0.22.0, gix-worktree-state v0.22.0, gix v0.75.0, gix-fsck v0.14.0, gitoxide-core v0.50.0, gitoxide v0.47.0, safety bump 32 crates ([`82ff92f`](https://github.com/GitoxideLabs/gitoxide/commit/82ff92fa943bad88dc7d5bfa100404de477a3608))
  - Merge pull request #2224 from GitoxideLabs/report ([`3313233`](https://github.com/GitoxideLabs/gitoxide/commit/3313233aa4e7009aed0ddf644f4271fd2a98e8d4))

</details>

## 0.7.0 (2025-10-22)

### Bug Fixes

- <csr-id-a929e9cbd00e8d6e43ccc31bf4d406548bede431/> assure that blob merges recognize conflicts in cojunction with empty bases.
  Previously, there were special cases that would cause it to auto-merge conflicts
  even though it shouldn't.
  This is now fixed just by removing the special case entirely, which seems
  unexpected in any of the test-suite.
- <csr-id-57614410a7b10c88b7bbab3e4a7ed1497568e29b/> run merge-filters in the worktree dir or the git dir.
  That way, we emulate what Git actually does as it will set the CWD
  of the process to either of these.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 15 commits contributed to the release over the course of 99 calendar days.
- 99 days passed between releases.
- 2 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.10.6, gix-utils v0.3.1, gix-actor v0.35.5, gix-trace v0.1.14, gix-validate v0.10.1, gix-path v0.10.21, gix-features v0.44.0, gix-hash v0.20.0, gix-hashtable v0.10.0, gix-object v0.51.0, gix-glob v0.22.0, gix-quote v0.6.1, gix-attributes v0.28.0, gix-command v0.6.3, gix-packetline-blocking v0.19.2, gix-filter v0.21.0, gix-fs v0.17.0, gix-chunk v0.4.12, gix-commitgraph v0.30.0, gix-revwalk v0.22.0, gix-traverse v0.48.0, gix-worktree-stream v0.23.0, gix-archive v0.23.0, gix-bitmap v0.2.15, gix-tempfile v19.0.0, gix-lock v19.0.0, gix-index v0.42.0, gix-config-value v0.15.2, gix-pathspec v0.13.0, gix-ignore v0.17.0, gix-worktree v0.43.0, gix-diff v0.54.0, gix-blame v0.4.0, gix-ref v0.54.0, gix-sec v0.12.1, gix-config v0.47.0, gix-prompt v0.11.2, gix-url v0.33.0, gix-credentials v0.31.0, gix-discover v0.42.0, gix-dir v0.16.0, gix-mailmap v0.27.3, gix-revision v0.36.0, gix-merge v0.7.0, gix-negotiate v0.22.0, gix-pack v0.61.0, gix-odb v0.71.0, gix-refspec v0.32.0, gix-shallow v0.6.0, gix-packetline v0.19.2, gix-transport v0.49.0, gix-protocol v0.52.0, gix-status v0.21.0, gix-submodule v0.21.0, gix-worktree-state v0.21.0, gix v0.74.0, gix-fsck v0.13.0, gitoxide-core v0.49.0, gitoxide v0.46.0, safety bump 42 crates ([`89fb308`](https://github.com/GitoxideLabs/gitoxide/commit/89fb308f1283b404b55916304f7d161fbf13fe10))
  - Merge pull request #2217 from GitoxideLabs/copilot/update-msrv-to-rust-1-82 ([`4da2927`](https://github.com/GitoxideLabs/gitoxide/commit/4da2927629c7ec95b96d62a387c61097e3fc71fa))
  - Fixup Copilot commits and thank clippy ([`b188a7d`](https://github.com/GitoxideLabs/gitoxide/commit/b188a7d834979eaa940fd94ec269367cd922d16d))
  - Update MSRV to 1.82 and replace once_cell with std equivalents ([`6cc8464`](https://github.com/GitoxideLabs/gitoxide/commit/6cc84641cb7be6f70468a90efaafcf142a6b8c4b))
  - Merge pull request #2207 from GitoxideLabs/improvements ([`fcb4448`](https://github.com/GitoxideLabs/gitoxide/commit/fcb4448382e6edaca8dec2a1fe23052163fd0ee2))
  - Assure that blob merges recognize conflicts in cojunction with empty bases. ([`a929e9c`](https://github.com/GitoxideLabs/gitoxide/commit/a929e9cbd00e8d6e43ccc31bf4d406548bede431))
  - Merge pull request #2203 from GitoxideLabs/improvements ([`c295db0`](https://github.com/GitoxideLabs/gitoxide/commit/c295db098db9576e12318e7c73864cf270297b1d))
  - Run merge-filters in the worktree dir or the git dir. ([`5761441`](https://github.com/GitoxideLabs/gitoxide/commit/57614410a7b10c88b7bbab3e4a7ed1497568e29b))
  - Merge pull request #2202 from GitoxideLabs/dependabot/cargo/cargo-4a7155215a ([`9365cc3`](https://github.com/GitoxideLabs/gitoxide/commit/9365cc3ae8ad92ba2703170ac2f9a1e4df2ac3be))
  - Bump the cargo group across 1 directory with 64 updates ([`838ff95`](https://github.com/GitoxideLabs/gitoxide/commit/838ff95cca60c453bd97bd458ce31b384d00347e))
  - Merge pull request #2113 from GitoxideLabs/release ([`dc7343c`](https://github.com/GitoxideLabs/gitoxide/commit/dc7343c25ec6a62445e52694f7f0d3f95f31edef))
  - Release gix-actor v0.35.4, gix-fs v0.16.1, gix-object v0.50.2, gix-ref v0.53.1 ([`79ba9d0`](https://github.com/GitoxideLabs/gitoxide/commit/79ba9d009ca7536fadfe27b4fa56d1460327c906))
  - Merge pull request #2100 from GitoxideLabs/release ([`202bc6d`](https://github.com/GitoxideLabs/gitoxide/commit/202bc6da79854d1fb6bb32b9c6bb2a6f882c77f5))
  - Release gix-actor v0.35.3, gix-path v0.10.20, gix-features v0.43.1, gix-object v0.50.1 ([`d64f257`](https://github.com/GitoxideLabs/gitoxide/commit/d64f257951754ea70b0179b83f76de957b712211))
  - Merge pull request #2075 from GitoxideLabs/improvements ([`784c046`](https://github.com/GitoxideLabs/gitoxide/commit/784c0465bf87011fe7dbf71a590d3f9e6c8696a8))

</details>

## 0.6.0 (2025-07-15)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 9 commits contributed to the release over the course of 79 calendar days.
- 80 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.10.3, gix-actor v0.35.2, gix-trace v0.1.13, gix-path v0.10.19, gix-features v0.43.0, gix-hash v0.19.0, gix-hashtable v0.9.0, gix-object v0.50.0, gix-glob v0.21.0, gix-attributes v0.27.0, gix-command v0.6.2, gix-packetline-blocking v0.19.1, gix-filter v0.20.0, gix-fs v0.16.0, gix-commitgraph v0.29.0, gix-revwalk v0.21.0, gix-traverse v0.47.0, gix-worktree-stream v0.22.0, gix-archive v0.22.0, gix-tempfile v18.0.0, gix-lock v18.0.0, gix-index v0.41.0, gix-config-value v0.15.1, gix-pathspec v0.12.0, gix-ignore v0.16.0, gix-worktree v0.42.0, gix-diff v0.53.0, gix-blame v0.3.0, gix-ref v0.53.0, gix-sec v0.12.0, gix-config v0.46.0, gix-prompt v0.11.1, gix-url v0.32.0, gix-credentials v0.30.0, gix-discover v0.41.0, gix-dir v0.15.0, gix-mailmap v0.27.2, gix-revision v0.35.0, gix-merge v0.6.0, gix-negotiate v0.21.0, gix-pack v0.60.0, gix-odb v0.70.0, gix-refspec v0.31.0, gix-shallow v0.5.0, gix-packetline v0.19.1, gix-transport v0.48.0, gix-protocol v0.51.0, gix-status v0.20.0, gix-submodule v0.20.0, gix-worktree-state v0.20.0, gix v0.73.0, gix-fsck v0.12.0, gitoxide-core v0.48.0, gitoxide v0.45.0, safety bump 43 crates ([`5a919c4`](https://github.com/GitoxideLabs/gitoxide/commit/5a919c48393020d47c7034946108577dd213b80a))
  - Update changelogs prior to release ([`65037b5`](https://github.com/GitoxideLabs/gitoxide/commit/65037b56918b90ac07454a815b0ed136df2fca3b))
  - Merge pull request #2011 from blinxen/main ([`fd49eee`](https://github.com/GitoxideLabs/gitoxide/commit/fd49eeeb850ea3c3956ca15be2bf4e04a3d319ad))
  - Update `imara-diff` to the latest version. ([`732adb8`](https://github.com/GitoxideLabs/gitoxide/commit/732adb87c90283bd8f8fce6d633eacc25e10b353))
  - Merge pull request #2014 from GitoxideLabs/zip ([`648022b`](https://github.com/GitoxideLabs/gitoxide/commit/648022b44e12f597cae55cc45830d0a19b87eb4c))
  - Release gix-glob v0.20.1, gix-attributes v0.26.1, gix-command v0.6.1, gix-filter v0.19.2, gix-worktree-stream v0.21.2, gix-archive v0.21.2 ([`f0ed2cc`](https://github.com/GitoxideLabs/gitoxide/commit/f0ed2cc0046f866e67944bff9aef0579c12d5852))
  - Merge pull request #2009 from GitoxideLabs/release-gix-index ([`c3f06ae`](https://github.com/GitoxideLabs/gitoxide/commit/c3f06ae424ab4e1918a364cabe8276297465a73a))
  - Release gix-path v0.10.18, gix-date v0.10.2, gix-traverse v0.46.2, gix-index v0.40.1 ([`d2b4c44`](https://github.com/GitoxideLabs/gitoxide/commit/d2b4c44fcb2bf43e80d67532262631a5086f08de))
  - Merge pull request #1971 from GitoxideLabs/new-release ([`8d4c4d1`](https://github.com/GitoxideLabs/gitoxide/commit/8d4c4d1e09f84c962c29d98a686c64228196ac13))

</details>

## 0.5.1 (2025-04-26)

### Commit Statistics

<csr-read-only-do-not-edit/>

- 3 commits contributed to the release.
- 1 day passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-date v0.10.1, gix-utils v0.3.0, gix-actor v0.35.1, gix-validate v0.10.0, gix-path v0.10.17, gix-features v0.42.1, gix-hash v0.18.0, gix-hashtable v0.8.1, gix-object v0.49.1, gix-glob v0.20.0, gix-quote v0.6.0, gix-attributes v0.26.0, gix-command v0.6.0, gix-packetline-blocking v0.19.0, gix-filter v0.19.1, gix-fs v0.15.0, gix-commitgraph v0.28.0, gix-revwalk v0.20.1, gix-traverse v0.46.1, gix-worktree-stream v0.21.1, gix-archive v0.21.1, gix-tempfile v17.1.0, gix-lock v17.1.0, gix-index v0.40.0, gix-config-value v0.15.0, gix-pathspec v0.11.0, gix-ignore v0.15.0, gix-worktree v0.41.0, gix-diff v0.52.1, gix-blame v0.2.1, gix-ref v0.52.1, gix-sec v0.11.0, gix-config v0.45.1, gix-prompt v0.11.0, gix-url v0.31.0, gix-credentials v0.29.0, gix-discover v0.40.1, gix-dir v0.14.1, gix-mailmap v0.27.1, gix-revision v0.34.1, gix-merge v0.5.1, gix-negotiate v0.20.1, gix-pack v0.59.1, gix-odb v0.69.1, gix-refspec v0.30.1, gix-shallow v0.4.0, gix-packetline v0.19.0, gix-transport v0.47.0, gix-protocol v0.50.1, gix-status v0.19.1, gix-submodule v0.19.1, gix-worktree-state v0.19.0, gix v0.72.1, gix-fsck v0.11.1, gitoxide-core v0.47.1, gitoxide v0.44.0 ([`e104545`](https://github.com/GitoxideLabs/gitoxide/commit/e104545b78951ca882481d4a58f4425a8bc81c87))
  - Bump all prior pratch levels to majors ([`5f7f805`](https://github.com/GitoxideLabs/gitoxide/commit/5f7f80570e1a5522e76ea58cccbb957249a0dffe))
  - Merge pull request #1969 from GitoxideLabs/new-release ([`631f07a`](https://github.com/GitoxideLabs/gitoxide/commit/631f07ad0c1cb93d9da42cf2c8499584fe91880a))

</details>

## 0.5.0 (2025-04-25)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 11 commits contributed to the release.
- 21 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0 ([`cc5b696`](https://github.com/GitoxideLabs/gitoxide/commit/cc5b696b7b73277ddcc3ef246714cf80a092cf76))
  - Adjusting changelogs prior to release of gix-path v0.10.16, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.1, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.1, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.1, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.1, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.52.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.1, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.1, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.47.0, gitoxide v0.43.0, safety bump 7 crates ([`49fa9f3`](https://github.com/GitoxideLabs/gitoxide/commit/49fa9f38110ba975d68f5ac3baefeb55f0a0501b))
  - Release gix-date v0.10.0, gix-utils v0.2.1, gix-actor v0.35.0, gix-validate v0.9.5, gix-path v0.10.15, gix-features v0.42.0, gix-hash v0.17.1, gix-object v0.49.0, gix-glob v0.19.1, gix-quote v0.5.1, gix-attributes v0.25.0, gix-command v0.5.1, gix-packetline-blocking v0.18.4, gix-filter v0.19.0, gix-fs v0.14.0, gix-commitgraph v0.27.1, gix-revwalk v0.20.0, gix-traverse v0.46.0, gix-worktree-stream v0.21.0, gix-archive v0.21.0, gix-tempfile v17.0.1, gix-lock v17.0.1, gix-index v0.39.0, gix-config-value v0.14.13, gix-pathspec v0.10.1, gix-ignore v0.14.1, gix-worktree v0.40.0, gix-diff v0.52.0, gix-blame v0.2.0, gix-ref v0.51.0, gix-sec v0.10.13, gix-config v0.45.0, gix-prompt v0.10.1, gix-url v0.30.1, gix-credentials v0.28.1, gix-discover v0.40.0, gix-dir v0.14.0, gix-mailmap v0.27.0, gix-revision v0.34.0, gix-merge v0.5.0, gix-negotiate v0.20.0, gix-pack v0.59.0, gix-odb v0.69.0, gix-refspec v0.30.0, gix-shallow v0.3.1, gix-packetline v0.18.5, gix-transport v0.46.0, gix-protocol v0.50.0, gix-status v0.19.0, gix-submodule v0.19.0, gix-worktree-state v0.18.0, gix v0.72.0, gix-fsck v0.11.0, gitoxide-core v0.46.0, gitoxide v0.43.0, safety bump 30 crates ([`db0b095`](https://github.com/GitoxideLabs/gitoxide/commit/db0b0957930e3ebb1b3f05ed8d7e7a557eb384a2))
  - Update changelogs prior to release ([`0bf84db`](https://github.com/GitoxideLabs/gitoxide/commit/0bf84dbc041f59efba06adcf422c60b5d6e350f0))
  - Merge pull request #1935 from pierrechevalier83/fix_1923 ([`3b1bef7`](https://github.com/GitoxideLabs/gitoxide/commit/3b1bef7cc40e16b61bcc117ca90ebae21df7c7b1))
  - J fmt ([`c3c6504`](https://github.com/GitoxideLabs/gitoxide/commit/c3c650448f92bcb27194ce0a51f7d604ce87920d))
  - Merge pull request #1949 from GitoxideLabs/dependabot/cargo/cargo-6893e2988a ([`b5e9059`](https://github.com/GitoxideLabs/gitoxide/commit/b5e905991155ace32ef21464e69a8369a773f02b))
  - Bump the cargo group with 21 updates ([`68e6b2e`](https://github.com/GitoxideLabs/gitoxide/commit/68e6b2e54613fe788d645ea8c942c71a39c6ede1))
  - Merge pull request #1917 from pierrechevalier83/issue_1887 ([`6307f57`](https://github.com/GitoxideLabs/gitoxide/commit/6307f571f969eb7ff2490e4c68dc7994fb2fecac))
  - Adapt to changes in gix-object ([`8969245`](https://github.com/GitoxideLabs/gitoxide/commit/8969245a6b5874d8524f508569ae1266e48d100e))
  - Merge pull request #1919 from GitoxideLabs/release ([`420e730`](https://github.com/GitoxideLabs/gitoxide/commit/420e730f765b91e1d17daca6bb1f99bdb2e54fda))

</details>

## 0.4.0 (2025-04-04)

A maintenance release without user-facing changes.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 14 commits contributed to the release.
- 76 days passed between releases.
- 0 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Thanks Clippy

<csr-read-only-do-not-edit/>

[Clippy](https://github.com/rust-lang/rust-clippy) helped 1 time to make code idiomatic. 

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0 ([`d248e3d`](https://github.com/GitoxideLabs/gitoxide/commit/d248e3d87d45ca3983cb9fd7c6143dacbd8301cc))
  - Release gix-sec v0.10.12, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0 ([`ada5a94`](https://github.com/GitoxideLabs/gitoxide/commit/ada5a9447dc3c210afbd8866fe939c3f3a024226))
  - Release gix-date v0.9.4, gix-utils v0.2.0, gix-actor v0.34.0, gix-features v0.41.0, gix-hash v0.17.0, gix-hashtable v0.8.0, gix-path v0.10.15, gix-validate v0.9.4, gix-object v0.48.0, gix-glob v0.19.0, gix-quote v0.5.0, gix-attributes v0.25.0, gix-command v0.5.0, gix-packetline-blocking v0.18.3, gix-filter v0.18.0, gix-fs v0.14.0, gix-commitgraph v0.27.0, gix-revwalk v0.19.0, gix-traverse v0.45.0, gix-worktree-stream v0.20.0, gix-archive v0.20.0, gix-tempfile v17.0.0, gix-lock v17.0.0, gix-index v0.39.0, gix-config-value v0.14.12, gix-pathspec v0.10.0, gix-ignore v0.14.0, gix-worktree v0.40.0, gix-diff v0.51.0, gix-blame v0.1.0, gix-ref v0.51.0, gix-config v0.44.0, gix-prompt v0.10.0, gix-url v0.30.0, gix-credentials v0.28.0, gix-discover v0.39.0, gix-dir v0.13.0, gix-mailmap v0.26.0, gix-revision v0.33.0, gix-merge v0.4.0, gix-negotiate v0.19.0, gix-pack v0.58.0, gix-odb v0.68.0, gix-refspec v0.29.0, gix-shallow v0.3.0, gix-packetline v0.18.4, gix-transport v0.46.0, gix-protocol v0.49.0, gix-status v0.18.0, gix-submodule v0.18.0, gix-worktree-state v0.18.0, gix v0.71.0, gix-fsck v0.10.0, gitoxide-core v0.46.0, gitoxide v0.42.0, safety bump 48 crates ([`b41312b`](https://github.com/GitoxideLabs/gitoxide/commit/b41312b478b0d19efb330970cf36dba45d0fbfbd))
  - Update changelogs prior to release ([`38dff41`](https://github.com/GitoxideLabs/gitoxide/commit/38dff41d09b6841ff52435464e77cd012dce7645))
  - Merge pull request #1915 from emilazy/push-qvyqmopsoltr ([`4660f7a`](https://github.com/GitoxideLabs/gitoxide/commit/4660f7a6f71873311f68f170b0f1f6659a02829d))
  - Migrate `gix_object::{try_ =>}compute_hash` users ([`3d7e379`](https://github.com/GitoxideLabs/gitoxide/commit/3d7e379f26cbe53ddb430427b8e88ce0966be456))
  - Migrate hashing API users to fallible versions ([`fbf6cc8`](https://github.com/GitoxideLabs/gitoxide/commit/fbf6cc897cfeff5ed2a2d5946c060e0cebbd1afd))
  - Merge pull request #1907 from EliahKagan/run-ci/raw ([`7b17da6`](https://github.com/GitoxideLabs/gitoxide/commit/7b17da6ca1dce275de0d32d0b0d6c238621e6ee3))
  - Drop trailing `,` just before `)` on same line in function calls ([`66a5ae1`](https://github.com/GitoxideLabs/gitoxide/commit/66a5ae1b586d583066402c801213a55141e2aad6))
  - Merge pull request #1854 from GitoxideLabs/montly-report ([`16a248b`](https://github.com/GitoxideLabs/gitoxide/commit/16a248beddbfbd21621f2bb57aaa82dca35acb19))
  - Thanks clippy ([`8e96ed3`](https://github.com/GitoxideLabs/gitoxide/commit/8e96ed37db680855d194c10673ba2dab28655d95))
  - Merge pull request #1800 from GitoxideLabs/improvements ([`cc7b614`](https://github.com/GitoxideLabs/gitoxide/commit/cc7b614e541aa4a485f470f36516589619e2de5e))
  - Adapt to changes in `gix-command` ([`28f712e`](https://github.com/GitoxideLabs/gitoxide/commit/28f712e97ebd805cbcffe184203c9089248b0ca8))
  - Merge pull request #1778 from GitoxideLabs/new-release ([`8df0db2`](https://github.com/GitoxideLabs/gitoxide/commit/8df0db2f8fe1832a5efd86d6aba6fb12c4c855de))

</details>

## 0.3.0 (2025-01-18)

<csr-id-17835bccb066bbc47cc137e8ec5d9fe7d5665af0/>

### Commit Statistics

<csr-read-only-do-not-edit/>

- 6 commits contributed to the release over the course of 27 calendar days.
- 27 days passed between releases.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-utils v0.1.14, gix-actor v0.33.2, gix-hash v0.16.0, gix-trace v0.1.12, gix-features v0.40.0, gix-hashtable v0.7.0, gix-path v0.10.14, gix-validate v0.9.3, gix-object v0.47.0, gix-glob v0.18.0, gix-quote v0.4.15, gix-attributes v0.24.0, gix-command v0.4.1, gix-packetline-blocking v0.18.2, gix-filter v0.17.0, gix-fs v0.13.0, gix-chunk v0.4.11, gix-commitgraph v0.26.0, gix-revwalk v0.18.0, gix-traverse v0.44.0, gix-worktree-stream v0.19.0, gix-archive v0.19.0, gix-bitmap v0.2.14, gix-tempfile v16.0.0, gix-lock v16.0.0, gix-index v0.38.0, gix-config-value v0.14.11, gix-pathspec v0.9.0, gix-ignore v0.13.0, gix-worktree v0.39.0, gix-diff v0.50.0, gix-blame v0.0.0, gix-ref v0.50.0, gix-sec v0.10.11, gix-config v0.43.0, gix-prompt v0.9.1, gix-url v0.29.0, gix-credentials v0.27.0, gix-discover v0.38.0, gix-dir v0.12.0, gix-mailmap v0.25.2, gix-revision v0.32.0, gix-merge v0.3.0, gix-negotiate v0.18.0, gix-pack v0.57.0, gix-odb v0.67.0, gix-refspec v0.28.0, gix-shallow v0.2.0, gix-packetline v0.18.3, gix-transport v0.45.0, gix-protocol v0.48.0, gix-status v0.17.0, gix-submodule v0.17.0, gix-worktree-state v0.17.0, gix v0.70.0, gix-fsck v0.9.0, gitoxide-core v0.45.0, gitoxide v0.41.0, safety bump 42 crates ([`dea106a`](https://github.com/GitoxideLabs/gitoxide/commit/dea106a8c4fecc1f0a8f891a2691ad9c63964d25))
  - Don't specify version numbers in dev-dependencies ([`7570daa`](https://github.com/GitoxideLabs/gitoxide/commit/7570daa50a93a2b99e9cd5228cb274f20839865f))
  - Update all changelogs prior to release ([`1f6390c`](https://github.com/GitoxideLabs/gitoxide/commit/1f6390c53ba68ce203ae59eb3545e2631dd8a106))
  - Merge pull request #1762 from GitoxideLabs/fix-1759 ([`7ec21bb`](https://github.com/GitoxideLabs/gitoxide/commit/7ec21bb96ce05b29dde74b2efdf22b6e43189aab))
  - Bump `rust-version` to 1.70 ([`17835bc`](https://github.com/GitoxideLabs/gitoxide/commit/17835bccb066bbc47cc137e8ec5d9fe7d5665af0))
  - Merge pull request #1739 from GitoxideLabs/new-release ([`d22937f`](https://github.com/GitoxideLabs/gitoxide/commit/d22937f91b8ecd0ece0930c4df9d580f3819b2fe))

</details>

## 0.2.0 (2024-12-22)

### Bug Fixes

- <csr-id-da585db16bae9f44e6300f31b0f784c356d5bd3f/> don't incorrectly mark as auto-resolved conflict if there was none.
  Previously it was possible to mark perfectly working content merges as conflicting
  if we would choose 'our' or 'their' resolution.
- <csr-id-a57192c0418aab2e3cd2ddb7b7a951cd3aaeb58f/> when binary merges are performed, adjust the returned resolution to indicate auto-resolution.
  Previously it wasn't possible to detect auto-resolution, even though it can be assumed if a
  resolution mode was provided.

### New Features (BREAKING)

- <csr-id-1c3ba812bd3df5991a457b68a962aa1fd87fa915/> replace `tree::Options::allow_lossy_resolution` with `*::tree_conflicts`.
  That way it's possible to steer how to resolve tree-related conflicts while
  making it possible to detect that a conflict happened.

### Bug Fixes (BREAKING)

- <csr-id-3e94b58f00334392da89fb2772034efe06d3a741/> assure that `tree::apply_index_entries()` cannot unknowingly leave unconflicted *and* conflicted entries.
  This is a massive footgun currently where incorrect usage is very likely while causing all kinds of mistakes.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 13 commits contributed to the release over the course of 28 calendar days.
- 28 days passed between releases.
- 4 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0 ([`beb0ea8`](https://github.com/GitoxideLabs/gitoxide/commit/beb0ea8c4ff94c64b7773772a9d388ccb403f3c1))
  - Release gix-date v0.9.3, gix-object v0.46.1, gix-command v0.4.0, gix-filter v0.16.0, gix-fs v0.12.1, gix-traverse v0.43.1, gix-worktree-stream v0.18.0, gix-archive v0.18.0, gix-ref v0.49.1, gix-prompt v0.9.0, gix-url v0.28.2, gix-credentials v0.26.0, gix-diff v0.49.0, gix-dir v0.11.0, gix-revision v0.31.1, gix-merge v0.2.0, gix-pack v0.56.0, gix-odb v0.66.0, gix-shallow v0.1.0, gix-packetline v0.18.2, gix-transport v0.44.0, gix-protocol v0.47.0, gix-status v0.16.0, gix-worktree-state v0.16.0, gix v0.69.0, gitoxide-core v0.44.0, gitoxide v0.40.0, safety bump 16 crates ([`c1ba571`](https://github.com/GitoxideLabs/gitoxide/commit/c1ba5719132227410abefeb54e3032b015233e94))
  - Update changelogs prior to release ([`7ea8582`](https://github.com/GitoxideLabs/gitoxide/commit/7ea85821c6999e3e6cf50a2a009904e9c38642a4))
  - Merge pull request #1705 from GitoxideLabs/merge ([`520c832`](https://github.com/GitoxideLabs/gitoxide/commit/520c832cfcfb34eb7617be55ebe2719ab35595fd))
  - Adapt to changes in `gix-diff` related to not tracking empty blobs anymore. ([`f53cec5`](https://github.com/GitoxideLabs/gitoxide/commit/f53cec5b2ce8aa6eeb4a3016511bbb1ac25fa2f7))
  - Don't incorrectly mark as auto-resolved conflict if there was none. ([`da585db`](https://github.com/GitoxideLabs/gitoxide/commit/da585db16bae9f44e6300f31b0f784c356d5bd3f))
  - Assure that `tree::apply_index_entries()` cannot unknowingly leave unconflicted *and* conflicted entries. ([`3e94b58`](https://github.com/GitoxideLabs/gitoxide/commit/3e94b58f00334392da89fb2772034efe06d3a741))
  - Implement support for resolving irreconcilable tree conflicts with 'ours' or 'ancestor' ([`e487cca`](https://github.com/GitoxideLabs/gitoxide/commit/e487cca78d8e6c5b51d2614daf05c98e1469ee69))
  - Merge pull request #1708 from EliahKagan/run-ci/mode ([`34efe03`](https://github.com/GitoxideLabs/gitoxide/commit/34efe03fdab97bbf5603a7ea605f37096ff1736a))
  - Add missing executable bits on fixture scripts ([`ed757ea`](https://github.com/GitoxideLabs/gitoxide/commit/ed757ea0f4f80968d80c5d9d75ba49f031ee77fc))
  - Replace `tree::Options::allow_lossy_resolution` with `*::tree_conflicts`. ([`1c3ba81`](https://github.com/GitoxideLabs/gitoxide/commit/1c3ba812bd3df5991a457b68a962aa1fd87fa915))
  - When binary merges are performed, adjust the returned resolution to indicate auto-resolution. ([`a57192c`](https://github.com/GitoxideLabs/gitoxide/commit/a57192c0418aab2e3cd2ddb7b7a951cd3aaeb58f))
  - Merge pull request #1701 from GitoxideLabs/release ([`e8b3b41`](https://github.com/GitoxideLabs/gitoxide/commit/e8b3b41dd79b8f4567670b1f89dd8867b6134e9e))

</details>

## 0.1.0 (2024-11-24)

<csr-id-2fdbcfe17cdcc480e320582d7c6b48f8b615bf3b/>

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

- <csr-id-3ee8b62dd025d6fdb0d9929dec7a561fa576f545/> provide a way to record and apply index changes.
  These changes will then be applicable to an index that is created
  from the written tree editor.
- <csr-id-09213bc1b2aa725af1571dff040415772e844c3a/> when blob-merging, clarify if something would have conflicted.
- <csr-id-9e106c4ab7ceea091cd8baef99a480739bd53c9d/> add `Conflict::is_unresolved()` as utility to see if multiple of them are considered unresolved.
- <csr-id-bd91d6ae97d1981a2366136040407590f64fdad4/> respect the `conflict-marker-size` attribute as well.
- <csr-id-4b1764ca9148e08ae9f11bca68e0689b12bc8c80/> add `tree()` and `commit()` merge support, en par with `merge-ORT` as far as tests go.
  Note that this judgement of quality is based on a limited amount of partially complex
  test, but it's likely that in practice there will be deviations of sorts.
  
  Also, given the complexity of the implementation it is definitely under-tested,
  but with that it's mostly en par with Git, unfortunatly.
  
  On the bright side, some of the tests are very taxing and I'd hope this
  means something for real-world quality.
- <csr-id-dd99991ec2bfb07ef571769abc32f1b35122d5ca/> add `blob::PlatformRef::id_by_pick()` to more efficiently pick merge results.
  This works by either selecting a possibly unchanged and not even loaded resource,
  instead of always loading it to provide a buffer, in case the user doesn't
  actually want a buffer.
  
  Note that this also alters `buffer_by_pick()` to enforce handling of the 'buffer-too-large'
  option.

### New Features (BREAKING)

- <csr-id-aff76f291a52fc6806944d72d249a8bd1b804c39/> Add more modes for checking for unresolved conflicts.
  They aim at making it possible to know if a conflict happened that was
  automatically resolved.
- <csr-id-9d43b753a225482645b22e4151bf7dc192c8c082/> add `commit::virtual_merge_base()` to produce the single merge-base to use.
  This allows more flexibility in conjunction with tree-merging, as
  commits as input aren't required.
  
  This is breaking as it changes the return value of `commit()`.
- <csr-id-1d2262f2ca416e3c22f9601e7eab11f3372b2128/> `Repository::merge_trees()` now has a fully-wrapped outcome.
  That way, more attached types are used for greater convenience.
- <csr-id-c1cf08cc47f98abda017544b1791ab3b4463cc77/> Don't fail on big files during blob-merge, but turn them into binary merges.
  Binary merges are mere choices of which side to pick, which works well for big files
  as well. Git doesn't define this well during its own merges, so there is some room here.

### Bug Fixes (BREAKING)

- <csr-id-de1cfb6a9caf5ac086c6411824835c75e888e2d7/> Adjust blob-merge baseline to also test the reverse of each operation
  This also fixes an issue with blob merge computations.
  
  It's breaking because the marker-size was reduced to `u8`.
- <csr-id-78a535572643a0657348aea3c2fed0123505f7fe/> prefer to receive borrowed `gix_command::Context` when it's just passed on.
  That way, the clone occours only when needed, without forcing the caller
  to pre-emptively clone each time it's called.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 44 commits contributed to the release.
- 14 commits were understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-glob v0.17.1, gix-command v0.3.11, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-sec v0.10.10, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-ignore v0.12.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0 ([`4000197`](https://github.com/GitoxideLabs/gitoxide/commit/4000197ecc8cf1a5d79361620e4c114f86476703))
  - Release gix-date v0.9.2, gix-actor v0.33.1, gix-hash v0.15.1, gix-features v0.39.1, gix-validate v0.9.2, gix-object v0.46.0, gix-path v0.10.13, gix-quote v0.4.14, gix-attributes v0.23.1, gix-packetline-blocking v0.18.1, gix-filter v0.15.0, gix-chunk v0.4.10, gix-commitgraph v0.25.1, gix-revwalk v0.17.0, gix-traverse v0.43.0, gix-worktree-stream v0.17.0, gix-archive v0.17.0, gix-config-value v0.14.10, gix-lock v15.0.1, gix-ref v0.49.0, gix-config v0.42.0, gix-prompt v0.8.9, gix-url v0.28.1, gix-credentials v0.25.1, gix-bitmap v0.2.13, gix-index v0.37.0, gix-worktree v0.38.0, gix-diff v0.48.0, gix-discover v0.37.0, gix-pathspec v0.8.1, gix-dir v0.10.0, gix-mailmap v0.25.1, gix-revision v0.31.0, gix-merge v0.1.0, gix-negotiate v0.17.0, gix-pack v0.55.0, gix-odb v0.65.0, gix-packetline v0.18.1, gix-transport v0.43.1, gix-protocol v0.46.1, gix-refspec v0.27.0, gix-status v0.15.0, gix-submodule v0.16.0, gix-worktree-state v0.15.0, gix v0.68.0, gix-fsck v0.8.0, gitoxide-core v0.43.0, gitoxide v0.39.0, safety bump 25 crates ([`8ce4912`](https://github.com/GitoxideLabs/gitoxide/commit/8ce49129a75e21346ceedf7d5f87fa3a34b024e1))
  - Prepare changelogs prior to release ([`bc9d994`](https://github.com/GitoxideLabs/gitoxide/commit/bc9d9943e8499a76fc47a05b63ac5c684187d1ae))
  - Merge pull request #1661 from GitoxideLabs/merge ([`0b7abfb`](https://github.com/GitoxideLabs/gitoxide/commit/0b7abfbdebe8c5ab30b89499a70dd7727de41184))
  - Provide a way to record and apply index changes. ([`3ee8b62`](https://github.com/GitoxideLabs/gitoxide/commit/3ee8b62dd025d6fdb0d9929dec7a561fa576f545))
  - Add more modes for checking for unresolved conflicts. ([`aff76f2`](https://github.com/GitoxideLabs/gitoxide/commit/aff76f291a52fc6806944d72d249a8bd1b804c39))
  - When blob-merging, clarify if something would have conflicted. ([`09213bc`](https://github.com/GitoxideLabs/gitoxide/commit/09213bc1b2aa725af1571dff040415772e844c3a))
  - Merge pull request #1662 from paolobarbolini/thiserror-v2 ([`7a40648`](https://github.com/GitoxideLabs/gitoxide/commit/7a406481b072728cec089d7c05364f9dbba335a2))
  - Upgrade thiserror to v2.0.0 ([`0f0e4fe`](https://github.com/GitoxideLabs/gitoxide/commit/0f0e4fe121932a8a6302cf950b3caa4c8608fb61))
  - Merge pull request #1658 from GitoxideLabs/merge ([`905e5b4`](https://github.com/GitoxideLabs/gitoxide/commit/905e5b42a6163f92edef8fab82d97aeb6f17cf06))
  - Add `commit::virtual_merge_base()` to produce the single merge-base to use. ([`9d43b75`](https://github.com/GitoxideLabs/gitoxide/commit/9d43b753a225482645b22e4151bf7dc192c8c082))
  - Merge pull request #1654 from EliahKagan/doctest-workspace ([`1411289`](https://github.com/GitoxideLabs/gitoxide/commit/141128942c26bd63fc6855e5137b98f8da814446))
  - Fix code fences in gix-merge `ConflictStyle` and `Driver` ([`2fdbcfe`](https://github.com/GitoxideLabs/gitoxide/commit/2fdbcfe17cdcc480e320582d7c6b48f8b615bf3b))
  - Merge pull request #1651 from GitoxideLabs/merge ([`a876533`](https://github.com/GitoxideLabs/gitoxide/commit/a8765330fc16997dee275866b18a128dec1c5d55))
  - `Repository::merge_trees()` now has a fully-wrapped outcome. ([`1d2262f`](https://github.com/GitoxideLabs/gitoxide/commit/1d2262f2ca416e3c22f9601e7eab11f3372b2128))
  - Add `Conflict::is_unresolved()` as utility to see if multiple of them are considered unresolved. ([`9e106c4`](https://github.com/GitoxideLabs/gitoxide/commit/9e106c4ab7ceea091cd8baef99a480739bd53c9d))
  - Remove a TODO that turned out to be unnecessary. ([`5b428a9`](https://github.com/GitoxideLabs/gitoxide/commit/5b428a9e931b19622ae76c25bfb1fe882744cd1f))
  - Merge pull request #1652 from EliahKagan/run-ci/chmod ([`8e99eba`](https://github.com/GitoxideLabs/gitoxide/commit/8e99eba2a284b35b5e9bcb97e47bfbbafc3df5d1))
  - Update tree-baseline archive ([`ab45415`](https://github.com/GitoxideLabs/gitoxide/commit/ab45415a0119cbdcbd2fcbe8c7b6e2580432f705))
  - Set +x in index in added-file-changed-content-and-mode ([`6faf11a`](https://github.com/GitoxideLabs/gitoxide/commit/6faf11a0e0916fdf03e4532d28e40674c369bf9b))
  - Set +x in index in same-rename-different-mode baseline ([`041bdde`](https://github.com/GitoxideLabs/gitoxide/commit/041bddecacb23b4925bcd9bcdc4a86aec2ea719d))
  - Merge pull request #1618 from GitoxideLabs/merge ([`3fb989b`](https://github.com/GitoxideLabs/gitoxide/commit/3fb989be21c739bbfeac93953c1685e7c6cd2106))
  - Respect the `conflict-marker-size` attribute as well. ([`bd91d6a`](https://github.com/GitoxideLabs/gitoxide/commit/bd91d6ae97d1981a2366136040407590f64fdad4))
  - Add `tree()` and `commit()` merge support, en par with `merge-ORT` as far as tests go. ([`4b1764c`](https://github.com/GitoxideLabs/gitoxide/commit/4b1764ca9148e08ae9f11bca68e0689b12bc8c80))
  - Adjust blob-merge baseline to also test the reverse of each operation ([`de1cfb6`](https://github.com/GitoxideLabs/gitoxide/commit/de1cfb6a9caf5ac086c6411824835c75e888e2d7))
  - Add `blob::PlatformRef::id_by_pick()` to more efficiently pick merge results. ([`dd99991`](https://github.com/GitoxideLabs/gitoxide/commit/dd99991ec2bfb07ef571769abc32f1b35122d5ca))
  - Don't fail on big files during blob-merge, but turn them into binary merges. ([`c1cf08c`](https://github.com/GitoxideLabs/gitoxide/commit/c1cf08cc47f98abda017544b1791ab3b4463cc77))
  - Prefer to receive borrowed `gix_command::Context` when it's just passed on. ([`78a5355`](https://github.com/GitoxideLabs/gitoxide/commit/78a535572643a0657348aea3c2fed0123505f7fe))
  - Merge pull request #1642 from GitoxideLabs/new-release ([`db5c9cf`](https://github.com/GitoxideLabs/gitoxide/commit/db5c9cfce93713b4b3e249cff1f8cc1ef146f470))
  - Release gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0 ([`f1364dc`](https://github.com/GitoxideLabs/gitoxide/commit/f1364dcb8aa66e3d8730e38445b045c5b63c56e6))
  - Add new changelog for gix-merge ([`fa3e260`](https://github.com/GitoxideLabs/gitoxide/commit/fa3e2600d7e39011f1d7f410249ebd0426a348a8))
  - Release gix-date v0.9.1, gix-utils v0.1.13, gix-actor v0.33.0, gix-hash v0.15.0, gix-trace v0.1.11, gix-features v0.39.0, gix-hashtable v0.6.0, gix-validate v0.9.1, gix-object v0.45.0, gix-path v0.10.12, gix-glob v0.17.0, gix-quote v0.4.13, gix-attributes v0.23.0, gix-command v0.3.10, gix-packetline-blocking v0.18.0, gix-filter v0.14.0, gix-fs v0.12.0, gix-chunk v0.4.9, gix-commitgraph v0.25.0, gix-revwalk v0.16.0, gix-traverse v0.42.0, gix-worktree-stream v0.16.0, gix-archive v0.16.0, gix-config-value v0.14.9, gix-tempfile v15.0.0, gix-lock v15.0.0, gix-ref v0.48.0, gix-sec v0.10.9, gix-config v0.41.0, gix-prompt v0.8.8, gix-url v0.28.0, gix-credentials v0.25.0, gix-ignore v0.12.0, gix-bitmap v0.2.12, gix-index v0.36.0, gix-worktree v0.37.0, gix-diff v0.47.0, gix-discover v0.36.0, gix-pathspec v0.8.0, gix-dir v0.9.0, gix-mailmap v0.25.0, gix-merge v0.0.0, gix-negotiate v0.16.0, gix-pack v0.54.0, gix-odb v0.64.0, gix-packetline v0.18.0, gix-transport v0.43.0, gix-protocol v0.46.0, gix-revision v0.30.0, gix-refspec v0.26.0, gix-status v0.14.0, gix-submodule v0.15.0, gix-worktree-state v0.14.0, gix v0.67.0, gix-fsck v0.7.0, gitoxide-core v0.42.0, gitoxide v0.38.0, safety bump 41 crates ([`3f7e8ee`](https://github.com/GitoxideLabs/gitoxide/commit/3f7e8ee2c5107aec009eada1a05af7941da9cb4d))
  - Merge pull request #1624 from EliahKagan/update-repo-url ([`795962b`](https://github.com/GitoxideLabs/gitoxide/commit/795962b107d86f58b1f7c75006da256d19cc80ad))
  - Update gitoxide repository URLs ([`64ff0a7`](https://github.com/GitoxideLabs/gitoxide/commit/64ff0a77062d35add1a2dd422bb61075647d1a36))
  - Merge pull request #1612 from Byron/merge ([`37c1e4c`](https://github.com/GitoxideLabs/gitoxide/commit/37c1e4c919382c9d213bd5ca299ed659d63ab45d))
  - Add some performance traces for blob-merges ([`b25fe4d`](https://github.com/GitoxideLabs/gitoxide/commit/b25fe4d052250ddace9a118b2247537b2a7fa09e))
  - Merge pull request #1611 from Byron/merge ([`5ffccd2`](https://github.com/GitoxideLabs/gitoxide/commit/5ffccd2f08d70576347e3ae17a66ca5a60f1d81c))
  - Make `blob::Platform::filter_mode` public. ([`b26eb26`](https://github.com/GitoxideLabs/gitoxide/commit/b26eb2618c1764f699530e251ddef4e6cf456ddd))
  - Merge pull request #1585 from Byron/merge ([`2261de4`](https://github.com/GitoxideLabs/gitoxide/commit/2261de470aeb77be080f9e423e1513bde85d9cc0))
  - Add platform tests and implementation ([`eb37dc3`](https://github.com/GitoxideLabs/gitoxide/commit/eb37dc36d8c42f5a7714c641244ce4a13111b0a1))
  - Add all relevant tests for the merge processing pipeline ([`a6f3e30`](https://github.com/GitoxideLabs/gitoxide/commit/a6f3e30017343c01ba61c49fe74ffc69e443a33c))
  - Implement `text` and `binary` merge algorithms, also with baseline tests for correctness. ([`0762846`](https://github.com/GitoxideLabs/gitoxide/commit/07628465a0a3f047ec809d287c9a4567b4acd607))
  - Sketch the entire API surface to capture all parts of blob-merges ([`9efa09f`](https://github.com/GitoxideLabs/gitoxide/commit/9efa09f10d042dc4d5db4edf4589594450a30b31))
  - Add the `gix-merge` crate for capturing merge algorithms ([`65efcb7`](https://github.com/GitoxideLabs/gitoxide/commit/65efcb7624031ae478056fbb49a07ef176f0c96b))

</details>

## v0.0.0 (2024-10-22)

<csr-id-64ff0a77062d35add1a2dd422bb61075647d1a36/>
