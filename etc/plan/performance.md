# Performance Plan

Imported on: 2026-04-30

Fully generated as overview, intiial motivation is to keep track of `gix-index` performance and possible improvements.

Source basis:

- All `TODO(perf)` markers in this checkout.
- The two `TODO(performance)` markers, as they are equivalent in intent.
- Nearby generic TODOs that may affect common Git performance paths.
- A local comparison against `/Users/byron/dev/github.com/git/git`.

Working assumption: priorities are based on ordinary interactive and networked Git use, not on synthetic microbenchmarks alone.

## Typical Workflows

These workflows define the impact ordering:

- `git status`, `git add`, and editor integrations that repeatedly refresh the index, classify the worktree, and scan untracked paths.
- `git commit`, `git checkout`, `git switch`, `git merge`, and `git reset --hard`, which depend on fast index reads, tree/index conversion, checkout, and content merge.
- `git diff`, `git log -- <path>`, and `git blame`, which traverse trees, objects, and diffs repeatedly.
- `git fetch`, `git pull`, and `git clone`, which stress negotiation, object lookup, pack ingestion, duplicate avoidance, and pack/index refresh.
- `git push`, `git gc`, `git repack`, and object verification, which stress pack traversal, delta reconstruction, bitmaps, and multi-pack metadata.
- `git archive` and export-style commands, which stream trees with attributes and filters.

## Priority Backlog

### P0: Status, Add, And Index Refresh

- [ ] **index-lookup-accelerator**: Make index path lookup acceleration a default fast path, not only an ignore-case helper.
  - Source TODOs:
    - `gix-dir/src/walk/classify.rs:396`: build a multi-threaded hash table so lookups are always accelerated, even for case-sensitive paths.
    - `crate-status.md:844`: always use multi-threaded initialization of the case-insensitive hash table to accelerate index lookups.
    - `gix-index/src/access/mod.rs:144`: multi-threaded insertion needs a raw table with multiple bucket locks.
  - Workflow impact: very high for `status`, `add`, checkout collision checks, untracked filtering, and pathspec-heavy commands in large worktrees.
  - Git comparison: Git's `name-hash.c` keeps `index_state.name_hash` and `dir_hash` for index lookups. It uses lazy initialization, a per-thread threshold (`LAZY_THREAD_COST`), bucket-derived locks, a two-phase directory/name hash build, and exact-then-ignore-case comparison. Git's `read-cache.c` wires name-hash updates into index entry insertion.
  - Plan:
    - Factor `gix-index::AccelerateLookup` into a general name/directory lookup accelerator usable for case-sensitive and case-insensitive callers.
    - Add a parallel build path with sharded insertion or per-thread partial tables merged at the end.
    - Benchmark `status` and `dirwalk` on large repos with `core.ignoreCase` true and false.

- [ ] **status-refresh-heuristics**: Close the status refresh heuristic gap.
  - Source TODOs:
    - `gix-status/src/index_as_worktree/function.rs:144`: decide when parallelization is not worth it; Git uses about 500 entries per thread, capped at 20 preload threads.
    - `src/plumbing/main.rs:433`: make thread-limit tuning configurable; macOS and Linux scale differently.
    - `gix/src/status/mod.rs:165`: make `Repository::is_dirty()` a dedicated early-stop implementation with parallelism.
  - Workflow impact: very high for prompts, IDEs, `git status`, and dirty checks in automation.
  - Git comparison: Git's `preload-index.c` uses `MAX_PARALLEL = 20`, `THREAD_COST = 500`, skips staged/submodule/up-to-date/skip-worktree/fsmonitor-valid entries, and marks entries up to date after `lstat`.
  - Plan:
    - Measure the current chunk heuristic against Git-like thresholds on Linux and macOS.
    - Add a dedicated `is_dirty()` path that stops after the first tree/index or index/worktree change.
    - Make thread policy configurable through repository/config options rather than command-specific constants.

