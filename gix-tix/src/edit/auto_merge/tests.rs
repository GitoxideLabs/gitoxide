use super::*;

fn input(repo: &gix::Repository, name: &str) -> Result<Input> {
    let reference: gix::refs::FullName = format!("refs/heads/{name}").try_into()?;
    let commit_id = repo.find_reference(reference.as_ref())?.peel_to_commit()?.id;
    Ok(Input {
        reference,
        commit_id,
        muted: false,
    })
}

fn candidate(repo: &gix::Repository, names: &[&str]) -> Result<gix::objs::Commit> {
    let mut commit = repo.head_commit()?.decode()?.into_owned()?;
    let definition = Definition {
        inputs: names.iter().map(|name| input(repo, name)).collect::<Result<_>>()?,
    };
    commit.parents = definition.inputs.iter().map(|input| input.commit_id).collect();
    commit.message = BString::default();
    definition.store(&mut commit);
    Ok(commit)
}

fn graph(repo: &gix::Repository) -> Result<crate::history::HistoryGraph> {
    let mut ids: Vec<_> = choices(repo)?.into_iter().map(|choice| choice.commit_id).collect();
    ids.push(repo.head_id()?.detach());
    crate::history::HistoryGraph::for_commits(repo, &ids)
}

fn apply(repo: &gix::Repository, selected_commit_id: ObjectId, change: Change) -> Result<(ObjectId, String)> {
    let operation = perform(
        repo,
        &graph(repo)?,
        selected_commit_id,
        change,
        rebase::CheckoutOptions::default(),
        |_| {},
    )?;
    let Some(result) = operation.result else {
        return Ok((selected_commit_id, operation.notice));
    };
    let outcome = result.complete()?;
    Ok((
        outcome.selected.context("AutoMerge has a selected result")?,
        operation.notice,
    ))
}

#[test]
fn creates_extends_refreshes_and_removes_inputs_without_moving_source_refs() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let b = input(&repo, "B")?;
    let c = input(&repo, "C")?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(b.reference.clone()))?;
    assert!(
        repo.head()?.is_detached(),
        "the generated commit has its own detached checkout"
    );
    assert_eq!(repo.head_id()?, merge_commit_id);
    assert_eq!(
        input(&repo, "A")?.commit_id,
        a.commit_id,
        "creation preserves the source branch"
    );
    assert_eq!(
        input(&repo, "B")?.commit_id,
        b.commit_id,
        "muting preserves the source branch"
    );
    let (merge_commit_id, _) = apply(&repo, merge_commit_id, Change::Add(c.reference))?;
    assert!(
        repo.head_commit()?.tree()?.find_entry("c").is_some(),
        "extending an AutoMerge updates the checkout"
    );
    let (unchanged, notice) = apply(&repo, merge_commit_id, Change::Add(a.reference.clone()))?;
    assert_eq!(unchanged, merge_commit_id);
    assert!(notice.contains("ancestor"), "adding an included tip explains the no-op");

    repo.reference(
        b.reference.clone(),
        c.commit_id,
        gix::refs::transaction::PreviousValue::Any,
        "external reset",
    )?;
    let (updated, _) = apply(&repo, merge_commit_id, Change::Remerge)?;
    let commit = repo.find_commit(updated)?.decode()?.into_owned()?;
    assert_eq!(commit.parents.len(), 2, "converged tips share a parent");
    assert_eq!(
        Definition::from_commit(&commit)?.expect("recipe remains").inputs.len(),
        3
    );
    let (updated, _) = apply(&repo, updated, Change::Remove(b.reference.clone()))?;
    let (collapsed, _) = apply(&repo, updated, Change::Remove(input(&repo, "C")?.reference))?;
    assert_eq!(
        collapsed, a.commit_id,
        "one surviving subscription collapses to its tip"
    );
    assert_eq!(repo.head_id()?, a.commit_id);
    assert_eq!(
        input(&repo, "B")?.commit_id,
        c.commit_id,
        "removal does not delete or move the input ref"
    );
    Ok(())
}

fn changed_tree(
    repo: &gix::Repository,
    mut commit: gix::objs::Commit,
    path: &str,
    contents: &str,
) -> Result<gix::objs::Commit> {
    let mut tree = repo.find_tree(commit.tree)?.edit()?;
    tree.upsert(path, gix::objs::tree::EntryKind::Blob, repo.write_blob(contents)?)?;
    commit.tree = tree.write()?.detach();
    Ok(commit)
}

