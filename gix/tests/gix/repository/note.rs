use gix::config::tree::Key;

#[test]
fn query_and_mutate_a_configured_notes_ref() -> crate::Result {
    assert_eq!(
        gix::config::tree::Core::NOTES_REF.environment_override(),
        Some("GIT_NOTES_REF")
    );

    let (mut repo, _tmp) = crate::util::basic_rw_repo()?;
    let mut config = repo.config_snapshot_mut();
    config.set_value(&gix::config::tree::Core::NOTES_REF, "refs/notes/review")?;
    config.set_value(&gix::config::tree::User::NAME, "user")?;
    config.set_value(&gix::config::tree::User::EMAIL, "user@example.com")?;
    config.commit()?;

    let target = repo.write_blob(b"annotated")?.detach();
    let mut notes = repo.notes().map_err(gix::Exn::into_error)?;
    assert_eq!(
        notes.default_ref().map(ToString::to_string).as_deref(),
        Some("refs/notes/review")
    );
    assert!(notes.get(target).map_err(gix::Exn::into_error)?.is_empty());

    assert_eq!(
        notes.add("review", target, b"first").map_err(gix::Exn::into_error)?,
        None
    );
    let found = notes.get(target).map_err(gix::Exn::into_error)?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].reference.to_string(), "refs/notes/review");
    assert_eq!(found[0].blob.data, b"first");
    drop(found);

    let previous = notes
        .add("review", target, b"second")
        .map_err(gix::Exn::into_error)?
        .expect("the first note is replaced");
    assert_eq!(repo.find_blob(previous)?.data, b"first");
    assert_eq!(
        notes.remove("review", target).map_err(gix::Exn::into_error)?,
        Some(repo.write_blob(b"second")?.detach())
    );
    assert!(notes.get(target).map_err(gix::Exn::into_error)?.is_empty());
    Ok(())
}
