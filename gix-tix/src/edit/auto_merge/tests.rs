use super::*;
use gix::prelude::ObjectIdExt;

fn input(repo: &gix::Repository, name: &str) -> Result<Choice> {
    let reference: gix::refs::FullName = format!("refs/heads/{name}").try_into()?;
    let commit_id = repo.find_reference(reference.as_ref())?.peel_to_commit()?.id;
    Ok(Choice {
        reference,
        commit_id,
        label: name.into(),
    })
}

fn named_input(choice: Choice) -> Input {
    Input {
        source: InputSource::Reference(choice.reference),
        commit_id: choice.commit_id,
        muted: false,
    }
}

fn candidate(repo: &gix::Repository, names: &[&str]) -> Result<gix::objs::Commit> {
    let mut commit = repo.head_commit()?.decode()?.into_owned()?;
    let definition = Definition {
        inputs: names
            .iter()
            .map(|name| input(repo, name).map(named_input))
            .collect::<Result<_>>()?,
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
fn unnamed_inputs_survive_creation_and_rewrites_without_tracking_refs() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    repo.edit_references([gix::refs::transaction::RefEdit::update(
        "HEAD".try_into()?,
        a.commit_id,
        gix::refs::transaction::PreviousValue::Any,
        "detach the unnamed input",
    )])?;
    repo.find_reference(a.reference.as_ref())?.delete()?;
    repo.find_reference(c.reference.as_ref())?.delete()?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::AddCommit(c.commit_id))?;
    let definition = Definition::from_commit(&repo.find_commit(merge_commit_id)?.decode()?.into_owned()?)?
        .context("the unnamed inputs form an AutoMerge")?;
    assert_eq!(
        definition.inputs.iter().map(|input| &input.source).collect::<Vec<_>>(),
        vec![
            &InputSource::Change(crate::change_id::for_commit(&repo, a.commit_id)?),
            &InputSource::Change(crate::change_id::for_commit(&repo, c.commit_id)?),
        ],
        "both inputs use change identities when no source ref exists"
    );
    assert!(
        crate::history::all_pins(&repo)?
            .iter()
            .all(crate::history::Pin::is_head),
        "the merge parents retain the inputs without ordinary pins"
    );
    let mut replacement = repo.find_commit(c.commit_id)?.decode()?.into_owned()?;
    replacement.message = "rewritten unnamed input\n".into();
    let outcome = rebase::perform(
        &repo,
        &super::super::loaded_graph(&repo)?,
        rebase::Edit::Replace {
            target: c.commit_id,
            commit: replacement,
        },
        rebase::Signature::RedoIfNeeded,
        rebase::Tree::CherryPick,
    )?
    .complete()?;
    let new_input_commit_id = outcome.map(c.commit_id).context("the input is rewritten")?;
    let new_merge_commit_id = outcome.map(merge_commit_id).context("its merge follows the rewrite")?;
    let definition = Definition::from_commit(&repo.find_commit(new_merge_commit_id)?.decode()?.into_owned()?)?
        .context("rewriting an input preserves AutoMerge")?;
    assert_eq!(definition.inputs[1].commit_id, new_input_commit_id);
    undo::record(&repo, "rewrite unnamed input", &outcome.ref_changes)?;
    undo::plan_undo(&repo)?
        .context("the rewrite is undoable")?
        .apply(&repo)?;
    assert_eq!(
        repo.head_id()?,
        merge_commit_id,
        "undo restores the old recipe and its parent"
    );
    undo::plan_redo(&repo)?
        .context("the rewrite is redoable")?
        .apply(&repo)?;
    assert_eq!(repo.head_id()?, new_merge_commit_id);
    let (remaining, _) = apply(
        &repo,
        new_merge_commit_id,
        Change::Remove(definition.inputs[1].source.clone()),
    )?;
    assert_eq!(
        remaining, a.commit_id,
        "removing a change input collapses to the remaining input"
    );
    Ok(())
}