#[test]
fn travel_refreshes_external_inputs_and_muted_replays_keep_their_original_patch() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "C")?.reference))?;
    let main = input(&repo, "main")?.commit_id;
    let mut new_base = changed_tree(
        &repo,
        repo.find_commit(main)?.decode()?.into_owned()?,
        "shared",
        "new base\n",
    )?;
    new_base.parents = [main].into_iter().collect();
    let new_base_commit_id = repo.write_object(&new_base)?.detach();
    let mut pending = repo.find_commit(a.commit_id)?.decode()?.into_owned()?;
    let original_tree_id = pending.tree;
    pending.parents = [new_base_commit_id].into_iter().collect();
    pending
        .extra_headers
        .push(("tix-rebase-parent".into(), main.to_string().into()));
    let pending_commit_id = repo.write_object(&pending)?.detach();
    repo.reference(
        a.reference.clone(),
        pending_commit_id,
        gix::refs::transaction::PreviousValue::Any,
        "external pending input",
    )?;
    let c = input(&repo, "C")?;
    let mut advanced = changed_tree(
        &repo,
        repo.find_commit(c.commit_id)?.decode()?.into_owned()?,
        "advanced",
        "C advanced\n",
    )?;
    advanced.parents = [c.commit_id].into_iter().collect();
    let advanced_commit_id = repo.write_object(&advanced)?.detach();
    repo.reference(
        c.reference,
        advanced_commit_id,
        gix::refs::transaction::PreviousValue::Any,
        "external advancement",
    )?;

    let travel =
        super::super::time_travel::perform(fixture.path(), false, merge_commit_id, &graph(&repo)?, &[], &[], false)?;
    travel.complete()?;
    let merged = repo.head_commit()?.decode()?.into_owned()?;
    let definition = Definition::from_commit(&merged)?.expect("travel preserves the AutoMerge");
    assert!(
        definition.inputs[0].muted,
        "a conflicting pending input stays optional during merge travel"
    );
    assert!(!definition.inputs[1].muted, "other inputs still merge");
    assert!(
        repo.find_tree(merged.tree)?.find_entry("advanced").is_some(),
        "travel rereads all named tips"
    );
    let input_commit_id = input(&repo, "A")?.commit_id;
    let input_commit = repo.find_commit(input_commit_id)?.decode()?.into_owned()?;
    assert!(rebase::is_pending(&input_commit));
    assert_eq!(
        input_commit.tree, original_tree_id,
        "muting never replaces the original patch with the destination tree"
    );
    assert_eq!(
        rebase::marked_parent_ref(&repo.find_commit(input_commit_id)?.decode()?)?,
        Some(Some(main)),
        "the replay base survives a conflict"
    );
    assert!(
        matches!(
            super::super::time_travel::perform(
                fixture.path(),
                false,
                input_commit_id,
                &graph(&repo)?,
                &[],
                &[],
                false
            )?,
            super::super::time_travel::Perform::Conflict(_)
        ),
        "direct travel offers ordinary conflict resolution"
    );
    Ok(())
}

#[test]
fn todos_resolve_inputs_from_later_fork_sections() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(c.reference.clone()))?;
    let main = input(&repo, "main")?.commit_id;
    let mut base = changed_tree(
        &repo,
        repo.find_commit(main)?.decode()?.into_owned()?,
        "base-added",
        "new base\n",
    )?;
    base.parents = [main].into_iter().collect();
    let base_commit_id = repo.write_object(&base)?.detach();
    let scope = vec![a.commit_id, c.commit_id, merge_commit_id];
    let plan = rebase::Plan {
        base: main,
        expected_refs: rebase::capture_refs(&repo, &scope, &[a.commit_id, c.commit_id])?,
        scope,
        steps: vec![
            rebase::PlanStep {
                parent: rebase::PlanParent::Existing(base_commit_id),
                commit: rebase::PlanCommit::Pick(a.commit_id),
                squash: Vec::new(),
            },
            rebase::PlanStep {
                parent: rebase::PlanParent::Step(0),
                commit: rebase::PlanCommit::Pick(merge_commit_id),
                squash: Vec::new(),
            },
            rebase::PlanStep {
                parent: rebase::PlanParent::Existing(base_commit_id),
                commit: rebase::PlanCommit::Pick(c.commit_id),
                squash: Vec::new(),
            },
        ],
        checkout: Some(rebase::PlanCheckout {
            target: rebase::PlanParent::Step(1),
            reference: None,
        }),
    };
    let outcome = rebase::perform_plan(&repo, &graph(&repo)?, plan)?.complete()?;
    let merged = repo
        .find_commit(outcome.selected.context("the plan selects the merge")?)?
        .decode()?
        .into_owned()?;
    let definition = Definition::from_commit(&merged)?.expect("pick retains the recipe");
    assert_eq!(definition.inputs[0].commit_id, input(&repo, "A")?.commit_id);
    assert_eq!(definition.inputs[1].commit_id, input(&repo, "C")?.commit_id);
    assert!(definition.inputs.iter().all(|input| !input.muted));
    assert!(repo.find_tree(merged.tree)?.find_entry("base-added").is_some());
    assert!(repo.find_tree(merged.tree)?.find_entry("c").is_some());
    Ok(())
}

