#[test]
fn query_and_mutate_a_configured_notes_ref() -> crate::Result {
    let (mut repo, _tmp) = crate::util::basic_rw_repo()?;
    let mut config = repo.config_snapshot_mut();
    config.set_value(&gix::config::tree::Core::NOTES_REF, "refs/notes/review")?;
    config.commit()?;

    assert!(
        repo.try_find_reference("refs/notes/review")?.is_none(),
        "configuring a notes reference does not create it"
    );
    let target = repo.write_blob(b"annotated")?;
    let mut notes = repo.notes()?;
    assert_eq!(
        notes.default_ref().map(ToString::to_string).as_deref(),
        Some("refs/notes/review"),
        "core.notesRef configures the default notes reference"
    );
    assert_eq!(
        notes.refs().map(ToString::to_string).collect::<Vec<_>>(),
        ["refs/notes/review"],
        "the default reference is initially the only selected notes reference"
    );
    assert!(notes.get(target)?.is_empty(), "the target initially has no notes");

    assert_eq!(
        notes.replace("review", target, b"first")?,
        None,
        "adding the first note does not replace an existing note"
    );
    assert!(
        repo.try_find_reference("refs/notes/review")?.is_some(),
        "adding a note creates its previously absent notes reference"
    );
    let found = notes.get(target)?;
    assert_eq!(found.len(), 1, "the target has exactly one note after insertion");
    assert_eq!(
        found[0].reference, "refs/notes/review",
        "the note is found through the configured notes reference"
    );
    assert_eq!(found[0].blob.data, b"first", "the inserted note data is returned");
    drop(found);

    let previous = notes
        .replace("review", target, b"second")?
        .expect("the first note is replaced");
    assert_eq!(
        repo.find_blob(previous)?.data,
        b"first",
        "replacement returns the previous note object"
    );
    assert_eq!(
        notes.remove("review", target)?,
        Some(repo.write_blob(b"second")?),
        "removal returns the repository-attached ID of the replacement note"
    );
    assert!(notes.get(target)?.is_empty(), "the target has no notes after removal");
    assert!(
        repo.try_find_reference("refs/notes/review")?.is_some(),
        "removing the last note preserves the notes reference"
    );
    Ok(())
}