#[test]
fn selected_commits_prefer_unambiguous_local_refs_and_otherwise_use_change_ids() -> gix_testtools::Result {
    use gix::refs::transaction::{PreviousValue, RefEdit};
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    let symbolic: FullName = "refs/worktree/tix/pins/symbolic".try_into()?;
    let pin: FullName = "refs/worktree/tix/pins/direct".try_into()?;
    repo.edit_references([RefEdit::update(
        symbolic.clone(),
        gix::refs::Target::Symbolic(c.reference.clone()),
        PreviousValue::Any,
        "remember C",
    )])?;
    let selections = additions(&repo, c.commit_id)?;
    assert_eq!(selections.len(), 1, "a symbolic pin and its branch identify one source");
    assert_eq!(selections[0].change, Change::Add(c.reference.clone()));
    assert_eq!(
        selections[0].merge_commit_id, a.commit_id,
        "off-HEAD selection targets HEAD"
    );
    repo.reference(pin.clone(), c.commit_id, PreviousValue::Any, "pin C")?;
    repo.reference("refs/heads/alias", c.commit_id, PreviousValue::Any, "another source")?;
    let selections = additions(&repo, c.commit_id)?;
    assert_eq!(
        selections
            .iter()
            .map(|selection| selection.change.clone())
            .collect::<Vec<_>>(),
        [
            Change::Add(c.reference.clone()),
            Change::Add("refs/heads/alias".try_into()?),
            Change::Add(pin.clone())
        ],
        "multiple sources use the picker with local branches before pins"
    );
    for name in [c.reference, symbolic, pin, "refs/heads/alias".try_into()?] {
        repo.find_reference(name.as_ref())?.delete()?;
    }
    repo.reference("refs/remotes/origin/C", c.commit_id, PreviousValue::Any, "remote C")?;
    repo.reference("refs/tags/C", c.commit_id, PreviousValue::Any, "tag C")?;
    let selections = additions(&repo, c.commit_id)?;
    assert_eq!(selections.len(), 1);
    assert_eq!(
        selections[0].change,
        Change::AddCommit(c.commit_id),
        "remote refs do not replace change tracking"
    );
    assert!(
        additions(&repo, input(&repo, "main")?.commit_id).is_err(),
        "ancestors are rejected before showing a picker"
    );
    assert!(
        additions(&repo, a.commit_id)?.len() > 1,
        "HEAD keeps the general ref picker"
    );

    repo.reference(
        "refs/heads/alias",
        a.commit_id,
        PreviousValue::Any,
        "ambiguous HEAD names",
    )?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, selections[0].change.clone())?;
    let definition = Definition::from_commit(&repo.find_commit(merge_commit_id)?.decode()?.into_owned()?)?
        .context("the selected commit creates an AutoMerge")?;
    assert_eq!(
        definition.inputs[0].source,
        InputSource::Change(crate::change_id::for_commit(&repo, a.commit_id)?)
    );
    assert_eq!(
        definition.inputs[1].source,
        InputSource::Change(crate::change_id::for_commit(&repo, c.commit_id)?)
    );
    Ok(())
}