#[test]
fn todo_inputs_keep_explicit_existing_ref_destinations() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(c.reference.clone()))?;
    let mut expected_refs = rebase::capture_refs(&repo, &[a.commit_id, c.commit_id, merge_commit_id], &[])?;
    let reference = expected_refs
        .iter_mut()
        .find(|expected| expected.name == c.reference)
        .context("C is editable in the todo")?;
    let mut refs = References::default();
    reference.destination = rebase::RefDestination::Step(0);
    let resolve = |refs: &mut References, reference: &rebase::PlanRef| {
        refs.resolve(
            &repo,
            &c.reference,
            &HashMap::new(),
            Some((std::slice::from_ref(reference), &[])),
        )
    };
    assert!(
        resolve(&mut refs, reference)
            .expect_err("the input step has not run")
            .to_string()
            .contains("unproduced step"),
        "an unproduced input cannot silently disappear"
    );
    reference.destination = rebase::RefDestination::Delete;
    assert_eq!(
        resolve(&mut refs, reference)?,
        None,
        "only explicit deletion removes the input"
    );
    reference.destination = rebase::RefDestination::Existing(a.commit_id);
    let plan = rebase::Plan {
        base: input(&repo, "main")?.commit_id,
        scope: vec![a.commit_id, merge_commit_id],
        expected_refs,
        steps: vec![
            rebase::PlanStep {
                parent: rebase::PlanParent::Existing(c.commit_id),
                commit: rebase::PlanCommit::Pick(a.commit_id),
                squash: Vec::new(),
            },
            rebase::PlanStep {
                parent: rebase::PlanParent::Step(0),
                commit: rebase::PlanCommit::Pick(merge_commit_id),
                squash: Vec::new(),
            },
        ],
        checkout: Some(rebase::PlanCheckout {
            target: rebase::PlanParent::Step(1),
            reference: None,
        }),
    };
    let outcome = rebase::perform_plan(&repo, &graph(&repo)?, plan)?.complete()?;
    let updated = repo.find_commit(outcome.selected.context("the merge remains selected")?)?;
    let definition = Definition::from_commit(&updated.decode()?.into_owned()?)?.expect("the recipe survives");
    assert_ne!(input(&repo, "A")?.commit_id, a.commit_id, "the A pick is rewritten");
    assert_eq!(
        input(&repo, "C")?.commit_id,
        a.commit_id,
        "C stays at its explicit old object"
    );
    assert_eq!(
        definition.inputs[1].commit_id, a.commit_id,
        "the recipe agrees with C's final ref"
    );
    Ok(())
}

