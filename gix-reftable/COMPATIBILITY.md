# Reftable compatibility evidence

This matrix records the executable evidence behind Gitoxide's native reftable
support. The behavior groups are derived from Git's `t0610` through `t0614`
reftable suites, but the tests and fixtures listed here are independently
expressed against the published format specification.

## Format exchange

| Format | Git writes, Gitoxide reads | Gitoxide writes, Git reads | Native round trip |
|---|---|---|---|
| version 1, SHA-1 | `reads_tables_written_by_git` | `git_reads_tables_written_by_this_crate` | `version_one_sha1_roundtrip` |
| version 2, SHA-1 | Git does not normally select this combination | Git does not normally select this combination | `version_two_sha1_roundtrip` |
| version 2, SHA-256 | `reads_tables_written_by_git` | `git_reads_tables_written_by_this_crate` | `version_two_sha256_roundtrip` |

The cross-implementation tests are in `tests/git_interop.rs`; native codec
round trips are in `tests/roundtrip.rs`. The Git-generated SHA-1 case uses
enough refs to exercise multi-block indexes and object lookup. Git 2.46 or
newer is required for fixture migration through
`git refs migrate --ref-format=reftable`; otherwise those cross-process cases
are skipped. Direct initialization through `--ref-format=reftable` requires Git
2.45 or newer.

## Repository and semantic matrix

| Behavior | Git to Gitoxide | Gitoxide to Git | Principal evidence |
|---|---|---|---|
| ordinary repository | open, read, update | init, read, update | `gix/tests/gix/repository/open.rs::non_bare_reftable`; `gix/tests/gix/init.rs::reftable` |
| bare repository | open, read, and update | SHA-1/SHA-256 init, clone, read, and update | `git_and_gix_created_bare_repositories_are_bidirectionally_writable`; `opens_git_created_bare_and_linked_worktree_repositories`; `bare_and_non_bare_*_repositories_interoperate_with_git` |
| linked worktree | read and route common/per-worktree refs | update the correct stack | `git_and_the_adapter_agree_on_linked_worktree_routing`; `linked_and_explicit_other_worktree_names_route_to_their_own_stacks` |
| unborn or symbolic `HEAD` | preserve symbolic target | direct init and clone without shadow refs | `empty_remote_keeps_an_authoritative_reftable_symbolic_head`; `reftable_clone_and_early_persist_is_a_valid_unborn_repository` |
| direct, symbolic, peeled, and deleted refs | decode and update | encode and update | `git_and_the_adapter_consume_each_others_transactions`; `compact_write_strategy_records_a_fully_peeled_tag`; backend contract suite |
| reflog creation, update, empty marker, and tombstone | decode Git history and tombstones | Git reads adapter and stack writes | `reads_git_log_tombstones_with_historical_keys`; `log_only_deletion_preserves_the_ref_and_deletes_empty_markers`; `git_reads_a_stack_updated_and_compacted_by_this_crate` |
| atomic batches, compare-and-swap, and refname conflicts | validate under the authoritative lock | publish one transaction table | `exercise_backend_contract`; `one_authoritative_stack_lock_covers_all_transaction_edits`; `atomically_replaces_a_parent_reference_with_its_child` |
| SHA-256 clone negotiation | read remote format before ref publication | Git reads resulting repository | `reftable_clone_adopts_remote_sha256_before_writing_fetched_refs` |

The shared backend contract is `gix-ref/tests/refs/store.rs`; it runs the same
logical assertions against files and reftable storage.

## Robustness and maintenance matrix