#[test]
fn mutations_follow_symbolic_references_to_their_direct_target() -> crate::Result {
    use gix::refs::{
        FullName, Target, TargetRef,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    };

    fn full_name(name: &str) -> FullName {
        name.try_into().expect("test reference names are valid")
    }

    fn create_symbolic_ref(repo: &gix::Repository, name: &str, target: &str) -> crate::Result {
        repo.edit_reference(RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: false,
                    message: "create symbolic notes reference".into(),
                },
                expected: PreviousValue::MustNotExist,
                new: Target::Symbolic(full_name(target)),
            },
            name: full_name(name),
            deref: false,
        })?;
        Ok(())
    }

    fn assert_symbolic_target(repo: &gix::Repository, name: &str, target: &str) -> crate::Result {
        assert_eq!(
            repo.find_reference(name)?.target(),
            TargetRef::Symbolic(target.try_into()?),
            "{name} remains symbolic and points to its original target"
        );
        Ok(())
    }

    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let annotated_blob_id = repo.write_blob(b"annotated")?;
    let direct = "refs/notes/direct";
    let inner_alias = "refs/notes/inner-alias";
    let outer_alias = "refs/notes/outer-alias";

    repo.notes()?.replace(direct, annotated_blob_id, b"first")?;

    let initial_commit_id = repo.find_reference(direct)?.id();
    create_symbolic_ref(&repo, inner_alias, direct)?;
    create_symbolic_ref(&repo, outer_alias, inner_alias)?;

    let mut notes = repo.notes()?.with_refs([outer_alias, direct])?;
    let found = notes.get(annotated_blob_id)?;
    assert_eq!(
        found.len(),
        2,
        "the alias and its direct target both provide the note, it's what Git does"
    );
    assert!(
        found.iter().all(|note| note.blob.data == b"first"),
        "both selected references initially cache the same notes tree"
    );
    drop(found);

    let previous_note_blob_id = notes
        .replace(outer_alias, annotated_blob_id, b"second")?
        .expect("the existing note is replaced through the symbolic chain");
    assert_eq!(
        repo.find_blob(previous_note_blob_id)?.data,
        b"first",
        "replacement returns the previous note"
    );
    assert_symbolic_target(&repo, outer_alias, inner_alias)?;
    assert_symbolic_target(&repo, inner_alias, direct)?;

    let replacement_commit_id = repo.find_reference(direct)?.id();
    assert_ne!(
        replacement_commit_id, initial_commit_id,
        "replacement advances the ultimate direct target"
    );
    assert_eq!(
        repo.find_commit(replacement_commit_id)?
            .parent_ids()
            .collect::<Vec<_>>(),
        [initial_commit_id],
        "the replacement commit parents the previous direct target"
    );
    let found = notes.get(annotated_blob_id)?;
    assert_eq!(found.len(), 2, "both selected references still provide the note");
    assert!(
        found.iter().all(|note| note.blob.data == b"second"),
        "cached alias and direct-target roots both observe the replacement"
    );
    drop(found);

    assert_eq!(
        notes.remove(outer_alias, annotated_blob_id)?,
        Some(repo.write_blob(b"second")?),
        "removal through the symbolic chain returns the repository-attached note ID"
    );
    assert_symbolic_target(&repo, outer_alias, inner_alias)?;
    assert_symbolic_target(&repo, inner_alias, direct)?;

    let removal_commit_id = repo.find_reference(direct)?.id();
    assert_ne!(
        removal_commit_id, replacement_commit_id,
        "removal advances the ultimate direct target"
    );
    assert_eq!(
        repo.find_commit(removal_commit_id)?.parent_ids().collect::<Vec<_>>(),
        [replacement_commit_id],
        "the removal commit parents the previous direct target"
    );
    assert!(
        notes.get(annotated_blob_id)?.is_empty(),
        "cached alias and direct-target roots both observe the removal"
    );
    Ok(())
}

#[test]
fn mutations_reject_non_commit_notes_ref_targets() -> crate::Result {
    use gix::refs::transaction::PreviousValue;

    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let annotated_blob_id = repo.write_blob(b"annotated")?;
    let valid_notes_ref = "refs/notes/valid";
    repo.notes()?.replace(valid_notes_ref, annotated_blob_id, b"note")?;

    let valid_notes_commit_id = repo.find_reference(valid_notes_ref)?.id();
    let notes_tree_id = repo.find_commit(valid_notes_commit_id)?.tree_id()?;
    let notes_tag_id = repo
        .tag(
            "notes-history",
            valid_notes_commit_id,
            gix::object::Kind::Commit,
            None,
            "tagged notes history",
            PreviousValue::MustNotExist,
        )?
        .id();

    let tree_ref = "refs/notes/tree";
    repo.reference(
        tree_ref,
        notes_tree_id,
        PreviousValue::MustNotExist,
        "create malformed tree-based notes reference",
    )?;
    repo.notes()?
        .replace(tree_ref, annotated_blob_id, b"replacement")
        .expect_err("a notes tree cannot be used as a commit parent");
    assert_eq!(
        repo.find_reference(tree_ref)?.id(),
        notes_tree_id,
        "failed replacement leaves the tree-based notes reference unchanged"
    );

    let tag_ref = "refs/notes/tag";
    repo.reference(
        tag_ref,
        notes_tag_id,
        PreviousValue::MustNotExist,
        "create malformed tag-based notes reference",
    )?;
    repo.notes()?
        .remove(tag_ref, annotated_blob_id)
        .expect_err("an annotated tag cannot be used as a commit parent");
    assert_eq!(
        repo.find_reference(tag_ref)?.id(),
        notes_tag_id,
        "failed removal leaves the tag-based notes reference unchanged"
    );
    Ok(())
}