#[test]
fn change_inputs_follow_the_rewritten_commit_without_following_inserted_children() -> gix_testtools::Result {
    for action in ["insert", "split", "remove"] {
        let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let a = input(&repo, "A")?;
        let c = input(&repo, "C")?;
        let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::AddCommit(c.commit_id))?;
        let mut child = repo.find_commit(c.commit_id)?.decode()?.into_owned()?;
        child.message = "new child\n".into();
        let edit = match action {
            "insert" => rebase::Edit::Insert {
                anchor: Some(c.commit_id),
                commit: child,
                reset_index: false,
            },
            "split" => {
                let mut lower = repo.find_commit(c.commit_id)?.decode()?.into_owned()?;
                lower.message = "rewritten lower part\n".into();
                rebase::Edit::Split {
                    target: c.commit_id,
                    source: lower,
                    upper: child,
                }
            }
            _ => rebase::Edit::Remove { target: c.commit_id },
        };
        let outcome = rebase::perform(
            &repo,
            &super::super::loaded_graph(&repo)?,
            edit,
            rebase::Signature::RedoIfNeeded,
            rebase::Tree::CherryPick,
        )?
        .complete()?;
        let updated = outcome.map(merge_commit_id).context("the merge has a destination")?;
        if action == "remove" {
            assert_eq!(
                updated, a.commit_id,
                "removing the change removes its subscription and collapses the merge"
            );
            continue;
        }
        let child_commit_id = outcome
            .map(c.commit_id)
            .context("the input ref moves to the new child")?;
        let expected = if action == "split" {
            repo.find_commit(child_commit_id)?
                .parent_ids()
                .next()
                .context("the split has a lower part")?
                .detach()
        } else {
            c.commit_id
        };
        let definition = Definition::from_commit(&repo.find_commit(updated)?.decode()?.into_owned()?)?
            .context("the merge keeps both inputs")?;
        assert_eq!(
            definition.inputs[1].commit_id, expected,
            "{action} preserves the logical input"
        );
        assert_ne!(
            expected, child_commit_id,
            "a change subscription never follows the inserted child"
        );
        assert_eq!(
            definition.inputs[1].source,
            InputSource::Change(crate::change_id::for_commit(&repo, c.commit_id)?)
        );
    }
    Ok(())
}

#[test]
fn todos_place_change_inputs_before_merging_and_remove_dropped_or_squashed_identities() -> gix_testtools::Result {
    for action in ["pick", "drop", "squash", "copy"] {
        let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
        let repo = crate::test_repository::open(fixture.path())?;
        let a = input(&repo, "A")?;
        let c = input(&repo, "C")?;
        let main = input(&repo, "main")?.commit_id;
        let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::AddCommit(c.commit_id))?;
        let mut duplicate = repo.find_commit(c.commit_id)?.decode()?.into_owned()?;
        crate::change_id::inherit(&repo, &mut duplicate, c.commit_id)?;
        duplicate.message = "another visible version\n".into();
        let duplicate_commit_id = repo.write_object(&duplicate)?.detach();
        repo.reference(
            "refs/heads/duplicate",
            duplicate_commit_id,
            gix::refs::transaction::PreviousValue::Any,
            "duplicate identity",
        )?;
        let mut base = changed_tree(
            &repo,
            repo.find_commit(main)?.decode()?.into_owned()?,
            "new-base",
            "new\n",
        )?;
        base.parents = [main].into_iter().collect();
        let base_commit_id = repo.write_object(&base)?.detach();
        let mut steps = vec![rebase::PlanStep {
            commit: rebase::PlanCommit::Pick(merge_commit_id),
            parent: rebase::PlanParent::Existing(main),
            squash: Vec::new(),
        }];
        let mut scope = vec![merge_commit_id];
        if action != "copy" {
            scope.push(c.commit_id);
        }
        if action == "squash" {
            scope.push(a.commit_id);
        }
        match action {
            "pick" | "copy" => steps.push(rebase::PlanStep {
                commit: if action == "pick" {
                    rebase::PlanCommit::Pick(c.commit_id)
                } else {
                    rebase::PlanCommit::Copy(c.commit_id)
                },
                parent: rebase::PlanParent::Existing(base_commit_id),
                squash: Vec::new(),
            }),
            "squash" => steps.push(rebase::PlanStep {
                commit: rebase::PlanCommit::Pick(a.commit_id),
                parent: rebase::PlanParent::Existing(main),
                squash: vec![c.commit_id],
            }),
            _ => {}
        }
        let plan = rebase::Plan {
            base: main,
            expected_refs: rebase::capture_refs(&repo, &scope, &[])?,
            scope,
            steps,
            checkout: Some(rebase::PlanCheckout {
                target: rebase::PlanParent::Step(0),
                reference: None,
            }),
        };
        let mut graph = super::super::loaded_graph(&repo)?;
        graph.switch_view(&[merge_commit_id, duplicate_commit_id], &[main]);
        let outcome = rebase::perform_plan(&repo, &graph, plan)?.complete()?;
        let result = repo
            .find_commit(outcome.selected.context("the todo selects its merge result")?)?
            .decode()?
            .into_owned()?;
        if action == "drop" || action == "squash" {
            assert!(
                Definition::from_commit(&result)?.is_none(),
                "{action} removes the tracked identity and collapses the merge"
            );
            assert_eq!(outcome.selected, Some(input(&repo, "A")?.commit_id));
        } else {
            let definition = Definition::from_commit(&result)?.context("the change subscription survives")?;
            let expected = if action == "pick" {
                outcome.map(c.commit_id).context("the pick is retained")?
            } else {
                c.commit_id
            };
            assert_eq!(
                definition.inputs[1].commit_id, expected,
                "{action} follows only the retained pick"
            );
            if action == "pick" {
                assert_ne!(expected, c.commit_id, "the later fork was rebased before its merge");
            }
        }
        assert_eq!(
            outcome
                .notice
                .as_deref()
                .is_some_and(|notice| notice.contains("ambiguous")),
            action == "copy",
            "explicit todo placements override ambiguous lookup; a copy leaves the original subscription alone"
        );
    }
    Ok(())
}