| Invariant | Principal evidence |
|---|---|
| concurrent readers see a complete old or new generation | `concurrent_readers_observe_complete_update_and_compaction_generations` |
| concurrent writers serialize on `tables.list` | `the_authoritative_list_lock_excludes_another_writer` |
| staged crash points leave a complete old or new generation readable | `gix_reftable::stack::tests::every_publication_failure_point_leaves_a_complete_generation` |
| clone object-format handoff never exposes refs under a mismatched hash configuration | `gix::clone::fetch::util::reftable_handoff_tests::every_handoff_stage_is_hash_compatible` |
| compaction revalidates races, retains data, and expires reflogs | stack compaction tests, including `compaction_revalidates_and_retains_an_append_that_wins_the_unlocked_race` and `reflog_expiry_rewrites_a_single_member_stack` |
| malformed lists, missing tables, corrupt blocks, and resource-limit violations fail closed | `malformed_missing_and_out_of_order_lists_fail_closed`; `rejects_corruption_truncation_and_limits`; mutation property tests; `table` and `stack_list` fuzz targets |
| verification checks hidden records, refname conflicts, placement, and every linked-worktree stack | `maintenance_verifies_and_optimizes_every_worktree_stack`; `verification_rejects_an_invalid_symbolic_target`; `verification_rejects_directory_file_conflicts_in_prebuilt_tables`; `misplaced_shared_records_do_not_shadow_the_common_stack_and_fail_verification` |
| abandoned complete tables are cleaned without touching reachable members | `cleanup_removes_only_identifiable_staged_and_safe_unlisted_tables` |
| Windows-style sharing violations defer obsolete-member deletion without blocking other cleanup | `sharing_violations_defer_cleanup_without_stopping_other_deletions` |

## Canonical Git and independent implementations

Git at `f78ce2f7b6df702f93d40b85d6bda92a3f65da79` (`2.55.GIT`) is the canonical
implementation used for compatibility testing. Its 90 Make/Clar and 104 Meson
reftable unit tests passed.
The focused `t0610` through `t0614` integration suites passed all 111 tests in
both SHA-1 and SHA-256 configurations. Git's HTTP and JGit-specific harnesses
do not define additional on-disk semantics; their behavior families inform the
repository exchange and multi-level-index cases above.

The following independently maintained implementations passed their relevant
native suites before exchanging tables with Gitoxide:

| Implementation | Revision | Exercised compatibility |
|---|---|---|
| JGit | `db8cba30cdb029ff8ef4f52824f4782e3c4077f7` | bidirectional V1/SHA-1 tables, random reference seeks, and object-ID lookup |
| Sley | `41a1c8373c6f3eb91d295694dd5d5dcbb7f48a59` | bidirectional indexed SHA-1 and SHA-256 tables |
| Dulwich | `02540a8a9c12a2776cf507993b180bbc595e003b` | bidirectional V1/SHA-1 tables within its supported subset |
| `hanwen/reftable` | `ca20b64f41b87a559a0ad0b8478484b827278704` | bidirectional SHA-1 and SHA-256 tables, exact lookup, object lookup, and log lookup |

These implementations provide additional cross-checks, not normative sources.
Compatibility is claimed only for their shared valid-format subsets, with the
published format specification and Git's behavior taking precedence when an
independent implementation emits invalid edge cases.

## Reproducing the evidence

```bash
GIX_TEST_IGNORE_ARCHIVES=1 cargo test -p gix-reftable --all-features
GIX_TEST_IGNORE_ARCHIVES=1 cargo test -p gix-ref --all-features
GIX_TEST_IGNORE_ARCHIVES=1 cargo test -p gix --features 'sha256,worktree-mutation,blocking-network-client' --test gix reftable
cargo check --manifest-path gix-reftable/fuzz/Cargo.toml --bins
cargo bench -p gix-reftable --bench operations --all-features -- --noplot
```

On 2026-09-01, nightly libFuzzer campaigns with AddressSanitizer completed
406,619 structured writer/reader round trips, 11,047,415 raw-table parser runs,
and 162,177 `tables.list` parser runs without a crash or artifact. The raw-table
corpus was seeded with Git-produced SHA-1/v1 and SHA-256/v2 tables; the stack
corpus included the 43 existing valid, malformed, and path-safety cases. The
structured target varies hash modes, versions, block shapes, alignment, restart
intervals, index inclusion, reference value types, and reflog operations, then
requires the reader to reproduce every written record and exact lookup. The
mutation property test provides an additional deterministic pass over
corruptions of valid encoded tables.

Performance baselines cover exact lookup, indexed prefix and full iteration,
writes, and eight-generation compaction at 100 and 10,000 references. The
recorded environment, measurements, and benchmark boundaries are in
[PERFORMANCE.md](PERFORMANCE.md).
