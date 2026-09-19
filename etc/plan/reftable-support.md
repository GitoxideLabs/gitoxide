# Reftable support implementation plan

## Hard rule: gitoxide-native development

**The implementation MUST imitate gitoxide's entire established development
style and programming paradigms, not merely its formatting, naming, or commit
messages. This is a hard, unbendable acceptance requirement for every task.**

Neither an implementer nor an agent may waive this rule. Working code, passing
tests, performance gains, convenience, deadlines, and the experimental nature
of the work do not excuse a different programming or development style. Do not
build a foreign-looking implementation and promise to make it idiomatic later.

Before designing or implementing a task, identify the closest current code,
tests, and relevant commits or review decisions. Match their ownership and
allocation model, abstraction boundaries, APIs, data representations, error
handling, concurrency, module organization, testing approach, and organization
of changes. Examine actual call sites, not only type definitions.

Before accepting or committing the task, compare the result against those
precedents. If conformity is unclear or the proposed approach conflicts with
them, **stop and resolve an approach that satisfies the rule with the user and
Byron**. Do not silently invent a competing paradigm or grant an exception.

This requirement also governs every smaller task split from this plan and every
delegated assignment. Copy the mandatory style-gate reminder into the beginning
of each such task, together with its relevant precedents. Completing a parent
milestone does not exempt its remaining subtasks.

The repository contains different layers and historical practices. Imitating it
means following the applicable current conventions, not copying obsolete code,
known mistakes, or whichever example makes a preferred design easiest to defend.
There is no blanket rule against traits, owned state, allocations, or necessary
abstractions.

## Context and evidence

The user reports that Byron has green-lighted this experiment and collaboration
toward incremental upstream integration. Obtaining that initial agreement is
therefore not an outstanding task. Architectural and review-boundary decisions
remain collaborative; this plan does not authorize posting or pushing.

The initial analysis used gitoxide commit
`77c8cd956c08a2757318d3a0e6ef30d7fa71286e` and `~/git`'s `upstream/master` at
`d38352cd43ab9745686d697872408bc3249a153f`. Refresh nearby precedents as the
implementation progresses instead of treating this snapshot as immutable policy.
Use Git's [format specification][git-format], [reference backend][git-backend],
and selected unit/integration tests for storage semantics, not as templates for
Rust architecture or test-suite organization.

### Programming paradigms and concrete choices

1. **Separate storage mechanics, Git policy, and repository convenience.**
   `gix-commitgraph` separates [`File::at()` from validated construction][graph-init],
   and `gix-chunk` separates [planning from writing][chunk-write].
   [Moving the object-writing trait into `gix-object`][write-trait-move] explicitly
   avoided forcing plumbing users to depend on `gix-odb`.
   Reftable codecs must not discover repository configuration or environment
   state. Keep low-level primitives independently usable, with thin convenience
   layers rather than a new general-purpose storage framework.

2. **Make ownership and allocation deliberate.**
   [`CommitRef`, `Commit`, and iterator views][object-types] distinguish borrowed
   reads from owned mutation. [`Find::try_find()`][object-find] returns data backed
   by the caller's reusable buffer. At the convenience layer,
   [`Repository::find_object()`][repo-find] acquires a buffer and
   [attached objects recycle it on drop][attached-objects].
   Follow those distinctions for records, blocks, and snapshots. Do not eagerly
   own and decode every record merely to simplify lifetimes, or forbid owned
   state where a write plan or persistent snapshot actually needs it.

3. **Keep mutable state explicit before adding shared mutation.**
   The [development guide][state-guide] puts buffers and caches in plumbing
   arguments first, adding interior mutability when the borrowing or sharing
   requirements justify it. Use `gix_features::threading` and `OwnShared` where
   appropriate rather than fixing a new layer to `Arc<Mutex<_>>`.
   Preserve both parallel and non-parallel configurations. Model snapshot
   lifetimes and reload boundaries before choosing the store's internal shape.

