use std::ffi::OsString;

use super::*;

struct Replay {
    source_commit_id: ObjectId,
    checkpoint_commit_id: ObjectId,
    pending_commit_id: ObjectId,
    completed_commit_id: ObjectId,
    checkpoint_blob_id: ObjectId,
}

fn replay(repo: &gix::Repository) -> Result<Replay> {
    let mut source = repo.head_commit()?.decode()?.into_owned()?;
    ensure!(source.parents.len() == 2, "the fixture HEAD is a two-parent merge");
    source.message = "original merge resolution\n".into();
    let source_commit_id = repo.write_object(&source)?.detach();
    let checkpoint_blob_id = repo.write_blob(b"accumulated replay edits\n")?.detach();
    let mut tree = repo.find_tree(source.tree)?.edit()?;
    tree.upsert("checkpoint", gix::objs::tree::EntryKind::Blob, checkpoint_blob_id)?;
    let checkpoint_commit_id = repo
        .write_object(&gix::objs::Commit {
            tree: tree.write()?.detach(),
            parents: [source_commit_id].into_iter().collect(),
            author: source.author.clone(),
            committer: source.committer.clone(),
            encoding: None,
            message: "Tix merge replay checkpoint\n".into(),
            extra_headers: vec![("tix-replay-checkpoint".into(), source_commit_id.to_string().into())],
        })?
        .detach();
    source.extra_headers.push((
        "tix-rebase-merge".into(),
        format!(
            "{source_commit_id} 1 parent {checkpoint_commit_id} {} {}",
            source.parents[0], source.parents[1]
        )
        .into(),
    ));
    source.message = "pending merge replay\n".into();
    let pending_commit_id = repo.write_object(&source)?.detach();
    source.extra_headers.clear();
    source.message = "completed merge replay\n".into();
    let completed_commit_id = repo.write_object(&source)?.detach();
    Ok(Replay {
        source_commit_id,
        checkpoint_commit_id,
        pending_commit_id,
        completed_commit_id,
        checkpoint_blob_id,
    })
}

fn git(repo: &gix::Repository, args: &[&str]) -> Result<()> {
    let output = gix_testtools::git_command(repo.workdir().context("the fixture has a worktree")?)
        .args(args)
        .output()?;
    ensure!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn publish(repo: &gix::Repository, replay: &Replay) -> Result<FullName> {
    let name: FullName = "refs/heads/replaying".try_into()?;
    let mut edits = vec![RefEdit::update(
        name.clone(),
        replay.pending_commit_id,
        PreviousValue::MustNotExist,
        "publish pending replay",
    )];
    let resources = prepare(repo, [replay.pending_commit_id], [], &edits, None)?;
    edits.extend(resources.forward);
    let changes = undo::changes_from_edits(edits.clone())?;
    repo.edit_references(edits)?;
    undo::record(repo, "publish pending replay", &changes)?;
    Ok(name)
}

fn completion(branch: &FullName, replay: &Replay) -> RefEdit {
    RefEdit::update(
        branch.clone(),
        replay.completed_commit_id,
        PreviousValue::MustExistAndMatch(Target::Object(replay.pending_commit_id)),
        "complete merge replay",
    )
}

#[test]
fn checkpoints_survive_gc_without_undo_and_remain_private() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let replay = replay(&repo)?;
    publish(&repo, &replay)?;
    let resource = reference(replay.pending_commit_id)?;
    assert_eq!(
        repo.find_reference(resource.as_ref())?.id(),
        replay.checkpoint_commit_id,
        "the private reference retains the accumulated tree and original merge"
    );

    let pin: FullName = "refs/worktree/tix/pins/checkpoint".try_into()?;
    repo.reference(
        pin.clone(),
        replay.checkpoint_commit_id,
        PreviousValue::MustNotExist,
        "test checkpoint visibility",
    )?;
    assert!(
        crate::history::all_pins(&repo)?.is_empty(),
        "internal checkpoints cannot become view pins"
    );
    repo.edit_reference(RefEdit::delete(
        pin,
        PreviousValue::MustExistAndMatch(Target::Object(replay.checkpoint_commit_id)),
    ))?;

    assert!(
        !crate::history::decorations(&repo, &[], &[])?.contains_key(&replay.checkpoint_commit_id),
        "private replay references do not decorate checkpoints"
    );
    assert!(
        !crate::history::ref_tree_revisions(&repo, true)?.contains(&OsString::from(resource.to_string())),
        "reference trees do not include replay resources"
    );
    for revision in [resource.to_string(), replay.checkpoint_commit_id.to_string()] {
        assert!(
            crate::history::resolve_revision(&repo, revision.as_bytes().as_bstr()).is_err(),
            "a replay resource cannot be selected as history"
        );
        assert!(
            crate::history::snapshot(&repo, &[revision.into()], &[], false).is_err(),
            "a replay resource cannot become a history root"
        );
    }
    assert_eq!(
        crate::history::resolve_revision(&repo, replay.source_commit_id.to_string().as_bytes().as_bstr())?.0,
        replay.source_commit_id,
        "retaining an original merge does not make that ordinary commit private"
    );

    undo::clear(&repo)?;
    git(&repo, &["reflog", "expire", "--expire=now", "--all"])?;
    git(&repo, &["gc", "--prune=now"])?;
    let repo = crate::test_repository::open(fixture.path())?;
    assert!(
        repo.find_commit(replay.source_commit_id).is_ok(),
        "active replay retains its original merge after GC"
    );
    assert!(
        repo.find_commit(replay.checkpoint_commit_id).is_ok(),
        "active replay retains its checkpoint after GC"
    );
    assert_eq!(
        repo.find_blob(replay.checkpoint_blob_id)?.data,
        b"accumulated replay edits\n",
        "the accumulator's unique content survives clearing undo history and GC"
    );
    Ok(())
}