#[test]
fn change_lookup_is_bounded_and_retains_ambiguous_or_missing_inputs() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let c = input(&repo, "C")?;
    let base_commit_id = input(&repo, "main")?.commit_id;
    let change_id = crate::change_id::for_commit(&repo, c.commit_id)?;
    let tracked = Input {
        source: InputSource::Change(change_id),
        commit_id: c.commit_id,
        muted: false,
    };
    let mut replacement = repo.find_commit(c.commit_id)?.decode()?.into_owned()?;
    replacement.message = "another version\n".into();
    crate::change_id::inherit(&repo, &mut replacement, c.commit_id)?;
    let replacement_commit_id = repo.write_object(&replacement)?.detach();
    let mut graph =
        crate::history::HistoryGraph::for_commits(&repo, &[c.commit_id, replacement_commit_id, base_commit_id])?;
    for (tips, hidden, expected, ambiguous) in [
        (vec![replacement_commit_id], vec![], c.commit_id, false),
        (
            vec![replacement_commit_id],
            vec![base_commit_id],
            replacement_commit_id,
            false,
        ),
        (
            vec![c.commit_id, replacement_commit_id],
            vec![base_commit_id],
            c.commit_id,
            true,
        ),
        (vec![base_commit_id], vec![base_commit_id], c.commit_id, false),
    ] {
        graph.switch_view(&tips, &hidden);
        let mut refs = References::for_graph(&graph);
        for _ in 0..2 {
            assert_eq!(
                refs.resolve_input(&repo, &tracked, &HashMap::new(), None)?,
                Some(expected),
                "bounded unique matches relocate; repeated ambiguous lookups retain the selected version"
            );
        }
        assert_eq!(
            refs.notice().is_some(),
            ambiguous,
            "only ambiguity needs an explanation"
        );
        if hidden.is_empty() {
            assert!(
                refs.change_ids.is_none(),
                "an unbounded view never builds a change-ID index"
            );
        }
    }
    let a = input(&repo, "A")?;
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::AddCommit(c.commit_id))?;
    let mut graph = crate::history::HistoryGraph::for_commits(
        &repo,
        &[
            merge_commit_id,
            a.commit_id,
            c.commit_id,
            replacement_commit_id,
            base_commit_id,
        ],
    )?;
    graph.switch_view(&[merge_commit_id, replacement_commit_id], &[base_commit_id]);
    let operation = perform(
        &repo,
        &graph,
        merge_commit_id,
        Change::Remerge,
        rebase::CheckoutOptions::default(),
        |_| {},
    )?;
    assert!(
        operation.notice.contains("ambiguous"),
        "operation graph expansion preserves the bounded lookup and its diagnostic"
    );
    assert_eq!(
        repo.head_commit()?.parent_ids().nth(1),
        Some(c.commit_id.attach(&repo)),
        "ambiguous remerge retains the stored version"
    );
    graph.switch_view(&[merge_commit_id, replacement_commit_id], &[]);
    let operation = perform(
        &repo,
        &graph,
        merge_commit_id,
        Change::Remerge,
        rebase::CheckoutOptions::default(),
        |_| {},
    )?;
    assert!(
        !operation.notice.contains("ambiguous"),
        "showing hidden history disables lookup rather than widening it"
    );
    let mut view =
        crate::history::HistoryGraph::for_commits(&repo, &[a.commit_id, replacement_commit_id, base_commit_id])?;
    view.switch_view(&[a.commit_id, replacement_commit_id], &[base_commit_id]);
    graph.bounded_history = view.bounded_history;
    let mut affected = vec![c.commit_id];
    let preparation = prepare(&repo, &graph, &mut affected, Some(merge_commit_id), None)?;
    assert!(
        affected.contains(&merge_commit_id),
        "an exact edit outside the lookup projection still updates its merge"
    );
    assert!(
        preparation.optional.contains(&c.commit_id),
        "the exact input is replayed before merging, regardless of lookup candidates"
    );
    Ok(())
}