#[test]
fn a_todo_conflict_continuation_maintains_auto_merge_descendants() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "C")?.reference))?;
    repo.reference(
        "refs/heads/combined",
        merge_commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "retain merge",
    )?;
    let scope = vec![a.commit_id, merge_commit_id];
    let plan = rebase::Plan {
        base: input(&repo, "main")?.commit_id,
        expected_refs: rebase::capture_refs(&repo, &scope, &[])?,
        scope,
        steps: vec![
            rebase::PlanStep {
                parent: rebase::PlanParent::Existing(input(&repo, "B")?.commit_id),
                commit: rebase::PlanCommit::Pick(a.commit_id),
                squash: Vec::new(),
            },
            rebase::PlanStep {
                parent: rebase::PlanParent::Step(0),
                commit: rebase::PlanCommit::Pick(merge_commit_id),
                squash: Vec::new(),
            },
        ],
        checkout: Some(rebase::PlanCheckout {
            target: rebase::PlanParent::Step(0),
            reference: Some(a.reference.clone()),
        }),
    };
    let rebase::PlanPerform::Conflict(mut conflict) = rebase::perform_plan(&repo, &graph(&repo)?, plan)? else {
        panic!("replaying A onto B conflicts on the shared file")
    };
    assert_eq!(conflict.original(), a.commit_id);
    conflict.persist_objects()?;
    let continuation = super::super::todo::prepare_continuation(
        conflict.repository(),
        &conflict.continuation_plan(),
        vec![merge_commit_id],
        false,
    )?;
    super::super::time_travel::materialize_plan_conflict_reporting(conflict, &[], false)?;
    std::fs::write(fixture.path().join("shared"), "resolved A and B\n")?;
    let staged = std::process::Command::new("git")
        .current_dir(fixture.path())
        .args(["add", "shared"])
        .status()?;
    assert!(staged.success(), "the resolved shared file is staged");
    let parsed = super::super::todo::parse(&repo, &continuation.document)?.context("the continuation parses")?;
    let mut ids = graph(&repo)?.edit_commit_ids();
    ids.extend_from_slice(&parsed.plan.scope);
    rebase::perform_plan(
        &repo,
        &crate::history::HistoryGraph::for_commits(&repo, &ids)?,
        parsed.plan,
    )?
    .complete()?;

    let merged = input(&repo, "combined")?.commit_id;
    let definition = Definition::from_commit(&repo.find_commit(merged)?.decode()?.into_owned()?)?
        .expect("the continuation retains the AutoMerge");
    assert_eq!(definition.inputs[0].commit_id, input(&repo, "A")?.commit_id);
    super::super::time_travel::perform(fixture.path(), false, merged, &graph(&repo)?, &[], &[], false)?;
    assert_eq!(
        std::fs::read_to_string(fixture.path().join("shared"))?,
        "resolved A and B\n"
    );
    assert!(
        fixture.path().join("c").is_file(),
        "travel includes the other input too"
    );
    Ok(())
}

#[test]
fn editing_an_input_updates_nested_merges_and_undo_restores_the_whole_operation() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let (first_merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "C")?.reference))?;
    repo.reference(
        "refs/heads/combined",
        first_merge_commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "name first merge",
    )?;
    // Construct an independent second recipe which follows the first one by name.
    let mut outer = candidate(&repo, &["combined", "B"])?;
    rebuild(
        &repo,
        &mut outer,
        &mut References::default(),
        &HashMap::new(),
        None,
        true,
    )?;
    let outer_commit_id = repo.write_object(&outer)?.detach();
    repo.reference(
        "refs/worktree/tix/pins/outer",
        outer_commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "retain outer merge",
    )?;

    let mut replacement = repo.find_commit(a.commit_id)?.decode()?.into_owned()?;
    replacement.message = "updated A\n".into();
    let outcome = rebase::perform(
        &repo,
        &graph(&repo)?,
        rebase::Edit::Replace {
            target: a.commit_id,
            commit: replacement,
        },
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::LeaveAsIsAndMark,
    )?
    .complete()?;
    let first_updated = outcome.map(first_merge_commit_id).context("the first merge remains")?;
    let outer_updated = outcome.map(outer_commit_id).context("the outer merge remains")?;
    assert_ne!(
        first_updated, first_merge_commit_id,
        "the checked-out merge is maintained"
    );
    assert_ne!(
        outer_updated, outer_commit_id,
        "dependent merges elsewhere in the projection are maintained too"
    );
    let outer = repo.find_commit(outer_updated)?.decode()?.into_owned()?;
    assert!(
        rebase::is_pending(&outer),
        "an off-checkout merge can wait for travel to generate its tree"
    );
    assert_eq!(
        Definition::from_commit(&outer)?.expect("outer recipe survives").inputs[0].commit_id,
        first_updated
    );
    assert_eq!(repo.head_id()?, first_updated);
    undo::record(&repo, "edit input", &outcome.ref_changes)?;
    undo::plan_undo(&repo)?.context("the edit is undoable")?.apply(&repo)?;
    assert_eq!(repo.head_id()?, first_merge_commit_id);
    assert_eq!(input(&repo, "A")?.commit_id, a.commit_id);
    assert_eq!(
        repo.find_reference("refs/worktree/tix/pins/outer")?
            .peel_to_commit()?
            .id,
        outer_commit_id
    );
    undo::plan_redo(&repo)?.context("the edit is redoable")?.apply(&repo)?;
    assert_eq!(repo.head_id()?, first_updated);
    Ok(())
}