4. **Prefer direct use of a suitable primitive over artificial intermediate data.**
   [Adding `tree::name_order()`][name-order-addition] was followed by
   [removing a synthetic entry and null object ID from binary search][bisect-simplification].
   The latter changed one file by +1/-13 lines. This is concrete evidence for
   reusing a clear domain operation, not evidence for a universal preference for
   iterator chains over loops. The [tree lookup performance note][tree-lookup]
   explicitly defers a vector/binary-search alternative until benchmarking can
   establish the tradeoff. Apply the same discipline to indexes, caches, merging,
   and compaction; do not optimize by intuition.

5. **Review call-site ergonomics as part of API design.**
   [`RefEdit` constructors][ref-edit] delegate common cases to fuller variants
   while leaving the data accessible. Their [adoption commit][ref-edit-adoption]
   removed repetitive defaults throughout the callers, rather than stopping at
   a plausible-looking API declaration.
   Use established `at`, `from_bytes`, `try_find`, conversion, and builder
   conventions where their semantics fit. Use `Options` for defaultable
   behavioral choices and `Context` for required operation data, following the
   [documented distinction][options-guide].
   Cheap platforms borrow the repository; stateful, reusable caches have the
   ownership their lifetime needs. Keep types and public entry points easy to
   discover, with implementation modules organized like the neighboring crate
   rather than imposing a new file-per-abstraction template.
   Keep simple cases simple; do not introduce getter or builder layers solely
   to hide ordinary plumbing data.

6. **Choose abstractions for an actual dependency or capability boundary.**
   The object [read][object-find] and [write][object-write] traits have real
   interchangeable providers and avoid dependency cycles. Conversely, existing
   ref-store scaffolding and the unmerged reftable proposals do not establish an
   approved public plugin interface.
   Byron plans to implement the [in-memory reference layer][memory-layer] in
   `gix-ref` himself, allowing transactions before flushing. Treat that work as
   an external prerequisite for integration, not an implementation task for us.
   Follow his actual API and concrete caller, lifetime, and layering requirements;
   do not preempt it with a new public trait, handle, or dispatch framework.

7. **Preserve byte-oriented data and domain semantics.**
   Reuse `gix_hash` IDs and hash kinds, byte strings, validated reference names,
   actors, and path utilities rather than adding string-only equivalents.
   [Reference equality tests][ref-equality] exercise natural counterparts and
   non-UTF-8 names directly. The [one-way heterogeneous comparisons][ref-comparison]
   deliberately preserve equality laws; convenience must not weaken invariants.
   Compare domain values naturally in tests instead of formatting and reparsing
   them just to make an assertion.

8. **Use the owning layer's current error conventions.**
   [`gix-error` documents][error-guide] the difference between a simple local
   error and a contextual error with a callee cause. Reuse its semantic error
   types where they fit, and preserve causes and the standard-error bridge.
   New plumbing uses this current model; existing modules still using
   `thiserror` are not a reason to bundle an unrelated migration.
   Invalid input, corruption, I/O failure, and unsupported behavior must not turn
   into success-shaped defaults or panics. Historical code using `unwrap()` is
   not an exception to the current safety guidance.

9. **Reuse operational context and synchronization rather than rebuilding it.**
   Pack verification has [explicit progress, interruption, cache, and options
   inputs][pack-verify]. A [review of repository-aware configuration][context-review]
   caught a convenience wrapper substituting process CWD for the repository's
   opening directory; Byron [acknowledged the correction][context-response].
   Reftable configuration, paths, and common/private worktree routing must
   preserve their established repository context. The [reference-locking
   correction][locking-correction] also demonstrates that interoperability with
   concurrent Git writers outweighs an attractive but incorrect optimization.

