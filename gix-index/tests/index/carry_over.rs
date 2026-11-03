use bstr::ByteSlice;
use filetime::FileTime;
use gix_index::{File, State, entry, extension::fs_monitor::Token};
use gix_testtools::TestResult;

#[test]
fn rebuilt_entries_keep_only_matching_monitor_validity() -> TestResult {
    for token in [
        Token::V1 { nanos_since_1970: 42 },
        Token::V2 {
            token: "previous".into(),
        },
    ] {
        let mut previous = State::new(gix_testtools::object_hash());
        for path in [
            "a-valid",
            "b-dirty",
            "c-id",
            "d-mode",
            "e-stat",
            "f-stage",
            "g-ita",
            "h-sparse",
            "i-submodule",
            "j-removed",
            "k-remove-flag",
            "l-symlink",
        ] {
            push(&mut previous, path);
        }
        previous.entries_mut()[1].flags.remove(entry::Flags::FSMONITOR_VALID);
        previous.entries_mut()[7].mode = entry::Mode::DIR;
        previous.entries_mut()[8].mode = entry::Mode::COMMIT;
        previous.entries_mut()[11].mode = entry::Mode::SYMLINK;
        previous.set_fs_monitor_token(Some(token.clone()));
        let previous = roundtrip(&previous)?;
        let mut rebuilt = previous.clone();
        for (entry, path) in rebuilt.entries_mut_with_paths() {
            entry.flags.insert(entry::Flags::FSMONITOR_VALID);
            match path.as_bytes() {
                b"c-id" => entry.id = gix_hash::ObjectId::empty_blob(gix_testtools::object_hash()),
                b"d-mode" => entry.mode = entry::Mode::FILE_EXECUTABLE,
                b"e-stat" => entry.stat.size += 1,
                b"f-stage" => entry.flags |= entry::Flags::from_stage(entry::Stage::Ours),
                b"g-ita" => entry.flags |= entry::Flags::INTENT_TO_ADD | entry::Flags::EXTENDED,
                b"k-remove-flag" => entry.flags |= entry::Flags::REMOVE,
                _ => {}
            }
        }
        rebuilt.remove_entries(|_, path, _| path == "j-removed");
        push(&mut rebuilt, "0-new");
        rebuilt.sort_entries();
        rebuilt.set_fs_monitor_token(Some(Token::V2 {
            token: "replace-me".into(),
        }));

        rebuilt.carry_over_fsmonitor_data_from(&previous)?;
        let monitor = rebuilt.fs_monitor().expect("the old checkpoint is retained");
        assert_eq!(monitor.token(), &token, "carry-forward keeps the source checkpoint");
        assert_eq!(
            monitor.valid_entries(rebuilt.entries().len()),
            Some(vec![
                false, true, false, false, false, false, false, false, false, false, false, true
            ]),
            "new positions map by identity and stat; preexisting flags do not grant validity"
        );
        let written = roundtrip(&rebuilt)?;
        assert_eq!(
            written.fs_monitor().expect("checkpoint was written").valid_entries(11),
            Some(vec![
                false, true, false, false, false, false, false, false, false, false, true
            ]),
            "serialization retains mapped validity while omitting removed entries"
        );
    }
    Ok(())
}

#[test]
fn changing_assume_valid_or_skip_worktree_requires_a_new_check() -> TestResult {
    for flag in [entry::Flags::ASSUME_VALID, entry::Flags::SKIP_WORKTREE] {
        for was_set in [false, true] {
            let mut previous = State::new(gix_testtools::object_hash());
            push(&mut previous, "a");
            previous.entries_mut()[0].flags.set(flag, was_set);
            previous.set_fs_monitor_token(Some(Token::V1 { nanos_since_1970: 1 }));
            let mut rebuilt = previous.clone();
            rebuilt.entries_mut()[0].flags.set(flag, !was_set);
            rebuilt.carry_over_fsmonitor_data_from(&previous)?;
            assert_eq!(
                rebuilt
                    .fs_monitor()
                    .expect("the checkpoint is retained")
                    .valid_entries(1),
                Some(vec![false]),
                "{flag:?}, previously {was_set}: changing checking policy invalidates monitor validity, as in Git"
            );
        }
    }
    Ok(())
}