#[test]
fn input_pins_survive_checkout_and_removal_choices_disambiguate_memberships() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let pin: FullName = "refs/worktree/tix/pins/input".try_into()?;
    repo.reference(
        pin.clone(),
        a.commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "pin input",
    )?;
    repo.edit_references([
        gix::refs::transaction::RefEdit::update(
            "HEAD".try_into()?,
            a.commit_id,
            gix::refs::transaction::PreviousValue::Any,
            "detach fixture",
        ),
        gix::refs::transaction::RefEdit::delete(a.reference, gix::refs::transaction::PreviousValue::Any),
    ])?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "C")?.reference))?;
    assert_eq!(repo.head_commit()?.decode()?.message, "✔️ 📌 ✔️ C\n");
    super::super::time_travel::perform(fixture.path(), false, a.commit_id, &graph(&repo)?, &[], &[], false)?
        .complete()?;
    assert!(
        repo.try_find_reference(pin.as_ref())?.is_some(),
        "checkout cannot consume a subscribed pin"
    );
    let (second_merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "B")?.reference))?;
    let memberships = removals(&repo, &graph(&repo)?, a.commit_id, false)?;
    assert_eq!(memberships.len(), 2, "the picker offers both memberships");
    assert!(
        memberships
            .iter()
            .any(|choice| choice.merge_commit_id == merge_commit_id)
    );
    assert!(
        memberships
            .iter()
            .any(|choice| choice.merge_commit_id == second_merge_commit_id)
    );
    assert!(memberships.iter().all(|choice| choice.label.contains("📌")));
    let (_, _) = apply(&repo, merge_commit_id, Change::Remove(pin.clone()))?;
    assert!(
        repo.try_find_reference(pin.as_ref())?.is_some(),
        "removing a subscription keeps the pin itself"
    );
    assert_eq!(
        repo.head_id()?,
        second_merge_commit_id,
        "removing from another merge keeps the checkout"
    );
    Ok(())
}

#[test]
fn missing_refs_are_pruned_and_an_empty_subscription_set_keeps_the_previous_result() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "B")?.reference))?;
    repo.find_reference("refs/heads/B")?.delete()?;
    let (collapsed, _) = apply(&repo, merge_commit_id, Change::Remerge)?;
    assert_eq!(collapsed, a.commit_id);
    repo.reference(
        "HEAD",
        merge_commit_id,
        gix::refs::transaction::PreviousValue::Any,
        "return to retained merge",
    )?;
    repo.find_reference(a.reference.as_ref())?.delete()?;
    let (kept, notice) = apply(&repo, merge_commit_id, Change::Remerge)?;
    assert_eq!(kept, merge_commit_id);
    assert!(
        notice.contains("no input refs"),
        "the unchanged result explains the missing inputs"
    );
    Ok(())
}

#[test]
fn dirty_checkout_and_concurrent_ref_changes_abort_without_moving_refs() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    std::fs::write(fixture.path().join("c"), "precious untracked file\n")?;
    assert!(
        perform(
            &repo,
            &graph(&repo)?,
            a.commit_id,
            Change::Add(c.reference.clone()),
            rebase::CheckoutOptions::default(),
            |_| {}
        )
        .is_err(),
        "checkout preflight runs before updating references"
    );
    assert_eq!(repo.head_id()?, a.commit_id);
    assert_eq!(
        std::fs::read_to_string(fixture.path().join("c"))?,
        "precious untracked file\n"
    );
    std::fs::remove_file(fixture.path().join("c"))?;
    let mut raced = false;
    let result = perform(
        &repo,
        &graph(&repo)?,
        a.commit_id,
        Change::Add(c.reference.clone()),
        rebase::CheckoutOptions::default(),
        |progress| {
            if progress.processed > 0 && !raced {
                repo.reference(
                    c.reference.clone(),
                    a.commit_id,
                    gix::refs::transaction::PreviousValue::Any,
                    "concurrent move",
                )
                .expect("the fixture permits a concurrent ref update");
                raced = true;
            }
        },
    );
    assert!(
        result.is_err(),
        "every input ref is checked atomically before publication"
    );
    assert_eq!(repo.head_id()?, a.commit_id);
    assert_eq!(
        input(&repo, "C")?.commit_id,
        a.commit_id,
        "the concurrent move is preserved"
    );
    Ok(())
}