#[test]
fn nested_change_inputs_keep_conflicting_replays_muted_and_retained_by_parents() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    let main = input(&repo, "main")?.commit_id;
    repo.find_reference(c.reference.as_ref())?.delete()?;
    let (inner_commit_id, _) = apply(&repo, a.commit_id, Change::AddCommit(c.commit_id))?;
    super::super::time_travel::perform(
        fixture.path(),
        false,
        main,
        &super::super::loaded_graph(&repo)?,
        &[],
        &[],
        false,
    )?
    .complete()?;
    let (outer_commit_id, _) = apply(&repo, main, Change::AddCommit(inner_commit_id))?;
    let mut base = changed_tree(
        &repo,
        repo.find_commit(main)?.decode()?.into_owned()?,
        "c",
        "conflicting base\n",
    )?;
    base.parents = [main].into_iter().collect();
    let base_commit_id = repo.write_object(&base)?.detach();
    let outcome = rebase::perform_plan(
        &repo,
        &super::super::loaded_graph(&repo)?,
        rebase::Plan {
            base: main,
            scope: vec![c.commit_id],
            expected_refs: Vec::new(),
            checkout: None,
            steps: vec![rebase::PlanStep {
                parent: rebase::PlanParent::Existing(base_commit_id),
                commit: rebase::PlanCommit::Pick(c.commit_id),
                squash: Vec::new(),
            }],
        },
    )?
    .complete()?;
    let new_input_commit_id = outcome.map(c.commit_id).context("the input is replayed")?;
    let inner_commit_id = outcome.map(inner_commit_id).context("the inner merge is retained")?;
    let outer_commit_id = outcome.map(outer_commit_id).context("the outer merge is retained")?;
    let input_commit = repo.find_commit(new_input_commit_id)?.decode()?.into_owned()?;
    assert!(
        rebase::is_pending(&input_commit),
        "an optional conflicting replay stays pending"
    );
    let inner = repo.find_commit(inner_commit_id)?.decode()?.into_owned()?;
    let inner_definition = Definition::from_commit(&inner)?.context("inner recipe survives")?;
    assert_eq!(inner_definition.inputs[1].commit_id, new_input_commit_id);
    assert!(
        inner_definition.inputs[1].muted,
        "the conflicting change contributes no tree"
    );
    assert_eq!(
        input_commit.tree,
        repo.find_commit(c.commit_id)?.tree_id()?,
        "the original patch is retained for later replay"
    );
    let outer = repo.find_commit(outer_commit_id)?.decode()?.into_owned()?;
    let outer_definition = Definition::from_commit(&outer)?.context("outer recipe survives")?;
    assert_eq!(
        outer_definition.inputs[1].commit_id, inner_commit_id,
        "the nested change follows the rebuilt inner merge"
    );
    assert!(
        !outer_definition.inputs[1].muted,
        "a rebuilt inner merge can contribute its clean tree"
    );
    assert_eq!(repo.head_id()?, outer_commit_id);
    assert!(repo.find_tree(outer.tree)?.find_entry("c").is_none());
    assert!(
        crate::history::all_pins(&repo)?
            .iter()
            .all(|pin| pin.id != new_input_commit_id),
        "the unnamed replay is retained by merge parents without a tracking pin"
    );
    Ok(())
}