#[test]
fn untracked_cache_survives_unchanged_entries_and_invalidates_affected_paths() -> TestResult {
    let previous = cached_index()?;
    let mut rebuilt = State::clone(&previous);
    rebuilt.entries_mut();
    rebuilt.carry_over_fsmonitor_data_from(&previous)?;
    assert_eq!(
        format!("{:?}", rebuilt.untracked()),
        format!("{:?}", previous.untracked()),
        "an unchanged rebuild preserves UNTR even without FSMN"
    );

    for change in [
        "add",
        "remove",
        "remove-flag",
        "ignore-id",
        "ignore-remove",
        "root-ignore",
    ] {
        let mut rebuilt = State::clone(&previous);
        match change {
            "add" => push(&mut rebuilt, "tracked-dir-with-ignore/new"),
            "remove" => rebuilt.remove_entries(|_, path, _| path == "tracked-dir-with-ignore/tracked-file"),
            "ignore-remove" => rebuilt.remove_entries(|_, path, _| path == "tracked-dir-with-ignore/.gitignore"),
            "root-ignore" => push(&mut rebuilt, ".gitignore"),
            _ => {
                let path = if change == "ignore-id" {
                    "tracked-dir-with-ignore/.gitignore"
                } else {
                    "tracked-dir-with-ignore/tracked-file"
                };
                let entry = rebuilt
                    .entry_mut_by_path_and_stage(path.into(), entry::Stage::Unconflicted)
                    .expect("the fixture tracks this path");
                if change == "ignore-id" {
                    entry.id = gix_testtools::object_hash().null();
                } else {
                    entry.flags.insert(entry::Flags::REMOVE);
                }
            }
        }
        rebuilt.sort_entries();
        rebuilt.carry_over_fsmonitor_data_from(&previous)?;
        for directory in rebuilt.untracked().expect("the cache is carried forward").directories() {
            let invalid = match directory.name().as_bytes() {
                b"" | b"tracked-dir-with-ignore" => true,
                b"nested-untracked-dir" | b"deep-untracked-dir" => change.contains("ignore"),
                _ => change == "root-ignore",
            };
            assert_eq!(
                directory.stat().is_none(),
                invalid,
                "{change}: membership changes invalidate ancestors, and ignore changes also invalidate descendants: {:?}",
                directory.name()
            );
        }
    }

    rebuilt.carry_over_fsmonitor_data_from(&State::new(previous.object_hash()))?;
    assert!(
        rebuilt.untracked().is_none(),
        "an absent source cache removes the destination cache"
    );
    Ok(())
}

#[test]
fn malformed_monitor_data_fails_without_changing_the_destination() -> TestResult {
    let mut previous = State::new(gix_testtools::object_hash());
    push(&mut previous, "a");
    previous.set_fs_monitor_token(Some(Token::V1 { nanos_since_1970: 1 }));
    let mut bytes = Vec::new();
    File::from_state(previous, std::path::PathBuf::new()).write_to(&mut bytes, Default::default())?;
    let extension = bytes
        .windows(4)
        .position(|window| window == b"FSMN")
        .expect("FSMN was written");
    let bitmap = extension + 8 + 4 + 8 + 4;
    for malformed in ["out-of-bounds", "missing-bits"] {
        let mut bytes = bytes.clone();
        if malformed == "out-of-bounds" {
            bytes[bitmap..bitmap + 4].copy_from_slice(&2u32.to_be_bytes());
        } else {
            bytes[bitmap + 8..bitmap + 24].fill(0);
        }
        let (previous, _) = State::from_bytes(
            &bytes,
            FileTime::now(),
            gix_testtools::object_hash(),
            Default::default(),
        )?;
        let mut destination = cached_index()?;
        destination.set_fs_monitor_token(Some(Token::V2 {
            token: "untouched".into(),
        }));
        destination.set_untracked(cached_index()?.untracked().cloned());
        let before = format!("{destination:?}");
        assert!(
            destination.carry_over_fsmonitor_data_from(&previous).is_err(),
            "{malformed}: corrupt validity cannot become clean entries"
        );
        assert_eq!(
            format!("{destination:?}"),
            before,
            "failure leaves the destination intact"
        );
    }
    Ok(())
}

#[test]
fn absent_source_extensions_clear_flags_and_different_hashes_fail_atomically() -> TestResult {
    let mut destination = cached_index()?;
    let cache = destination.set_untracked(None);
    for entry in destination.entries_mut() {
        entry.flags.insert(entry::Flags::FSMONITOR_VALID);
    }
    destination.set_fs_monitor_token(Some(Token::V1 { nanos_since_1970: 1 }));
    destination.set_untracked(cache);
    let other_hash = match destination.object_hash() {
        gix_hash::Kind::Sha1 => gix_hash::Kind::Sha256,
        _ => gix_hash::Kind::Sha1,
    };
    let before = format!("{destination:?}");
    assert!(
        destination
            .carry_over_fsmonitor_data_from(&State::new(other_hash))
            .is_err(),
        "different hash kinds cannot share worktree caches"
    );
    assert_eq!(
        format!("{destination:?}"),
        before,
        "failure leaves the destination intact"
    );

    let empty = State::new(destination.object_hash());
    destination.carry_over_fsmonitor_data_from(&empty)?;
    assert!(
        destination.fs_monitor().is_none(),
        "an absent checkpoint is carried over too"
    );
    assert!(
        destination.untracked().is_none(),
        "an absent untracked cache is carried over too"
    );
    assert!(
        destination
            .entries()
            .iter()
            .all(|entry| !entry.flags.contains(entry::Flags::FSMONITOR_VALID)),
        "old destination flags cannot outlive their checkpoint"
    );
    Ok(())
}

fn push(index: &mut State, path: &str) {
    index.dangerously_push_entry(
        Default::default(),
        gix_testtools::object_hash().null(),
        entry::Flags::FSMONITOR_VALID,
        entry::Mode::FILE,
        path.into(),
    );
}

fn roundtrip(index: &State) -> TestResult<State> {
    let mut bytes = Vec::new();
    File::from_state(index.clone(), std::path::PathBuf::new()).write_to(&mut bytes, Default::default())?;
    Ok(State::from_bytes(&bytes, FileTime::now(), index.object_hash(), Default::default())?.0)
}

fn cached_index() -> TestResult<File> {
    Ok(File::at(
        crate::fixture_index_path_needs_archive("untracked_cache_nested"),
        gix_testtools::object_hash(),
        false,
        Default::default(),
    )?)
}