#[test]
fn a_plan_maintains_offscreen_merges_and_replays_pending_inputs_outside_its_scope() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(c.reference.clone()))?;
    let main = input(&repo, "main")?.commit_id;
    let mut base = changed_tree(
        &repo,
        repo.find_commit(main)?.decode()?.into_owned()?,
        "new-base",
        "base\n",
    )?;
    base.parents = [main].into_iter().collect();
    let base_commit_id = repo.write_object(&base)?.detach();
    let mut pending = repo.find_commit(c.commit_id)?.decode()?.into_owned()?;
    pending.parents = [base_commit_id].into_iter().collect();
    pending
        .extra_headers
        .push(("tix-rebase-parent".into(), main.to_string().into()));
    let pending_commit_id = repo.write_object(&pending)?.detach();
    repo.reference(
        c.reference,
        pending_commit_id,
        gix::refs::transaction::PreviousValue::Any,
        "pending outside todo",
    )?;
    let plan = rebase::Plan {
        base: main,
        scope: vec![a.commit_id],
        steps: vec![rebase::PlanStep {
            parent: rebase::PlanParent::Existing(base_commit_id),
            commit: rebase::PlanCommit::Pick(a.commit_id),
            squash: Vec::new(),
        }],
        checkout: None,
        expected_refs: rebase::capture_refs(&repo, &[a.commit_id], &[a.commit_id])?,
    };
    // Neither the pending C nor its new parent was present in the original projection.
    let frozen = crate::history::HistoryGraph::for_commits(&repo, &[main, a.commit_id, c.commit_id, merge_commit_id])?;
    let outcome = rebase::perform_plan(&repo, &frozen, plan)?.complete()?;
    let updated = outcome.map(merge_commit_id).context("the dependent merge survives")?;
    let commit = repo.find_commit(updated)?.decode()?.into_owned()?;
    assert_ne!(updated, merge_commit_id);
    assert!(
        !rebase::is_pending(&commit),
        "the derived checkout is finalized even outside the explicit todo scope"
    );
    assert!(
        Definition::from_commit(&commit)?
            .expect("recipe retained")
            .inputs
            .iter()
            .all(|input| !input.muted)
    );
    assert!(
        !rebase::is_pending(&repo.find_commit(input(&repo, "C")?.commit_id)?.decode()?.into_owned()?),
        "an input outside the todo is replayed before merging"
    );
    assert!(repo.find_tree(commit.tree)?.find_entry("new-base").is_some());
    assert_eq!(outcome.selected, Some(updated));
    Ok(())
}

#[test]
fn generated_content_and_self_subscriptions_are_rejected() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    repo.reference(
        "refs/heads/alias",
        a.commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "ambiguous source",
    )?;
    assert!(
        perform(
            &repo,
            &graph(&repo)?,
            a.commit_id,
            Change::Add(input(&repo, "C")?.reference),
            rebase::CheckoutOptions::default(),
            |_| {}
        )
        .is_err()
    );
    repo.find_reference("refs/heads/alias")?.delete()?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "C")?.reference))?;
    assert!(super::super::head::perform(repo.clone(), &graph(&repo)?, super::super::head::Kind::Amend, None).is_err());
    assert!(
        super::super::reword::apply_message_reporting(
            repo.clone(),
            &graph(&repo)?,
            merge_commit_id,
            b"replacement title",
            None
        )
        .is_err()
    );
    let err = super::super::time_travel::attach_reporting(fixture.path(), false, &[], false)
        .expect_err("the remembered input cannot be attached to its own merge");
    assert!(err.to_string().contains("track itself"));
    assert_eq!(input(&repo, "A")?.commit_id, a.commit_id);
    repo.reference(
        a.reference,
        merge_commit_id,
        gix::refs::transaction::PreviousValue::Any,
        "external self-reference",
    )?;
    assert!(
        perform(
            &repo,
            &graph(&repo)?,
            merge_commit_id,
            Change::Remerge,
            rebase::CheckoutOptions::default(),
            |_| {}
        )
        .is_err(),
        "external self-subscriptions are rejected too"
    );
    assert_eq!(repo.head_id()?, merge_commit_id);
    Ok(())
}