#[test]
fn change_subscriptions_can_select_another_version_and_remove_one_of_multiple_memberships() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    let change_id = crate::change_id::for_commit(&repo, c.commit_id)?;
    repo.find_reference(c.reference.as_ref())?.delete()?;
    let (first_merge_commit_id, _) = apply(&repo, a.commit_id, Change::AddCommit(c.commit_id))?;
    let mut replacement = repo.find_commit(c.commit_id)?.decode()?.into_owned()?;
    crate::change_id::inherit(&repo, &mut replacement, c.commit_id)?;
    replacement.message = "explicit alternative\n".into();
    let replacement_commit_id = repo.write_object(&replacement)?.detach();
    let main = input(&repo, "main")?.commit_id;
    let mut projection = crate::history::HistoryGraph::for_commits(
        &repo,
        &[
            first_merge_commit_id,
            a.commit_id,
            c.commit_id,
            replacement_commit_id,
            main,
        ],
    )?;
    projection.switch_view(&[first_merge_commit_id, replacement_commit_id], &[main]);
    let updated = perform(
        &repo,
        &projection,
        first_merge_commit_id,
        Change::AddCommit(replacement_commit_id),
        rebase::CheckoutOptions::default(),
        |_| {},
    )?
    .result
    .context("an explicit alternative updates the merge")?
    .complete()?
    .selected
    .context("the updated merge is selected")?;
    let mut recipe = repo.find_commit(updated)?.decode()?.into_owned()?;
    let definition = Definition::from_commit(&recipe)?.context("the recipe survives replacement")?;
    assert_eq!(
        definition.inputs.len(),
        2,
        "another version replaces the same identity instead of adding it twice"
    );
    assert_eq!(definition.inputs[1].commit_id, replacement_commit_id);
    assert_eq!(
        definition.title(),
        format!("✔️ A ✔️ {}", change_id.to_reverse_hex_with_len(7)).as_str()
    );
    definition.store(&mut recipe);
    assert_eq!(
        Definition::from_commit(&recipe)?,
        Some(definition),
        "ref and change metadata round-trip together"
    );
    assert_eq!(recipe.parents.as_slice(), &[a.commit_id, replacement_commit_id]);

    super::super::time_travel::perform(fixture.path(), false, a.commit_id, &graph(&repo)?, &[], &[], false)?
        .complete()?;
    let (second_merge_commit_id, _) = apply(&repo, a.commit_id, Change::Add(input(&repo, "B")?.reference))?;
    let (second_merge_commit_id, _) = apply(&repo, second_merge_commit_id, Change::AddCommit(replacement_commit_id))?;
    let memberships = removals(&repo, &graph(&repo)?, replacement_commit_id, false)?;
    assert_eq!(
        memberships.len(),
        2,
        "unnamed inputs offer the same membership picker as refs"
    );
    assert!(
        memberships
            .iter()
            .all(|selection| selection.change == Change::Remove(InputSource::Change(change_id)))
    );
    let selected = memberships
        .iter()
        .find(|selection| selection.merge_commit_id == updated)
        .context("the first merge is offered")?;
    apply(&repo, selected.merge_commit_id, selected.change.clone())?;
    assert_eq!(
        repo.head_id()?,
        second_merge_commit_id,
        "removing from another merge preserves the checkout"
    );
    assert_eq!(
        removals(&repo, &graph(&repo)?, replacement_commit_id, false)?.len(),
        1,
        "only the selected membership is removed"
    );
    Ok(())
}