#[test]
fn completion_retires_the_resource_and_undo_restores_it() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let replay = replay(&repo)?;
    let branch = publish(&repo, &replay)?;
    let resource = reference(replay.pending_commit_id)?;
    let mut edits = vec![completion(&branch, &replay)];
    let resources = prepare(
        &repo,
        [replay.completed_commit_id],
        [replay.pending_commit_id],
        &edits,
        None,
    )?;
    assert_eq!(
        resources.forward.len(),
        1,
        "only the unreachable replay owner loses its resource"
    );
    edits.extend(resources.forward);
    let changes = undo::changes_from_edits(edits.clone())?;
    repo.edit_references(edits)?;
    undo::record(&repo, "complete merge replay", &changes)?;
    assert!(
        repo.try_find_reference(resource.as_ref())?.is_none(),
        "completion removes the obsolete checkpoint reference"
    );

    git(&repo, &["reflog", "expire", "--expire=now", "--all"])?;
    git(&repo, &["gc", "--prune=now"])?;
    let repo = crate::test_repository::open(fixture.path())?;
    undo::plan_undo(&repo)?
        .context("completion has an undo entry")?
        .apply(&repo)?;
    assert_eq!(
        repo.find_reference(branch.as_ref())?.id(),
        replay.pending_commit_id,
        "undo restores the pending owner"
    );
    assert_eq!(
        repo.find_reference(resource.as_ref())?.id(),
        replay.checkpoint_commit_id,
        "undo restores the retained checkpoint after GC"
    );
    undo::plan_redo(&repo)?
        .context("completion can be redone")?
        .apply(&repo)?;
    assert!(
        repo.try_find_reference(resource.as_ref())?.is_none(),
        "redo retires the checkpoint again"
    );
    Ok(())
}