- [ ] **untracked-fsmonitor-cache**: Use untracked-cache and fsmonitor data aggressively where valid.
  - Source TODOs:
    - `crate-status.md:846`: accelerated walk with the `UNTR` extension.
    - `gix-index/src/extension/untracked_cache.rs:29` and `:33`: understand directory stat data and extension semantics fully.
    - `gix-status/src/index_as_worktree/traits.rs:105`: make streaming I/O interruptible.
  - Workflow impact: very high for `status` in large repositories with many ignored/untracked files.
  - Git comparison: Git's `wt-status.c` passes `istate->untracked` into `fill_directory()` and avoids full scans when valid; `read-cache.c` also tweaks fsmonitor and untracked-cache state after index load.
  - Plan:
    - Finish documenting and validating `UNTR` semantics.
    - Integrate valid untracked-cache state into `gix-dir` walks and status collection.
    - Preserve interruptibility for editor integrations and long-running worktree scans.

### P0: Index Read, Decode, And Tree-To-Index

- [ ] **index-decode-storage**: Fix index decode storage costs before adding more threads.
  - Source TODOs:
    - `gix-index/src/decode/entries.rs:185`: `path_backing.extend_from_slice()` causes large `memmove` time despite apparent capacity.
    - `crate-status.md:856`: threaded index read spends most time storing paths and currently has little benefit.
    - `gix-index/src/decode/entries.rs:118`: entries behave like an intrusive path-keyed collection; this likely affects ignore-case and lookup.
    - `gix-index/src/entry/flags.rs:11`: use persisted path length to save in-memory entry size.
  - Workflow impact: very high for every command that opens a large repository.
  - Git comparison: Git's `read-cache.c` memory-maps the index, allocates cache entries from mempools, uses the EOIE and IEOT extensions for threaded extension and entry loading, and stores path bytes inline in `struct cache_entry` (`name[FLEX_ARRAY]`). It also records an index entry offset table when threaded reads are requested.
  - Plan:
    - Profile the `path_backing` growth path and replace it with either pre-sized storage, chunked storage, or per-thread path arenas merged without repeated moves.
    - Evaluate an entry representation that stores path length compactly and gives lookup structures stable references.
    - Treat parallel decode as second-stage work after path storage stops dominating.

- [ ] **tree-index-sort**: Avoid post-sort work when initializing an index from a tree.
  - Source TODO:
    - `gix-index/src/init.rs:107`: `remove_file_directory_conflicts()` sorts only to protect against invalid trees; typical valid trees already compare in order.
  - Workflow impact: high for checkout, archive setup, worktree stream, and operations that synthesize indexes from trees.
  - Git comparison: Git's cache-tree machinery (`cache-tree.c`) validates tree/index order and can prime an index's cache tree directly from a tree. `read-cache.c` and `cache-tree.c` keep sorted index invariants central.
  - Plan:
    - Insert entries in sorted order when walking trees and detect invalid file/directory conflicts during insertion.
    - Add a fast path for already sorted valid trees and a fall-back verifier for malformed data.
    - Reuse or extend cache-tree information when constructing an index from `HEAD`.

- [ ] **worktree-stream-traversal**: Reduce double traversal in worktree streaming and archive creation.
  - Source TODOs:
    - `gix/src/repository/worktree.rs:88`: use the index at `HEAD` if possible.
    - `gix/src/repository/worktree.rs:89`: non-HEAD trees are effectively traversed twice; object-cache sharing across copied ODB handles is not trivial.
  - Workflow impact: medium to high for `archive`, export, and tooling that streams a tree with attributes.
  - Git comparison: Git relies heavily on cache-tree and index state to avoid redundant tree work when the requested tree matches index/HEAD state.
  - Plan:
    - Fast-path `HEAD` through an existing index plus cache-tree where available.
    - For non-HEAD trees, evaluate an explicit shared object/header cache for the two traversal consumers.

### P0: Object Database And Pack Access

- [ ] **odb-refresh-wait**: Replace spin/yield waiting during dynamic pack-index refresh.
  - Source TODO:
    - `gix-odb/src/store_impls/dynamic/load_index.rs:152`: a potentially hot loop should probably be a condition variable.
  - Workflow impact: high for fetch, clone, object lookup under concurrency, and commands that observe newly written packs.
  - Git comparison: Git's pack store preparation is centralized in `packfile.c`, and pack loading/re-preparation is explicit. It avoids leaving multiple callers in a spin loop while another caller is loading the same pack metadata.
  - Plan:
    - Replace the yield loop with a wait/notify primitive around index-load state transitions.
    - Measure concurrent object lookup during fetch and after pack creation.