#[test]
fn common_base_is_used_when_every_input_is_pending() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?.with_object_memory();
    let main = input(&repo, "main")?.commit_id;
    let mut rewrites = HashMap::new();
    for name in ["A", "B"] {
        let input = input(&repo, name)?;
        let mut commit = repo.find_commit(input.commit_id)?.decode()?.into_owned()?;
        commit
            .extra_headers
            .push(("tix-rebase-parent".into(), main.to_string().into()));
        rewrites.insert(input.commit_id, Some(repo.write_object(&commit)?.detach()));
    }
    let mut commit = candidate(&repo, &["A", "B"])?;
    rebuild(&repo, &mut commit, &mut References::default(), &rewrites, None, true)?;
    assert_eq!(
        commit.tree,
        repo.find_commit(main)?.tree_id()?,
        "muted pending patches do not leak into the generated tree"
    );
    assert_eq!(commit.message, "💥 A 💥 B\n");
    Ok(())
}

#[test]
fn creation_and_checkout_share_one_undo_step() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let operation = perform(
        &repo,
        &graph(&repo)?,
        a.commit_id,
        Change::Add(input(&repo, "C")?.reference),
        rebase::CheckoutOptions::default(),
        |_| {},
    )?;
    let outcome = operation.result.context("creation prepares a merge")?.complete()?;
    let changes = outcome.ref_changes;
    let merge_commit_id = repo.head_id()?.detach();
    assert!(fixture.path().join("c").is_file());
    undo::record(&repo, "AutoMerge", &changes)?;
    undo::plan_undo(&repo)?.context("creation is undoable")?.apply(&repo)?;
    assert!(
        !repo.head()?.is_detached(),
        "undo restores the original attached checkout"
    );
    assert_eq!(repo.head_id()?, a.commit_id);
    assert!(!fixture.path().join("c").exists(), "undo restores the original tree");
    undo::plan_redo(&repo)?.context("creation is redoable")?.apply(&repo)?;
    assert_eq!(repo.head_id()?, merge_commit_id);
    assert!(repo.head()?.is_detached());
    assert!(fixture.path().join("c").is_file());
    Ok(())
}

#[test]
fn attaching_an_advanced_input_maintains_its_merges_and_is_undoable() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "C")?.reference))?;
    repo.reference(
        "refs/heads/combined",
        merge_commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "retain merge",
    )?;
    let mut advanced = changed_tree(
        &repo,
        repo.find_commit(a.commit_id)?.decode()?.into_owned()?,
        "advance",
        "advanced A\n",
    )?;
    advanced.parents = [a.commit_id].into_iter().collect();
    let advanced_commit_id = repo.write_object(&advanced)?.detach();
    super::super::time_travel::perform(
        fixture.path(),
        false,
        advanced_commit_id,
        &graph(&repo)?,
        &[],
        &[],
        false,
    )?;
    let (_, changes) = super::super::time_travel::attach_reporting(fixture.path(), false, &[], false)?;
    let updated = input(&repo, "combined")?.commit_id;
    assert_eq!(repo.head_id()?, advanced_commit_id, "attachment keeps its chosen tip");
    assert_eq!(input(&repo, "A")?.commit_id, advanced_commit_id);
    assert_eq!(
        Definition::from_commit(&repo.find_commit(updated)?.decode()?.into_owned()?)?
            .expect("the dependent merge keeps its subscriptions")
            .inputs[0]
            .commit_id,
        advanced_commit_id,
        "attachment advances the named input in its dependent merge"
    );
    undo::record(&repo, "attach input", &changes)?;
    undo::plan_undo(&repo)?
        .context("attachment can be undone")?
        .apply(&repo)?;
    assert!(repo.head()?.is_detached());
    assert_eq!(input(&repo, "A")?.commit_id, a.commit_id);
    assert_eq!(input(&repo, "combined")?.commit_id, merge_commit_id);
    Ok(())
}