#[test]
fn reachable_occurrences_keep_the_resource_until_their_final_refs_move() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let replay = replay(&repo)?;
    let branch = publish(&repo, &replay)?;
    let mut descendant = repo.find_commit(replay.pending_commit_id)?.decode()?.into_owned()?;
    descendant.parents = [replay.pending_commit_id].into_iter().collect();
    descendant.extra_headers.clear();
    descendant.message = "a descendant on another branch\n".into();
    let descendant_commit_id = repo.write_object(&descendant)?.detach();
    let tag_id = repo
        .write_object(&gix::objs::Tag {
            target: replay.pending_commit_id,
            target_kind: gix::objs::Kind::Commit,
            name: "retained".into(),
            tagger: None,
            message: "retain a pending occurrence\n".into(),
            signature: None,
        })?
        .detach();
    for (name, target) in [
        ("refs/heads/other", descendant_commit_id),
        ("refs/remotes/origin/replaying", replay.pending_commit_id),
        ("refs/tags/replaying", tag_id),
        ("refs/stash", descendant_commit_id),
        ("refs/worktree/tix/pins/replaying", replay.pending_commit_id),
    ] {
        let name: FullName = name.try_into()?;
        repo.reference(
            name.clone(),
            target,
            PreviousValue::MustNotExist,
            "retain another occurrence",
        )?;
        let mut edits = vec![completion(&branch, &replay)];
        assert!(
            prepare(&repo, [], [replay.pending_commit_id], &edits, None)?
                .forward
                .is_empty(),
            "a reachable occurrence through {name} retains the checkpoint"
        );
        let deletion = RefEdit::delete(name.clone(), PreviousValue::MustExistAndMatch(Target::Object(target)));
        edits.push(deletion.clone());
        assert_eq!(
            prepare(&repo, [], [replay.pending_commit_id], &edits, None)?
                .forward
                .len(),
            1,
            "cleanup evaluates the final reference state for {name}"
        );
        repo.edit_reference(deletion)?;
    }
    let alias: FullName = "refs/heads/alias".try_into()?;
    let symbolic = RefEdit::update(
        alias,
        Target::Symbolic(branch.clone()),
        PreviousValue::MustNotExist,
        "add symbolic root",
    );
    let edits = vec![completion(&branch, &replay), symbolic];
    assert_eq!(
        prepare(&repo, [], [replay.pending_commit_id], &edits, None)?
            .forward
            .len(),
        1,
        "a new symbolic root follows the branch's projected destination"
    );
    assert!(
        prepare(
            &repo,
            [],
            [replay.pending_commit_id],
            &[completion(&branch, &replay)],
            Some(replay.pending_commit_id)
        )?
        .forward
        .is_empty(),
        "a final detached checkout retains the pending occurrence"
    );

    repo.reference(
        "ORIG_HEAD",
        replay.pending_commit_id,
        PreviousValue::Any,
        "retain through a pseudo-reference",
    )?;
    repo.edit_reference(RefEdit::update(
        "refs/heads/pseudo-alias".try_into()?,
        Target::Symbolic("ORIG_HEAD".try_into()?),
        PreviousValue::MustNotExist,
        "reference a worktree-dependent target",
    ))?;
    assert!(
        prepare(
            &repo,
            [],
            [replay.pending_commit_id],
            &[completion(&branch, &replay)],
            None
        )?
        .forward
        .is_empty(),
        "a symbolic root outside the enumerated reference namespaces conservatively retains resources"
    );
    Ok(())
}

#[test]
fn other_worktrees_and_incomplete_reachability_prevent_retirement() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let replay = replay(&repo)?;
    let branch = publish(&repo, &replay)?;
    let pending = replay.pending_commit_id.to_string();
    git(&repo, &["worktree", "add", "--detach", "linked", &pending])?;
    let linked = crate::test_repository::open(fixture.path().join("linked"))?;
    let edits = vec![completion(&branch, &replay)];
    assert!(
        prepare(&repo, [], [replay.pending_commit_id], &edits, None)?
            .forward
            .is_empty(),
        "a detached linked HEAD retains the pending replay"
    );
    assert!(
        prepare(&linked, [], [replay.pending_commit_id], &edits, None)?
            .forward
            .is_empty(),
        "cleanup from the linked worktree includes its own HEAD"
    );
    let mut edits = edits;
    edits.push(RefEdit::update(
        "worktrees/linked/HEAD".try_into()?,
        replay.completed_commit_id,
        PreviousValue::MustExistAndMatch(Target::Object(replay.pending_commit_id)),
        "project linked checkout",
    ));
    assert_eq!(
        prepare(&repo, [], [replay.pending_commit_id], &edits, None)?
            .forward
            .len(),
        1,
        "qualified linked HEAD edits retire only its final unreachable occurrence"
    );

    let mut broken = repo.find_commit(replay.completed_commit_id)?.decode()?.into_owned()?;
    broken.parents = [ObjectId::from_hex(b"1234567890123456789012345678901234567890")?]
        .into_iter()
        .collect();
    broken.message = "missing ancestry\n".into();
    let broken_commit_id = repo.write_object(&broken)?.detach();
    repo.reference(
        "refs/heads/broken",
        broken_commit_id,
        PreviousValue::MustNotExist,
        "test incomplete ancestry",
    )?;
    assert!(
        prepare(&repo, [], [replay.pending_commit_id], &edits, None)?
            .forward
            .is_empty(),
        "missing ancestry conservatively keeps checkpoints when unreachability cannot be proven"
    );
    Ok(())
}