- [ ] **loose-object-single-lookup**: Avoid duplicate loose-object lookup work.
  - Source TODOs:
    - `gix-odb/src/store_impls/dynamic/find.rs:292`: remove loose DB `contains()` plus `try_find()` double lookup.
    - `gix-odb/src/store_impls/dynamic/header.rs:166`: same double lookup for headers.
  - Workflow impact: medium to high for loose-heavy repositories, freshly created objects, and test fixtures.
  - Git comparison: Git's ODB lookup API returns object info/content from one lookup path and then moves packs toward the front after hits.
  - Plan:
    - Add a single-call loose-object API that can report missing, header-only, or full object data.
    - Preserve the current borrow-safety shape by returning owned metadata where needed.

- [ ] **pack-delta-base-cache**: Revisit pack delta reconstruction and base caching.
  - Source TODO:
    - `gix-pack/src/data/file/decode/entry.rs:381`: optimize memory-intensive delta-chain reconstruction after more tests exist.
  - Workflow impact: high for checkout after fetch, clone verification, diff/log over packed history, and blame.
  - Git comparison: Git's `packfile.c` uses a `delta_base_cache` keyed by pack and base offset with an LRU size limit, a small preallocated delta stack, and delayed insertion into the cache to avoid races while unpacking.
  - Plan:
    - Add coverage for long and shared delta chains first.
    - Compare current buffer swapping with Git's base-cache strategy and adopt an LRU base cache if repeated base materialization is visible in profiles.

- [ ] **fetch-duplicate-objects**: Investigate duplicate received objects during fetch.
  - Source TODO:
    - `gix/tests/gix/remote/fetch.rs:128`: tests observe substantial duplication when receiving objects.
  - Workflow impact: high for fetch, pull, and clone.
  - Git comparison: Git uses bitmaps, multi-pack-index, pack reuse, and object-entry de-duplication in pack-object paths.
  - Plan:
    - Determine if duplication is transport negotiation, pack ingestion, replacement refs, or iteration/reporting.
    - Add a counter-based regression test around unique received object IDs and pack refresh count.

### P1: Diff, Blame, Merge, And History Queries

- [ ] **blame-rename-diff-cache**: Avoid duplicated tree diff work in blame rename detection.
  - Source TODO:
    - `gix-blame/src/file/function.rs:582`: rename tracking repeats tree diff work after the no-rewrite pass.
  - Workflow impact: medium to high for `git blame`, especially with `-M`, `-C`, and path history over rename-heavy repositories.
  - Git comparison: Git's `blame.c` first looks for unchanged origins, then renames, then optionally moves/copies. It carries `blame_origin`, `blame_entry`, score thresholds, and optional blame Bloom data rather than blindly repeating the same tree walk for every case.
  - Plan:
    - Feed the first path-limited tree diff into a rewrite tracker when an addition or deletion is found.
    - Cache parent/current tree IDs and blob headers per commit pair.
    - Evaluate a blame-specific origin cache modeled after Git's two-pass origin/rename structure.

- [ ] **merge-conflict-structures**: Audit merge tree conflict data structures.
  - Source TODO:
    - `gix-merge/src/tree/mod.rs:376`: a better data structure may be needed for some directory/file conflict representation.
  - Workflow impact: medium for merges and rebases in large trees.
  - Git comparison: Git's merge and rename machinery keeps maps for directory rename counts and per-path pairing in `diffcore-rename.c`.
  - Plan:
    - Profile rename and directory/file conflict heavy merges.
    - Replace ad hoc per-conflict representation only if lookup or rewrite costs dominate.