#[test]
fn selections_are_revalidated_after_head_or_input_moves() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    let stale = additions(&repo, c.commit_id)?.remove(0);
    let (merge_commit_id, _) = apply(&repo, a.commit_id, Change::AddCommit(c.commit_id))?;
    assert!(
        perform(
            &repo,
            &graph(&repo)?,
            stale.merge_commit_id,
            stale.change,
            rebase::CheckoutOptions::default(),
            |_| {}
        )
        .is_err(),
        "a selection prepared for a different HEAD cannot silently target the new HEAD"
    );
    let mut descendant = repo.find_commit(merge_commit_id)?.decode()?.into_owned()?;
    descendant.extra_headers.clear();
    descendant.parents = [merge_commit_id].into_iter().collect();
    descendant.message = "child of AutoMerge\n".into();
    let descendant_commit_id = repo.write_object(&descendant)?.detach();
    assert!(
        additions(&repo, descendant_commit_id).is_err(),
        "the picker rejects descendants of an AutoMerge HEAD"
    );
    assert!(
        apply(&repo, merge_commit_id, Change::AddCommit(descendant_commit_id)).is_err(),
        "execution rechecks the cycle guard"
    );
    let (same, notice) = apply(&repo, merge_commit_id, Change::AddCommit(c.commit_id))?;
    assert_eq!(same, merge_commit_id);
    assert!(notice.contains("ancestor"), "an included change explains the no-op");
    Ok(())
}

#[test]
fn creates_extends_refreshes_and_removes_inputs_without_moving_source_refs() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(fixture.path())
            .args(["pack-refs", "--all", "--prune"])
            .status()?
            .success(),
        "AutoMerge creation also resolves packed inputs"
    );
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let b = input(&repo, "B")?;
    let c = input(&repo, "C")?;
    let input_log = repo.git_dir().join("logs/refs/heads/B");
    let before_log = std::fs::read(&input_log)?;
    let outcome = perform(
        &repo,
        &graph(&repo)?,
        a.commit_id,
        Change::Add(b.reference.clone()),
        rebase::CheckoutOptions::default(),
        |_| {},
    )?
    .result
    .context("creation prepares a merge")?
    .complete()?;
    let merge_commit_id = outcome.selected.context("creation selects the merge")?;
    assert!(
        outcome
            .ref_changes
            .iter()
            .all(|change| change.name != a.reference && change.name != b.reference),
        "unchanged inputs do not become undo changes"
    );
    assert_eq!(
        std::fs::read(input_log)?,
        before_log,
        "reading inputs leaves their reflogs intact"
    );
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
    let (updated, _) = apply(
        &repo,
        updated,
        Change::Remove(InputSource::Reference(b.reference.clone())),
    )?;
    let (collapsed, _) = apply(
        &repo,
        updated,
        Change::Remove(InputSource::Reference(input(&repo, "C")?.reference)),
    )?;
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
    let (_, _) = apply(
        &repo,
        merge_commit_id,
        Change::Remove(InputSource::Reference(pin.clone())),
    )?;
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
        notice.contains("no inputs"),
        "the unchanged result explains the missing inputs"
    );
    Ok(())
}

#[test]
fn dirty_checkout_aborts_without_moving_refs() -> gix_testtools::Result {
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
    Ok(())
}

