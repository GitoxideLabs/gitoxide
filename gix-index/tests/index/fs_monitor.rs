use filetime::FileTime;
use gix_index::{State, entry, extension::fs_monitor::Token};
use gix_testtools::TestResult;

#[test]
fn checkpoints_and_valid_entries_roundtrip() -> TestResult {
    for token in [
        Token::V1 {
            nanos_since_1970: 1_234_567_890,
        },
        Token::V2 {
            token: b"opaque:\xff:checkpoint".into(),
        },
    ] {
        let mut index = State::new(gix_testtools::object_hash());
        push(&mut index, "a", entry::Flags::FSMONITOR_VALID, entry::Mode::FILE);
        push(&mut index, "b", entry::Flags::empty(), entry::Mode::FILE);
        index.set_fs_monitor_token(Some(token.clone()));
        let written = decode(&encode(&index)?)?;
        let monitor = written.fs_monitor().expect("the extension was written");
        assert_eq!(monitor.token(), &token, "checkpoints retain their protocol and bytes");
        assert_eq!(
            monitor.valid_entries(2),
            Some(vec![true, false]),
            "set bitmap bits identify dirty entries"
        );
        assert!(
            written
                .entries()
                .iter()
                .all(|entry| !entry.flags.contains(entry::Flags::FSMONITOR_VALID)),
            "reading the index must not trust a monitor before querying it"
        );

        let rewritten = decode(&encode(&written)?)?;
        assert_eq!(
            rewritten
                .fs_monitor()
                .expect("the checkpoint remains available")
                .valid_entries(2),
            Some(vec![false, false]),
            "rewriting without querying the monitor conservatively invalidates its entries"
        );
    }
    Ok(())
}

#[test]
fn written_bitmap_tracks_current_entries_and_excludes_removed_entries() -> TestResult {
    let mut index = State::new(gix_testtools::object_hash());
    let valid = entry::Flags::FSMONITOR_VALID;
    push(&mut index, "g", valid, entry::Mode::SYMLINK);
    push(&mut index, "f", valid, entry::Mode::DIR);
    push(
        &mut index,
        "e",
        valid | entry::Flags::INTENT_TO_ADD | entry::Flags::EXTENDED,
        entry::Mode::FILE,
    );
    push(
        &mut index,
        "d",
        valid | entry::Flags::from_stage(entry::Stage::Ours),
        entry::Mode::FILE,
    );
    push(&mut index, "c", valid, entry::Mode::COMMIT);
    push(&mut index, "b", valid, entry::Mode::FILE);
    push(&mut index, "a", valid, entry::Mode::FILE);
    index.set_fs_monitor_token(Some(Token::V2 { token: "next".into() }));
    index.entries_mut()[5].flags.insert(entry::Flags::REMOVE);
    index.entries_mut()[6].flags.remove(valid);
    index.sort_entries();
    assert!(
        index.fs_monitor().is_none(),
        "entry mutations invalidate the previous bitmap"
    );
    index.set_fs_monitor_token(Some(Token::V2 { token: "next".into() }));

    let written = decode(&encode(&index)?)?;
    assert_eq!(written.entries().len(), 6, "removed entries have no bitmap position");
    assert_eq!(
        written
            .fs_monitor()
            .expect("the checkpoint was written")
            .valid_entries(6),
        Some(vec![false, false, false, false, false, true]),
        "current flags and ordering win; gitlinks, conflicts, intent-to-add and sparse directories stay dirty"
    );
    Ok(())
}