#[test]
fn publication_checks_existing_checkpoint_refs_and_has_a_rollback() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let replay = replay(&repo)?;
    let resource = reference(replay.pending_commit_id)?;
    let edits = prepare(&repo, [replay.pending_commit_id], [], &[], None)?;
    repo.edit_references(edits.forward)?;
    assert!(
        prepare(&repo, [replay.pending_commit_id], [replay.pending_commit_id], &[], None)?
            .forward
            .is_empty(),
        "publishing an existing pending owner retains its resource without duplicate edits"
    );
    repo.edit_references(edits.rollback)?;
    assert!(
        repo.try_find_reference(resource.as_ref())?.is_none(),
        "rollback removes the newly retained resource"
    );
    repo.reference(
        resource,
        replay.source_commit_id,
        PreviousValue::MustNotExist,
        "divert checkpoint",
    )?;
    assert!(
        prepare(&repo, [replay.pending_commit_id], [], &[], None).is_err(),
        "a mismatched checkpoint reference rejects publication rather than overwriting a resource"
    );
    Ok(())
}

#[test]
fn saved_continuations_retain_owners_and_checkpoints_until_consumed() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let replay = replay(&repo)?;
    let branch = publish(&repo, &replay)?;
    let mut descendant = repo.find_commit(replay.pending_commit_id)?.decode()?.into_owned()?;
    descendant.parents = [replay.pending_commit_id].into_iter().collect();
    descendant.extra_headers.clear();
    descendant.message = "pending continuation descendant\n".into();
    let descendant_commit_id = repo.write_object(&descendant)?.detach();
    descendant.parents.clear();
    descendant.message = "unreachable squash source\n".into();
    let squash_commit_id = repo.write_object(&descendant)?.detach();
    let scope = [replay.pending_commit_id, descendant_commit_id, squash_commit_id];
    let owner = Some((replay.pending_commit_id, scope.as_slice()));
    let retained = continuation_edits(&repo, owner, None)?;
    repo.edit_references(retained.forward)?;
    assert!(
        continuation_edits(&repo, owner, Some(replay.pending_commit_id))?
            .forward
            .is_empty(),
        "a repeated conflict keeps the sources still needed by its next continuation"
    );
    let next = continuation_edits(
        &repo,
        Some((replay.pending_commit_id, &scope[..2])),
        Some(replay.pending_commit_id),
    )?;
    assert_eq!(
        next.forward.len(),
        1,
        "a smaller continuation retires only the source its owner no longer needs"
    );
    repo.edit_references(next.forward)?;
    assert!(
        repo.try_find_reference(continuation_reference(replay.pending_commit_id, squash_commit_id)?.as_ref())?
            .is_none(),
        "sources outside the next continuation's scope are released"
    );
    repo.edit_references(next.rollback)?;
    assert_eq!(
        repo.find_reference(continuation_reference(replay.pending_commit_id, squash_commit_id)?.as_ref())?
            .id(),
        squash_commit_id,
        "rollback restores sources retired from the ownership group"
    );
    for commit_id in scope {
        let name = continuation_reference(replay.pending_commit_id, commit_id)?;
        assert!(
            crate::history::resolve_revision(&repo, name.as_bstr()).is_err(),
            "saved continuation references remain private"
        );
    }

    let edits = vec![completion(&branch, &replay)];
    assert!(
        prepare(
            &repo,
            [replay.completed_commit_id],
            [replay.pending_commit_id],
            &edits,
            None
        )?
        .forward
        .is_empty(),
        "amending the conflicted HEAD leaves the saved todo's checkpoint ownership intact"
    );
    repo.edit_references(edits)?;
    undo::clear(&repo)?;
    git(&repo, &["reflog", "expire", "--expire=now", "--all"])?;
    git(&repo, &["gc", "--prune=now"])?;
    let repo = crate::test_repository::open(fixture.path())?;
    for commit_id in scope
        .into_iter()
        .chain([replay.source_commit_id, replay.checkpoint_commit_id])
    {
        assert!(
            repo.find_commit(commit_id).is_ok(),
            "every saved continuation source and its merge replay checkpoint survives GC"
        );
    }

    let mut edits = continuation_edits(&repo, None, Some(replay.pending_commit_id))?.forward;
    let released = prepare(&repo, [], [replay.pending_commit_id], &edits, None)?;
    assert_eq!(
        released.forward.len(),
        1,
        "consuming the saved todo also releases its obsolete checkpoint"
    );
    edits.extend(released.forward);
    let changes = undo::changes_from_edits(edits.clone())?;
    repo.edit_references(edits)?;
    undo::record(&repo, "consume continuation", &changes)?;
    assert!(
        repo.references()?.prefixed(PREFIX.as_bstr())?.next().is_none(),
        "the completed continuation leaves no replay resource references"
    );
    git(&repo, &["reflog", "expire", "--expire=now", "--all"])?;
    git(&repo, &["gc", "--prune=now"])?;
    let repo = crate::test_repository::open(fixture.path())?;
    undo::plan_undo(&repo)?
        .context("consuming the continuation can be undone")?
        .apply(&repo)?;
    for commit_id in scope {
        assert_eq!(
            repo.find_reference(continuation_reference(replay.pending_commit_id, commit_id)?.as_ref())?
                .id(),
            commit_id,
            "undo restores every saved continuation source after GC"
        );
    }
    assert_eq!(
        repo.find_reference(reference(replay.pending_commit_id)?.as_ref())?.id(),
        replay.checkpoint_commit_id,
        "undo restores the original owner's checkpoint resource as well"
    );
    undo::plan_redo(&repo)?
        .context("consuming the continuation can be redone")?
        .apply(&repo)?;
    assert!(
        repo.references()?.prefixed(PREFIX.as_bstr())?.next().is_none(),
        "redo releases the continuation and checkpoint references again"
    );
    Ok(())
}