10. **Use small, purposeful tests and the existing isolation infrastructure.**
    The [development guide][test-guide] requires practical test-first work,
    selective use of Git's cases and fixtures, fuzz regressions, and benchmarks.
    The [commit-message boundary tests][message-tests] are a useful example of a
    short table of meaningful byte and newline cases with invariant descriptions.
    Use [cached, archived, version-aware fixtures][fixture-tools] and
    [isolated Git commands][git-isolation]; never use the source checkout as a
    test repository or mutate shared read-only fixtures.
    In [the environment-test review][isolation-review], Byron preferred process
    isolation because serialization was harder to reason about.
    This is not a ban on serialization: [the shared helper's own review][isolation-lock]
    requires synchronization while capturing the environment and spawning, not
    while waiting for the child. Reuse that helper instead of writing another
    launcher. A [standalone integration test replacing self-execution][standalone-test]
    is another useful, bounded precedent.

### Commit messages and development workflow

Follow [purposeful conventional commits][commit-guide], not generic conventional
commit boilerplate:

- Use changelog prefixes for user-visible features/fixes, with the appropriate
  crate scope and `!` for breaking changes. Do not add `chore:` or `refactor:` to
  ordinary internal work.
- Use Markdown and backticks for code identifiers, crate names, and commands.
  Bodies explain context, intent, and justification, not an inventory of the
  diff. Keep important non-obvious correctness or tradeoff explanations.
- Test the relevant behavior first, implement the smallest coherent change, and
  inspect its callers and documentation before declaring the task complete.
  Tests being green does not establish that the API is good.
- Keep every committed tree independently buildable and passing. Keep a breaking
  API change and required workspace adaptations together. There are
  [historical standalone failing-test commits][historical-red-test], but the
  [current self-containment rule][self-containment] takes precedence: the red
  phase happens locally, and tests and implementation land together when needed.
- Preserve fine-grained, topic-oriented development and review history. The
  [guide endorses Stacked Git][history-guide]; this does not require a particular
  number of commits or simultaneously open stacked PRs.

Recent history includes explicitly agent-assisted messages, and older history
contains terse review subjects and now-obsolete practices. Neither should be
copied mechanically. Specific code changes, explicit review decisions, and the
current written conventions are stronger evidence than authorship metadata or
the absence of a reviewer objection.

### What actual PR sizes and review sequences show

Sample: the twelve most recently merged eligible PRs with GitHub author `Byron`
as of 2026-09-18. Complete searches covered September 5-18 and returned fifteen
candidates; instruction/QA-only #2966 and #2988 were excluded, then the twelve
newest remaining feature/fix PRs were selected. Developer tooling and API
migrations containing features/fixes were retained. The selected merge dates
span September 6-16; no release/dependency-only candidates occurred.

| PR | Review topic | Files | Added | Deleted | Commits |
| --- | --- | ---: | ---: | ---: | ---: |
| [#2999](https://github.com/GitoxideLabs/gitoxide/pull/2999) | Configurable SBOM generation | 7 | 991 | 25 | 2 |
| [#3002](https://github.com/GitoxideLabs/gitoxide/pull/3002) | Reject ambiguous numeric dates | 4 | 29 | 13 | 1 |
| [#2989](https://github.com/GitoxideLabs/gitoxide/pull/2989) | Error-conversion review | 149 | 2094 | 1247 | 13 |
| [#2996](https://github.com/GitoxideLabs/gitoxide/pull/2996) | HTTP authentication challenges | 15 | 455 | 18 | 3 |
| [#2990](https://github.com/GitoxideLabs/gitoxide/pull/2990) | Several improvements, including test isolation | 49 | 1193 | 300 | 4 |
| [#2992](https://github.com/GitoxideLabs/gitoxide/pull/2992) | Bodyless commit-title newline | 4 | 41 | 8 | 1 |
| [#2983](https://github.com/GitoxideLabs/gitoxide/pull/2983) | URL path access | 4 | 494 | 3 | 1 |
| [#2979](https://github.com/GitoxideLabs/gitoxide/pull/2979) | Missing `objects/info` | 3 | 53 | 2 | 1 |
| [#2975](https://github.com/GitoxideLabs/gitoxide/pull/2975) | Repository-aware config path | 4 | 157 | 0 | 1 |
| [#2974](https://github.com/GitoxideLabs/gitoxide/pull/2974) | Standalone config path | 3 | 97 | 24 | 1 |
| [#2971](https://github.com/GitoxideLabs/gitoxide/pull/2971) | Null-ID, configuration, and branch changes | 17 | 920 | 133 | 5 |
| [#2970](https://github.com/GitoxideLabs/gitoxide/pull/2970) | Unsupported-reftable diagnostic | 3 | 25 | 5 | 1 |

The medians are **4 files, 1 commit, and 315 changed lines** (additions plus
deletions); ranges are **3-149 files, 1-13 commits, and 30-3341 changed lines**.
These describe this short sample, not a universal style rule or size budget.
GitHub authorship does not establish that Byron personally wrote every line.
For example, #2983 contains 338 added test lines; #3002 includes a binary fixture
archive reported as +0/-0. Separate implementation, tests, fixture scripts,
binary/generated artifacts, and mechanical caller adaptations when sizing work.

There is direct evidence for incremental boundaries:

- [#2996's three commits][challenge-commits] proceed through credential context,
  transport retention, and protocol forwarding: one behavior across ordered
  layers, not arbitrary equal-sized patches.
- #2974 provides a standalone operation; #2975 adds repository-aware convenience.
- [#2944][review-followup] explicitly follows #2942 and takes a limited review
  slice. [#2989][error-followup] follows #2944 and calls for examining a few
  conversion commits before accelerating further review.
- [The `RefEdit` constructor adoption][ref-edit-adoption] touched 25 files
  (+696/-1232). Required coherent caller adaptation can be larger than a typical
  small fix without constituting an unrelated second feature.

The sample also contains broader multi-concern PRs. It does not prove that Byron
always requires one concern per PR, a common stacked-PR size, or a fixed stack
depth. The [approximately 500-SLOC contribution threshold][discussion-threshold]
is a discussion requirement, **not a PR-size cap**.

For this experiment, propose the smallest independently understandable review
question, keep each commit single-purpose, and split separable behavior.
Document dependencies between increments. A task below is a milestone, **not
permission to make one milestone-sized commit or PR**. Do not split required
breaking-change adaptations merely to hit a line target.

## Scope, prior work, and architecture

Complete the [published `gix-reftable` roadmap][roadmap]: native stack reads and
writes, transactions, reflogs, compaction, table management, backend selection,
and explicit migration in both directions. Read-only operation is an intermediate
milestone, not completion.

Keep [discussion #2798][reftable-discussion] and the unmerged proposals in view:
[#2446][traits-proposal] tried ref-store traits but retained file-specific types;
[#2452][port-proposal] contains standalone code and a `PLAN.md` that explicitly
leaves repository integration unfinished; [#2965][integrated-proposal] provides
a broad integration and test checklist, not an accepted baseline to import
wholesale. Their heads are available locally under
`origin/reftable-research/{2446,2452,2965}`. Select useful behavioral evidence
without treating proposal checkmarks or reported test results as our validation.

The intended dependency direction is `gix -> gix-ref -> gix-reftable`.
`gix-reftable` owns format/table/stack mechanics; `gix-ref` owns reference policy;
`gix` owns repository configuration and orchestration. Reuse existing hashing,
actors, compression, checksums, paths, locking, temporary files, and errors.
Preserve the explicit files-store API and existing files-backed behavior.
Do not add a C binding, production Git-subprocess backend, shadow loose-ref
database, automatic conversion, or unrelated plugin framework.

Complete independent native storage in Tasks 2-5 before beginning integration
in `gix-ref` or `gix`. Encoding symbolic-reference records does not mean resolving
them; reference availability, namespaces, `RefEdit`, and `HEAD` reflog synthesis
remain higher-level policy.

Byron owns the planned in-memory references and transactions-before-flushing
implementation in `gix-ref`. Our work is the native storage layer and its later
adapter, not a competing implementation of his reference layer. Native storage
can proceed independently, but integration requires his suitable API to be
available and its staging, visibility, rollback, conflict, and flush contract
to be agreed.

The exact facade, record lifetimes, and snapshot ownership must pass the hard
rule against that API, the completed storage layer, and real callers. Replacing
`gix::RefStore` affects public types, iterator lifetimes, peeling, reflogs,
transactions, errors, and configuration; it is not a one-line alias change.

## Branch and progress workflow

`reftables` is the effort mainline: it holds this plan and accumulates accepted
code through merge commits. Code-only topics such as `reftables-task1` are based
on the appropriate upstream code history, without plan-only commits or plan
updates in their PRs.

Merge a reviewed code topic into `reftables` first, using a merge commit. Only
then record its completed work in a separate plan-update commit on `reftables`.
Do not mark a task complete on the mainline merely because its topic exists.

When working on a code-only topic, consult the mainline plan with
`git show reftables:etc/plan/reftable-support.md`. Its mandatory style rule still
applies; do not copy the plan into the code changes to make it available.

## Implementation tasks

Each task starts with the mandatory reminder. Any extracted task or delegated
assignment must start with it too.

### Task 1: Git-generated fixtures and reference oracles

> **Mandatory style gate:** Apply the [hard rule](#hard-rule-gitoxide-native-development)
> before and throughout this task. Imitate gitoxide's programming paradigms and
> development process, not only formatting. No implementation or commit passes
> without satisfying it.

Establish `gix-reftable` as the workspace crate containing these fixtures and
Git-oracle checks. It must not depend on `gix-ref`, even for tests: `gix-ref`
will eventually depend on this crate. Use `gix-testtools` without its
repository-snapshot or worktree-exclusion features to preserve that boundary.

Use existing `gix-ref` tests, the [focused byte-boundary tests][message-tests],
and `gix-testtools` as behavioral precedents, not dependencies on a reference
policy implementation. Select relevant cases from Git's reftable unit tests,
`t0610`, `t0614`, `t1460`, and mixed-format submodule tests; do not translate
their shell harness or every case.

Establish small Git-generated fixtures and expectations for reference records,
symbolic targets, peeled tags, namespaced records, common/private worktree
storage, and reflog records. Use both hash formats, disposable writable copies,
stable archived bytes where needed, and the isolated Git helpers. Verify that
Git reads the archived data after relocation. This is groundwork for native
format tests, not a claim that a reader or writer is implemented.

Keep `gix-ref` policy-API contracts, including `PreviousValue`, namespace
transparency, and dereferenced `HEAD` edits, for Task 6. Those contracts must
not pull a higher-level reference-store API into the format crate.

Review cuts: one fixture or coherent oracle family at a time. Do not construct
a generalized test framework or a production Git-subprocess backend.

### Task 2: Single-table codec and reader

> **Mandatory style gate:** Apply the [hard rule](#hard-rule-gitoxide-native-development)
> before and throughout this task. Imitate gitoxide's programming paradigms and
> development process, not only formatting. No implementation or commit passes
> without satisfying it.

Use [borrowed object views][object-types], [validated file construction][graph-init],
and [low-level compression with thin adapters][compression] as precedents.
Implement the native `gix-reftable` crate using the workspace's conventions and
directory-backed `mod.rs` modules. Keep format mechanics independent of
repository policy, with explicit byte/buffer lifetimes and contextual failures.

Start with tests for headers, footers/CRC, hash declarations, varints, records,
restart points, aligned/unaligned blocks, indexes, and compressed logs. Cover
version 1 SHA-1, version 2 hash IDs, peeled and symbolic references, tombstones,
empty/log-only tables, and optional sections. Malformed input, truncation,
overflow, and both per-block and aggregate buffering bounds belong here, as
does fuzzing from the first parser, not only at the end.

Review cuts: envelope/validation, reference blocks, log blocks, and indexed seek
are separate behavioral increments. Git-written files must be readable before
our writer can supply self-confirming fixtures.

### Task 3: Stack reads, merged iteration, and snapshots

> **Mandatory style gate:** Apply the [hard rule](#hard-rule-gitoxide-native-development)
> before and throughout this task. Imitate gitoxide's programming paradigms and
> development process, not only formatting. No implementation or commit passes
> without satisfying it.

Use existing explicit state/buffer ownership and borrowed file views as
precedents; do not start with a monolithic shared mutable repository object.
Test merged records against a small in-memory model before adding filesystem
reload behavior. This model is only a test oracle, not an implementation of
Byron's in-memory reference layer.

Cover newest-record precedence, tombstone masking, reference lookup, prefix/full
iteration, log ordering, repeated seeks, and hash consistency. Establish coherent
iterator snapshots and refresh boundaries for later operations. Test the race
where compaction removes a table between manifest reading and opening it.
Do not eagerly decode the whole stack to sidestep lifetime or buffering design.

Review cuts: pure merging, manifest/opening, then snapshot refresh and races.
Owned state is justified by its lifecycle, not forbidden or introduced by habit.

### Task 4: Native table writing and stack transactions

> **Mandatory style gate:** Apply the [hard rule](#hard-rule-gitoxide-native-development)
> before and throughout this task. Imitate gitoxide's programming paradigms and
> development process, not only formatting. No implementation or commit passes
> without satisfying it.

Use [planned streaming writes][chunk-write], existing lock/tempfile facilities,
and the native record/stack APIs as precedents. Avoid a second generic I/O or
transaction framework. Keep this work in `gix-reftable`, independent of `gix-ref`.
Test the writer against Git, not only our reader.

Start with sorted record/block output, hash formats, indexes, compressed logs,
update indices, and byte/semantic interoperability. Then test `tables.list.lock`,
reload of the latest stack while locked, native record batches, complete-table
creation, and atomic manifest publication with appropriate durability ordering.
Expose the primitives an adapter needs to validate pending changes against the
locked current state at flush, without embedding `PreviousValue` or repository
policy in the codec or stack.

Publication is atomic per stack, not across multiple stacks. Test rollback
before publication, contention, and injected failures as each operation is
introduced, including interaction with actual Git writers.

Review cuts: table writer, append publication, then stack transaction guarantees.

### Task 5: Compaction and table management

> **Mandatory style gate:** Apply the [hard rule](#hard-rule-gitoxide-native-development)
> before and throughout this task. Imitate gitoxide's programming paradigms and
> development process, not only formatting. No implementation or commit passes
> without satisfying it.

Reuse stack merging and writer primitives, with explicit progress/interruption
and options where appropriate. Do not copy Git's C state organization or invent
a general storage engine. Keep pure selection/merging separate from publication.

Test full and partial compaction, geometric selection, bounded stack growth,
reflog retention/expiry, and stale-table cleanup. Partial compaction must retain
tombstones that hide lower-table data; deleted refs and expired logs must not
reappear. Exercise active readers, concurrent writers, lock/fsync failures, and
Windows file-deletion behavior. Drive retention and expiration mechanics with
caller-supplied policy and resolved `Options`, not configuration or environment
lookup in the storage crate.

Review cuts: compaction correctness, concurrent publication/cleanup, then
caller-driven retention and maintenance operations. Benchmark before tuning;
performance cannot substitute for correct publication or compatible locking.

### Task 6: Integration with Byron's in-memory `gix-ref`

> **Mandatory style gate:** Apply the [hard rule](#hard-rule-gitoxide-native-development)
> before and throughout this task. Imitate gitoxide's programming paradigms and
> development process, not only formatting. No implementation or commit passes
> without satisfying it.

**Prerequisites:** Tasks 2-5 are complete, and Byron's suitable in-memory
reference API is available and agreed. He owns that implementation; our scope
is its native-storage adapter and integration with real callers. Do not
implement his layer or preempt its public traits, handles, or transaction model.

Establish the policy-API contracts deferred from Task 1 here, in `gix-ref`.
Test the files baseline and adapter first: staging in-memory transactions leaves
on-disk refs and reflogs untouched until explicit flush. Visibility, rollback,
and conflicts follow the agreed interface. Flushing integrates native locking,
reload, revalidation of pending changes, and publication, surfacing conflicts.
Preserve files behavior; atomicity remains per stack, not repository-wide.

Cover lookup, symbolic references and peeling, namespace/worktree routing,
`PreviousValue`, `HEAD` reflog synthesis, and directory/file name conflicts in
`gix-ref`, not the format crate. The table format permits prefix-conflicting
names; reference policy determines which edits are allowed.

Resolve `extensions.refStorage` alongside object format in early repository
configuration, before reference-dependent conditional includes. Reject invalid
or unsupported formats instead of guessing from directories or falling back
to files. Actual `HEAD` is table-backed; `FETCH_HEAD` and `MERGE_HEAD` remain
special files, without making every root reference a special-file exception.
Keep applicable `reftable.*` configuration lookup in the integrating layers and
pass resolved options to storage.

Review cuts: contract against the actual in-memory interface, persistence
adapter, then repository-facing reads, writes, reflogs, and revision parsing.
Keep public API changes and their required caller adaptations together. Change
high-level unsupported-reference expectations when support arrives, while
retaining direct files-backend placeholder-decoding tests.

### Task 7: Repository creation, clone, and writable callers

> **Mandatory style gate:** Apply the [hard rule](#hard-rule-gitoxide-native-development)
> before and throughout this task. Imitate gitoxide's programming paradigms and
> development process, not only formatting. No implementation or commit passes
> without satisfying it.

Follow the existing `gix` creation, early configuration, clone, and ref-edit
layers. Keep convenience behavior tied to the repository's captured context,
not ambient process state. Inspect real API and CLI callers together.

Test explicit storage selection, `init.defaultRefFormat`,
`GIT_DEFAULT_REF_FORMAT`, and option precedence while preserving the files
default when no alternative is selected. Exercise commit, branch/tag updates,
fetch, bare and linked-worktree repositories, namespaces, and mixed-format
submodules through the neutral facade.

During cloning, defer hash-dependent table initialization until the remote hash
is known. Preserve existing configuration and reject unsafe retargeting rather
than discard refs, pseudo-refs, or reflog-only state. These cases include
[the clone-state scenario raised on #2965][clone-review]; the report is a test
target, not a reproduced finding against this checkout. Storage format is a
local choice, not a new wire-protocol capability.

Review cuts: creation/selection, safe clone initialization, then remaining
writable integration and CLI journeys. Keep tests and required adaptations with
each behavior rather than accumulating a final integration patch.

### Task 8: Explicit migration in both directions

> **Mandatory style gate:** Apply the [hard rule](#hard-rule-gitoxide-native-development)
> before and throughout this task. Imitate gitoxide's programming paradigms and
> development process, not only formatting. No implementation or commit passes
> without satisfying it.

Reuse the established readers, writers, ref policies, and configuration-update
facilities. Keep migration orchestration separate from table codecs; do not
introduce an automatic conversion fallback during ordinary opening.

Start with preservation assertions for references, dangling symbolic references,
reflog-only/empty logs, identities, timestamps, messages, and special raw files.
Provide explicit, quiescent files-to-reftable and reftable-to-files migration.
Build and verify the destination before configuration/layout cutover and retain
a recoverable original through completion. Test failures before and during
cutover without claiming multi-file atomicity that is not provided.

Initially reject migration of repositories with linked worktrees,
[matching the inspected Git revision][migration-worktrees]. Make this restriction
explicit rather than silently losing private refs or logs.

Review cuts: lossless export/import, each migration direction, then API/CLI
orchestration with recovery and interoperability coverage.

## Acceptance throughout, not a final testing phase

The hard style rule is checked before any task can be accepted, even when all
functional checks pass. Each delivered increment also needs its targeted tests,
both hash formats and applicable feature/threading configurations, relevant
documentation, and unchanged existing files-backend behavior.

Require Git-created data to work in gitoxide and gitoxide-created data to work
in Git, including alternating writers and selected CLI journeys. Preserve fuzz
regressions, exercise resource bounds, and benchmark lookup, iteration, writes,
and compaction before performance changes. Use the existing repository tooling;
do not add another QA framework for this feature.

Update roadmap checkboxes only for delivered behavior, after merging the code
topic into the effort mainline. Complete independent native storage in
Tasks 2-5 before integration in Task 6; Byron's in-memory reference API is an
additional prerequisite for that integration. Repository creation and migration
follow in Tasks 7-8. Completion requires the full scope, not just read support.

[graph-init]: ../../gix-commitgraph/src/file/init.rs#L16-L143
[chunk-write]: ../../gix-chunk/src/file/write.rs#L73-L127
[write-trait-move]: https://github.com/GitoxideLabs/gitoxide/commit/7325c584c3f0973975935b307630e29bc36ef5c7
[object-types]: ../../gix-object/src/lib.rs#L96-L173
[object-find]: ../../gix-object/src/traits/find.rs#L17-L35
[object-write]: ../../gix-object/src/traits/mod.rs#L5-L68
[repo-find]: ../../gix/src/repository/object.rs#L52-L65
[attached-objects]: ../../gix/src/types.rs#L34-L50
[state-guide]: ../../DEVELOPMENT.md#L206-L218
[name-order-addition]: https://github.com/GitoxideLabs/gitoxide/commit/c6cf668e9c25f8a73cbb0318a5dd56fa5952f825
[bisect-simplification]: https://github.com/GitoxideLabs/gitoxide/commit/579544e14e0dde22733c58624c7454c50aa1df75
[tree-lookup]: ../../gix-object/src/tree/ref_iter.rs#L63-L69
[ref-edit]: ../../gix-ref/src/transaction/mod.rs#L125-L187
[ref-edit-adoption]: https://github.com/GitoxideLabs/gitoxide/commit/f5b3feb54438b4361b77403ea788a606ecd96b40
[options-guide]: ../../DEVELOPMENT.md#L241-L255
[memory-layer]: https://github.com/GitoxideLabs/gitoxide/discussions/1281#discussioncomment-15336286
[ref-equality]: ../../gix-ref/tests/refs/equality.rs#L87-L132
[ref-comparison]: ../../gix-ref/src/compare.rs#L122-L126
[error-guide]: ../../gix-error/src/lib.rs#L3-L127
[pack-verify]: ../../gix-pack/src/index/verify.rs#L33-L145
[context-review]: https://github.com/GitoxideLabs/gitoxide/pull/2975#discussion_r3949349346
[context-response]: https://github.com/GitoxideLabs/gitoxide/pull/2975#discussion_r3949594556
[locking-correction]: https://github.com/GitoxideLabs/gitoxide/commit/cde8272517fe1480ad283fbbbe1cea0d488e75e9
[test-guide]: ../../DEVELOPMENT.md#L15-L34
[message-tests]: ../../gix-object/tests/object/commit/message.rs#L40-L94
[fixture-tools]: ../../tests/tools/src/lib.rs#L752-L829
[git-isolation]: ../../tests/tools/src/lib.rs#L2112-L2160
[isolation-review]: https://github.com/GitoxideLabs/gitoxide/pull/2996#discussion_r4012537722
[isolation-lock]: https://github.com/GitoxideLabs/gitoxide/pull/2990#discussion_r4002155735
[standalone-test]: https://github.com/GitoxideLabs/gitoxide/commit/1eced45c2a9a00f363350bec6714ef22899f5639
[commit-guide]: ../../DEVELOPMENT.md#L40-L77
[historical-red-test]: https://github.com/GitoxideLabs/gitoxide/commit/743b9796e3c9e9a6ebc83524620475f4e5f1a91f
[self-containment]: ../../DEVELOPMENT.md#L80-L85
[history-guide]: ../../DEVELOPMENT.md#L86-L98
[challenge-commits]: https://github.com/GitoxideLabs/gitoxide/pull/2996/commits
[review-followup]: https://github.com/GitoxideLabs/gitoxide/pull/2944
[error-followup]: https://github.com/GitoxideLabs/gitoxide/pull/2989
[discussion-threshold]: ../../CONTRIBUTING.md#L6-L7
[roadmap]: ../../crate-status.md#L1045-L1052
[reftable-discussion]: https://github.com/GitoxideLabs/gitoxide/discussions/2798
[traits-proposal]: https://github.com/GitoxideLabs/gitoxide/pull/2446
[port-proposal]: https://github.com/GitoxideLabs/gitoxide/pull/2452
[integrated-proposal]: https://github.com/GitoxideLabs/gitoxide/pull/2965
[compression]: ../../gix-zlib/src/stream/deflate.rs#L10-L218
[clone-review]: https://github.com/GitoxideLabs/gitoxide/pull/2965#discussion_r3920582328
[git-format]: https://github.com/git/git/blob/d38352cd43ab9745686d697872408bc3249a153f/Documentation/technical/reftable.adoc
[git-backend]: https://github.com/git/git/blob/d38352cd43ab9745686d697872408bc3249a153f/refs/reftable-backend.c
[migration-worktrees]: https://github.com/git/git/blob/d38352cd43ab9745686d697872408bc3249a153f/t/t1460-refs-migrate.sh#L117-L124