- [ ] **diff-preprocess-rescans**: Remove avoidable rescans in diff preprocessing.
  - Source TODO:
    - `gix-imara-diff/src/myers/preprocess.rs:132`: do not unnecessarily rescan lines.
  - Workflow impact: medium for large file diffs and blame.
  - Git comparison: Git's xdiff stack has dedicated preprocessing and histogram/patience variants. `gix-imara-diff` already carries Git-inspired histogram behavior, so this should remain benchmark-led.
  - Plan:
    - Add benchmarks for common source-file diffs and large generated files.
    - Cache sliding-window counts or use prefix/suffix pruning data already computed during tokenization.

### P1: Repository Discovery, Refs, And Path Handling

- [ ] **packed-ref-lookup**: Implement packed-buffer aware reference lookup in general handles.
  - Source TODO:
    - `gix-ref/src/store/general/handle/find.rs:28`: implement lookup with packed-buffer handling.
  - Workflow impact: medium for commands that resolve many refs, such as branch listing, fetch negotiation, and revision parsing.
  - Git comparison: Git's ref backend keeps packed refs as a searchable table and layers loose refs on top.
  - Plan:
    - Add a packed-ref buffer search path with loose-over-packed precedence.
    - Benchmark ref-heavy repos with many tags.

- [ ] **literal-path-normalization**: Avoid path normalization through pattern machinery for single literal paths.
  - Source TODOs:
    - `gitoxide-core/src/repository/blame.rs:29`, `gitoxide-core/src/repository/merge/file.rs:37`, `tests/it/src/commands/blame_copy_royal.rs:58`: normalize paths without going through patterns.
  - Workflow impact: medium for porcelain commands over individual paths.
  - Plan:
    - Add a literal path normalization helper below pathspec parsing.
    - Keep pathspec matching for real pattern inputs only.

### P2: Allocation And Micro-Hotspots

- [ ] **hex-prefix-stack-decode**: Remove heap allocation from odd-length hex prefix parsing.
  - Source TODO:
    - `gix-hash/src/prefix.rs:120`: decode odd hex prefixes without heap allocation.
  - Workflow impact: low to medium, but may matter for abbreviated object lookup and ref/path disambiguation.
  - Git comparison: Git's object abbreviation logic in `object-name.c` and `packfile.c` works against fixed-size object IDs and pack fanout searches rather than allocating per prefix.
  - Plan:
    - Decode into a stack buffer sized by `Kind::longest()` and copy directly into `ObjectId`.
    - Add a microbenchmark for odd and even prefix parsing.

- [ ] **small-attribute-values**: Use a small byte-string representation for attribute values.
  - Source TODO:
    - `gix-attributes/src/state.rs:8`: a small byte string could provide an estimated 5 percent improvement.
  - Workflow impact: low to medium for status, archive, checkout, and filter-heavy repos.
  - Plan:
    - Prototype a `smallvec`-backed byte string with display/serde wrappers.
    - Validate JSON behavior and attribute query benchmarks before adopting.

- [ ] **signed-data-reader**: Stream signed commit data without allocation.
  - Source TODO:
    - `gix-object/src/commit/mod.rs:33`: implement `std::io::Read` for `SignedData`.
  - Workflow impact: low to medium, mostly signature verification and tooling over many signed commits.
  - Plan:
    - Add a segmented reader over the two non-signature ranges instead of building a `BString`.

- [ ] **packetline-borrow-decode**: Remove extra packet-line decoding caused by borrow-checker workarounds.
  - Source TODO:
    - `gix-packetline/src/blocking_io/read.rs:110`: avoid additional decoding of the internal buffer.
  - Workflow impact: low to medium for fetch/clone protocol handling.
  - Plan:
    - Revisit API lifetimes and return owned or borrowed line data explicitly.

- [ ] **tempfile-registry-map**: Re-evaluate tempfile registry maps.
  - Source TODO:
    - `gix-tempfile/src/lib.rs:75`: use a `gix-hashtable` slot-map once available.
  - Workflow impact: low for typical workflows, higher for checkout/merge paths that create many temporary files.
  - Plan:
    - Benchmark the existing mutexed map against a slot-map and the `hp-hashmap` feature.