#[test]
fn consuming_one_continuation_keeps_another_continuations_shared_sources() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let replay = replay(&repo)?;
    let first_scope = [replay.pending_commit_id, replay.source_commit_id];
    let second_scope = [replay.completed_commit_id, replay.source_commit_id];
    let first = Some((replay.pending_commit_id, first_scope.as_slice()));
    let second = Some((replay.completed_commit_id, second_scope.as_slice()));
    repo.edit_references(continuation_edits(&repo, first, None)?.forward)?;
    repo.edit_references(continuation_edits(&repo, second, None)?.forward)?;
    repo.edit_references(continuation_edits(&repo, None, Some(replay.pending_commit_id))?.forward)?;
    assert!(
        repo.try_find_reference(continuation_reference(replay.pending_commit_id, replay.source_commit_id)?.as_ref())?
            .is_none(),
        "consuming the first continuation releases only its ownership group"
    );
    git(&repo, &["reflog", "expire", "--expire=now", "--all"])?;
    git(&repo, &["gc", "--prune=now"])?;
    let repo = crate::test_repository::open(fixture.path())?;
    assert!(
        repo.find_commit(replay.source_commit_id).is_ok(),
        "the second continuation retains their shared source after the first is consumed"
    );
    assert_eq!(
        repo.find_reference(continuation_reference(replay.completed_commit_id, replay.source_commit_id)?.as_ref())?
            .id(),
        replay.source_commit_id,
        "each saved continuation owns an independent source reference"
    );
    repo.edit_references(continuation_edits(&repo, None, Some(replay.completed_commit_id))?.forward)?;
    assert!(
        repo.references()?.prefixed(PREFIX.as_bstr())?.next().is_none(),
        "consuming both continuations releases both ownership groups"
    );
    Ok(())
}

#[test]
fn continuation_release_rejects_mismatched_and_symbolic_targets() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("history.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let replay = replay(&repo)?;
    let resource = continuation_reference(replay.pending_commit_id, replay.source_commit_id)?;
    for target in [
        Target::Object(replay.completed_commit_id),
        Target::Symbolic("refs/heads/missing".try_into()?),
    ] {
        repo.edit_reference(RefEdit::update(
            resource.clone(),
            target.clone(),
            PreviousValue::Any,
            "divert continuation resource",
        ))?;
        assert!(
            continuation_edits(&repo, None, Some(replay.pending_commit_id)).is_err(),
            "release requires a direct target matching the commit ID in its name"
        );
        assert_eq!(
            repo.find_reference(resource.as_ref())?.target().into_owned(),
            target,
            "rejecting an invalid resource leaves it untouched"
        );
    }
    Ok(())
}
