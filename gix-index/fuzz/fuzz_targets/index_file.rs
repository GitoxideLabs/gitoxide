#![no_main]

use filetime::FileTime;
use gix_index::{decode, write, File, State};
use libfuzzer_sys::fuzz_target;
use std::{hint::black_box, path::PathBuf};

fn fuzz(input: &[u8]) {
    let options = decode::Options {
        alloc_limit_bytes: Some(8 * 1024 * 1024),
        ..Default::default()
    };

    let Ok((state, checksum)) = State::from_bytes(
        input,
        FileTime::from_unix_time(0, 0),
        gix_index::hash::Kind::Sha1,
        options,
    ) else {
        return;
    };

    _ = black_box(checksum);
    _ = black_box(state.version());
    _ = black_box(state.timestamp());
    _ = black_box(state.object_hash());
    _ = black_box(state.is_sparse());
    _ = black_box(state.path_backing());
    _ = black_box(state.had_end_of_index_marker());
    _ = black_box(state.had_offset_table());
    _ = black_box(state.tree());
    _ = black_box(state.link());
    _ = black_box(state.resolve_undo());
    _ = black_box(state.untracked());
    _ = black_box(state.fs_monitor());
    _ = black_box(state.verify_entries());
    _ = black_box(state.verify_extensions(false, gix_object::find::Never));

    for entry in state.entries().iter().take(8) {
        let path = entry.path(&state);
        _ = black_box(path);
        _ = black_box(entry.id);
        _ = black_box(entry.flags);
        _ = black_box(entry.mode);
        _ = black_box(entry.stage());
        _ = black_box(entry.stage_raw());
        _ = black_box(state.entry_range(path));
        _ = black_box(state.entry_index_by_path_and_stage(path, entry.stage()));
    }

    if state.link().is_none() {
        let file = File::from_state(state.clone(), PathBuf::from("fuzz-input.index"));
        let mut out = Vec::new();
        if let Ok((_version, digest)) = file.write_to(&mut out, write::Options::default()) {
            _ = black_box(State::from_bytes(
                &out,
                FileTime::from_unix_time(0, 0),
                gix_index::hash::Kind::Sha1,
                decode::Options {
                    alloc_limit_bytes: Some(8 * 1024 * 1024),
                    expected_checksum: Some(digest),
                    ..Default::default()
                },
            ));
        }
    }
}

fuzz_target!(|input: &[u8]| {
    fuzz(input);
});