#[test]
fn custom_commit_message_is_used_for_mutations() -> crate::Result {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let annotated_blob_id = repo.write_blob(b"annotated")?;
    let mut notes = repo.notes()?.with_commit_message("custom notes update");

    notes.replace("review", annotated_blob_id, b"note")?;
    {
        let commit = repo.find_reference("refs/notes/review")?.peel_to_commit()?;
        assert_eq!(
            commit.message_raw()?,
            "custom notes update",
            "replacement uses the configured commit message"
        );
    }

    notes.remove("review", annotated_blob_id)?;
    let commit = repo.find_reference("refs/notes/review")?.peel_to_commit()?;
    assert_eq!(
        commit.message_raw()?,
        "custom notes update",
        "removal uses the configured commit message"
    );
    Ok(())
}

#[test]
fn query_and_mutate_multiple_notes_refs() -> crate::Result {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let target = repo.write_blob(b"annotated")?;
    let notes_refs = ["refs/notes/review", "refs/notes/security"];
    let mut notes = repo.notes()?.with_refs(notes_refs)?;
    assert_eq!(
        notes.refs().map(ToString::to_string).collect::<Vec<_>>(),
        notes_refs,
        "explicitly selected notes references are available in lookup order"
    );

    for name in notes_refs {
        assert!(
            repo.try_find_reference(name)?.is_none(),
            "selecting {name} does not create it"
        );
    }

    assert_eq!(
        notes.replace("review", target, b"review note")?,
        None,
        "adding the review note creates a new mapping"
    );
    assert_eq!(
        notes.replace("notes/security", target, b"security note")?,
        None,
        "adding the security note creates an independent mapping"
    );

    for name in notes_refs {
        assert!(
            repo.try_find_reference(name)?.is_some(),
            "writing a note auto-creates {name}"
        );
    }

    let unmatched = repo.notes()?.with_refs(["/refs/notes/*", "refs/notes/*/"])?;
    assert_eq!(
        unmatched.refs().count(),
        0,
        "reference names have neither a leading nor trailing slash"
    );

    notes = notes.with_refs(["refs/notes/revie?", "refs/notes/[s]ecurity"])?;
    assert_eq!(
        notes.refs().map(ToString::to_string).collect::<Vec<_>>(),
        notes_refs,
        "question-mark and bracket globs expand without requiring an asterisk"
    );

    let found = notes.get(target)?;
    assert_eq!(found.len(), 2, "one note is returned from each selected reference");
    assert_eq!(
        found[0].reference, "refs/notes/review",
        "the first note follows the selected reference order"
    );
    assert_eq!(found[0].blob.data, b"review note", "the review note data is returned");
    assert_eq!(
        found[1].reference, "refs/notes/security",
        "the second note follows the selected reference order"
    );
    assert_eq!(
        found[1].blob.data, b"security note",
        "the security note data is returned"
    );
    Ok(())
}

#[test]
fn add_to_an_exact_fully_qualified_reference() -> crate::Result {
    let (repo, _tmp) = crate::util::basic_rw_repo()?;
    let target = repo.write_blob(b"annotated")?.detach();
    let reference: gix::refs::FullName = "refs/worktree/tix/notes".try_into()?;
    let mut notes = repo.notes()?;

    assert_eq!(
        notes.replace_at_ref(reference.as_ref(), target, b"[commit]\n\ttodo = true\n")?,
        None
    );
    let mut exact = repo.notes()?.with_refs([reference.as_bstr()])?;
    assert_eq!(
        exact.get(target)?.first().map(|note| note.blob.data.as_slice()),
        Some(b"[commit]\n\ttodo = true\n".as_slice()),
        "the exact worktree-local ref stores the note"
    );
    assert!(
        repo.try_find_reference("refs/notes/refs/worktree/tix/notes")?.is_none(),
        "exact writes do not apply notes shorthand"
    );
    Ok(())
}