- [ ] **git-date-token-parser**: Replace brute-force Git date parsing.
  - Source TODO:
    - `gix-date/src/parse/git.rs:11`: learn from Git's parser instead of generated brute-force parsing.
  - Workflow impact: low for normal operations, medium for log/rev parsing over many date filters.
  - Git comparison: Git's `date.c` uses a token scanner (`parse_date_basic`) with alpha, digit, and timezone matchers, then falls back to approxidate.
  - Plan:
    - Replace format-chain parsing with a token scanner for Git-specific formats.
    - Keep fixtures generated from Git for compatibility.

## Git-Inspired Candidates Without Current TODOs

- [ ] **sparse-index-parity**: Sparse-index parity for worktree-heavy commands.
  - Git comparison: Git's `sparse-index.c` can collapse full index ranges into sparse directory entries and expand on demand.
  - Candidate impact: very high for monorepos and sparse checkouts.
  - Next step: audit which `gix-status`, `gix-dir`, checkout, and merge code paths force full-index behavior.

- [ ] **commit-graph-bloom**: Commit-graph generation and Bloom-filter use in history walks.
  - Git comparison: Git's `revision.c` uses commit-graph generation numbers to bound traversal, and blame can initialize Bloom data.
  - Candidate impact: high for `log -- <path>`, merge-base, blame, and negotiation.
  - Next step: verify all revision walkers prefer generation numbers and changed-path filters when available.

- [ ] **midx-revindex-bitmaps**: Multi-pack-index reverse-index and bitmap-driven object enumeration.
  - Git comparison: Git's `midx.c`, `pack-revindex.c`, and `packfile.c` use MIDX lookup, reverse-index chunks, and prefixed-object iteration across MIDX before falling back to individual packs.
  - Candidate impact: high for fetch, clone, gc, prefix lookup, and object iteration in repos with many packs.
  - Next step: compare `gix-pack` and `gix-odb` MIDX support against Git's lookup order and reverse-index cache behavior.

- [ ] **pack-reuse-delta-policy**: Pack reuse and path-based delta compression policies.
  - Git comparison: Git's `builtin/pack-objects.c` uses pack reuse, bitmap reuse, delta islands, path-based regions, threaded delta search, and delta-cache limits.
  - Candidate impact: high for push, fetch serving, gc, and repack.
  - Next step: treat this as a separate pack-generation roadmap after read-side pack performance is measured.

## Measurement Plan

- Use Git's `GIT_TRACE_PERFORMANCE=true` and `GIT_TRACE2_PERF=1` baselines on the same repositories as `gix` measurements.
- Maintain at least these benchmark repositories: small everyday repo, `git/git`, Linux-sized history, a many-untracked-files worktree, a sparse checkout, and a many-pack repository.
- Track wall time, syscalls/stat count, object reads, pack index refreshes, peak RSS, and allocation counts.
- For each P0 item, require one benchmark that represents an interactive command and one regression test that protects the intended fast path.

## Immediate Next Moves

- [ ] **profile-status-worktrees**: Profile `gix status` on large case-sensitive and case-insensitive worktrees and compare against Git's name-hash/preload behavior.
- [ ] **profile-index-decode**: Profile index decode to identify why `path_backing` triggers `memmove`, then choose arena/chunked storage before expanding threaded decode.
- [ ] **implement-odb-wait-notify**: Replace the dynamic ODB pack-index load wait loop with wait/notify.
- [ ] **fetch-dup-counter**: Add a fetch duplicate-object diagnostic counter before changing negotiation or pack ingestion.
- [ ] **prototype-prefix-stack**: Prototype stack-only odd-prefix parsing as a low-risk starter change.

## Exit Criteria

- [ ] **status-scale-exit**: `status` and dirty checks scale with index size and untracked-file count within an agreed multiplier of Git on the benchmark corpus.
- [ ] **index-throughput-exit**: Index read throughput improves on large indexes without increasing peak memory disproportionately.
- [ ] **object-lookup-wait-exit**: Concurrent object lookup during fetch/clone no longer spins while another thread loads pack metadata.
- [ ] **pack-benchmark-coverage-exit**: Object lookup and pack delta reconstruction have benchmark coverage before cache/data-structure rewrites land.
- [ ] **todo-triage-exit**: The remaining performance TODOs are either implemented, benchmarked and rejected, or converted into narrower tracked tasks.