#[test]
fn mutable_entry_and_path_access_invalidates_the_checkpoint() -> TestResult {
    let cache_index = gix_index::File::at(
        crate::fixture_index_path_needs_archive("untracked_cache_populated"),
        gix_testtools::object_hash(),
        false,
        Default::default(),
    )?;
    let cache = cache_index.untracked().expect("the fixture has an untracked cache");
    let mut index = State::new(gix_testtools::object_hash());
    push(&mut index, "a", entry::Flags::FSMONITOR_VALID, entry::Mode::FILE);
    let operations: &[fn(&mut State)] = &[
        |index| {
            index.entries_mut();
        },
        |index| {
            index.entries_mut_and_pathbacking();
        },
        |index| {
            let _ = index.entries_mut_with_paths();
        },
        |index| {
            let paths = index.path_backing().to_owned();
            let _ = index.entries_mut_with_paths_in(&paths);
        },
        |index| {
            index.entry_mut_by_path_and_stage("a".into(), entry::Stage::Unconflicted);
        },
        |index| {
            push(index, "b", entry::Flags::empty(), entry::Mode::FILE);
        },
        |index| {
            index.sort_entries();
        },
        |index| {
            index.sort_entries_by(|_, _| std::cmp::Ordering::Equal);
        },
        |index| {
            index.remove_entries(|_, _, _| false);
        },
        |index| {
            index.remove_entry_at_index(0);
        },
        |index| {
            index.take_path_backing();
        },
        |index| {
            let paths = index.take_path_backing();
            index.set_fs_monitor_token(Some(Token::V1 { nanos_since_1970: 1 }));
            index.return_path_backing(paths);
        },
    ];
    for operation in operations {
        let mut index = index.clone();
        index.set_fs_monitor_token(Some(Token::V1 { nanos_since_1970: 1 }));
        index.set_untracked(Some(cache.clone()));
        operation(&mut index);
        assert!(
            index.fs_monitor().is_none(),
            "mutable access cannot retain a position-based bitmap"
        );
        assert!(
            index.untracked().is_none(),
            "mutable access invalidates cached index membership"
        );
    }
    Ok(())
}

#[test]
fn bitmap_bounds_and_malformed_storage_are_not_trusted() -> TestResult {
    let mut index = State::new(gix_testtools::object_hash());
    index.set_fs_monitor_token(Some(Token::V1 { nanos_since_1970: 1 }));
    assert_eq!(
        index.fs_monitor().expect("checkpoint is present").valid_entries(3),
        Some(vec![true; 3]),
        "entries beyond a valid bitmap are implicitly clean"
    );
    let mut bytes = encode(&index)?;
    let extension = bytes
        .windows(4)
        .position(|window| window == b"FSMN")
        .expect("extension was written");
    let bitmap = extension + 8 + 4 + 8 + 4;
    bytes[bitmap..bitmap + 4].copy_from_slice(&1u32.to_be_bytes());
    let decoded = decode(&bytes)?;
    let monitor = decoded
        .fs_monitor()
        .expect("bitmap shape is validated when obtaining entry validity");
    assert!(
        monitor.valid_entries(0).is_none(),
        "bit count cannot exceed index entry count"
    );
    assert!(
        monitor.valid_entries(1).is_none(),
        "missing declared bits cannot be assumed clean"
    );
    Ok(())
}

#[test]
fn tokens_can_be_removed_and_nul_tokens_cannot_be_written() -> TestResult {
    let mut index = State::new(gix_testtools::object_hash());
    index.set_fs_monitor_token(Some(Token::V2 {
        token: b"invalid\0token".into(),
    }));
    assert!(encode(&index).is_err(), "NUL bytes would corrupt the token boundary");
    index.set_fs_monitor_token(None);
    assert!(
        decode(&encode(&index)?)?.fs_monitor().is_none(),
        "disabled monitors leave no extension"
    );
    Ok(())
}

fn push(index: &mut State, path: &str, flags: entry::Flags, mode: entry::Mode) {
    index.dangerously_push_entry(
        Default::default(),
        gix_testtools::object_hash().null(),
        flags,
        mode,
        path.into(),
    );
}

fn encode(index: &State) -> TestResult<Vec<u8>> {
    let mut bytes = Vec::new();
    gix_index::File::from_state(index.clone(), std::path::PathBuf::new()).write_to(&mut bytes, Default::default())?;
    Ok(bytes)
}

fn decode(bytes: &[u8]) -> TestResult<State> {
    Ok(State::from_bytes(bytes, FileTime::now(), gix_testtools::object_hash(), Default::default())?.0)
}