#[test]
fn finishing_review_updates_merges_in_both_the_review_and_return_histories() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let (return_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "C")?.reference))?;
    let started = super::super::review::start(
        fixture.path(),
        false,
        &super::super::loaded_view_graph(&repo)?,
        a.commit_id,
        input(&repo, "main")?.commit_id,
    )?;
    assert!(started.checkout_error.is_none(), "review checks out its base");
    let review_commit_id = super::super::head::perform(
        repo.clone(),
        &super::super::loaded_view_graph(&repo)?,
        super::super::head::Kind::Amend,
        None,
    )?
    .context("the reviewed patch amends the review commit")?;
    repo.reference(
        "refs/heads/review-input",
        review_commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "name review input",
    )?;
    let (review_merge_commit_id, _) = apply(&repo, review_commit_id, Change::Add(a.reference.clone()))?;
    repo.reference(
        "refs/heads/review-combined",
        review_merge_commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "retain review merge",
    )?;
    let super::super::review::Finish::Complete(finished) = super::super::review::finish(
        repo.clone(),
        &super::super::loaded_view_graph(&repo)?,
        review_commit_id,
        None,
    )?
    else {
        panic!("review finishing restores the existing return AutoMerge")
    };

    assert_eq!(
        repo.head_id()?,
        finished
            .outcome
            .map(return_commit_id)
            .context("the return merge survives")?
    );
    assert_eq!(input(&repo, "A")?.commit_id, finished.commit);
    for commit_id in [repo.head_id()?.detach(), input(&repo, "review-combined")?.commit_id] {
        let definition = Definition::from_commit(&repo.find_commit(commit_id)?.decode()?.into_owned()?)?
            .expect("review finishing preserves the merge recipe");
        for input in definition.inputs {
            assert_eq!(
                input.commit_id,
                repo.find_reference(input.reference.as_ref())?.peel_to_commit()?.id,
                "every subscription follows its final ref after review finishing"
            );
        }
    }
    Ok(())
}

#[test]
fn mutes_the_entire_conflicting_input_and_continues_with_later_inputs() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?.with_object_memory();
    let mut commit = candidate(&repo, &["A", "B", "C"])?;
    let mut refs = References::default();
    assert_eq!(
        rebuild(&repo, &mut commit, &mut refs, &HashMap::new(), None, true)?,
        Rebuilt::Commit
    );
    let definition = Definition::from_commit(&commit)?.expect("the merge retains its recipe");
    assert_eq!(
        definition.inputs.iter().map(|input| input.muted).collect::<Vec<_>>(),
        [false, true, false]
    );
    assert_eq!(commit.parents.len(), 3, "muting never removes a parent");
    assert_eq!(commit.message, "✔️ A 💥 B ✔️ C\n");
    let tree = repo.find_tree(commit.tree)?;
    assert!(tree.find_entry("b").is_none(), "even B's clean file is omitted");
    assert!(tree.find_entry("c").is_some(), "later inputs still contribute");
    Ok(())
}

#[test]
fn converged_refs_keep_their_subscriptions_with_one_git_parent() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?.with_object_memory();
    let mut commit = candidate(&repo, &["A", "B"])?;
    let a_commit_id = input(&repo, "A")?.commit_id;
    let b_commit_id = input(&repo, "B")?.commit_id;
    let rewrites = HashMap::from([(b_commit_id, Some(a_commit_id))]);
    assert_eq!(
        rebuild(&repo, &mut commit, &mut References::default(), &rewrites, None, true)?,
        Rebuilt::Commit
    );
    assert_eq!(commit.parents.as_slice(), &[a_commit_id]);
    assert_eq!(
        Definition::from_commit(&commit)?
            .expect("a one-parent AutoMerge is still automatic")
            .inputs
            .len(),
        2
    );
    Ok(())
}

#[test]
fn malformed_recipes_are_rejected_and_pin_titles_do_not_expose_ref_names() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let mut commit = candidate(&repo, &["A", "B"])?;
    let mut definition = Definition::from_commit(&commit)?.expect("the candidate is automatic");
    definition.inputs[1].reference = "refs/worktree/tix/pins/abcd".try_into()?;
    definition.inputs[1].muted = true;
    assert_eq!(definition.title(), "✔️ A 💥 📌");
    commit.extra_headers.push((HEADER.into(), "invalid".into()));
    assert!(
        Definition::from_commit(&commit).is_err(),
        "malformed metadata must never drive a rewrite"
    );
    Ok(())
}