#[test]
fn concurrent_input_changes_are_picked_up_by_the_next_remerge() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_writable("auto_merge.sh")?;
    let repo = crate::test_repository::open(fixture.path())?;
    let a = input(&repo, "A")?;
    let c = input(&repo, "C")?;
    let mut advanced = changed_tree(
        &repo,
        repo.find_commit(c.commit_id)?.decode()?.into_owned()?,
        "c",
        "advanced C\n",
    )?;
    advanced.parents = [c.commit_id].into_iter().collect();
    let advanced_commit_id = repo.write_object(&advanced)?.detach();
    let mut raced = false;
    let outcome = perform(
        &repo,
        &graph(&repo)?,
        a.commit_id,
        Change::Add(c.reference.clone()),
        rebase::CheckoutOptions::default(),
        |progress| {
            if progress.processed > 0 && !raced {
                repo.reference(
                    c.reference.clone(),
                    advanced_commit_id,
                    gix::refs::transaction::PreviousValue::MustExistAndMatch(c.commit_id.into()),
                    "concurrent move",
                )
                .expect("the fixture permits a concurrent ref update");
                raced = true;
            }
        },
    )?
    .result
    .context("creation prepares a merge")?
    .complete()?;
    assert!(raced, "the input advances after preparation starts");
    let merge_commit_id = outcome.selected.context("creation selects the merge")?;
    assert_eq!(repo.head_id()?, merge_commit_id, "the snapshot merge is checked out");
    let commit = repo.find_commit(merge_commit_id)?.decode()?.into_owned()?;
    assert_eq!(
        commit.parents.as_slice(),
        &[a.commit_id, c.commit_id],
        "the merge keeps the input commits it actually used"
    );
    assert_eq!(
        Definition::from_commit(&commit)?
            .context("the snapshot keeps its recipe")?
            .inputs,
        vec![named_input(a), named_input(c)],
        "the recipe records the same snapshot as the merge parents"
    );
    assert_eq!(
        input(&repo, "C")?.commit_id,
        advanced_commit_id,
        "publication preserves the concurrent input advance"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.path().join("c"))?,
        "C\n",
        "checkout uses the snapshot tree"
    );
    let (refreshed_commit_id, _) = apply(&repo, merge_commit_id, Change::Remerge)?;
    assert_ne!(
        refreshed_commit_id, merge_commit_id,
        "remerge catches up with the input"
    );
    assert_eq!(
        repo.find_commit(refreshed_commit_id)?
            .parent_ids()
            .last()
            .context("the refreshed merge retains its inputs")?,
        advanced_commit_id,
        "the refreshed merge includes the advanced input"
    );
    assert_eq!(
        std::fs::read_to_string(fixture.path().join("c"))?,
        "advanced C\n",
        "remerge checks out the advanced input's tree"
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
                repo.find_reference(input.source.reference().expect("this input has a ref").as_ref())?
                    .peel_to_commit()?
                    .id,
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
    definition.inputs[1].source = InputSource::Reference("refs/worktree/tix/pins/abcd".try_into()?);
    definition.inputs[1].muted = true;
    assert_eq!(definition.title(), "✔️ A 💥 📌");
    let change_id = crate::change_id::for_commit(&repo, definition.inputs[1].commit_id)?;
    definition.inputs[1].source = InputSource::Change(change_id);
    assert_eq!(
        definition.title(),
        format!("✔️ A 💥 {}", change_id.to_reverse_hex_with_len(7)).as_str()
    );
    definition.inputs[0].source = InputSource::Change(change_id);
    definition.store(&mut commit);
    assert!(
        Definition::from_commit(&commit).is_err(),
        "two versions of one change cannot become duplicate subscriptions"
    );
    commit.extra_headers.clear();
    commit.extra_headers.push((
        HEADER.into(),
        format!("1 {} included change-id invalid", definition.inputs[0].commit_id).into(),
    ));
    assert!(
        Definition::from_commit(&commit).is_err(),
        "invalid change IDs cannot drive a rewrite"
    );
    commit.extra_headers.clear();
    commit.extra_headers.push((HEADER.into(), "invalid".into()));
    assert!(
        Definition::from_commit(&commit).is_err(),
        "malformed metadata must never drive a rewrite"
    );
    Ok(())
}
