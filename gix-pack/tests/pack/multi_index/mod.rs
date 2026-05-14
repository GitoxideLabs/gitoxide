use std::path::PathBuf;

use gix_pack::multi_index::File;

fn multi_index(object_hash: gix_hash::Kind) -> (File, PathBuf) {
    let path = crate::scripted_fixture_read_only("make_pack_gen_repo_multi_index.sh")
        .expect("test fixture exists")
        .join(".git/objects/pack/multi-pack-index");
    let file = gix_pack::multi_index::File::at(&path, None).expect("multi-index fixture opens");
    assert_eq!(file.object_hash(), object_hash);
    (file, path)
}

mod access;
mod fuzzed;

mod verify;

mod write;
