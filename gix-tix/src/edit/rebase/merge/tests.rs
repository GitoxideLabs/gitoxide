use std::path::Path;

use anyhow::{Context, Result};
use gix::ObjectId;

use crate::edit::{rebase, time_travel};

fn git(path: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = gix_testtools::git_command(path).args(args).output()?;
    anyhow::ensure!(
        output.status.success(),
        "git {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

fn branch(repo: &gix::Repository, name: &str) -> Result<ObjectId> {
    Ok(repo.find_reference(&format!("refs/heads/{name}"))?.peel_to_commit()?.id)
}

fn changed_tree(repo: &gix::Repository, commit_id: ObjectId, path: &str, content: &str) -> Result<ObjectId> {
    let mut commit = repo.find_commit(commit_id)?.decode()?.into_owned()?;
    let mut tree = repo.find_tree(commit.tree)?.edit()?;
    tree.upsert(path, gix::objs::tree::EntryKind::Blob, repo.write_blob(content)?)?;
    commit.tree = tree.write()?.detach();
    Ok(repo.write_object(&commit)?.detach())
}

fn replay_merge(
    repo: &gix::Repository,
    source_commit_id: ObjectId,
    parents: &[ObjectId],
    eager: bool,
) -> Result<ObjectId> {
    let graph = crate::history::HistoryGraph::for_commits(repo, &[source_commit_id])?;
    let outcome = rebase::perform_plan(
        repo,
        &graph,
        rebase::Plan {
            base: branch(repo, "main")?,
            scope: vec![source_commit_id],
            steps: vec![rebase::PlanStep {
                parents: parents.iter().copied().map(rebase::PlanParent::Existing).collect(),
                commit: rebase::PlanCommit::Pick(source_commit_id),
                squash: Vec::new(),
            }],
            checkout: None,
            expected_refs: rebase::capture_refs(repo, &[source_commit_id], &[])?,
            eager: if eager { vec![0] } else { Vec::new() },
            selection: Some(rebase::PlanParent::Step(0)),
        },
    )?
    .complete()?;
    outcome.selected.context("merge replay selects its result")
}

fn assert_recorded_merge(path: &Path, commit_id: ObjectId, octopus: bool) -> Result<()> {
    for (name, expected) in [
        ("shared", "recorded resolution\n"),
        ("merge-only", "only in the recorded merge\n"),
        ("left", "left contribution\n"),
    ] {
        assert_eq!(
            git(path, &["show", &format!("{commit_id}:{name}")])?,
            expected.as_bytes(),
            "replay retains the recorded {name} contents"
        );
    }
    if octopus {
        assert_eq!(
            git(path, &["show", &format!("{commit_id}:extra")])?,
            b"extra contribution\n",
            "the third parent's contribution remains in the merge"
        );
    }
    Ok(())
}

#[test]
fn secondary_parent_updates_preserve_resolutions_eagerly_and_after_gc_and_travel() -> gix_testtools::Result {
    for name in ["diamond", "octopus"] {
        let mut result_trees = Vec::new();
        for eager in [true, false] {
            let fixture = gix_testtools::scripted_fixture_writable("rebase_merge.sh")?;
            let path = fixture.path();
            let repo = crate::test_repository::open(path)?;
            let source_commit_id = branch(&repo, name)?;
            let source = repo.find_commit(source_commit_id)?.decode()?.into_owned()?;
            let mut parents = source.parents.to_vec();
            parents[1] = changed_tree(&repo, parents[1], "right", "right contribution\nright update\n")?;
            let selected_commit_id = replay_merge(&repo, source_commit_id, &parents, eager)?;
            let commit = repo.find_commit(selected_commit_id)?.decode()?.into_owned()?;
            assert_eq!(
                commit.parents.as_slice(),
                parents,
                "all parent slots retain their order"
            );
            assert_eq!(
                rebase::is_pending(&commit),
                !eager,
                "only deferred merge replay remains pending"
            );
            let result_commit_id = if eager {
                selected_commit_id
            } else {
                assert!(
                    !git(path, &["for-each-ref", "refs/tix/replay/"])?.is_empty(),
                    "pending merge replay retains its recorded source"
                );
                drop(repo);
                git(path, &["reflog", "expire", "--expire=now", "--all"])?;
                git(path, &["gc", "--prune=now"])?;
                let repo = crate::test_repository::open(path)?;
                let graph = crate::edit::loaded_graph(&repo)?;
                time_travel::perform(path, false, selected_commit_id, &graph, &[], &[], Default::default())?
                    .complete()?;
                repo.head_id()?.detach()
            };
            let repo = crate::test_repository::open(path)?;
            let commit = repo.find_commit(result_commit_id)?.decode()?.into_owned()?;
            assert!(!rebase::is_pending(&commit), "the replayed merge is final");
            assert_eq!(commit.parents.as_slice(), parents, "travel retains every parent slot");
            assert_eq!(commit.message, source.message, "the recorded merge message is retained");
            assert_eq!(commit.author, source.author, "the recorded merge author is retained");
            assert_eq!(
                crate::change_id::for_commit(&repo, result_commit_id)?,
                crate::change_id::for_commit(&repo, source_commit_id)?,
                "merge replay preserves the source change identity"
            );
            assert_recorded_merge(path, result_commit_id, name == "octopus")?;
            assert_eq!(
                git(path, &["show", &format!("{result_commit_id}:right")])?,
                b"right contribution\nright update\n",
                "a change made only through the secondary parent reaches the merge"
            );
            assert!(
                git(path, &["for-each-ref", "refs/tix/replay/"])?.is_empty(),
                "final replay releases its recovery resources"
            );
            result_trees.push(commit.tree);
        }
        assert_eq!(
            result_trees[0], result_trees[1],
            "eager and restarted replay produce the same tree"
        );
    }
    Ok(())
}

#[test]
fn unchanged_pending_parents_do_not_block_eager_merge_replay() -> gix_testtools::Result {
    for unchanged in [true, false] {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_merge.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let mut source = repo.find_commit(branch(&repo, "diamond")?)?.decode()?.into_owned()?;
        let mut side = repo.find_commit(source.parents[1])?.decode()?.into_owned()?;
        side.extra_headers
            .push(("tix-rebase-parent".into(), side.parents[0].to_string().into()));
        let pending_side_commit_id = repo.write_object(&side)?.detach();
        if unchanged {
            source.parents[1] = pending_side_commit_id;
        }
        let source_commit_id = repo.write_object(&source)?.detach();
        let parents = [
            changed_tree(&repo, source.parents[0], "left", "left contribution\nleft update\n")?,
            pending_side_commit_id,
        ];
        let result_commit_id = replay_merge(&repo, source_commit_id, &parents, true)?;
        let result = repo.find_commit(result_commit_id)?.decode()?.into_owned()?;
        assert_eq!(
            result.parents.as_slice(),
            parents,
            "replay retains the fixed pending parent in its original slot"
        );
        assert_eq!(
            rebase::is_pending(&result),
            !unchanged,
            "only a changed pending parent requires finalization before merge replay"
        );
        if unchanged {
            assert_eq!(
                git(fixture.path(), &["show", &format!("{result_commit_id}:left")])?,
                b"left contribution\nleft update\n",
                "the changed final parent is replayed despite the fixed pending side"
            );
        }
    }
    Ok(())
}

#[test]
fn travel_through_final_merges_finishes_pending_sides_before_amend() -> gix_testtools::Result {
    for with_descendant in [false, true] {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_merge.sh")?;
        let path = fixture.path();
        let repo = crate::test_repository::open(path)?;
        let mut source = repo.find_commit(branch(&repo, "diamond")?)?.decode()?.into_owned()?;
        let mut side = repo.find_commit(source.parents[1])?.decode()?.into_owned()?;
        side.extra_headers
            .push(("tix-rebase-parent".into(), side.parents[0].to_string().into()));
        side.parents[0] = changed_tree(&repo, side.parents[0], "upstream", "new upstream content\n")?;
        let pending_side_commit_id = repo.write_object(&side)?.detach();
        source.parents[1] = pending_side_commit_id;
        let source_commit_id = repo.write_object(&source)?.detach();
        let parents = [
            changed_tree(&repo, source.parents[0], "left", "left contribution\nleft update\n")?,
            pending_side_commit_id,
        ];
        let merge_commit_id = replay_merge(&repo, source_commit_id, &parents, true)?;
        let mut destination = repo.find_commit(merge_commit_id)?.decode()?.into_owned()?;
        assert!(!rebase::is_pending(&destination), "the recorded merge is already final");
        let destination_commit_id = if with_descendant {
            destination.parents = [merge_commit_id].into_iter().collect();
            destination.message = "final descendant of a merge with a pending side".into();
            repo.write_object(&destination)?.detach()
        } else {
            merge_commit_id
        };
        git(
            path,
            &[
                "update-ref",
                "refs/heads/destination",
                &destination_commit_id.to_string(),
            ],
        )?;
        let graph = crate::edit::loaded_graph(&repo)?;
        drop(repo);
        time_travel::perform(path, false, destination_commit_id, &graph, &[], &[], Default::default())?.complete()?;

        let repo = crate::test_repository::open(path)?;
        let destination = repo.head_commit()?.decode()?.into_owned()?;
        let merge = if with_descendant {
            repo.find_commit(destination.parents[0])?.decode()?.into_owned()?
        } else {
            destination
        };
        assert!(
            !rebase::is_pending(&repo.find_commit(merge.parents[1])?.decode()?.into_owned()?),
            "travel reaches the pending side through finalized commits"
        );
        assert_eq!(
            std::fs::read(path.join("upstream"))?,
            b"new upstream content\n",
            "the side's pending change reaches the destination worktree"
        );
        std::fs::write(path.join("merge-only"), "amended merge content\n")?;
        let graph = crate::edit::loaded_graph(&repo)?;
        assert!(
            crate::edit::head::amend_reporting(repo, &graph)?.is_some(),
            "the completed travel leaves an editable checkout"
        );
    }
    Ok(())
}

#[test]
fn rebasing_all_parents_applies_a_common_ancestor_change_once() -> gix_testtools::Result {
    for name in ["diamond", "octopus"] {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_merge.sh")?;
        let path = fixture.path();
        let repo = crate::test_repository::open(path)?;
        let source_commit_id = branch(&repo, name)?;
        let source = repo.find_commit(source_commit_id)?.decode()?.into_owned()?;
        let base_commit_id = branch(&repo, "main")?;
        let new_base_commit_id = changed_tree(&repo, base_commit_id, "common", "initial\nnew root line\n")?;
        let mut scope = source.parents.to_vec();
        scope.push(source_commit_id);
        let mut steps: Vec<_> = source
            .parents
            .iter()
            .map(|commit_id| rebase::PlanStep {
                parents: vec![rebase::PlanParent::Existing(new_base_commit_id)],
                commit: rebase::PlanCommit::Pick(*commit_id),
                squash: Vec::new(),
            })
            .collect();
        let merge_step = steps.len();
        steps.push(rebase::PlanStep {
            parents: (0..merge_step).map(rebase::PlanParent::Step).collect(),
            commit: rebase::PlanCommit::Pick(source_commit_id),
            squash: Vec::new(),
        });
        let graph = crate::history::HistoryGraph::for_commits(&repo, &scope)?;
        let outcome = rebase::perform_plan(
            &repo,
            &graph,
            rebase::Plan {
                base: new_base_commit_id,
                expected_refs: rebase::capture_refs(&repo, &scope, &[])?,
                scope,
                steps,
                checkout: None,
                eager: vec![merge_step],
                selection: Some(rebase::PlanParent::Step(merge_step)),
            },
        )?
        .complete()?;
        let result_commit_id = outcome.selected.context("the rebased merge is selected")?;
        let result = repo.find_commit(result_commit_id)?.decode()?.into_owned()?;
        assert_recorded_merge(path, result_commit_id, name == "octopus")?;
        for commit_id in std::iter::once(result_commit_id).chain(result.parents.iter().copied()) {
            assert_eq!(
                git(path, &["show", &format!("{commit_id}:common")])?,
                b"initial\nnew root line\n",
                "the inherited change is present exactly once in every parent and the merge"
            );
            assert!(
                !rebase::is_pending(&repo.find_commit(commit_id)?.decode()?.into_owned()?),
                "all parent paths required by the merge are eagerly finalized"
            );
        }
        let expected_parents: Vec<_> = source
            .parents
            .iter()
            .map(|parent| outcome.map(*parent).context("each original parent has a successor"))
            .collect::<Result<_>>()?;
        assert_eq!(
            result.parents.as_slice(),
            expected_parents,
            "replay retains the diamond's ordered edges"
        );
    }
    Ok(())
}

#[test]
fn metadata_only_parent_rewrites_leave_merges_final() -> gix_testtools::Result {
    for name in ["diamond", "octopus"] {
        let fixture = gix_testtools::scripted_fixture_writable("rebase_merge.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let source_commit_id = branch(&repo, name)?;
        let source = repo.find_commit(source_commit_id)?.decode()?.into_owned()?;
        let mut parents = Vec::new();
        for commit_id in &source.parents {
            let mut parent = repo.find_commit(*commit_id)?.decode()?.into_owned()?;
            parent.message = "updated parent message".into();
            parents.push(repo.write_object(&parent)?.detach());
        }
        let result_commit_id = replay_merge(&repo, source_commit_id, &parents, false)?;
        let result = repo.find_commit(result_commit_id)?.decode()?.into_owned()?;
        assert!(
            !rebase::is_pending(&result),
            "unchanged parent content needs no deferred merge replay"
        );
        assert_eq!(result.tree, source.tree, "metadata rewrites preserve the recorded tree");
        assert_eq!(
            result.parents.as_slice(),
            parents,
            "metadata rewrites update every ordered parent link"
        );
        assert!(
            git(fixture.path(), &["for-each-ref", "refs/tix/replay/"])?.is_empty(),
            "a finalized metadata rewrite owns no recovery resources"
        );
    }
    Ok(())
}

#[test]
fn converged_parent_slots_survive_amend_and_a_saved_continuation() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("rebase_merge.sh")?;
    let path = fixture.path();
    let repo = crate::test_repository::open(path)?;
    let source_commit_id = branch(&repo, "octopus")?;
    let source = repo.find_commit(source_commit_id)?.decode()?.into_owned()?;
    let right_commit_id = source.parents[1];
    let extra_commit_id = source.parents[2];
    let intended_parents = [right_commit_id, right_commit_id, extra_commit_id];
    let graph = crate::history::HistoryGraph::for_commits(&repo, &[source_commit_id])?;
    let rebase::PlanPerform::Conflict(mut conflict) = rebase::perform_plan(
        &repo,
        &graph,
        rebase::Plan {
            base: branch(&repo, "main")?,
            scope: vec![source_commit_id],
            steps: vec![rebase::PlanStep {
                parents: intended_parents
                    .iter()
                    .copied()
                    .map(rebase::PlanParent::Existing)
                    .collect(),
                commit: rebase::PlanCommit::Pick(source_commit_id),
                squash: Vec::new(),
            }],
            checkout: Some(rebase::PlanCheckout {
                target: rebase::PlanParent::Step(0),
                reference: None,
            }),
            expected_refs: rebase::capture_refs(&repo, &[source_commit_id], &[])?,
            eager: vec![0],
            selection: Some(rebase::PlanParent::Step(0)),
        },
    )?
    else {
        return Err("changing the first parent must conflict with the recorded resolution".into());
    };
    conflict.persist_objects()?;
    let plan = conflict.continuation_plan();
    assert_eq!(
        plan.steps[0].parents.len(),
        3,
        "the saved plan retains every source slot"
    );
    let saved = crate::edit::todo::prepare_continuation(conflict.repository(), &plan, Vec::new(), true)?.document;
    let outcome = conflict.into_conflict().persist(rebase::CheckoutOptions::default())?;
    let pending_commit_id = outcome.selected.context("the conflicted merge is selected")?;
    let pending = repo.find_commit(pending_commit_id)?.decode()?.into_owned()?;
    assert_eq!(
        pending.parents.as_slice(),
        [right_commit_id, extra_commit_id],
        "Git parent edges are deduplicated while replay retains source-slot correspondence"
    );
    let state = super::State::read(&pending)?.context("the merge carries replay state")?;
    assert_eq!(state.parent_index, 0, "the conflict is in the first parent slot");
    assert_eq!(
        state.phase,
        super::Phase::Parent,
        "the parent candidate itself conflicted"
    );
    let changed_target = changed_tree(&repo, right_commit_id, "common", "a different target\n")?;
    let mut retargeted = pending.clone();
    retargeted.tree = source.tree;
    let error = match super::rewrite(
        &repo,
        pending_commit_id,
        &mut retargeted,
        &[changed_target, right_commit_id, extra_commit_id],
        true,
        true,
    ) {
        Err(error) => error,
        Ok(_) => return Err("a resolution cannot silently change its first replay target".into()),
    };
    assert!(
        error.to_string().contains("before changing a replayed parent"),
        "even a conflict at index zero protects its already presented parent target"
    );

    git(path, &["read-tree", "--reset", "-u", &source.tree.to_string()])?;
    let graph = crate::edit::loaded_graph(&repo)?;
    let amended =
        crate::edit::head::amend_index_reporting(repo, &graph)?.context("the staged resolution completes the merge")?;
    let resolved_commit_id = amended.selected.context("amend selects the completed merge")?;
    let repo = crate::test_repository::open(path)?;
    let resolved = repo.find_commit(resolved_commit_id)?.decode()?.into_owned()?;
    assert!(
        !rebase::is_pending(&resolved),
        "one resolution finishes all remaining unchanged slots"
    );
    assert_eq!(
        resolved.tree, source.tree,
        "the staged recorded tree is accepted as the resolution"
    );
    assert_eq!(
        resolved.parents.as_slice(),
        [right_commit_id, extra_commit_id],
        "the completed merge has the two distinct resulting parents"
    );

    let parsed = crate::edit::todo::parse(&repo, &saved)?.context("the saved continuation remains valid")?;
    assert_eq!(
        parsed.plan.steps[0].parents.len(),
        3,
        "the saved todo still names three intended slots"
    );
    let mut scope = parsed.plan.scope.clone();
    scope.push(resolved_commit_id);
    let graph = crate::history::HistoryGraph::for_commits(&repo, &scope)?;
    let continued = rebase::perform_plan(&repo, &graph, parsed.plan)?.complete()?;
    let result_commit_id = continued.selected.context("continuing selects the resolved merge")?;
    let result = repo.find_commit(result_commit_id)?.decode()?.into_owned()?;
    assert_eq!(
        result.tree, resolved.tree,
        "the completed merge tree is not replayed again"
    );
    assert_eq!(
        result.parents, resolved.parents,
        "saved slots do not restore duplicate Git parents"
    );
    assert!(
        !rebase::is_pending(&result),
        "continuing a completed merge remains final"
    );
    let mut result = result;
    let mut resolved = resolved;
    result.extra_headers.sort();
    resolved.extra_headers.sort();
    assert_eq!(
        result, resolved,
        "republishing the completed merge changes at most the ordering of metadata headers"
    );
    Ok(())
}
